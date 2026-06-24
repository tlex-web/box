# Pitfalls Research

**Domain:** Adding a scriptable `--json`/`--clip` spine + per-command depth + PS7 completions + config-file defaults to an existing Windows-PowerShell-7 Rust CLI (`box` v2.0 — Toolbox → Toolkit)
**Researched:** 2026-06-24
**Confidence:** HIGH (Windows/PS7/serde_json behaviors verified against official Microsoft docs, Rust std docs, and the serde-rs/json issue tracker; mapped onto v1's already-proven discipline in RETROSPECTIVE.md / STATE.md)

> **Scope note:** these are integration pitfalls specific to *adding v2's features to this codebase*, not generic Rust advice (v1's generic Windows pitfalls — UNC paths, MAX_PATH, junction loops, reserved filenames, install/PATH/SmartScreen — are archived in `PITFALLS-v1.0.md` and remain in force). Every prevention here ties back to a v1 pattern the project already owns: the `is_color_on()`-gated pure-ANSI walker, the pure I/O-free pre-flight + dry-run-default + `--force` + snapshot-the-tree tests, the RAII `RawGuard` terminal restore, strict 0/1/2 exit codes, and the binary-only-crate test invocation (`cargo test --bin box`). The two highest-priority families are **`--json` STDOUT hygiene** and **destructive `--delete`/`--move`/`--backup` safety**.

---

## Critical Pitfalls

### Pitfall 1: `--json` STDOUT contamination — any stray byte on stdout breaks `ConvertFrom-Json`

**What goes wrong:**
`box <cmd> --json | ConvertFrom-Json` fails (or silently parses the wrong thing) because stdout carries something other than the single JSON document: a progress bar, a spinner, a log line, a human-readable header/footer banner, an ANSI color escape, a "N directories, M files" summary, a `--verify` "OK" line, or a stray `println!` debug. PS7's `ConvertFrom-Json` consumes the *entire* stdout stream as one payload, so one extra line is a hard parse error or a corrupted object.

**Why it happens:**
v1's commands were built for a human reader, so they freely interleave chrome (summaries, colored accents, the new progress bars) with data on stdout. `--json` is bolted on *over* that existing print path. The trap is treating `--json` as "also print JSON" instead of "print JSON and nothing else."

**How to avoid — STATE THE RULE:**
> **Under `--json`, stdout MUST contain exactly one thing: the JSON document, UTF-8, no BOM, terminated by a single `\n`. Every other byte the command would normally emit — progress bars, spinners, color, summaries, banners, human labels, prompts — goes to STDERR or is suppressed entirely.**

Concretely:
- **Progress/spinners (indicatif) render to STDERR, always** — construct with `ProgressBar::new(n).with_draw_target(ProgressDrawTarget::stderr())`, or `ProgressBar::hidden()` under `--json`. indicatif's default target is stderr, but the project must make this *explicit and asserted*, because hash/dupes/flatten progress is new in v2 and stdout-bound progress is the single most common `--json` corrupter.
- **Suppress human chrome under `--json`** — gate the summary line, the `--verify` "OK"/"FAIL" text, colored accents, and any header behind `if !json`. Don't merely strip color; suppress the whole human framing.
- **One stdout writer for the JSON, one stderr writer for everything else** — make the data/chrome split structural, not per-`println!` discipline.
- **Reuse the v1 piped-equivalence test as a `--json` purity test:** assert `box <cmd> --json` stdout `serde_json::from_slice::<Value>()` round-trips *and* that stdout contains no ESC byte (`0x1B`) and no leading BOM (`EF BB BF`). This is the v1 `_piped_no_ansi` pattern extended.

**Warning signs:**
`ConvertFrom-Json: Conversion from JSON failed with error: Unexpected character`; a JSON object that has the right shape but an extra trailing summary line; color escapes visible in `$result` after parsing; progress bar fragments appearing in a redirected `> out.json` file.

**Phase to address:**
The cross-cutting `--json` spine phase, built FIRST (before per-command depth) so every later command inherits the discipline — exactly as `is_color_on()` was foundational in v1.

---

### Pitfall 2: `--json` correctness — encoding/BOM, number precision, NaN/Infinity, non-UTF-8 Windows paths, and the object-vs-array-vs-NDJSON shape contract

