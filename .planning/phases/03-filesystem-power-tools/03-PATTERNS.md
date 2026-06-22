# Phase 3: Filesystem Power Tools - Pattern Map

**Mapped:** 2026-06-22
**Files analyzed:** 13 (5 new command modules, 2 core edits, 2 wiring edits, 1 manifest check, 5 new test files; some files are both new + shared-core)
**Analogs found:** 13 / 13 (every new file has an in-repo analog — this is a low-novelty, copy-the-pattern phase)

> ⚠️ **Manifest correction (read before planning the deps step):** RESEARCH.md and CONTEXT.md § Integration Points say "new deps to add: `blake3`, `sha2`, `md-5`, `rayon`, `regex`". **They are ALREADY in `Cargo.toml`** (lines 37-42), pre-added in Phase 2's "lean bundle" so Wave-2/3 command plans never touch the manifest. `base16ct = "1.0.0"` (line 42) **and** `const-hex = "1.19.1"` (line 43) are also already present. So the deps step is a **verify-and-feature-check**, not an add:
> - `base16ct` is pinned WITHOUT the `alloc` feature → `base16ct::lower::encode_string` (which returns an owned `String`) **will not compile** until `features = ["alloc"]` is added, OR use the no-alloc `encode_str(&mut buf)` into a 64-byte stack buffer (RESEARCH Assumption A1 / Open Question 1). `const-hex` is an alternative already on hand.
> - No `Cargo.toml` line needs adding for hashing/regex/rayon — only the `base16ct` feature flag (or a switch to `const-hex`/`encode_str`) is an open decision.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/commands/hash/mod.rs` | command | file-I/O + transform (stream→digest) | `src/commands/base64/mod.rs` (input layer) + `src/commands/flatten/mod.rs` (RunCommand shell) | role-match (no streaming-hash analog exists) |
| `src/commands/tree/mod.rs` | command | request-response (walk→render) | `src/commands/flatten/mod.rs` (walk + color-gated render) | role-match |
| `src/commands/du/mod.rs` | command | batch (walk→accumulate→sort→render) | `src/commands/flatten/mod.rs` (walk + `human_size` + summary) | role-match |
| `src/commands/dupes/mod.rs` | command | batch + parallel transform (group→hash→group) | `src/commands/flatten/mod.rs` (walk) + `hash` infra (BLAKE3) | partial (no parallel/rayon analog exists) |
| `src/commands/bulk_rename/mod.rs` | command | file-I/O write (plan→preflight→rename) | `src/commands/flatten/mod.rs` + `src/commands/flatten/rename.rs` | exact (the dry-run/collision model is flatten's) |
| `src/core/output.rs` (edit: add `human_size`) | utility | transform | `src/commands/flatten/mod.rs:332-344` (the fn being promoted) | exact (verbatim move) |
| `src/core/input.rs` (edit: add `--file` branch) | utility | file-I/O | `src/core/input.rs` `resolve`/`resolve_bytes` (extend in place) | exact (documented extension point, lines 24-25) |
| `src/cli.rs` (edit: 5 unit variants → Args structs) | config | — | `src/cli.rs:36` `Flatten(FlattenArgs)` | exact |
| `src/main.rs` (edit: remove 5 `not_implemented` arms) | config | — | `src/main.rs:61` `Commands::Flatten(args) => args.run()` | exact |
| `tests/hash.rs` | test | — | `tests/flatten.rs` (assert_cmd + assert_fs + NO_COLOR) | role-match |
| `tests/tree.rs` + `tests/cmd/tree.trycmd` | test | — | `tests/flatten.rs` + `tests/cmd/cowsay-single.trycmd` + `tests/cli.rs::trycmd` | exact |
| `tests/du.rs` | test | — | `tests/flatten.rs` | role-match |
| `tests/dupes.rs` | test | — | `tests/flatten.rs` | role-match |
| `tests/bulk_rename.rs` | test | — | `tests/flatten.rs` | exact |

---

## Pattern Assignments

### `src/commands/hash/mod.rs` (command, file-I/O + transform)

**Analogs:** `src/commands/base64/mod.rs` (the `core::input` consumer + `clap::Args` + `RunCommand` shell) and `src/commands/flatten/mod.rs` (the `human_size`/error-context style). No streaming-hash analog exists in the repo, so the hashing core is new (build per RESEARCH Pattern 1/2); everything *around* it is copied.

**`RunCommand` shell + input layer** — copy from `base64/mod.rs:41-61`:
```rust
impl RunCommand for Base64Args {
    fn run(self) -> anyhow::Result<()> {
        // Byte-exact input (binary safe): arg → piped stdin → exit-2 on a no-arg
        // interactive TTY (D-04 branch 3).
        let bytes = crate::core::input::read_input_bytes(self.input)?;
        ...
    }
}
```
For `hash`: stream rather than buffer — read bytes via the `--file`-extended input path (D-05), NOT `read_input_bytes` returning a whole `Vec` if the file is large; but the *acquisition precedence* (arg vs `-` vs piped vs TTY-no-block) is exactly base64's. The stdin label is `"-"`.

**`clap::Args` struct + a flag-with-value (`--algo`)** — copy the struct shape from `base64/mod.rs:29-39`:
```rust
#[derive(Debug, Args)]
pub struct Base64Args {
    #[arg(long)]
    pub decode: bool,
    #[arg(long = "url-safe")]
    pub url_safe: bool,
    /// omit to read from piped stdin (`-` also means stdin).
    pub input: Option<String>,
}
```
`hash` adds an `#[arg(long, value_enum, default_value_t = Algo::Sha256)]` field (clap `ValueEnum` derive on the `Algo` enum) + `--verify Option<String>` + the `--file`/positional path. (Discretion D: `--file` flag vs positional — must route through `core::input`.)

