---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Toolbox to Toolkit
status: ready_to_plan
stopped_at: Phase 09 complete (3/3) — ready to discuss Phase 10
last_updated: 2026-07-14T12:41:56.719Z
last_activity: 2026-06-28 -- Phase 09 execution started
progress:
  total_phases: 6
  completed_phases: 3
  total_plans: 14
  completed_plans: 36
  percent: 50
---

# Project State: box — Rust CLI Toolbox

**Last updated:** 2026-06-28
**Updated by:** execute-plan (08-06 complete — RENM-V2-02 `bulk-rename --backup`: a JSON undo MANIFEST [a zero-drift serde projection of the pre-flight-cleared `Plan`, one `{old,new,applied}` per renamed file with ABSOLUTE paths] written + `File::sync_all()`'d to `%LOCALAPPDATA%\box\undo\box-undo-<unix_millis>.json` [OUTSIDE the renamed tree, LOCALAPPDATA not APPDATA] BEFORE the first `std::fs::rename`, then each entry flips `applied:true` [rewrite+fsync] as its rename returns → a mid-batch I/O error leaves an `applied`-partitioned, reconcilable manifest; `--backup` is a no-op on dry-run, `--force`-only, path echoed to stderr, abort writes NEITHER manifest NOR rename; `--undo` replay Deferred; new tests/bulk_rename_backup.rs with manifest-written/dry-run-noop/abort-writes-nothing/partition-recoverable [real locked-target mid-batch] tests; the mandatory adversarial code-review gate was approved; D-38 logged [manifest not byte copies, D-22 applied]; full suite + clippy green. **Phase 8 implementation complete — all 10 reqs done; awaiting orchestrator phase verification + close-out.**)

---

## Project Reference

**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

**Current Focus:** Phase 10 — fun & system depth

**Milestone:** v2.0 Toolbox → Toolkit — EXECUTING. Phase 6 (scriptable-core foundation) complete. v1.0 Full Toolbox shipped & archived 2026-06-24 (all 23 commands; see `.planning/MILESTONES.md`).

See: .planning/PROJECT.md · .planning/ROADMAP.md · .planning/REQUIREMENTS.md (all current as of 2026-06-25)

---

## Current Position

Phase: 10
Plan: Not started
Status: Ready to plan
Last activity: 2026-07-14

Progress: [██████████] 100%

## Phase Map

v1.0 (Phases 1–5) complete & archived — see `.planning/milestones/v1.0-ROADMAP.md`. v2.0 phases:

| Phase | Name | Requirements | Status |
|-------|------|-------------|--------|
| 6 | Scriptable-Core Foundation | SPINE-01, SPINE-03, SPINE-05, HASH-V2-01 (4) | Complete (2/2 plans — all 4 reqs done) |
| 7 | Spine Rollout | SPINE-02, SPINE-04 (2) | Complete (3/3 plans — 07-01 Wave-7a + 07-02 Wave-7b + 07-03 Wave-7c done; --json on 16/16, --clip on 6/6 new; SPINE-02/SPINE-04 done) |
| 8 | Filesystem Depth | HASH-V2-02, FLAT-V2-01/02, DUPE-V2-01/02, RENM-V2-01/02, TREE-V2-01, DU-V2-01/02 (10) | Executing — all 6 plans done, awaiting phase verification (Wave 1: 08-01 HASH-V2-02 + FLAT-V2-01, 08-02 TREE-V2-01 + DU-V2-01/02, 08-03 DUPE-V2-01 + RENM-V2-01; Wave 2: 08-04 FLAT-V2-02 flatten --move + 08-05 DUPE-V2-02 dupes --delete + 08-06 RENM-V2-02 bulk-rename --backup [all 3 adversarial reviews approved]; all 10 reqs complete) |
| 9 | Dev-Transform & Visual Depth | UUID-V2-01, EPOC-V2-01, COLR-V2-01, JSON-V2-01, PASS-V2-01, LOL-V2-01, MTRX-V2-01, QR-V2-01, ASCI-V2-01 (9) | Not started |
| 10 | Fun & System Depth | COW-V2-01, FORT-V2-01, 8BAL-V2-01, ROST-V2-01, POMO-V2-01/02, WTHR-V2-01 (7) | Not started |
| 11 | Meta-Commands | CFG-01, CMP-01 (2) | Not started |

---

## Performance Metrics

**Plans executed (v2.0):** 11 / 18 planned
**v1.0 (archived):** 22 plans, 22 succeeded, 0 failed, 5/5 phases — see `.planning/MILESTONES.md`.

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 6 | 06-01 | ~10 min | 3 | 11 |
| 6 | 06-02 | ~35 min | 2 | 7 |
| 7 | 07-01 | 19 min | 3 | 17 |
| 7 | 07-02 | 12 min | 3 | 10 |
| 7 | 07-03 | 13 min | 3 | 7 |
| 8 | 08-01 | 15 min | 3 | 7 |
| 8 | 08-02 | ~30 min | 3 | 4 |
| 8 | 08-03 | ~30 min | 3 | 4 |
| 8 | 08-04 | ~10 min | 3 (2 code + 1 review gate) | 2 |
| 8 | 08-05 | ~25 min | 3 (2 code + 1 review gate) | 2 |
| 8 | 08-06 | ~20 min | 3 (2 code + 1 review gate) | 2 |

---

## Accumulated Context

### Key Decisions (v2.0 — locked at requirements/roadmap time)

| Decision | Rationale |
|----------|-----------|
| Order phases by integration risk: spine first, then rollout, then depth, then meta | v1 retrospective #1 lesson — the only architecture risk lives in the shared `--json`/`--clip`/config spine, so build it ONCE on the 2 cheapest commands (uuid+hash) before 21 adopt it (a flaw costs 2 commands of rework, not 23) |
| BLAKE3-default (HASH-V2-01) + config resolver (SPINE-05) co-located in Phase 6 (D-6) | So the `hash.default_algo = "sha256"` escape hatch exists the moment the breaking default flips |
| 27 depth reqs split into 3 area-grouped phases (8 fs / 9 dev+visual / 10 fun+system) | One 27-req phase is unworkable for plan/wave sizing; area grouping keeps each phase a sane unit. Meta-command phase renumbered to 11, stays strictly last (D-7) |
| 3 destructive depth flags each get own plan + adversarial review (08-04/05/06) | FLAT-V2-02 `--move`, DUPE-V2-02 `--delete`, RENM-V2-02 `--backup` — the v1 Phase-3 bulk-rename gate: dry-run default, `--force`, abort-all-before-any pre-flight, snapshot-tree-unchanged test per abort path |
| `completions` (CMP-01) in Phase 11, after all depth (D-7) | Generated from the live final `Cli` — must include every Phase-8/9/10 flag |
| Config: hand-roll `toml` 1.1.2 + `dirs` 6.0.0, `Option<T>` + `.or().or().unwrap_or()` (D-1) | Matches v1 hand-roll ethos; precedence CLI > env > config > builtin; missing/malformed file → defaults, never errors a normal command |
| `--json` house style: ONE document, no NDJSON, no BOM, no ANSI/progress on stdout (SPINE-01) | PS7 `ConvertFrom-Json` needs one buffered doc; `init_output` forces `COLOR_ON=false` under json/clip; per-command JSON-purity test is the regression backstop |
| `windows 0.61` GO both features (D-2): `du --on-disk` + `pomodoro --sound` | Pin 0.61 to unify with transitive `windows ^0.61` from tauri-winrt-notification 0.7.2 |
| Bare `u64` for large JSON numbers (D-3); `to_string_lossy()` for non-UTF-8 paths (D-4) | One rule for the whole spine; never `to_str().unwrap()` (panics on non-UTF-8 NTFS names) |
| `config_path()` reads `%APPDATA%` env var FIRST, `dirs::config_dir()` only as fallback (06-01 Rule 1 deviation from the planned dirs-first form) | `dirs` 6.0 → `dirs-sys` 0.5 resolves `config_dir()` via `SHGetKnownFolderPath`, which IGNORES the `APPDATA` env var → per-process config isolation impossible (integration tests + any APPDATA-relocating CI couldn't point the lookup at a temp dir). The plan's "var_os(APPDATA) is the fallback only unless dirs resists" clause applies; dirs resists. Identical `%APPDATA%\box\config.toml` target. |
| Env-tier spelling locked to `BOX_HASH_DEFAULT_ALGO`; `.or(env)` slot DEFERRED to 06-02 (06-01 Claude's Discretion) | 06-01 wires only the pure resolver + config tier; the live env→Algo parse lands with `hash`'s compute-default flip in 06-02 (reuses one FromStr/ValueEnum parse for env+config). Clipboard confirmation wording locked to `"Copied to clipboard"`. |
| `BOX_HASH_DEFAULT_ALGO` env tier wired live in 06-02 via `parse_algo(s) = Algo::from_str(s,true).ok()` (clap ValueEnum, case-insensitive), reused for both env + config — single-sourced spelling table | The hash compute default now resolves `self.algo.or_else(env).or(config().default_hash_algo).unwrap_or(Blake3)` (CLI>env>config>builtin). An unrecognized env value returns None and falls through (never errors a normal `box hash`). `hash` uses this inline `.or()` chain, NOT `resolve_algo` (per the plan's line-162 spec); `resolve_algo` keeps its forward-compat allow. |
| D-05 verify probe: capture the path string into `path_for_probe` BEFORE `read_file_or_stdin(self.path)` consumes it; re-open + blake3 on a 64-hex no-`--algo` mismatch (06-02) | `ResolvedInput.reader` is single-pass `Box<dyn Read>`. `path_for_probe` is `Some` only for a real path (`p != "-"`), `None` for stdin (no second read → static hint). Decisive hint when blake3 matches the file, static otherwise; stderr-only, suppressed under `--json`, exit STAYS 1. Hint token `--algo blake3` styled `.yellow()` when `is_color_on()`. |
| `{Row}/{Output}` serde struct feeds BOTH human + JSON paths; always-wrapped `{results,count}` even for N=1 (06-02 pilot literals, frozen for Phase 7) | `uuid` → `{results:[{uuid,version:"v4"}],count}`; `hash` → `{results:[{path,algo,digest}],count}`. The pure renderer fills the struct, `is_json_on()` forks (emit_json | out_line) — no drift. Object never a bare array (Phase-8 multi-item compatible). `tests/uuid.rs::json_purity` is the copy-me JSON-purity test for all 23 commands. |
| **D-18 (07-01) A1 RESOLVED:** `base64 --decode --json` is binary-safe — decoded bytes re-encoded to base64 in the `output` field, NEVER `String::from_utf8(...).unwrap()` (T-07a-01) | A JSON string can't hold non-UTF-8 bytes; re-encoding to base64 is lossless + round-trippable. `json_decode_non_utf8` is the regression backstop. The human binary-decode path is unchanged (raw `write_all`) and is NOT clip-supported (`out_line` is line-oriented). Encode is always ASCII-safe → flat `{output, mode}`. |
| **D-19 (07-01) color JSON hex locked LOWERCASE** (`#{:02x}` → `#rrggbb`); human `Hex` row stays UPPERCASE | Only the JSON `hex` field is lowercased so `json_purity` is deterministic (`.hex == "#ff0000"`); the existing human render (`#{:02X}`) is byte-stable (trycmd snapshot unchanged). Nested D-17 shape `{hex, rgb:{r,g,b}, hsl:{h,s,l}}`. |
| **D-20 (07-01) epoch unified shape** — input resolved to one `epoch: i64` BEFORE the `is_json_on()` fork so `{epoch,utc,local}` JSON never branches on mode (D-17) | `epoch_output()` and `format_timestamp()` share the same `DateTime::from_timestamp`/`with_timezone(&Local)` math (no-drift). JSON datetime strings drop the `Local:`/`UTC:` label prefixes (the keys convey direction). Human path routes through `out_line` for a free future clip adoption. |
| **D-21 (07-01) SC4 = parse-but-ignore** — `matrix`/`pomodoro`/`lolcat`/`ascii`/`clip` never call `emit_json`/`is_json_on`; the global flags parse but emit NO JSON document | `display_only_omit_json` (tests/cli.rs) asserts the runnable subset live (clip piped stdin, ascii `tiny.png` fixture, lolcat arg); matrix/pomodoro (loop/block) covered by source state + a grep gate (0 non-comment matches). Each module carries a `# Spine omission (SC4)` doc note. |
| **D-22 (07-02) du/dupes --json from existing buffered models** — du `{results:[{name,is_dir,size}],count,total_bytes,total_children}`; dupes `{results:[{size,paths}],count,wasted_bytes}` (D-11/D-17) | du's `total_bytes`/`total_children` are the full-scan `total`/`rows.len()` captured BEFORE `--top` so they stay full-scan (the human summary's invariant); `--top` still truncates `.results`. dupes serializes `Vec<PathBuf>` via a `DupeRow{size,paths:Vec<String>}` projection (`to_string_lossy().into_owned()`, D-4) — `DupeGroup` keeps `PathBuf` for the human render; empty → `{results:[],count:0,wasted_bytes:0}`. |
| **D-23 (07-02) A4 RESOLVED — tree --json builds a REAL recursive node tree** via a NEW `build_node` recursion reusing the printer's `read_children`/`sort_children` (no-drift); `{name, type:"dir"\|"file", size?, children:[]}`, root-rule EXCEPTION (D-17) | The current flat printing recursion (`render_dir`) is untouched. `Node.type` renames `kind`; `size` is `skip_serializing_if=Option::is_none` (files only). `--depth` honored exactly like `render_dir`: `descend = depth <= max`, so a directory AT the cap appears with empty `children`. Locked by `json_recursive_shape`. |
| **D-24 (07-02) flatten/bulk-rename --json = D-13 plan projection orthogonal to --force (D-12)** — `{results:[{src,dst,action,reason}],count,dry_run,…}`; dry-run+json=plan, real+json=executed | `action` = lowercased `RowStatus` via a shared `action_str()` reusing `kind.status()` (no-drift); the RAW fields are serialized, NEVER `format_row` output. `dry_run` flips with `--force`. Real-run captures actual `copied`/`bytes_written`. `--json` suppresses per-row human prints in the execute loop via a captured `let json = is_json_on()` guard, emitting one document after the loop. |
| **D-25 (07-02) A3 RESOLVED — bulk-rename --force --json emits applied rows (D-12 override) + abort keeps stdout byte-empty (D-09)** | The human `--force` path stays silent-on-success; only `--json` emits rows (the whole plan projection, so non-empty). The conflict/abort path guards `print_plan_with_conflicts` behind `if !is_json_on()` — under `--json` the `bail!` error (→ stderr, exit 1 via main.rs) is the ONLY output; NO `{"error":…}` on stdout. Locked by `json_abort_empty_stdout` (tested under both dry-run and `--force`). |
| **D-26 (07-03) A2 RESOLVED — core::output::clip_feed(&str) is the ONE sanctioned spine addition this phase** (the "print X, copy Y" tee out_line cannot express) | Mirrors out_line's tee half (push_str + '\n', gated on CLIP_ON) but omits the println!. qr keeps `println!` for the glyph block and calls `clip_feed(&input)` so `qr --clip` copies the SOURCE TEXT, not the ▀▄ glyphs (D-15); under `--json --clip` emit_json's own tee copies the document (no double-feed). Locked by `clip_feed_tees_only` + the #[ignore]d `clip_copies_source_text` (pasted==input). NO other core::output primitive added. |
| **D-27 (07-03) json --json is D-16 identity passthrough** — emit_json(&value) on the parsed Value VERBATIM, NOT wrapped; the --json fork is FIRST and wins over --compact | json is the ONE direct-serde command (root-rule exception alongside tree). The machine document is always the pretty serde form (so `--json --compact` yields pretty — the decisive `json_identity_passthrough` discriminator). The plain `to_string_pretty` + `--compact` human branches route through `out_line` so `--clip`/`--compact --clip` tee the printed form; the colored `print!` branch is left as-is (never reached under --clip, COLOR_ON forced false). Invalid → bail! (exit 1, empty stdout) unchanged (D-09). |
| **D-28 (07-03) weather --json is D-17 current-only**; unit/wind_unit read from forecast.current_units (never hardcoded — imperial wind label is "mp/h", Pitfall WTHR-3) | `WeatherOutput{location, temperature, unit, conditions, wind_speed, wind_unit, humidity}` built from the parsed `forecast`; f64 fields straight from `forecast.current` (finite real API data, never hand-computed NaN/Inf, Pitfall 2). Offline `json_purity` via a one-shot loopback `TcpListener` serving `forecast_imperial.json` + a `lat,lon` location (skips geocoding → only the forecast GET runs); asserts `unit=="°F"`/`wind_unit=="mp/h"` to prove the label is from current_units. The stderr resolved-location echo stays off the --json stdout channel. NO forecast/daily/hourly fields (Phase 10). |

| **D-29 (08-01) HASH-V2-02 --json partial failure (A1 RESOLVED)** — emit the `{results,count}` document with ONLY the successful rows AND exit 1 | A partial-success refinement of D-09 (whose empty-stdout rule targets TOTAL failure). Best-effort coreutils parity: each unreadable file logs `error: …` on stderr, the rest are still hashed; `std::process::exit(1)` after rows flush (out_line/emit_json each end on a newline → stdout line-flushed before exit). `--verify` stays single-input (first path). Locked by `tests/hash.rs::{multi_file_two_space,json_multifile_purity,partial_failure_exit1}`. |
| **D-30 (08-01) progress is stderr-only** via `ProgressBar::with_draw_target(Some(n), ProgressDrawTarget::stderr())` behind a `!is_json_on() && len > THRESHOLD` guard | Cutoffs (Claude's Discretion): hash file-count bar for >8 files, flatten copy bar for >16 plan items; below the cutoff no bar (keeps the common case + existing snapshots clean). Never constructed under `--json` (Pitfall 2 — stdout JSON purity). The copy-me pattern for du/dupes progress in 08-02/08-03. |
| **D-31 (08-01) flatten `encode_relative(rel, sep)`** splits on the REAL path separators then joins with `sep`; dedupe numeric suffix stays `_` | Splitting on `/`/`\` (not on `sep`) keeps a multi-char/unusual separator correct and is byte-identical to v1 for the default `_`. The dedupe `_{n}` suffix is a within-output uniqueness counter, not a segment join, so it is unaffected by `--separator`. `--separator` rejects `/`/`\` before any I/O (T-8-01); `--extensions` is a pure lowercased-set compare, no glob/regex (T-8-01-INJ). `--include-hidden` bypasses the D-06 prune; all three fold into the single `build_plan` walk (no-drift). |

| **D-32 (08-02) TREE-V2-01 gitignore is an ancestor-stack `Vec<Gitignore>` push/pop threaded via a `WalkCtx` through BOTH `render_dir` + `build_node` (the shared `read_children` chokepoint), checked deepest-first** — matcher-as-filter, NOT the recursive `ignore` walker (D-20) | `--gitignore` loads each dir's own `.gitignore` rooted at that dir (so `matched(abs_path, is_dir)` strips the right prefix); `is_ignored` checks the stack `.rev()` so a deeper `!whitelist` re-shows a file an ancestor `*.glob` hid (eza #1086 — the `keep.log` test). `--ignore` globs are the SHALLOWEST matcher via `add_line(None, glob)`. `--dirs-only` filters AFTER the ignore pass; `--sort size` = files biggest-first (ties alpha) with dirs (`size:None`) sorted to the end, `--sort name`/none = the v1 D-08 order. Empty stack + no dirs-only + no sort = byte-identical to v1 (trycmd pin green). `WalkCtx` bundles `max_depth`/`sizes`/`opts` to stay under clippy's `too_many_arguments`. |
| **D-33 (08-02) DU-V2-01/02: percent column is RENDER-only (A2 — no `f64` in JSON), basis = the full-scan total** (the on-disk total under `--on-disk`); `--exclude` globset matched relative to the target root; Win32 `compressed_size` localized in `du/mod.rs` | `percent_str(size,total)` guards `total==0 → 0.0%` (never `NaN`, Pitfall 3), `<0.1%` for tiny-nonzero; `band_color` REPLACES the lone `.cyan()` — `>50%` red, `10–50%` yellow, else plain, gated on `is_color_on()`. `--exclude` drops matching immediate children (no row) AND keeps matching descendant files out of `dir_total` (root-relative `strip_prefix`); empty set = unchanged default. `--on-disk` sums each descendant's `GetCompressedFileSizeW` (dirs have no intrinsic compressed size); JSON gains a top-level `on_disk:bool` marker; per-module FFI (NOT shared `core::fs`) per the 08-02/08-03 wave-isolation choice. |

| **D-34 (08-03) DUPE-V2-01: dupes runs a size→partial(16 KiB BLAKE3)→full cascade; hardlink aliases sharing one `(volume_serial, file_index)` are collapsed before `wasted = (distinct_inodes-1)*size`** — identity via the STABLE Win32 `GetFileInformationByHandle` (localized `file_identity` in `dupes/mod.rs`), NOT the nightly `windows_by_handle` std fields (RESEARCH Pitfall 1 correction to STATE.md:113) | The partial stage re-buckets size-candidates by `(size, partial_hash)`; only `(size,partial)` buckets of `>=2` reach the full `par_iter`. The full hash stays the SOLE grouping arbiter — the partial stage is a pure pre-filter that provably can't change grouping (so a black-box grouping test can't distinguish 2-stage from 3-stage; `multistage_splits` is a green-from-start regression guard). `wasted_space` made hardlink-aware IN PLACE: `distinct_inodes` calls `file_identity` per path and counts an identity error as that path's own inode, so the synthetic-path unit tests keep matching `(len-1)*size` — no signature change, no dead code. JSON `{results,count,wasted_bytes}` shape unchanged; the human render still LISTS all alias paths, only the wasted figure collapses them. Per-module FFI (wave-isolation, D-33). |
| **D-35 (08-03) RENM-V2-01: `--case upper\|lower\|title` (title on the STEM only, extension preserved) + literal `{n}` (`{{n}}` escape) numbered over the SORTED source order, assigned BETWEEN `build_plan` and the UNCHANGED `preflight_plan`** (D-21 apply order: re.replace → {n} → --case) | `apply_number_and_case_to_plan` operates on every regular FILE — real renames AND no-op `(unchanged)` skips — so `--case` applies even when `re.replace` was a no-op (`"(.*)" "$1" --case upper` uppercases everything); directory/symlink skips excluded (only files numbered). The byte-exact no-op check is RE-RUN post-transform and tallies kept in sync (Rename↔Skip flips). Counter = `start`, step `step`, over the source-path sort (reproducible, Pitfall 7); `width = number_width.unwrap_or(digit_count(file_count))`. `{{n}}` escaped via a NUL sentinel. `Case` is a `pub` clap `ValueEnum` (matches `hash::Algo`/`tree::SortMode`; satisfies `private_interfaces`). The load-bearing abort-all collision/cycle/separator detector is byte-for-byte untouched; default `box bulk-rename` output preserved. |

| **D-36 (08-04) FLAT-V2-02: `flatten --move` executes in TWO phases — copy+verify EVERY file (safe_copy create-new → dest-exists + size-match), THEN delete EVERY source — rather than the plan's per-item copy→verify→delete loop** (dry-run is the DEFAULT, `--force` to execute; empty source dirs preserved) | The per-item loop the plan's `<action>` prose described would DELETE items 1..N-1 before a copy error on item N, violating the plan's own must_haves truth + threat T-8-04 ("every abort path — incl. mid-batch copy error — leaves the source byte-for-byte unchanged"). Two-phase is the ONLY ordering satisfying that invariant: Phase 1 copies+size-verifies all (any error `?`-propagates with ZERO sources deleted); Phase 2 (delete all) is unreachable until the whole batch verified, so a failed/short copy can never orphan a source. Trade-offs (reviewed + accepted at the mandatory adversarial code-review gate): peak disk doubles for the batch; a rare Phase-2 `remove_file` error leaves a fully-copied + partially-deleted tree — recoverable, NO data loss (T-8-04-TOCTOU accepted, single-process local CLI). `--json` reuses `FlattenOutput`/`flatten_rows` (dry_run flips with `--force`); copy mode (no `--move`) byte-identical to 08-01. `snapshot_tree(before)==snapshot_tree(after)` per abort path is the data-loss backstop (copy-me for 08-05/08-06). |

| **D-37 (08-05) DUPE-V2-02: `dupes --delete` keep-first over the sorted groups + hardlink-safe via `file_identity` collapse + abort-all-before-any pre-flight; the pre-flight DOES I/O (one `file_identity` read per member during `build_delete_plan`) — an honest clarification of the plan's "pure pass" wording, NOT a behavioral deviation** (dry-run is the DEFAULT, `--force` to execute) | Hardlink-safe candidate selection cannot be pure: it must read each member's `(volume_serial, file_index)` to know whether a candidate is an alias of the kept inode. The abort-all-before-any guarantee is preserved verbatim — the ENTIRE plan (every keep/delete/alias decision) is computed before a single `remove_file`; ANY pre-flight problem `bail!`s (exit 1) with NOTHING deleted (the human plan printed only `if !is_json_on()`, so `--json` abort keeps stdout empty, D-09). Keep-first takes `paths[0]` over the already-deterministic 08-03 `(hash,path)` sort, so a group can NEVER lose its last real copy (keep-≥1 is structural, threat T-8-05). Candidates = `paths[1..]` MINUS any sharing the kept member's identity (an alias of the kept inode is never deleted — frees nothing, destroys a name, Pitfall 6 / T-8-05-HL). `remove_file` is reached only under `--force` AND a clean pre-flight, `?`-propagating on first error (T-8-05-PARTIAL). `--delete --json` emits a `DeleteOutput` (per-group kept/deleted projection) with a `dry_run` marker flipping with `--force`, within the `{results,count,…}` family; read-only `dupes` (no `--delete`) byte-identical to 08-03. `snapshot_tree(before)==snapshot_tree(after)` per abort path is the data-loss backstop. Trade-off reviewed + accepted at the mandatory adversarial code-review gate (TOCTOU between identity-read and delete = accepted T-8-05-TOCTOU, single-process local CLI). |

| **D-38 (08-06) RENM-V2-02: `bulk-rename --backup` writes a JSON undo MANIFEST (not byte copies — D-22 applied) + `File::sync_all()`'d BEFORE the first `std::fs::rename`, with each entry flipping `applied:true` (rewrite+fsync) as its rename returns → an `applied`-partitioned, reconcilable manifest on any mid-batch error** (dry-run no-op, `--force`-only) | A pure `std::fs::rename` (`MoveFileExW`) changes only the NAME, so the `{old → new}` map IS the entire reversible state — copying file bytes protects data that was never at risk and doubles disk for no recoverability gain. A pure `build_manifest(plan)` maps every `ItemKind::Rename` item to a `BackupEntry { old, new, applied:false }` (ABSOLUTE paths via `parent.join(...)` + `to_string_lossy()`, D-4) — a zero-drift serde projection of the SAME pre-flight-cleared `Plan` the executor consumes (wrapped in `BackupManifest { id, dir, entries }`). The FULL all-`applied:false` manifest is `write_manifest`'d (`serde_json::to_writer_pretty` + `File::sync_all()`) to `%LOCALAPPDATA%\box\undo\box-undo-<unix_millis>.json` (LOCALAPPDATA not APPDATA, OUTSIDE the renamed tree so `--recursive` never re-walks it + renaming the target dir never orphans it, Pitfall 8; fallback to the target dir only if LOCALAPPDATA unset; `<id>` sortable, A5) strictly AFTER `preflight_plan` returns clean and BEFORE the first rename; the path is echoed to stderr. Inside the `--force` loop each `std::fs::rename(...)` Ok flips that entry's `applied:true` + rewrite+fsync, so a mid-batch I/O error leaves a manifest whose flags EXACTLY partition done (new exists/old gone) vs pending (old exists) — the dir is reconcilable. `--backup` is orthogonal to and only meaningful with `--force` (dry-run no-op); the abort-all-before-any `bail!` (unchanged) writes NEITHER manifest NOR rename; non-`--backup` `bulk-rename` byte-identical to 08-03. `--undo` replay explicitly Deferred (manual reverse documented in the module doc). New `tests/bulk_rename_backup.rs` points the command's LOCALAPPDATA at a 2nd temp dir; `backup_partition_recoverable` induces a REAL mid-batch failure (a locked target) and asserts the applied-flag partition against on-disk reality. Reviewed + approved at the mandatory adversarial code-review gate (T-8-06/-LOC/-ABORT/-SILENT/-DOS mitigated; T-8-06-ACCUM accepted — manifests accumulate, no auto-cleanup, hold only path names). |

Full v1.0 decision log preserved in PROJECT.md Key Decisions + `.planning/milestones/v1.0-ROADMAP.md`.

### Critical Pitfalls to Remember (carried from v1 + new for v2)

- **v2 #1 failure mode:** `--json` stdout contamination — stray progress/ANSI/BOM bytes break `ConvertFrom-Json`. Progress → stderr via `ProgressDrawTarget::stderr()`, suppressed entirely under `is_json_on()`.
- **Config precedence:** every config-overridable flag is `Option<T>` with NO `default_value`; resolve `cli.or(env).or(config).unwrap_or(builtin)`. A missing/malformed config must never error a normal `box uuid`.
- **`dupes --delete` hardlink false-positive:** detect shared identity via `(volume_serial, file_index)` from `fs::metadata(path)` — NOT `DirEntry::metadata()` (returns `None` for those fields). Collapse before computing wasted space.
- **`flatten --move`:** copy → verify (dest exists + size matches) → delete source. Never delete before confirming the copy.
- **`std::fs::rename` SILENTLY OVERWRITES on Windows** (no `create_new` for moves) — the abort-all-before-any pre-flight is the only backstop (v1 bulk-rename pattern; reuse for `--backup`).
- **`arboard` clipboard must run on the main thread only** — `flush_clip()` runs once in `main()` after successful dispatch.
- **`--clip`/`--json` force `COLOR_ON=false`** so the clipboard/JSON never gets ANSI escapes.
- **Terminal loops (`lolcat --animate`, `matrix` extensions):** arm RAII `RawGuard` immediately after `enable_raw_mode()?`; detect TTY first, degrade to static when piped/`--json`; single-flush-per-frame; `KeyEventKind::Press`-only quit filter.
- `box` is binary-only — unit tests run via `cargo test --bin box`, NOT `--lib`.
- Build target: `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"`.
- BLAKE3-default breaking change: loud `--help`/PROJECT note; `--algo sha256` + `hash.default_algo` config restore old behavior; `--verify` 64-hex still maps to sha256 (transitional mismatch hint).

### Architecture Established (v1 base — v2 grafts onto, does NOT rewrite)

- Single Rust crate; `src/commands/<cmd>/mod.rs` per command; `RunCommand` trait `fn run(self) -> anyhow::Result<()>` (signature UNCHANGED in v2).
- `src/core/`: `errors.rs`, `output.rs` (`COLOR_ON` + `is_color_on()` + `human_size` + `terminal_width`), `fs.rs`, `input.rs`. `main.rs` ~123 lines: parse + dispatch + 0/1/2 exit only.
- **v2 NEW/MODIFIED:** `core/output.rs` (+`JSON_ON`/`CLIP_ON` atomics, `init_output`, `is_json_on`, `emit_json`, `out_line`, `CLIP_BUF`, `flush_clip`); `core/config.rs` (NEW); `core/errors.rs` (+`BoxError::Config`); `cli.rs` (+global `--json`/`--clip`, +`Completions`/`Config` variants); `main.rs` (+`init_config`/`init_output`/`flush_clip`); per command: one `#[derive(Serialize)]` output struct + `is_json_on()` fork + `out_line` primary output. New crates: `clap_complete`, `toml`, `dirs`, `indicatif`, `chrono-tz` (epoch tz), `windows 0.61`; `uuid "v7"` feature.

### Todos

- [ ] Code-review advisory follow-ups (01-REVIEW.md, non-blocking, carried from v1): WR-03/WR-04 install.ps1 PATH empty-segment + smoke-test-by-abspath; IN-02/IN-03 share one flatten render path between dry-run and real run.

### Blockers

None.

---

## Session Continuity

**To resume:** Read `.planning/ROADMAP.md` for phase goals, then this file for position/context.

**Last session:** 2026-06-28T11:34:49.550Z
**Stopped at:** Phase 9 context gathered
**Resume file:** .planning/phases/09-dev-transform-visual-depth/09-CONTEXT.md

**Next action:** **Phase 8 (Filesystem Depth) implementation is COMPLETE — all 6 plans done, all 10 requirements delivered.** Wave 1: 08-01 (HASH-V2-02 + FLAT-V2-01), 08-02 (TREE-V2-01 + DU-V2-01/02), 08-03 (DUPE-V2-01 + RENM-V2-01); Wave 2 (destructive, each with an approved adversarial code-review gate): 08-04 (FLAT-V2-02 flatten --move), 08-05 (DUPE-V2-02 dupes --delete), 08-06 (RENM-V2-02 bulk-rename --backup). **08-06 shipped** `bulk-rename --backup`: a JSON undo MANIFEST (a zero-drift serde projection of the pre-flight-cleared `Plan` — one `{old,new,applied}` per renamed file, ABSOLUTE paths) `File::sync_all()`'d to `%LOCALAPPDATA%\box\undo\box-undo-<unix_millis>.json` (OUTSIDE the renamed tree, LOCALAPPDATA not APPDATA, Pitfall 8) BEFORE the first `std::fs::rename`, then each entry flips `applied:true` (rewrite+fsync) as its rename returns → an `applied`-partitioned, reconcilable manifest on a mid-batch error (D-38); `--backup` is a dry-run no-op + `--force`-only, path echoed to stderr, the abort-all-before-any `bail!` writes NEITHER manifest NOR rename; `--undo` replay Deferred; new `tests/bulk_rename_backup.rs` (manifest-written/dry-run-noop/abort-writes-nothing/partition-recoverable via a real locked-target mid-batch). **The orchestrator now owns Phase 8 verification + `phase.complete` — the phase is NOT yet formally marked complete here.** One out-of-scope follow-up carried forward: a `style: cargo fmt` repo-root sweep to clear the pre-existing formatting drift logged in `deferred-items.md` (the 08-06 gates `cargo test` + `cargo clippy --all-targets -D warnings` are both clean; the two 08-06-authored files are fmt-clean). After phase close-out: **Phase 9 (Dev-Transform & Visual Depth)** — UUID/EPOC/COLR/JSON/PASS + visuals LOL/MTRX/QR/ASCI. Full `cargo test` green and clippy `--all-targets -D warnings` clean.

---
*State reset to v2.0 phase map: 2026-06-25 by roadmapper (v1.0 plan-by-plan execution log archived with the milestone; v2.0 accumulated context — locked decisions D-1..D-7, v2 pitfalls, the v1→v2 architecture graft — preserved above).*