**What goes wrong:**
`--json` emits *syntactically* valid-looking output that still breaks consumers:
- **BOM / encoding:** a UTF-8 BOM (`EF BB BF`) prepended to stdout. PS7 defaults to `utf8NoBOM`, and a leading BOM MAY be treated as an error by JSON parsers — `ConvertFrom-Json` can choke on it. Any output path that injects a BOM corrupts the document.
- **Large-integer precision:** epoch nanos, file sizes (`u64` bytes), and dupe wasted-space totals can exceed 2^53−1 (9,007,199,254,740,991). serde_json serializes `u64`/`i64` *exactly*, but a JS consumer or any double-based parser silently rounds (`9007199254740993` → `…992`). PS7's `ConvertFrom-Json` is better (parses big integers to `Int64`/`BigInteger`), but emitting bare 64-bit integers is still a cross-consumer hazard the spine must decide on deliberately.
- **NaN / Infinity:** any float field (du percentages, weather temps, color HSL, passgen entropy bits) that computes to `NaN` or `±Infinity` is **not valid JSON** — serde_json errors on serialize, or if coerced produces `NaN`/`Infinity` literals no strict parser accepts.
- **Non-UTF-8 Windows paths:** NTFS filenames are UTF-16 and can contain unpaired surrogates that are **not** valid UTF-8. flatten/tree/du/dupes/hash all emit paths. `to_string_lossy()` silently substitutes U+FFFD (the emitted path no longer round-trips to the real file — data corruption for a machine consumer); `to_str().unwrap()` panics. Both are wrong for a `--json` field a consumer feeds back to a filesystem call.
- **Shape contract drift:** a command emits a bare object for one file but a top-level array for many, or NDJSON (one object per line), and the consumer's `ConvertFrom-Json` either errors or only sees the first record. Inconsistency across commands (`hash` → object, `dupes` → array, `tree` → NDJSON) makes the spine unusable.

**Why it happens:**
JSON "looks done" when it parses on the happy path with ASCII data. The failure modes only surface with real Windows paths, real large files, and real cross-language consumers — none of which appear in a quick smoke test.