**Typed exit-2 error for a bad `--verify` length** — mirror the `MissingInput` pattern. The error variant lives in `src/core/errors.rs:6-25` and is downcast-mapped in `src/main.rs:97-103`:
```rust
// src/core/errors.rs — add an UnsupportedHashLength variant next to MissingInput:
#[error("no input: pass an argument or pipe data")]
MissingInput,
```
```rust
// src/main.rs:97-103 — extend this downcast arm so the new variant → exit 2:
if let Some(crate::core::errors::BoxError::MissingInput) =
    e.downcast_ref::<crate::core::errors::BoxError>()
{
    ExitCode::from(2)
} else {
    ExitCode::from(1)
}
```
A successful-but-mismatched `--verify` stays exit 1 (a plain `Ok(())`/`bail!` path); only the unsupported-length usage error gets the typed exit-2 variant (RESEARCH Pitfall 1).

**Unit-test style for pure fns (known-answer hashes)** — mirror `base64/mod.rs:91-136` (`#[cfg(test)] mod tests` co-located in `mod.rs`, round-trip/known-answer asserts).

---

### `src/commands/tree/mod.rs` (command, request-response: walk → render)

**Analog:** `src/commands/flatten/mod.rs` — same walk + color-gated render skeleton; the glyphs differ (Unicode box-drawing vs flatten's ASCII status glyphs).

**Walk with hidden-prune + no-follow** — copy verbatim from `flatten/mod.rs:212-215`:
```rust
let walker = WalkDir::new(src_root)
    .follow_links(false)
    .into_iter()
    .filter_entry(|e| !is_hidden(e));
```
`is_hidden` is `core::fs::is_hidden` (D-06); never re-implement (RESEARCH Pitfall 7 — root exemption already handled).

**Color gating (directory names only)** — follow the `is_color_on()` gate used in `core::output::format_row` (`output.rs:121-129`):
```rust
let glyph_str = if is_color_on() {
    match status { RowStatus::Copy => glyph.green().to_string(), ... }
} else {
    glyph.to_string()
};
```
For tree, gate `.blue().bold()` on directory names the same way so piped output is byte-identical minus ANSI (D-10). Box-drawing glyphs (`├── └── │   ` + `    ` gap) are RESEARCH Pattern 3 constants — they are STRUCTURE, so Unicode is correct here (distinct from flatten's ASCII status glyphs).

**Path normalization** — `core::fs::normalize_path` (`fs.rs:34-36`, dunce), as flatten does at `flatten/mod.rs:100-103`.

**Size column (`--sizes`)** — call the promoted `core::output::human_size` (see shared pattern below); per-file only, dirs blank (D-10).

---

### `src/commands/du/mod.rs` (command, batch: walk → accumulate → sort → render)

**Analog:** `src/commands/flatten/mod.rs` (walk + `human_size` + the locked-summary style).

**Same walk skeleton** as tree (copy `flatten/mod.rs:212-215`). Accumulate per immediate child: file = own size, dir = recursive sum of non-hidden descendants (`metadata().len()`, RESEARCH A4).

**Determinism — sort before printing** (RESEARCH Pitfall 6): sort by `(size desc, name asc)`; never rely on walk order. flatten doesn't need this (its output follows walk order), so this is a du/dupes-specific addition, but the *summary + human_size* render is flatten's:

**Summary line + `human_size`** — model on flatten's locked-summary helpers (`output.rs:208-225`) and its `human_size` call site (`flatten/mod.rs:199`):
```rust
real_run_summary(copied, plan.renamed, plan.skipped, &human_size(bytes_written))
```
du writes its own `{X} of {Y} entries shown. {TOTAL} total.` summary (X = after `--top`, Y = all children, TOTAL = full-scan), right-aligning the size column to the widest shown `human_size` value (D-11/D-12). Trailing `/` (ASCII) marks dirs so the distinction survives piping.

---

### `src/commands/dupes/mod.rs` (command, batch + parallel transform)

**Analogs:** `src/commands/flatten/mod.rs` (the walk) + the `hash` command's BLAKE3 infra. **No rayon/parallel analog exists in the repo** — the `par_iter` phase is net-new (build per RESEARCH Pattern 4); the walk and the deterministic-sort discipline are copied.

**Walk** — copy `flatten/mod.rs:212-215` (hidden-prune, no-follow). No noise/`ignore` list (D-07).

**Determinism** (RESEARCH Pitfall 6) — rayon order is arbitrary: `collect` the `(hash, path)` pairs, then `sort()` by `(hash, path)` before emitting groups. Same rule as du.

**Reuse the hash command's BLAKE3 path** for content equality (D-13) — the `hash_blake3` helper from `hash/mod.rs` (RESEARCH Pattern 1, `blake3::Hasher::update_reader`); cryptographic-criticality is irrelevant for grouping, BLAKE3 is chosen for speed.

**Read-only** — there is NO write path here; do NOT touch `safe_copy`/rename. Output = groups ≥2 + wasted-space summary (sum of redundant copies), using `human_size`.

---

### `src/commands/bulk_rename/mod.rs` (command, file-I/O write: plan → preflight → execute)

**Analogs (the richest match in the phase):** `src/commands/flatten/mod.rs` (the plan→preview→execute split, `arrow_col`, the case-folded `occupied` set) + `src/commands/flatten/rename.rs` (the case-fold/dedupe/separator-injection model).

> ⚠️ **Module name:** the CLI name `bulk-rename` is not a Rust ident — name the module `bulk_rename` and keep `#[command(name = "bulk-rename")]` on the enum variant (precedent: `eight_ball` / `#[command(name = "8ball")]` at `cli.rs:52-53`).

**Plan struct + dry-run/execute split** — copy the shape from `flatten/mod.rs:69-91` and the `run()` branch at `flatten/mod.rs:137-204`:
```rust
struct PlanItem {
    src: PathBuf,
    src_label: String,
    dst_name: Option<String>,   // None for skips
    kind: ItemKind,             // Copy/Rename/Skip → for bulk-rename: Rename/Skip only
    reason: Option<String>,     // "[collision]", "(skipped: directory)", "(unchanged)"
}
```
```rust
// flatten/mod.rs:138-146 — dry-run is a branch that prints the plan + summary and returns:
if self.dry_run {                       // bulk-rename INVERTS this: dry-run is DEFAULT, --force executes (D-19)
    print_plan(&plan);
    println!();
    println!("{}", dry_run_summary(plan.to_copy, plan.renamed, plan.skipped));
    return Ok(());
}
```
Note the inversion: flatten's default is real + `--dry-run` opts out; bulk-rename's default is dry-run + `--force` opts in (D-19). No `+`/Copy glyph (nothing is created) — only `~` (Rename) and `-` (Skip).

**Row rendering (aligned `old -> new`)** — reuse `core::output::format_row` + `arrow_col` exactly as flatten does (`flatten/mod.rs:148-188`, `output.rs:109-169`, `arrow_col` at `flatten/mod.rs:302-309`). The `RowStatus::{Rename, Skip}` glyphs (`~`, `-`) and `[collision]` inline reason are already the output module's vocabulary.

**Symlink/dir skip rows** — copy flatten's symlink-skip convention (`flatten/mod.rs:231-242`):
```rust
if is_symlink {
    plan.items.push(PlanItem { ..., kind: ItemKind::Skip,
        reason: Some("(skipped: symlink)".to_string()) });
    plan.skipped += 1;
    continue;
}
```
bulk-rename adds `(skipped: directory)` (D-15) and `(unchanged)` (D-18.4 no-op) rows the same way.

**Case-folded collision set (the core safety model, D-18)** — the data structure and folding come from `flatten/mod.rs:125-132` (seed an `occupied: HashSet<String>` from `read_dir`, keyed `to_lowercase()`) and `flatten/rename.rs:99-115` (`dedupe` keys on full-Unicode `to_lowercase()`, WR-01):
```rust
// flatten/mod.rs:125-132 — seed the occupied set, full-Unicode fold:
let mut occupied: HashSet<String> = HashSet::new();
for entry in std::fs::read_dir(&out_root)? {
    occupied.insert(entry.file_name().to_string_lossy().to_lowercase());
}
```
```rust
// flatten/rename.rs:99-103 — the full-Unicode fold key (NOT to_ascii_lowercase, WR-01):
pub fn dedupe(name: &str, occupied: &HashSet<String>) -> String {
    let key = name.to_lowercase();
    if !occupied.contains(&key) { return name.to_string(); }
    ...
}
```
**Critical divergence (D-18):** flatten's `safe_copy` has a `create_new` backstop (`fs.rs:80-116`); `std::fs::rename` has **NO such backstop** and silently overwrites on Windows (RESEARCH Pitfall 4). So bulk-rename must do ABORT-ALL-BEFORE-ANY-RENAME pre-flight: check every planned target vs (a) other planned targets and (b) pre-existing on-disk names not being renamed away; detect cycles/swaps; abort the whole batch (exit 1, nothing written) on any clobber. The set is keyed `(parent_dir, folded_name)` per D-14 (per-directory scope; RESEARCH Open Question 3). A case-only change (`foo`→`Foo`) is byte-different and IS a real rename — compare EXACT non-folded names to avoid a false self-collision (RESEARCH Pitfall 5).

**Path-separator injection guard (V5, security)** — reuse the invariant proven by `flatten/rename.rs:35-50` + its `encode_no_separator` test (`rename.rs:144-180`): a regex `${1}` replacement that produces a `/` or `\` must be refused/skipped, never executed (the rename is scoped to the parent dir). flatten's "no separator survives" property is the model.

**Per-file `.context(...)` on each `std::fs::rename`** — mirror `safe_copy`'s context style (`fs.rs:82-95`) and flatten's call site (`flatten/mod.rs:173-174`):
```rust
let n = safe_copy(&item.src, &dst)
    .with_context(|| format!("flattening {}", item.src.display()))?;
```
bulk-rename: `std::fs::rename(src, dst).with_context(|| format!("renaming {} -> {}", ...))?`; stop on the first *unexpected* I/O error (a predictable collision already aborted pre-flight) (D-19, FOUND-06).

---

### `src/core/output.rs` (edit: promote `human_size`) — D-12

**Analog (verbatim move):** `src/commands/flatten/mod.rs:332-344` (the fn) + `flatten/mod.rs:350-357` (its unit test). Move both into `core::output`, make `pub`, and update flatten's one call site (`flatten/mod.rs:199`) to `core::output::human_size`:
```rust
// flatten/mod.rs:332-344 — move this EXACTLY (1024-based, decimal-style labels, stop at TB):
fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 { return format!("{bytes} B"); }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 { size /= 1024.0; unit += 1; }
    format!("{size:.1} {}", UNITS[unit])
}
```
```rust
// flatten/mod.rs:350-357 — the test moves with it:
#[test] fn human_size_scales() {
    assert_eq!(human_size(0), "0 B");
    assert_eq!(human_size(512), "512 B");
    assert_eq!(human_size(1024), "1.0 KB");
    assert_eq!(human_size(1536), "1.5 KB");
    assert_eq!(human_size(1024 * 1024), "1.0 MB");
}
```
Place it alongside `terminal_width`/`truncate_middle` in `output.rs` (Discretion D: exact location/signature). Do NOT add the `humansize` crate. tree/du then call `core::output::human_size`.

---

### `src/core/input.rs` (edit: add `--file PATH` branch) — D-05/D-06

**Analog (extend in place):** the documented extension point at `input.rs:24-25` ("A future `--file PATH` layer (D-06, Phase 3) slots in ahead of the stdin branch inside these helpers without reshaping the public signatures"). The branch slots into the `resolve_bytes` match (`input.rs:98-108`):
```rust
// input.rs:98-108 — the byte resolver the --file branch extends.
// New precedence (D-05): explicit arg → --file PATH (read the file) → piped stdin → TTY-no-block.
fn resolve_bytes<R: Read>(arg: Option<String>, is_tty: bool, mut reader: R) -> anyhow::Result<Vec<u8>> {
    match arg.as_deref() {
        Some(s) if s != "-" => Ok(s.as_bytes().to_vec()),
        _ if is_tty => Err(BoxError::MissingInput.into()),
        _ => {
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).context("failed to read input from stdin")?;
            Ok(buf)
        }
    }
}
```
The `--file` branch reads the named file's bytes ahead of the stdin branch; the `-` sentinel and `MissingInput`→exit-2 semantics are inherited unchanged. `hash` is the first consumer (D-05). Keep the inner-resolver `is_tty`/`Read` injection so the new branch stays unit-testable without a terminal (the existing test style, `input.rs:111-153`). For large files, `hash` should stream rather than `read_to_end` a whole `Vec` (RESEARCH anti-pattern: no whole-file buffering) — so the `--file` path may expose a reader/`File` handle, not just `Vec<u8>` (Discretion D: signature shape).

---

### `src/cli.rs` (edit) + `src/main.rs` (edit) — concrete before/after

**Analog:** the `Flatten` variant + its dispatch arm, already real.

**`src/cli.rs` — swap each unit variant for a real Args struct.** Before/after for `hash` (same shape for tree/du/dupes; bulk-rename keeps its `#[command(name)]`):
```rust
// BEFORE (cli.rs:56-66):
/// Compute and verify file hashes
Hash,
...
/// Bulk-rename files with a regex and dry-run preview
#[command(name = "bulk-rename")]
BulkRename,
```
```rust
// AFTER — model on the live Flatten variant (cli.rs:35-36):
/// Compute and verify file hashes
Hash(crate::commands::hash::HashArgs),
...
/// Bulk-rename files with a regex and dry-run preview
#[command(name = "bulk-rename")]
BulkRename(crate::commands::bulk_rename::BulkRenameArgs),
```

