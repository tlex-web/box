# Phase 3: Filesystem Power Tools - Context

**Gathered:** 2026-06-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Build **five filesystem commands** on the existing `walkdir` + `core::fs` infrastructure established by `flatten`:

- `hash` — file checksums + `--verify` (HASH-01)
- `tree` — box-drawing directory tree render (TREE-01)
- `du` — disk-usage, size-sorted biggest-first (DU-01)
- `dupes` — content-duplicate finder (DUPE-01)
- `bulk-rename` — regex rename, dry-run-first (RENM-01)

Four are **read-only**; only `bulk-rename` writes (renames in place). Each command swaps its unit `Commands::*` variant in `src/cli.rs` for a real Args struct implementing `RunCommand`, removes its `not_implemented(...)` arm in `src/main.rs`, and ships with `assert_cmd`/`trycmd` tests. All reuse the Phase-1 core: `core::output` color gating (D-09/D-10 of Phase 1), strict 0/1/2 exit codes, `data→stdout / messages→stderr`, `core::fs::{normalize_path, is_hidden, safe_copy}`, and (for `hash`) `core::input`.

**In scope:** HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 (5 requirements) + the deferred `--file PATH` input layer (Phase 2 D-06, implemented here by `hash`) + promoting flatten's `human_size` helper into `core::output` for shared use.

**Not in scope (v2 deferrals — see REQUIREMENTS.md):** BLAKE3/multi-file/progress for hash (HASH-V2-01); gitignore/dirs-only/sort-by-size for tree (TREE-V2-01); percentage bars/color ranges/apparent-size for du (DU-V2-01); multi-stage hashing/`--delete`/hardlink-aware for dupes (DUPE-V2-01); case transforms/sequential numbering/`--backup`/replace_all for bulk-rename (RENM-V2-01).

</domain>

<decisions>
## Implementation Decisions

### hash — algorithm policy (HASH-01)

- **D-01:** **Default algorithm = SHA-256** (`--algo blake3` switches). This resolves a documented conflict: `.planning/ROADMAP.md` Phase-3 success-criterion #1 and `.planning/REQUIREMENTS.md` HASH-01 (the **binding acceptance contract**) lock SHA-256 default, while `.planning/STATE.md` Key Decisions and `CLAUDE.md` carry an advisory **"BLAKE3 default"** note that predates the lock. The contract wins (and HASH-V2-01 itself files BLAKE3 under *v2*). ⚠️ **The STATE.md/CLAUDE.md "BLAKE3 as default" lines are SUPERSEDED** — read them as "BLAKE3 available via `--algo blake3`". Output format: `<hash>  <filename>` (two spaces, coreutils style).
- **D-02:** **`--algo` set = `{sha256` (default)`, blake3, sha512, md5}`** — the four families a checksum tool is actually asked to match. Crates (locked in CLAUDE.md): `sha2` 0.11.0 (sha256 + sha512), `blake3` 1.8.5, `md-5` (RustCrypto, **hyphenated** — the `md5` crate does not implement `digest`). **Excluded:** `sha1` (broken, rarely requested for checksums), `sha224`/`sha384` (almost never appear in published checksums) — pure bloat; revisit in v2 only if asked. Net new dependency vs the locked stack: only `md-5`.
- **D-03:** **Implementation = enum-dispatch hasher, NOT a unified `dyn Digest`.** RustCrypto algos (sha256/sha512/md5) share one `digest::Digest` code path; **blake3 gets its own arm on the *stable* native `blake3::Hasher`**. Rationale: blake3's `digest::Digest` impl is behind the `traits-preview` feature, which docs.rs explicitly marks *"unstable, may break between patch versions"* — avoid coupling the build to it. Stream every algorithm (no whole-file buffering): blake3 via `Hasher::update_reader`, RustCrypto via incremental `update`.
- **D-04:** **`--verify EXPECTEDHASH` = auto-detect algorithm by the expected hash's hex length**, with `--algo` as an explicit override. Lengths: 32 hex → md5, 64 hex → **sha256** (wins the sha256/blake3 tie; `--algo blake3 --verify …` disambiguates), 128 hex → sha512. **Case-insensitive** compare (lowercase both sides). **Plain `==`, NOT constant-time** — a file checksum is a public integrity value, not a secret. Exit **0 on match / 1 on mismatch** (HASH-01); an expected hash whose length matches no supported algo → exit 2 (bad args, FOUND-03).
- **D-05:** **Input = `core::input` + implement the deferred `--file PATH` layer** (Phase 2 D-06; `hash` is the intended first consumer — see `src/core/input.rs` module docs lines 24-25). The `--file`/positional reader slots in ahead of the stdin branch without reshaping signatures. **stdin filename label = `-`** (coreutils convention; matches the `-` sentinel already wired in `input.rs`). Byte-exact reads (stream/`read_input_bytes`).