**How to avoid:**
- **No BOM, ever:** write JSON via `serde_json::to_writer(stdout_lock, &value)` then a single `\n`. Add a test asserting stdout's first 3 bytes are not `EF BB BF`.
- **Decide a number policy per field and document it for the whole spine:** for values that can exceed 2^53 and that consumers might treat as doubles (sizes, epoch nanos), either emit as a JSON **string** (`"size_bytes": "10995116277760"`) or commit to PS7-first with a documented "JS consumers must string-encode" caveat. Pick ONE rule. **Keep `arbitrary_precision` OFF** (v1 04-01 D-04 already locked this — it breaks `Value` round-tripping; serde-rs/json #505/#721/#845).
- **Guard floats:** never serialize a raw `f64` that could be `NaN`/`Inf`. Format to a fixed-precision string or emit `null` for undefined (e.g. du percentage of an empty tree). Add a divide-by-zero/empty-input unit test for every float field.
- **Windows paths in JSON:** standardize on `to_string_lossy()` with an explicit, tested, documented decision (acceptable for display), and for a path field meant to be machine-fed-back, either mark lossy paths (`"path_lossy": true`) or refuse + exit 1 on a non-Unicode path under `--json`. Never `to_str().unwrap()` in a `--json` path (no panics).
- **One shape rule for the spine** — recommended: **a single top-level JSON value per invocation** (object for single-result commands; array for list commands like dupes/tree/du). Avoid NDJSON unless a command is explicitly a stream; if used, document it and don't feed it to plain `ConvertFrom-Json`. Lock each command's shape with a snapshot test.

**Warning signs:**
A non-ASCII/surrogate path renders as `?`/`�`; a 5 TB file's size is off by a few bytes after a JS round-trip; `ConvertFrom-Json` errors only on certain files; a consumer script that works on one file breaks on a directory.

**Phase to address:**
The `--json` spine phase (define number/path/shape/BOM policy once), enforced per-command in each depth phase.

---

### Pitfall 3: Config precedence bug — a config file silently overriding an explicitly-passed CLI flag

**What goes wrong:**
The classic config-layering bug: the user runs `box hash --algo sha256 file`, the config has `algo = "blake3"`, and config wins — the explicit flag is silently ignored. Precedence MUST be **CLI flag (explicit) > environment variable > config file > built-in default**, and an *explicitly passed* flag must **always** win.

**Why it happens:**
clap's derive API fills every field with its `default_value` when the user omits the flag, so by the time `run()` sees the struct there is **no way to distinguish "user passed `--algo blake3`" from "user omitted `--algo`, so it defaulted to blake3."** A naive `config.algo.unwrap_or(args.algo)` merge then can't tell "config unset" from "flag defaulted," and picks the wrong winner. This is the single most common config-precedence defect.

**How to avoid:**
- **Make config-overridable CLI fields `Option<T>` with NO clap `default_value`.** `None` = "user did not pass it"; `Some(v)` = "user explicitly passed it." Resolve: `let algo = args.algo.or(env_algo).or(config.algo).unwrap_or(DEFAULT_ALGO);` — explicit flag wins by construction. (Alternative: clap `ArgMatches::value_source` to detect `ValueSource::CommandLine` vs `DefaultValue`; `Option<T>`-no-default is simpler and unit-testable.)
- **Unit-test the precedence matrix** as a pure function `resolve(flag, env, config, default) -> value` — the v1 "pure I/O-free pre-flight" pattern applied to config. Test all 16 present/absent combinations across the four layers.
- **A missing or malformed config file MUST fall back to defaults, never hard-error the CLI.** `box uuid` with a corrupt config must still print a UUID. Tolerant loader: file-not-found → silent defaults; parse error → one warning to **stderr** + continue with defaults (never exit non-zero just because config is bad). Only `box config` (the meta-command that edits config) may surface a hard error.

**Warning signs:**
A flag the user passed "does nothing"; behavior changes based on whether a config file exists; no test where flag and config disagree; `box <cmd>` fails entirely after a hand-edited config typo.

**Phase to address:**
The config-file-defaults phase (`config` meta-command). The precedence resolver is the first thing built and the most-tested unit there.

---

### Pitfall 4: Destructive flags (`dupes --delete`, `flatten --move`, `bulk-rename --backup`) bypassing v1's safety discipline

**What goes wrong:**
v2 adds **new data-loss surface** to commands that were read-only or copy-only in v1:
- `dupes` was **strictly read-only** in v1 (the `dupes_never_writes` snapshot test enforced it). `--delete` makes it delete files for the first time.
- `flatten` *copied* (originals untouched) in v1. `--move` deletes the source.
- `bulk-rename` had abort-all-before-any + dry-run-default; `--backup` adds a new write path (the backup copies themselves, which can collide).

The pitfall is implementing these as a naive loop that acts file-by-file, so a failure/collision **partway through** leaves the tree half-mutated — the exact silent-data-loss class v1 spent two code-review BLOCKERs eliminating (bulk-rename `..` escape, flatten `create_new` clobber).

**Why it happens:**
The new flag is treated as "just add a `fs::remove_file`/`fs::rename` in the existing print loop," skipping the pure pre-flight + dry-run-default + snapshot-test ritual because the surrounding command "already works."

**How to avoid — inherit v1's exact discipline:**
- **Dry-run is the DEFAULT; `--force` (or a confirm) executes only after a clean pure pre-flight.** This is the bulk-rename 03-05 / D-19 pattern verbatim: plan → preview → execute.
- **Abort-all-before-any:** compute the full plan as a pure I/O-free `preflight(...) -> Vec<Conflict>` *before* touching the filesystem. Any conflict (target exists, would delete a hardlink-shared inode, would empty a dupe group, path escapes the root) refuses the **entire** operation and exits 2 — never partially apply. Reuse the `Conflict` enum shape from bulk-rename.
- **`flatten --move` must confirm the destination write before deleting the source.** Copy → verify (dest exists + size matches) → only then remove source. Never bare `fs::rename` across volumes (fails across drives on Windows; a naive fallback can delete-after-failed-copy). Open dest with `create_new` (the v1 hardening); a failure at any step leaves the source intact.
- **`dupes --delete` must keep at least one copy of every group** and show what will be deleted first (dry-run default). Deleting a whole group = data loss.
- **Every abort path gets a snapshot-the-tree test** asserting the directory is byte-for-byte unchanged after a refused/dry-run op — the v1 pattern that "made the silent-overwrite class of bugs catchable."
- **Per-flag adversarial code review** (not just verification) — v1's review caught the two path-escape/clobber BLOCKERs verification missed.

**Warning signs:**
A destructive command with no dry-run default; a `fs::remove_file`/`fs::rename` inside a `for` loop with no prior full-plan computation; no snapshot test for the abort path; `--move` as bare `fs::rename`; a dupes delete that can empty a group.

**Phase to address:**
The filesystem-depth phase(s) adding `--delete`/`--move`/`--backup` (flatten/dupes/bulk-rename). Each destructive flag is its own plan with its own snapshot test + code review, sequenced on the shared registry as in v1.

---

### Pitfall 5: `dupes --delete` hardlink false-positive — two paths sharing one inode are NOT wasteful duplicates

**What goes wrong:**
`dupes` groups by identical content (size pre-filter → BLAKE3). Two paths that are **hardlinks to the same underlying file** have identical content, so they land in the same group — but they occupy storage **once**, not twice. Reporting them as "wasted space" is wrong, and **`--delete`-ing one frees nothing while destroying a legitimate second name for the data** (and can break whatever relies on that path). On NTFS this is real: hardlinks are supported and common (dedup tools, package managers).

**Why it happens:**
Content-equality is necessary but not sufficient for "wasteful duplicate." v1's dupes was read-only so a false grouping was cosmetic; with `--delete`, a hardlink false-positive becomes data/structure loss.

**How to avoid:**
- **Detect hardlinks via NTFS file identity, not content alone.** Use `std::os::windows::fs::MetadataExt`: two paths are the *same physical file* iff they share `file_index()` **and** the same volume serial. `number_of_links() > 1` flags a file that has other hardlinks at all (`BY_HANDLE_FILE_INFORMATION.nNumberOfLinks`).
- **Confirmed std gotcha:** `file_index()` and `number_of_links()` return `None` when metadata came from `DirEntry::metadata()`. You **must** obtain metadata via `fs::metadata(path)` / `File::metadata()` (a handle-based call) for these fields to populate. A walkdir `DirEntry` won't give you the inode — re-`stat` the path.
- **In a dupe group, collapse paths sharing `(volume_serial, file_index)` into one logical entry** before computing wasted space; make `--delete` hardlink-aware by skipping (and reporting) any group member that shares an inode with a kept member.
- **Snapshot test** with a real hardlink fixture (`std::fs::hard_link`) asserting `--delete` does not reduce the link count to zero and does not report shared-inode space as wasted.

**Warning signs:**
"Wasted space" that doesn't match `du`; deleting a "duplicate" frees no disk; a group of two identical paths where one is a known hardlink; metadata that always shows `number_of_links == None` (you're reading from `DirEntry` — the inode is invisible).