**`src/main.rs` — remove the `not_implemented` arm, dispatch to `.run()`.** Before/after for `hash`:
```rust
// BEFORE (main.rs:71-75):
Commands::Hash => Err(not_implemented("hash")),
Commands::Tree => Err(not_implemented("tree")),
Commands::Du => Err(not_implemented("du")),
Commands::Dupes => Err(not_implemented("dupes")),
Commands::BulkRename => Err(not_implemented("bulk-rename")),
```
```rust
// AFTER — model on the live Flatten arm (main.rs:61):
Commands::Hash(args) => args.run(),
Commands::Tree(args) => args.run(),
Commands::Du(args) => args.run(),
Commands::Dupes(args) => args.run(),
Commands::BulkRename(args) => args.run(),
```
Also register each new module in `src/commands/mod.rs` (`pub mod hash;` etc., alongside the existing `pub mod flatten;` at `mod.rs:7-17`). When the LAST `not_implemented` arm for these five is removed, check whether `use crate::commands::stub::not_implemented;` (`main.rs:22`) is still used by other stubs (lolcat/matrix/etc. remain) — it IS, so keep the import.

---

### `tests/hash.rs` / `tests/du.rs` / `tests/dupes.rs` (test, integration)

**Analog:** `tests/flatten.rs` — black-box `assert_cmd` + `assert_fs` temp trees + `NO_COLOR=1`, one test per VALIDATION-matrix row. All dev-deps already present (`Cargo.toml:49-55`).