### Shared traversal & noise policy (tree / du / dupes)

- **D-06:** **All three read-only walkers reuse `walkdir` + `core::fs::is_hidden` — skip hidden by default.** Consistent with `flatten` (Phase 1 D-12); `is_hidden` is already Windows-correct (`FILE_ATTRIBUTE_HIDDEN`), never prunes the dotted root (walkdir#142), and unit-tested. Dot-prefixed noise (`.git`, `.venv`, `.cache`) is pruned for free. A future **`--all` / `-a`** flag to include hidden is **NOT this phase** — and when added, it must use ONE verb across the toolbox (reconcile with flatten's deferred `--include-hidden`; standardize on `--all`/`-a`). Reuse flatten's `follow_links(false)` for symlink-loop safety.
- **D-07:** **No noise-directory skip list and NO `ignore` crate in Phase 3.** `node_modules`/`target`/`build`/`dist` are shown as literal truth. Rationale: lean-binary goal (the `ignore` crate drags `globset`→`regex-automata`+`aho-corasick` + `crossbeam-*`, and its output is CWD-dependent/non-deterministic), uniform traversal across all three commands, and TREE-V2-01 / DU-V2-01 / DUPE-V2-01 already **defer** gitignore/ignore-pattern handling to v2. CLAUDE.md's `ignore`-for-dupes recommendation is a *v2* feature. (If vendored-dup noise later proves unacceptable, a tiny **dep-free hardcoded skip list scoped to dupes** is the lean v2 entry point — strictly preferred over pulling in `ignore`.)

### tree — render (TREE-01)

- **D-08:** **Sort = directories first, then files, each case-insensitive alphabetical** (the eza `--group-directories-first` convention — reads better than classic `tree`'s mixed alphabetical). Depth-first: print a directory, then immediately descend.
- **D-09:** **Glyphs = standard Unicode box-drawing** `├──`, `└──`, `│  `, `   `. TREE-01 explicitly requires box-drawing characters, and PS7 renders them reliably with the default font (this is distinct from flatten's ASCII `+`/`~`/`-` *status* glyphs — those convey state in a pipe-greppable way; tree's branches convey structure).
- **D-10:** **`--sizes` shows per-file size ONLY** — directories show a blank size column. Recursive dir totals are `du`'s job (and sort-by-size is TREE-V2); auto-aggregating here would double the walk cost and blur the two commands. **`--depth N`** limits displayed depth. **Summary line** `N directories, M files` printed to stdout (GNU `tree` convention; satisfies TREE-01's count summary). Color: directory names only (e.g. `.blue().bold()`); file names + branches default; all through `is_color_on()` so piped output is byte-identical minus ANSI.

### du — disk usage (DU-01)

- **D-11:** **Row model = one row per IMMEDIATE CHILD of the target dir**; directories show their **recursive total**, files show their own size; sorted **biggest-first**. **`--depth N` = reporting/aggregation cap** (how deep totals are rolled up to), **`--top N` = post-sort truncation** of the shown list — both operate on one flat sorted list. A **trailing `/`** (ASCII, source of truth) marks directories so the dir/file distinction survives piping. **Summary line always reflects the FULL scan total**, not just shown rows: `{X} of {Y} entries shown. {TOTAL} total.` Color: the size value only (single accent, e.g. `.cyan()` — NOT color-coded ranges, that's DU-V2); path/name uncolored.

### Size formatting (shared by tree --sizes and du)

- **D-12:** **Promote flatten's existing `human_size` helper into `core::output` and reuse it** — it is ~13 lines, already tested, **1024-based math with decimal-style labels** (`B`/`KB`/`MB`/`GB`/`TB`, the Windows-Explorer hybrid convention this audience reads daily) and lives today at `src/commands/flatten/mod.rs`. **Do NOT add the `humansize` crate** (dep + `libm` bloat for output a tested local fn already produces, and it would change the established label style). Spec: `< 1024` → `{n} B` (no decimal); else `{value:.1} {unit}` dividing by 1024 until `< 1024` or `TB`. `du` right-aligns the size column to the widest shown value.

### dupes — content duplicates (DUPE-01)

- **D-13:** **Identity = size pre-filter, THEN content hash** (REQUIREMENTS-locked): group candidate files by byte-size first, then hash only within same-size groups (most unique files never get hashed). Reuse the `hash` command's hashing infrastructure; **content-equality hash = BLAKE3** (fastest; cryptographic-criticality is irrelevant for equality grouping). Use **`rayon` 1.12** for the parallel content-hash phase (CLAUDE.md). Output = groups of identical files + a **wasted-space summary** (sum of redundant copies). **No deletion or modification** (Out of Scope, locked). Skip hidden (D-06); no noise list (D-07).

### bulk-rename — regex rename (RENM-01)

- **D-14:** **Scope = top-level files of the target dir by default** (matches the ROADMAP `./photos` example, minimal blast radius); **`--recursive` opt-in** reuses flatten's `WalkDir::new(...).follow_links(false).filter_entry(!is_hidden)` walk with `min_depth(1)`, files only. **Collision detection is scoped per containing directory** (two files in different dirs may both legitimately become `img_1.jpg`).
- **D-15:** **Targets = files only.** Directories and symlinks/junctions are skipped (reuse flatten's symlink skip) and shown as `-` rows `(skipped: directory)` / `(skipped: symlink)`. (Renaming dirs mid-walk is hazardous under `--recursive`.)
- **D-16:** **Match target = the FULL base name** (filename including extension). Extension protection comes from **pattern discipline** (e.g. `(\d+)` cannot touch `.jpg`), NOT from pre-splitting the stem (fragile for `.gitignore`, `archive.tar.gz`, extensionless names). `--help` documents that the pattern sees the whole name; the dry-run preview shows exactly what changes.
- **D-17:** **Replacement = `Regex::replace` (FIRST match only)**, regex-crate `$1` / `${1}` capture syntax (matches ROADMAP `img_$1`). `replace_all` would rewrite *every* match (`2024_v2.jpg` → `img_2024_vimg_2.jpg`) — surprising for a destructive op; **`--all` is deferred to v2 (RENM-V2)**. Document the `$1abc` foot-gun in `--help` (`${1}abc` needed; an unbraced `$1abc` parses as group `1abc` → empty; nonexistent group → empty string).
- **D-18:** **Safety model — pre-flight, in-memory, ABORT-ALL-BEFORE-ANY-RENAME.** ⚠️ Load-bearing fact: **`std::fs::rename` SILENTLY OVERWRITES an existing destination on Windows** (maps to `MoveFileExW` replace semantics) — there is **no `create_new` backstop for moves** the way `safe_copy` has one for copies, so correctness rests entirely on pre-flight detection:
  1. Build a **case-folded** (`to_lowercase`, full-Unicode per WR-01) occupied/existing name set **per directory**.
  2. Every planned target is checked against (i) other planned targets AND (ii) pre-existing on-disk names **not** themselves being renamed away → any clobber/collision **aborts the whole batch** (exit 1, nothing written) with all conflicts listed.
  3. **Cycles/swaps** (`a→b`, `b→a`, or any target equal to another item's source) → **DETECT-AND-ABORT** (no two-phase temp-name pass in v1; that's a clean v2 follow-up — silent multi-step magic is exactly what a destructive tool shouldn't do).
  4. **Skip no-op renames** (new name byte-identical) → `-` `(unchanged)`. **EXCEPTION:** a **case-only change** (`foo`→`Foo`) is byte-different and IS a real rename — detect it by comparing **exact (non-folded) names**, so it is not falsely flagged as a self-collision even though both fold to the same key (NTFS is case-insensitive/preserving; `std::fs::rename` updates the stored casing).
- **D-19:** **UX = dry-run preview is the DEFAULT** (writes nothing); **`--force` executes.** Reuse flatten's glyph output (`~` rename, `-` skip; no `+` since nothing is created), aligned `old -> new` arrows via the existing `format_row`/`arrow_col`/`terminal_width` helpers, `[collision]` inline reason, and a parallel dry-run-vs-real summary. Each `std::fs::rename` is `.context(...)`-wrapped per file (FOUND-06 deep-path/permission surfacing); stop on first unexpected I/O error (a *predictable* collision never reaches execution — it aborted pre-flight).

### Claude's Discretion
- Module layout under `src/commands/<cmd>/mod.rs` per command; the exact `core::output` location/signature for the promoted `human_size`; the precise per-directory collision-set data structure in bulk-rename.
- Whether `dupes` shows an optional `indicatif` spinner during hashing (not required for v1 — keep it simple; progress bars are HASH-V2/DUPE-V2).
- Exact `--algo` value spellings and the hash-algorithm enum names; whether `--file` for hash is positional or a flag (as long as it routes through the `core::input` extension point).
- Exact color shades, alignment column widths, and the precise wording of summary/error strings within the locked families.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements & success criteria (the binding contract)
- `.planning/ROADMAP.md` § "Phase 3: Filesystem Power Tools" — goal + the 5 success criteria (exact CLI behaviors: `hash`/`--algo`/`--verify`, `tree`/`--sizes`/`--depth`, `du`/`--top`/`--depth`, `dupes` groups + wasted space, `bulk-rename` dry-run/`--force`/collision-abort).
- `.planning/REQUIREMENTS.md` § "Filesystem Tools" — HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 acceptance criteria; § "v2 Requirements" (HASH-V2-01, TREE-V2-01, DU-V2-01, DUPE-V2-01, RENM-V2-01) for what is explicitly deferred; § "Out of Scope" (dupes auto-delete, bulk-rename execute-by-default, du/dupes TUI).

### Architecture & locked patterns to reuse (do NOT re-derive)
- `.planning/STATE.md` § "Architecture Established", § "Critical Pitfalls to Remember", § "Key Decisions" — RunCommand trait, `src/core/` layout, `dunce::canonicalize`, ANSI bootstrap order, MSVC + crt-static. ⚠️ The **"BLAKE3 as default hash"** Key-Decision line is **SUPERSEDED by D-01** (read as "BLAKE3 via `--algo blake3`").
- `.planning/phases/01-foundation-flatten/01-CONTEXT.md` — D-09/D-10 (glyph-is-truth output + `COLOR_ON` gating, byte-identical-minus-ANSI when piped), D-12 (skip-hidden default), D-15 (collision-rename + reserved-name + case-fold patterns), the dry-run plan→preview→execute split.
- `.planning/phases/02-pure-transform-utilities/02-CONTEXT.md` — D-04/D-05 (`core::input` precedence + `read_input`/`read_input_bytes` shapes), **D-06 (the deferred `--file PATH` layer this phase implements in `hash`)**.

### Source files to read before implementing
- `src/core/fs.rs` — `normalize_path` (dunce), `is_hidden` (the walkdir `filter_entry` predicate all 3 walkers reuse, D-06), `safe_copy` (the `create_new` defense — note it has NO rename equivalent, D-18).
- `src/core/input.rs` — `read_input`/`read_input_bytes` + the `-` sentinel + the `--file` extension point (module docs lines 24-25) for `hash`.
- `src/core/output.rs` — `is_color_on()`, `terminal_width()`, `truncate_middle()`, the owo-colors global-override gating; destination for the promoted `human_size` (D-12).
- `src/commands/flatten/mod.rs` + `src/commands/flatten/rename.rs` — the `human_size` helper (D-12), `format_row`/`arrow_col` output style, the case-folded `occupied`-name collision set, symlink/skip-reason conventions that `bulk-rename` and `du`/`tree` mirror (D-14/D-18/D-19).
- `src/cli.rs` / `src/main.rs` — the `Commands` enum (5 unit variants to swap for real Args structs) + the `not_implemented(...)` dispatch arms to remove.

### Tech stack (locked crate versions — use as-is, do not re-research)
- `CLAUDE.md` (project root) — confirmed versions: `blake3` 1.8.5 (use the **stable native `Hasher`**, not `traits-preview`), `sha2` 0.11.0, `md-5` (RustCrypto, hyphenated), `walkdir` 2.5.0 (already a dep), `rayon` 1.12.0 (dupes), `regex` (bulk-rename), `indicatif` 0.18.4 (optional/deferred); plus the "What NOT to Use" table (`humansize` is implicitly excluded by D-12; `ignore` crate deferred by D-07).

**No external ADRs/specs exist** — all decisions are captured above and in the files listed.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `core::fs::is_hidden` — the exact `walkdir` `filter_entry` predicate `tree`/`du`/`dupes` use to skip hidden by default (D-06); Windows-correct, root-safe, unit-tested.
- `core::fs::normalize_path` (dunce) — UNC-safe canonicalization for every path argument.
- `core::fs::safe_copy` — its `create_new` overwrite-refusal is the *model* for write safety, but note **it has no `std::fs::rename` analog** — bulk-rename must do pre-flight collision detection instead (D-18).
- `core::input::{read_input, read_input_bytes}` + `-` sentinel — `hash`'s input path; the `--file PATH` extension point (D-06/D-05) slots in here.
- `core::output::{is_color_on, terminal_width, truncate_middle}` + owo-colors global-override gating — every styled output (tree colors, du size accent) routes through this so piped output is byte-identical minus ANSI.
- `flatten`'s `human_size` (1024-based, tested) — promote to `core::output` and reuse for tree/du sizes (D-12).
- `flatten`'s `format_row`/`arrow_col` + glyph output + case-folded `occupied` set — `bulk-rename`'s preview and collision model reuse these (D-19/D-18).

### Established Patterns
- `RunCommand` trait + static dispatch in `src/commands/mod.rs` and `src/main.rs` — each new command follows it; remove the `not_implemented(...)` arm and swap the `src/cli.rs` unit variant.
- Glyph-is-source-of-truth output, color as decoration, byte-identical-minus-ANSI when piped (Phase 1 D-09/D-10); `data→stdout`, `messages→stderr`, exit 0/1/2.
- Dry-run plan→preview→execute split (flatten) — `bulk-rename` inherits it (D-19).
- Tests: integration via `assert_cmd` in `tests/<cmd>.rs`; CLI snapshots via `trycmd` (note trycmd normalizes `\`→`/` in Windows path snapshots).

### Integration Points
- New deps to add to `Cargo.toml`: `blake3`, `sha2`, `md-5` (hash/dupes), `rayon` (dupes), `regex` (bulk-rename). `walkdir` already present.
- `core::output` gains the promoted `human_size` (shared by tree + du).
- `core::input` gains the `--file PATH` layer (first consumer: `hash`).
- Each command removes one `not_implemented(...)` arm in `src/main.rs` and swaps one unit variant in `src/cli.rs`.

</code_context>

<specifics>
## Specific Ideas

- `hash` output format: `<hash>  <filename>` (two spaces, coreutils style); stdin label is `-`.
- `du ./project --depth 1 --top 5` target output (dirs show recursive total, biggest-first, trailing `/` marks dirs, summary reflects full-scan total):
  ```
  112.4 MB  node_modules/
   38.7 MB  target/
    4.2 MB  assets/
  312.0 KB  Cargo.lock
   18.5 KB  README.md

  5 of 9 entries shown. 153.6 MB total.
  ```
- `bulk-rename` dry-run preview (default; `~` rename, `-` skip; aligned `old -> new`):
  ```
  ~ IMG_0042.jpg          -> img_0042.jpg
  ~ IMG_0043.jpg          -> img_0043.jpg
  - notes.txt                                  (unchanged)
  - subfolder                                  (skipped: directory)

  Dry run: 2 to rename, 1 unchanged, 1 skipped. Re-run with --force to apply.
  ```
- `bulk-rename` collision abort (exit 1, nothing written, in BOTH dry-run and `--force`):
  ```
  ~ IMG_0042.jpg          -> img_0001.jpg
  ~ IMG_0099.jpg          -> img_0001.jpg   [collision]

  Aborted: 1 collision detected. img_0042.jpg and img_0099.jpg both rename to img_0001.jpg. No files were renamed.
  ```
- `tree` uses Unicode box-drawing (`├── └── │`); dirs-first sort; `--sizes` per-file only; summary `N directories, M files`.

</specifics>

<deferred>
## Deferred Ideas

- **Reconcile stale "BLAKE3 default" notes** in `.planning/STATE.md` Key Decisions and `CLAUDE.md` to "BLAKE3 via `--algo blake3`" (action for the planner/transition — D-01 supersedes them; not a feature).
- **`--all` / `-a` show-hidden flag** for tree/du/dupes — v2; when added, standardize ONE "show hidden" verb across the toolbox (reconcile with flatten's deferred `--include-hidden`).
- **Noise-directory skipping** (hardcoded list or `.gitignore` via the `ignore` crate) for du/dupes — deferred per TREE-V2-01/DU-V2-01/DUPE-V2-01; the lean v2 entry point is a dep-free hardcoded list scoped to dupes, NOT the `ignore` crate.
- **Two-phase temp-name swap/cycle rename** for bulk-rename (a→b, b→a) — v1 detects-and-aborts; auto-resolution is a v2 follow-up.
- **bulk-rename `--all` (replace_all)**, case transforms, sequential numbering, `--backup`, `--json` — RENM-V2-01.
- **hash**: BLAKE3-as-default reconsideration, multi-file hashing, `--json`, progress bar, plus `sha1`/`sha224`/`sha384` algos — HASH-V2-01.
- **du** percentage bars / color-coded ranges / apparent-size; **tree** gitignore/dirs-only/sort-by-size — DU-V2-01 / TREE-V2-01.
- **indicatif progress** for dupes scan / large-file hash — v2.

### Reviewed Todos (not folded)
None — `gsd-sdk query todo.match-phase 3` returned 0 matches.

</deferred>

---

*Phase: 3-filesystem-power-tools*
*Context gathered: 2026-06-22*