**Phase to address:**
The dupes-depth phase (multi-stage hashing + `--delete` + hardlink-aware). The hardlink-identity check is a success criterion, not an afterthought.

---

### Pitfall 6: BLAKE3-default breaking change silently breaking existing `hash`/`--verify` workflows

**What goes wrong:**
v1 `hash` defaulted to **SHA-256** (the HASH-01 binding contract; `sha256sum`/Docker interop). v2 flips the default to **BLAKE3** (HASH-V2-01 — the major-version trigger). Every script that ran `box hash file` expecting a SHA-256 hex, every stored `box hash file > sums.txt` baseline, and every `box hash --verify <sha256hex> file` (where the user omitted `--algo` assuming SHA-256) **silently produces or expects the wrong algorithm.** `--verify` is especially dangerous: a 64-hex BLAKE3 and a 64-hex SHA-256 are length-indistinguishable, so v1's length-autodetect (which mapped 64→sha256) must now map 64→blake3, and old SHA-256 baselines fail with a confusing mismatch rather than a clear "algorithm changed."

**Why it happens:**
A default-value change is invisible at the call site — the command line is byte-identical, only the output changed. Users don't read changelogs for a utility they've scripted.

**How to avoid:**
- **Communicate loudly:** state in `box hash --help`, the README, and the v2 changelog that the default is now BLAKE3 and SHA-256 interop requires `--algo sha256`. This is the project's documented breaking-change obligation.
- **Preserve the explicit override** (`--algo sha256|sha512|md5|blake3`, from v1 03-01) so every old workflow works with a one-flag edit.
- **Make the default configurable** via the new config file (`hash.algo = "sha256"`) so a SHA-256-dependent user restores v1 behavior globally without editing every script — and so the precedence resolver (Pitfall 3) is exercised by a real, high-value case.
- **Flip the `--verify` length-autodetect tie deliberately:** 64-hex now auto-detects BLAKE3; `--algo sha256 --verify <64hex>` is the only way to verify a SHA-256. Carry forward v1 WR-01 (autodetect fires ONLY when `--algo` is unset) so an explicit `--algo` is never overridden by length inference. Test `--algo sha256 --verify` of a known SHA-256 baseline.
- **Consider a transitional hint on mismatch:** when `--verify` fails on a 64-hex value, hint "note: the default algorithm is now BLAKE3; pass --algo sha256 if this is a SHA-256 checksum."

**Warning signs:**
A `--verify` that used to pass now fails after upgrade; CI checksums drift; "hash output changed" with no command change; a 64-hex verify that mismatches with no explanation.

**Phase to address:**
The hash-depth phase (BLAKE3-default + multi-file + progress). The breaking-change docs + config default + `--verify` autodetect-flip are success criteria. The config-default piece depends on the config phase landing first (ordering dependency — see mapping).

---

### Pitfall 7: ANSI color leaking into `--json`/piped output (the `is_color_on()` gate must extend to every new colored feature)

**What goes wrong:**
v2 adds new colored outputs: `color` HSL/CSS swatch, `8ball` sentiment coloring, `du` color ranges / percentage bars, `tree` sort-by-size coloring, `lolcat --animate`. If any emits ANSI without passing the `is_color_on()` gate, it corrupts `--json` (Pitfall 1) AND breaks the v1 invariant that piped output is byte-identical-minus-color.

**Why it happens:**
New color paths get added directly with `.truecolor(...)`/owo-colors trait methods, forgetting the v1 rule that *all* styling is reached only **after** the `is_color_on()` gate (and under `--json`, suppressed entirely — color-off is necessary but not sufficient; the human chrome must also be gone).