**Runner helper + forced plain output** — copy `tests/flatten.rs:15-24`:
```rust
fn flatten(src: &Path, out: &Path, dry_run: bool) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("flatten").arg(src).arg(out);
    if dry_run { cmd.arg("--dry-run"); }
    cmd.env("NO_COLOR", "1");          // force plain output regardless of runner TTY
    cmd.assert()
}
```
Every new test sets `NO_COLOR=1` (RESEARCH determinism rule). Assert exit codes explicitly: `.success()`/`.code(0)`, `.failure().code(1)` (verify mismatch, collision abort), `.code(2)` (bad `--verify` length) — copy the `.failure().code(1)` / `.code(2)` style from `tests/flatten.rs:167-170` and `tests/cli.rs:43-61`.

**Known-answer hashes (hash/dupes):** embed the literal lowercase hex of fixed bytes (e.g. SHA-256/BLAKE3 of `b"box"` or an empty file) so the test is a true known-answer test, not a round-trip (RESEARCH determinism rules). Use distinct file sizes in du/dupes fixtures so the sort key is a total order.

**Symlink-test skip guard (bulk-rename/dupes)** — reuse the privilege-aware skip helper from `tests/flatten.rs:182-210` + `273-281`:
```rust
#[cfg(windows)]
fn make_file_symlink(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()  // false if unprivileged → test skips
}
```