**How to avoid:**
- **Route every new colored token through the v1 `is_color_on()`-gated pure-walker pattern** (04-01 json colorize, 04-02 lolcat `rgb_at`): the walker always emits ANSI and is reached only when the caller already checked the gate, so the no-color path is byte-identical minus escapes. Do NOT use `owo_colors::set_override` (v1 01-02 decision: gate on our own `COLOR_ON` AtomicBool).
- **`--json` forces color off AND chrome off:** `is_color_on()` must return false whenever `--json` is set (and when stdout is not a TTY). Treat `--json` as a stronger gate than piped-detection.
- **Add the per-command `_piped_no_ansi` test** for every newly-colored command, scanning stdout for `0x1B`.

**Warning signs:**
ANSI escapes in a redirected file; `du --json` with color codes inside string values; a new colored command with no `_piped_no_ansi` test; a percentage bar that renders as garbage when piped.

**Phase to address:**
Every phase that adds a colored feature (color/8ball/du depth, tree depth, lolcat animate). The gate is inherited infra; the per-command test is the success criterion.

---

### Pitfall 8: Terminal-loop commands (`lolcat --animate`, `matrix` color/speed) breaking RAII-restore + single-flush-per-frame, or animating when piped

**What goes wrong:**
v2 adds `lolcat --animate` (new — lolcat was a one-shot recolor in v1) and extends `matrix` with color/speed/charset. The pitfalls:
- **Per-character flush instead of per-frame** → ~5 FPS stutter (the documented STATE.md matrix pitfall). New animate loops must buffer the whole frame with `queue!` and `flush()` **exactly once per frame**.
- **Terminal left in raw mode / alternate screen on exit** if the loop panics or an early-return skips cleanup. v1's RAII `RawGuard` (Drop = Show + LeaveAlternateScreen + disable_raw_mode) must be armed **immediately after** `enable_raw_mode()?` and **before** the fallible `EnterAlternateScreen` (the v1 04-04 CR-01 fix). `lolcat --animate` reuses this guard, never hand-rolls cleanup.
- **Animating when piped** — `box lolcat --animate | clip` or `> file.txt` must NOT enter raw mode / emit cursor moves; it falls back to the one-shot static render. An animation loop on a non-TTY produces garbage and may hang.
- **Ctrl+C/q/Esc not restoring** — in raw mode crossterm delivers Ctrl+C as a `KeyEvent` (NOT SIGINT), needing a `KeyEventKind::Press`-only filter to avoid the Windows Press+Release double-fire (v1 04-04 / T-04M-02).

**Why it happens:**
`--animate` looks like "loop the existing recolor with a sleep," skipping the raw-mode state machine and TTY-detection that `matrix`/`pomodoro` already established.

**How to avoid:**
- **Reuse the `RawGuard` RAII type** from matrix/pomodoro verbatim for `lolcat --animate`; arm it immediately after entering raw mode.
- **Detect TTY first:** non-terminal stdout → `--animate` degrades to the static one-shot render (and `--json` disables it). Never enter raw mode on a non-TTY.
- **Single flush per frame;** `event::poll(Duration)` doubles as the frame timer and keypress read (v1 04-04 D-09), single-threaded, no background thread.
- **`KeyEventKind::Press`-only** quit filter on Ctrl+C/q/Esc; keep the loop panic-free so the Drop guard is the real restore path under `panic = "abort"`.
- **Human-verify gate in PS7** for any new animation (v1 required this for matrix).

**Warning signs:**
Cursor invisible / terminal stuck after Ctrl+C; ~5 FPS stutter; garbage when `--animate` is piped; a second `RawGuard`-less cleanup path; `q` needing two presses (Release double-fire).

**Phase to address:**
The visuals-depth phase (lolcat animate, matrix color/speed/charset). RawGuard + TTY-gate + single-flush are success criteria; each gets a non-hanging smoke test + human-verify.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Add `--json` by `println!`-ing JSON alongside existing human output | Fast to wire up | Stdout contamination (Pitfall 1) breaks every consumer; impossible to un-pick later | **Never** — the data/chrome split must be structural from day one |
| Per-command ad-hoc JSON shape (object here, array there, NDJSON elsewhere) | Each command ships independently | Spine is unusable; consumers special-case every command | Never — define the shape policy once in the spine phase |
| `to_string_lossy()` on paths in `--json` with no marker/test | Compiles, works for ASCII | Silent path corruption on non-UTF-8 NTFS names; un-round-trippable | Only with an explicit, tested, documented decision (Pitfall 2) |
| `config.value.unwrap_or(args.value)` merge | One line | Inverts precedence — config beats explicit flag (Pitfall 3) | Never — use `Option`-no-default + `.or()` chain |
| Hard-error on malformed config | "Fail fast" | A config typo bricks every command including `box uuid` | Never for normal commands; only `box config` may hard-error |
| `dupes --delete` by content-equality alone | Simpler | Deletes hardlink aliases / empties groups → data loss (Pitfall 5) | Never — must be hardlink-aware + keep-one |
| `flatten --move` as bare `fs::rename` | One call | Fails cross-volume; naive fallback deletes source after failed copy | Never — copy→verify→delete, `create_new` dest |
| `lolcat --animate` as "loop recolor + sleep" | Quick | No RawGuard → stuck terminal; animates when piped | Never — reuse matrix RawGuard + TTY gate |
| Bare 64-bit integers in JSON, ignore JS consumers | Natural serde output | Silent precision loss for sizes/epochs in JS/double parsers | Acceptable only if documented PS7-first with the caveat stated |
| Skip the `_piped_no_ansi` / `--json` purity test on a new colored command | Less test code | Color leak ships undetected; corrupts pipelines | Never — the test is the v1 standard |

## Integration Gotchas

Common mistakes when connecting to external services / OS APIs.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| indicatif progress bars | Default-render to stdout, contaminating `--json` and redirects | Explicit `ProgressDrawTarget::stderr()`; `ProgressBar::hidden()` under `--json`; assert no progress bytes on stdout |
| arboard clipboard (`--clip`) | Calling from a worker thread; assuming `set_text` succeeds for huge/non-text payloads | Main-thread only (v1 STATE.md rule); validate UTF-8 before `set_text`; bound payload size and error cleanly (exit 1) rather than panicking; image/non-text clipboard is out of `--clip`'s text contract |
| `std::os::windows::fs::MetadataExt` (hardlinks for dupes) | Reading `number_of_links()`/`file_index()` from `DirEntry::metadata()` → always `None` | Call `fs::metadata(path)`/`File::metadata()` (handle-based) so the inode/link-count fields populate; compare `(volume_serial, file_index)` for same-file identity |
| `du` apparent vs on-disk size (sparse/compressed NTFS) | Using `metadata().len()` (logical size) and calling it "on disk" | For on-disk size call `GetCompressedFileSize` (via the `windows` crate) — sparse/NTFS-compressed files occupy less than logical; `len()` over-reports. Make apparent-size the documented default, on-disk an explicit opt-in; never silently mix |
| `clap_complete` PowerShell completions | Printing the script and assuming it "just works" / persists across sessions | Generated script uses `Register-ArgumentCompleter -Native`; it only lasts the session unless dot-sourced from `$PROFILE`. `box completions powershell` prints the script to stdout (clean, no chrome) with docs telling the user to add it to `$PROFILE` (or write to a file + source). Document the PS 5.0+ requirement |
| `pomodoro` completion sound | Pulling a heavy audio crate (rodio/cpal + symphonia) for one beep | Zero-dep Windows beep: `MessageBeep`/`Beep` via the `windows` crate, or the console `\a` bell, gated behind `--sound`. Don't drag an audio stack into a 5 MB CLI |
| Open-Meteo `weather --forecast`/cache | Caching to a path that doesn't exist; never invalidating; non-2xx treated as success | Cache under `%LOCALAPPDATA%` (create dir if missing); TTL-stamp the cache; reuse v1's `non-2xx = Err(StatusCode)` match-arm split (05-04 WTHR-1), not a post-success status check |
| Config file location/encoding (Windows) | Hard-coding a path; assuming LF; assuming write permission | Use `%APPDATA%\box\config.toml` (create dir on first write); tolerate CRLF (parser handles both); a read-only or missing config degrades to defaults, not error |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Per-character terminal flush in `lolcat --animate`/`matrix` | Visible stutter, ~5 FPS, high CPU | Buffer whole frame with `queue!`, `flush()` once per frame (v1 D-08) | Any animation; immediately on a full-width terminal |
| `dupes` hashing every file instead of size-pre-filtering | Long scans, full-disk reads | Keep v1's size-bucket pre-filter; only same-size buckets ≥2 get hashed; rayon-parallel the hash phase | Directories with thousands of files / large media |
| Buffering whole files to hash instead of streaming | Memory blowup on multi-GB files | Keep v1's 64 KiB streaming (RustCrypto) / native `update_reader` (blake3); never `read_to_end` | Multi-GB files; `hash` on disk images / VMs |
| Building the entire `--json` array in memory for a huge tree | RAM spike, slow first byte | Acceptable for `box` scale (one-shot CLI, human-sized trees); document the single-value shape; revisit only if a command targets million-file trees | Not expected at this project's scale — don't over-engineer |
| `du` re-`stat`-ing every file for on-disk size on a deep tree | Slower than logical-size scan | Only call `GetCompressedFileSize` when on-disk mode is requested; logical `len()` is the fast default | Large trees under the opt-in on-disk flag |

## Security Mistakes