---

### `tests/tree.rs` + `tests/cmd/tree.trycmd` (test, snapshot)

**Analogs:** `tests/flatten.rs` (the `tree.rs` integration side) + `tests/cmd/cowsay-single.trycmd` (the snapshot format) + `tests/cli.rs:100-103` (the single `trycmd()` runner that globs `tests/cmd/*.trycmd` — a new `tree.trycmd` is auto-picked up, no new runner needed).

**Snapshot format** — copy `tests/cmd/cowsay-single.trycmd` (a fenced `$ box <cmd> ...` transcript):
```
$ box cowsay "Hello, box!"
 _____________
< Hello, box! >
 -------------
...
```
For `tree.trycmd`: `$ box tree <fixture>` then the exact box-drawing transcript. ⚠️ **trycmd normalizes `\`→`/` in Windows path snapshots** (STATE.md [02-04]) — expect forward-slash paths in the `.trycmd` even though the binary prints backslashes; lock exact byte renders via unit tests on the pure formatting fns and use trycmd for the end-to-end shape (RESEARCH determinism rules). The `trycmd()` test (`cli.rs:101-103`) already globs `tests/cmd/*.trycmd`, so adding `tree.trycmd` needs no test-code change.

---

## Shared Patterns

### Hidden-prune walk (tree / du / dupes / bulk-rename --recursive)
**Source:** `src/core/fs.rs:46-61` (`is_hidden`), applied in `src/commands/flatten/mod.rs:212-215`.
**Apply to:** all four walkers (D-06).
```rust
let walker = WalkDir::new(root)
    .follow_links(false)
    .into_iter()
    .filter_entry(|e| !is_hidden(e));
```
`is_hidden` exempts the root (depth 0) so a dotted target isn't pruned to zero (walkdir#142, RESEARCH Pitfall 7). Never re-implement it. No `ignore` crate / noise list (D-07).

### Path normalization (all 5 commands)
**Source:** `src/core/fs.rs:34-36` (`normalize_path`, dunce), call site `src/commands/flatten/mod.rs:100-103`.
**Apply to:** every path argument, BEFORE containment/collision logic.
```rust
let root = normalize_path(&self.path)
    .with_context(|| format!("resolving {}", self.path.display()))?;
```
Never `std::fs::canonicalize` (leaks `\\?\`, RESEARCH Pitfall — FOUND-06).

### Color gating (tree dir names, du size value)
**Source:** `src/core/output.rs:32-34` (`is_color_on`) + the gate pattern at `output.rs:121-129`.
**Apply to:** every styled token in tree/du, so piped output is byte-identical minus ANSI (D-10).
```rust
let styled = if is_color_on() { name.blue().bold().to_string() } else { name.to_string() };
```

### Typed exit-2 error (hash bad `--verify` length)
**Source:** `src/core/errors.rs:6-25` (`BoxError`) + the downcast arm `src/main.rs:97-103`.
**Apply to:** hash only (the new `UnsupportedHashLength` variant). A mismatched verify is exit 1; only the usage error is exit 2 (RESEARCH Pitfall 1).
```rust
// errors.rs — add next to MissingInput:
#[error("unsupported --verify hash length: {len} (expected 32/64/128 hex)")]
UnsupportedHashLength { len: usize },
// main.rs:97 — extend the downcast to map BOTH MissingInput and UnsupportedHashLength → exit 2.
```

### Plan → dry-run-preview → execute split (bulk-rename)
**Source:** `src/commands/flatten/mod.rs:69-204` (Plan struct, `run()` branch) + `output.rs:109-225` (`format_row`/`arrow_col`/summaries).
**Apply to:** bulk-rename (D-19), INVERTED — dry-run is the default, `--force` executes.

### Case-folded collision detection (bulk-rename)
**Source:** `src/commands/flatten/mod.rs:125-132` (seed `occupied` from `read_dir`, fold `to_lowercase`) + `src/commands/flatten/rename.rs:99-115` (`dedupe`, full-Unicode fold, WR-01) + `rename.rs:35-50` (separator-injection guard).
**Apply to:** bulk-rename (D-18) — but with NO `safe_copy`/`create_new` backstop (that exists only for copies, `fs.rs:80-116`); pre-flight ABORT-ALL is the only guard for renames.

### Integration-test harness (every new `tests/<cmd>.rs`)
**Source:** `tests/flatten.rs:15-24` (runner + `NO_COLOR=1`), `tests/cli.rs:43-61` (exit-code asserts), `tests/cli.rs:100-103` (trycmd glob runner).
**Apply to:** all five new test files + `tree.trycmd`. All dev-deps already in `Cargo.toml:49-55`.

---

## No Analog Found

No file is *without* an analog, but two sub-capabilities are net-new (build from RESEARCH, not from a copied pattern) — flagged so the planner points the executor at the verified RESEARCH signatures rather than a non-existent in-repo example:

| Capability (within a file that DOES have an analog) | File | Reason | Source instead |
|------|------|--------|----------------|
| Streaming enum-dispatch hasher (sha2/md-5 via `digest::Digest` 0.11 + native `blake3::Hasher`) | `src/commands/hash/mod.rs` | No hashing code exists anywhere in the repo | RESEARCH Pattern 1 (verified signatures); deps already in `Cargo.toml:37-42` — only `base16ct` `alloc` feature is an open check |
| `rayon` parallel content-hash (`par_iter` + deterministic sort) | `src/commands/dupes/mod.rs` | No parallel/rayon code exists in the repo | RESEARCH Pattern 4; `rayon` already in `Cargo.toml:39` |

---

## Metadata

**Analog search scope:** `src/cli.rs`, `src/main.rs`, `src/commands/` (all 11 modules; flatten read in full as the richest analog), `src/core/{fs,input,output,errors}.rs`, `tests/` (all 13 test files + `tests/cmd/*.trycmd`), `Cargo.toml`.
**Files scanned:** 18 source/test/config files read in full or in the relevant range.
**Key correction surfaced:** hashing/regex/rayon/base16ct/const-hex deps are ALREADY in `Cargo.toml` (lines 37-43) — the deps step is verify-and-feature-check, not add (only `base16ct`'s `alloc` feature / a switch to `encode_str`/`const-hex` is open).
**Pattern extraction date:** 2026-06-22