Domain-specific issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Path-escape via crafted names in destructive flags (`--move`/`--delete`/`--backup`) | Operation escapes the target dir, deletes/overwrites outside scope (the v1 bulk-rename `..` BLOCKER class) | Reuse the pure pre-flight separator/`..`/`.` refusal (v1 CR-01); canonicalize with `dunce::canonicalize` (never `std::fs::canonicalize` — UNC); abort-all-before-any |
| Terminal-escape injection re-emitted by `lolcat --animate` | Piped input carrying ANSI/control escapes manipulates the terminal | Keep v1's unconditional `strip_ansi_escapes::strip_str` before recolor (04-02 / T-04L-01) on both color and no-color paths |
| Reading config from a world-writable / CWD / arbitrary path | Config injection alters command behavior (e.g. forces `hash.algo`) | Use the user-scoped `%APPDATA%\box` path; don't read config from CWD or an env-specified arbitrary path without intent; precedence still lets an explicit flag override (Pitfall 3) |
| `--clip` pushing sensitive output (passgen) to clipboard silently | Secret lingers in clipboard history / cloud clipboard sync | Document that `passgen --clip` puts a secret on the clipboard; don't auto-clip secrets — make it opt-in |
| Over-engineering: constant-time checksum compare | Wasted complexity | Keep v1's plain `eq_ignore_ascii_case` — a file checksum is PUBLIC, not a secret (03-01 / T-03-01) |

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `--json`/`--clip` flag names/behavior inconsistent across commands | Users can't predict the spine; muscle memory breaks | Cross-cutting flags defined once, identical semantics everywhere (the point of the spine phase) |
| Destructive flag with no preview | User runs `dupes --delete` and loses files they didn't mean to | Dry-run default + explicit `--force`/confirm; show exactly what will be deleted/moved first (v1 D-19) |
| BLAKE3-default change with no signposting | Scripts silently break; users confused why hashes "changed" | Loud `--help`/changelog note + `--algo sha256` escape hatch + config default (Pitfall 6) |
| Completions that silently don't persist | User installs completions, they vanish next session, looks broken | `box completions powershell` output + clear "add to `$PROFILE`" instructions; document session-only behavior of a one-off run |
| Progress bar that scrolls/duplicates when not a TTY | Redirected logs full of `\r` spam | Render progress to stderr with a TTY-aware draw target; hidden under `--json`/non-TTY |
| `weather --forecast` cache staleness with no indication | User sees yesterday's weather, thinks it's live | TTL-stamp cache; optionally show "cached Xm ago"; `--no-cache` escape hatch |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **`--json` output:** Often missing — verify stdout has NO ANSI (`0x1B`), NO BOM (`EF BB BF`), NO progress/summary chrome, exactly one JSON document + one `\n`; pipe through `ConvertFrom-Json` in a real PS7 test.
- [ ] **`--json` floats:** Often missing the NaN/Infinity/divide-by-zero case — verify every float field handles empty/zero input without emitting `NaN`.
- [ ] **`--json` large numbers:** Often missing the >2^53 case — verify a 5 TB size / nanosecond epoch survives the documented number policy.
- [ ] **`--json` non-UTF-8 paths:** Often missing — verify a path with a non-ASCII/surrogate name doesn't panic and is handled per policy.
- [ ] **Config precedence:** Often missing the "explicit flag beats config" test — verify `--algo sha256` wins over `algo = "blake3"` in config.
- [ ] **Config robustness:** Often missing — verify a missing AND a malformed config both fall back to defaults (CLI still works), not a hard error.
- [ ] **`dupes --delete` hardlinks:** Often missing — verify a hardlinked pair (`fs::hard_link` fixture) is NOT counted as wasted space and is NOT blindly deleted.
- [ ] **`flatten --move`:** Often missing the cross-volume + failed-write case — verify the source is NOT deleted if the destination write fails.
- [ ] **`bulk-rename --backup`:** Often missing — verify backup files don't themselves collide and an abort leaves the tree byte-for-byte unchanged.
- [ ] **BLAKE3 default:** Often missing — verify `--algo sha256 --verify <64-hex SHA-256>` still works after the autodetect flip.
- [ ] **`lolcat --animate` / `matrix`:** Often missing — verify Ctrl+C/q/Esc restore the terminal, single-flush-per-frame, and `--animate` does NOT animate when piped (`| cat` / `> file`).
- [ ] **`du` on-disk size:** Often missing — verify a sparse/NTFS-compressed file reports less than logical size under the on-disk flag.
- [ ] **PS7 completions:** Often missing — verify the generated script is dot-sourceable from `$PROFILE` and actually tab-completes `box <Tab>` in a fresh PS7 session (human-verify).
- [ ] **`--clip`:** Often missing — verify main-thread only, large payload doesn't panic, Unicode round-trips in PS7.

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| `--json` stdout contamination shipped | LOW | Move offending writes to stderr / behind `if !json`; add the no-ESC/no-BOM purity test that should have existed; re-verify with `ConvertFrom-Json` |
| Config precedence inverted | MEDIUM | Refactor flags to `Option`-no-default + `.or()` resolver; add the 16-case precedence matrix test; audit every command that reads config |
| `dupes --delete` deleted hardlink data | HIGH | May be unrecoverable if it was the last name of an inode — recover from backup. Prevention is the only real defense; ship hardlink-awareness before `--delete` is exposed |
| `flatten --move` deleted source on failed copy | HIGH | Recover from backup/recycle bin if present; fix to copy→verify→delete; add the failed-write snapshot test |
| BLAKE3-default broke a CI checksum baseline | LOW | Document the change; users add `--algo sha256` or set the config default; regenerate baselines if needed |
| Terminal stuck after `--animate` panic | LOW | RawGuard Drop should already restore; if not, the user runs `Reset`/reopens PS7. Fix: arm RawGuard before the fallible enter, keep loop panic-free |
| Non-UTF-8 path panicked a `--json` run | LOW | Replace `to_str().unwrap()` with the documented lossy/refuse policy; add the surrogate-path test |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls. (Phases continue numbering from v1's Phase 5; exact numbers assigned by the roadmapper — grouping shown by area, with the cross-cutting `--json`/config spine intended to land BEFORE the per-command depth phases that depend on it.)

| Pitfall | Prevention Phase / Area | Verification |
|---------|------------------------|--------------|
| 1. `--json` stdout contamination | Cross-cutting `--json` spine (built first) | No-ESC + no-BOM stdout scan; real `ConvertFrom-Json` round-trip per command |
| 2. `--json` correctness (BOM/number/NaN/path/shape) | `--json` spine (policy) + each depth phase (per-command) | Surrogate-path test, >2^53 size test, NaN-on-empty test, no-BOM first-bytes test, shape snapshot |
| 3. Config precedence (explicit flag wins) | Config-defaults phase (`config` meta-command) | 16-case precedence matrix unit test; explicit-flag-beats-config test; missing+malformed config falls back |
| 4. Destructive flags bypass v1 discipline | Filesystem-depth phases (flatten/dupes/bulk-rename) | Dry-run-default test; abort-all-before-any pre-flight test; snapshot-the-tree-unchanged test; per-flag code review |
| 5. dupes hardlink false-positive | Dupes-depth phase | `fs::hard_link` fixture: not-wasted-space + not-deleted; metadata from `fs::metadata` not `DirEntry` |
| 6. BLAKE3-default breaking change | Hash-depth phase (depends on config phase for the config default) | `--help`/changelog note present; `--algo sha256 --verify` of SHA-256 baseline passes; config `hash.algo` honored |
| 7. ANSI leak into `--json`/piped | Every colored-feature phase (color/8ball/du/tree/lolcat) | Per-command `_piped_no_ansi` test; `--json` forces color off |
| 8. Terminal-loop discipline (animate/matrix) | Visuals-depth phase | RawGuard restore on Ctrl+C/q/Esc (human-verify); single-flush-per-frame; no-animate-when-piped test |

## Sources

- Microsoft Learn — `about_Character_Encoding` (PowerShell 7): PS7 defaults to `utf8NoBOM`; BOM handling. https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding
- Microsoft Learn — `Register-ArgumentCompleter`: `-Native` registration, session-scoped unless added to `$PROFILE`. https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/register-argumentcompleter
- Microsoft Learn — `BY_HANDLE_FILE_INFORMATION` (fileapi.h): `nNumberOfLinks`, `nFileIndexHigh/Low`, volume serial → file identity. https://learn.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-by_handle_file_information
- Rust std docs — `std::os::windows::fs::MetadataExt`: `number_of_links()`/`file_index()` return `None` from `DirEntry::metadata`, populated via `fs::metadata`/`File::metadata`. https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
- serde-rs/json — `Number` docs + issues #505/#721/#845 (`arbitrary_precision` round-trip), #329 (64-bit-as-string interop): u64/i64 exact in serde, doubles lose precision >2^53. https://docs.rs/serde_json/latest/serde_json/value/struct.Number.html , https://github.com/serde-rs/json/issues/329
- JSON number precision (IEEE 754 / 2^53 / Number.MAX_SAFE_INTEGER): cross-consumer large-integer hazard. https://jsonic.io/guides/json-number-precision
- clap-rs/clap PR #732 / issue #729 — PowerShell completion via `Register-ArgumentCompleter -Native`, paste into profile or source from file. https://github.com/clap-rs/clap/pull/732/files
- Project v1 artifacts (primary): `.planning/RETROSPECTIVE.md` (snapshot-the-tree tests, pure pre-flight, verification≠review), `.planning/STATE.md` Accumulated Context / "Critical Pitfalls to Remember" (RawGuard, single-flush-per-frame, `is_color_on()` gate, arboard main-thread, `dunce::canonicalize`, BLAKE3-via-`--algo`, binary-only `cargo test --bin box`), `.planning/PROJECT.md` (v2.0 milestone scope + BLAKE3 breaking-change), `.planning/research/PITFALLS-v1.0.md` (archived v1 Windows pitfalls still in force).

---
*Pitfalls research for: scriptable-spine + per-command-depth additions to a Windows-PS7 Rust CLI (`box` v2.0)*
*Researched: 2026-06-24*
