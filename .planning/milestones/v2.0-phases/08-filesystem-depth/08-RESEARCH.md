# Phase 8: Filesystem Depth - Research

**Researched:** 2026-06-27
**Domain:** Per-command depth flags on six already-shipped filesystem commands of a Windows-PowerShell-7 Rust CLI (`box`), riding the frozen Phase-6/7 `--json`/`--clip` spine
**Confidence:** HIGH (every claim anchored to verified source line numbers + crates.io/docs.rs/std API checks; one HIGH-impact correction to the project's own STATE.md/PITFALLS.md surfaced below)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward — LOCKED upstream (NOT re-decided this phase):**
- **Spine is frozen** (Phase 6/7): every new field feeds ONE `#[derive(Serialize)]` struct → both human render AND `emit_json` (no-drift); `is_json_on()` fork happens FIRST; progress (indicatif) → stderr, suppressed under `--json`; piped/`--json` output byte-identical-minus-ANSI; multi-row commands keep the `{results,count}` root (+ locked sibling totals); `tree` keeps its recursive-node root-rule exception.
- **Field policies (D-3/D-4):** large numbers = bare `u64` (document the >2^53 caveat); non-UTF-8 paths = `to_string_lossy()`, NEVER `to_str().unwrap()`.
- **`du --on-disk` dependency (D-2):** GO — `windows 0.61` (`GetCompressedFileSizeW`), pinned to unify with the transitive `windows ^0.61` from `tauri-winrt-notification 0.7.2`.
- **`dupes --delete` safety model (D-5):** GO, safe + non-interactive — keep-first in the sorted (deterministic) path order, dry-run default, `--force`, hardlink-aware, abort-all-before-any pre-flight, snapshot-the-tree tests, mandatory adversarial review. No interactive prompt, no `--keep <dir>` this phase.
- **`flatten --move` mechanics (SC2):** dry-run default; `--force` runs copy → verify (dest exists + size matches) → delete-source; every abort path leaves the source tree byte-for-byte unchanged. Reuse the existing collision-rename + containment-guard from the copy path. Empty source directories left in place after a move (only files relocate).
- **`dupes` multi-stage identity (DUPE-V2-01):** size → partial-hash → full-BLAKE3 cascade, reusing the existing rayon parallel-hash path; hardlink-aware = paths sharing one NTFS file-index are COLLAPSED (counted once, never reported as wasted space).

**D-20 — `tree` gitignore:** opt-in `--gitignore` flag (default output UNCHANGED). Mechanism = layer an `ignore::gitignore::Gitignore` matcher as a FILTER inside `read_children` (NOT `ignore::WalkBuilder`) so the single chokepoint feeding both `render_dir` and `build_node` stays no-drift. `--ignore <glob>` folds into the SAME matcher via `GitignoreBuilder::add_line`; `--dirs-only` is a `child.is_dir` filter applied AFTER the ignore filter; **`--sort size`** is an explicit flag that REPLACES the D-08 dirs-first comparator only when set (tie-break alpha; dirs carry `size: None` → sort to a defined end). `core::fs::is_hidden` (D-06) and the D-08 comparator stay untouched. New dep `ignore = "0.4"`. Nested-`.gitignore` correctness (eza #1086 class) is the main test target; use `matched(rel_path, is_dir)` with paths rooted relative to the tree target.

**D-21 — `bulk-rename` case + numbering:** `--case upper|lower|title` applied to the whole resulting name AFTER `re.replace` (NOT replacement-string escapes — the `regex` crate has no `\U`/`\L`). `{n}` is a literal token in the `replacement` string, expanded in a small post-replace pass (≠ regex `${n}`); counter assigned over the deterministic SORTED plan order. `--number-width` (zero-pad, or auto-width from file count) + `--start` / `--step`. Apply order: `re.replace` → `{n}` expansion → `--case` fold. `title` operates on the STEM. Every generated name still flows through the existing per-directory ABORT-ALL pre-flight unchanged. No new dep.

**D-22 — `bulk-rename --backup`:** write a JSON undo MANIFEST, NOT file copies (a pure rename is reversible from the `{old → new}` name map alone). One `{old, new}` (absolute paths) record per affected file + an `applied: bool` per entry, in `%LOCALAPPDATA%\box\undo\<id>.json` (OUTSIDE the renamed tree so `--recursive` never re-walks it and it survives renaming the target dir); echo the manifest path to stderr; fallback to the target dir only if `LOCALAPPDATA` unset. Direct serde projection of the existing build-once `Plan`. Write + fsync the FULL manifest BEFORE the first `fs::rename`, then flip each entry `applied: true` as its rename returns. `--backup` is orthogonal to and only meaningful with `--force` (no-op on dry-run). Deferred: an automated `box bulk-rename --undo` replay subcommand.

**D-23 — `du` percentage + colors + on-disk + exclude:** Color = percentage-of-total bucketing (`>50%` red, `10–50%` yellow, else default), REPLACING the lone `.cyan()` accent (size-value-only; name uncolored; gated on `is_color_on()`). Percentage basis = the full-scan TOTAL captured BEFORE `--top` truncation; right-aligned `42.3%` fixed-width column BEFORE the size value; tiny-but-nonzero row → `<0.1%`, genuine zero-byte row → `0.0%`. `--on-disk` = recursive sum of each descendant file's `GetCompressedFileSizeW` allocated size (paralleling `dir_total`); when set, the percentage basis switches to the on-disk total. `--json`: the `size` field carries on-disk bytes when `--on-disk` is set, plus a top-level `on_disk: bool` marker. `--exclude <glob>` = `globset`, matched against the path relative to the target root, applied to BOTH immediate children AND recursive descendants.

### Claude's Discretion
- **`hash` multi-file partial-failure policy (HASH-V2-02):** lean coreutils best-effort — print each readable file's digest, stderr error per unreadable file, exit 1 if any failed. Planner may instead choose fail-fast-abort. Default is best-effort-continue.
- **`flatten` filter semantics (FLAT-V2-01):** `--extensions <list>` = comma-separated, case-insensitive, final-extension match; `--separator <str>` = the collision-encoding join char (currently `_`); `--include-hidden` flips the `is_hidden` prune. Exact spellings/parsing are the planner's call.
- **"Large input" progress threshold (HASH-V2-02 / FLAT-V2-01):** planner picks a sensible byte/file-count cutoff; never under `--json`; always stderr.
- **`{n}` literal-brace escaping, `title`-case algorithm, exact du color band cutoffs (50% / 10%)** — taste calls, keep documented and tested.
- **New deps to add this phase:** `indicatif 0.18.4`, `windows 0.61`, `ignore 0.4`, `globset 0.4`.

### Deferred Ideas (OUT OF SCOPE)
- Automated `box bulk-rename --undo` replay subcommand (RENM-V2-02 needs only the backup written + dir recoverable).
- `dupes` interactive review UI + `--keep <dir>` selection / cross-volume dedupe-to-hardlinks (DUPE-V3).
- `du` continuous truecolor gradient / visual size bars.
- `tree --gitignore` on-by-default (revisit as a `box config` key in Phase 11).
- Any change to the frozen `--json`/`--clip`/config spine — Phase 8 only ADDS fields/flags onto it.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HASH-V2-02 | `hash` accepts multiple file args + coreutils `digest␣␣filename` per line + stderr progress for large inputs | Multi-file loop into the existing `{results,count}` `HashOutput`; the two-space row format is ALREADY at `hash/mod.rs:281`; `read_file_or_stdin` per path; indicatif on stderr (see hash section) |
| FLAT-V2-01 | `flatten` gains `--extensions`, `--separator`, `--include-hidden`, stderr progress bar | All fold into `build_plan`'s walk (`flatten/mod.rs:326-399`); `--separator` threads into `encode_relative`/`dedupe` (`rename.rs`); progress over the known `plan.items.len()` on the real-run loop |
| FLAT-V2-02 | `flatten --move` = copy→verify→delete; dry-run default, `--force` | NEW plan reusing `safe_copy` (`core::fs:80`) + containment guard + collision-rename; verify via dest metadata; snapshot-tree tests per abort path (own plan + adversarial review) |
| DUPE-V2-01 | `dupes` multi-stage (size→partial→full BLAKE3) + hardlink-aware | Insert a partial-hash stage between `collect_by_size` (`dupes:181`) and the rayon full-hash (`dupes:123`); hardlink collapse via `GetFileInformationByHandle` (see correction below) |
| DUPE-V2-02 | `dupes --delete` keep-first, dry-run default, `--force`, hardlink-safe, abort-all-before-any | Keep-first over the already-sorted groups (`dupes:134`); pre-flight + snapshot-tree tests (own plan + adversarial review) |
| RENM-V2-01 | `bulk-rename` case transforms + `{n}` numbering | Post-passes at the `re.replace` site (`bulk_rename:513`); counter over sorted plan; no new dep |
| RENM-V2-02 | `bulk-rename --backup` pre-rename backup, dir recoverable on failure | JSON undo manifest = serde projection of `Plan`; write+fsync before the executor loop (`bulk_rename:366-410`); own plan + adversarial review |
| TREE-V2-01 | `tree` gitignore + `--dirs-only` + `--ignore <glob>` + sort-by-size | `ignore::Gitignore` matcher-as-filter inside `read_children` (`tree:279-310`); `--sort size` swaps `sort_children` (`tree:313`) |
| DU-V2-01 | `du` percentage column + color-coded ranges + `--exclude <glob>` | Percentage from the full-scan total (`du:120`); band color REPLACES `color_size`'s `.cyan()` (`du:249-255`); `globset` filter in `collect_rows`/`dir_total` |
| DU-V2-02 | `du --on-disk` allocated/compressed NTFS size via `GetCompressedFileSizeW` | Win32 per-file path paralleling `dir_total` (`du:216-234`); `windows 0.61` `Win32_Storage_FileSystem`; `on_disk: bool` in JSON |
</phase_requirements>

## Summary

Phase 8 is "deepening, not rewrite." Every one of the six commands already has its `#[derive(Serialize)]` output struct from Phase 7, so each new depth flag adds a field/argument that the existing `is_json_on()` fork carries into both the human render and `emit_json` for free — no-drift is structural, not a thing the planner must re-engineer. The hard design choices are LOCKED in CONTEXT.md (D-20…D-23 + carried-forward locks). The planner's job is to slot new arguments into the existing `clap` `Args` structs, new logic into the existing pure plan/walk functions at the verified line anchors below, and a per-requirement test seam (Validation Architecture) — most importantly the snapshot-the-tree-unchanged assertion for every abort path of the three destructive flags.

**One HIGH-IMPACT CORRECTION to the project's own docs:** STATE.md (line 113) and PITFALLS.md (Pitfall 5) instruct using `std::os::windows::fs::MetadataExt::file_index()` / `volume_serial_number()` / `number_of_links()` for `dupes` hardlink detection. **These three methods are nightly-only — unstable, behind `#![feature(windows_by_handle)]`, tracking issue rust-lang/rust#63010, still OPEN as of June 2026** `[VERIFIED: doc.rust-lang.org/std + github.com/rust-lang/rust/issues/63010]`. `box` builds on **stable** MSVC, so these cannot be called. The correct stable path is `GetFileInformationByHandle` from the `windows 0.61` crate (same `Win32_Storage_FileSystem` feature module already being added for `du --on-disk`), reading `BY_HANDLE_FILE_INFORMATION { dwVolumeSerialNumber, nFileIndexHigh, nFileIndexLow, nNumberOfLinks }` off an open `File` handle. This converts a "pure std" assumption into a localized Win32 wrapper — but it costs no new dependency because `du --on-disk` already pulls the exact feature.

**Primary recommendation:** Plan the six commands in the roadmap's six-plan structure (08-01 hash+flatten / 08-02 tree+du / 08-03 dupes-multistage+bulk-rename-case / 08-04 flatten --move / 08-05 dupes --delete / 08-06 bulk-rename --backup). Add the four deps (`indicatif 0.18.4`, `windows 0.61` direct with `Win32_Storage_FileSystem`+`Win32_Foundation`, `ignore 0.4`, `globset 0.4`) once, up front, in plan 08-01 or 08-02 so later plans never touch the manifest. Use the verified line anchors as surgical insertion points; reuse the listed functions VERBATIM.

## Architectural Responsibility Map

`box` is a single-process CLI, not a multi-tier service — the "tiers" are its internal layers. Each capability maps to the layer that owns it, so the planner can verify a flag's logic lands in the right module.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| New flags / arg parsing | `clap` derive on each `Args` struct | `cli.rs` (global `--json`/`--clip` already wired) | Per-command args live on the command's own struct; globals are untouched (Phase 8 adds NO `cli.rs`/`main.rs` plumbing) |
| Output shape (human + JSON) | `core::output` (consume, don't extend) | per-command `#[derive(Serialize)]` struct | Spine is frozen; new fields ride existing structs; `emit_json`/`out_line`/`is_json_on` reused verbatim |
| Filesystem walk + plan building | per-command pure fns (`build_plan`, `collect_rows`, `read_children`, `collect_by_size`) | `walkdir` / `core::fs::is_hidden` | Depth filters fold into the existing single-source-of-truth walk so human + JSON cannot drift |
| Destructive execution | per-command executor loop (own plan) | `core::fs::safe_copy`, `std::fs::rename`/`remove_file` | Dry-run-default → `--force` → abort-all-before-any pre-flight template (proven in bulk-rename) |
| Gitignore / glob matching | `ignore 0.4` / `globset 0.4` (runtime deps) | matcher threaded into the per-command walk | BurntSushi crates; matcher-as-filter (NOT WalkBuilder) preserves the shared chokepoint |
| On-disk size / hardlink identity | `windows 0.61` Win32 FFI, localized to `du`/`dupes` modules | `std::os::windows::io::AsRawHandle` bridge | Windows-specific; isolate FFI in a tiny safe wrapper per the v1 arboard/winrt pattern |
| Progress indication | `indicatif 0.18.4` → stderr only | `std::io::stderr().is_terminal()` gate | Never stdout; `ProgressDrawTarget::hidden()` under `--json` (Pitfall 1) |
| Backup journal | `bulk-rename` module + serde | `%LOCALAPPDATA%\box\undo\` (NOT `%APPDATA%`) | Local (non-roaming) scope, outside the renamed tree |

## Standard Stack

### Core (new this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `indicatif` | 0.18.4 | stderr progress bars for `hash`/`flatten` | The canonical Rust CLI progress crate; named in CLAUDE.md's stack but never pulled in v1 — this is its first real use `[VERIFIED: crates.io 0.18.4]` |
| `ignore` | 0.4 (0.4.26) | `tree --gitignore` matcher-as-filter | BurntSushi, pure-Rust, `+crt-static`/PS7-clean (no DLL). Already in the build graph as a **dev**-dep (via assert_fs→globwalk) — Phase 8 promotes it to a runtime dep `[VERIFIED: crates.io 0.4.26 + cargo tree]` |
| `globset` | 0.4 (0.4.18) | `du --exclude <glob>` matching | BurntSushi, the glob engine inside `ignore`; same dev→runtime promotion `[VERIFIED: crates.io 0.4.18 + cargo tree]` |
| `windows` | 0.61 (resolved 0.61.3) | `du --on-disk` (`GetCompressedFileSizeW`) + `dupes` hardlink identity (`GetFileInformationByHandle`) | Microsoft-official Win32 binding; **already compiled in the tree** transitively via `tauri-winrt-notification 0.7.2` — adding it as a DIRECT dep with `Win32_Storage_FileSystem` unions features into the SAME 0.61.3 build (D-2 pin rationale, verified by `cargo tree -i windows`) `[VERIFIED: crates.io + cargo tree -i windows → v0.61.3]` |

### Supporting (already present — reuse verbatim, NO manifest change)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `walkdir` | 2.5 | recursive/depth-1 directory iteration | Every command's existing walk; depth filters fold into it |
| `rayon` | 1.12 | parallel hashing | `dupes` partial + full hash stages |
| `blake3` | 1.8.5 | content hashing | `dupes` partial (over `Read::take(N)`) + full; `hash` multi-file |
| `regex` | 1.12.4 | `bulk-rename` pattern | `--case`/`{n}` are POST-passes; regex semantics untouched |
| `serde`/`serde_json` | 1 / 1.0.150 | `--json` structs + `--backup` manifest | New fields on existing structs; manifest is a serde projection of `Plan` |
| `dunce` | 1.0 | UNC-safe canonicalization | `normalize_path`, reused in every command |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `windows` `GetFileInformationByHandle` | `std MetadataExt::file_index()` | **REJECTED — nightly-only (issue #63010 open).** `box` is stable MSVC; this is not callable. See correction. |
| `windows` `GetCompressedFileSizeW` | `filesize` crate | `filesize` is unmaintained since 2020-03-19; wraps the same API. Call it directly. |
| `ignore::Gitignore` matcher-as-filter | `ignore::WalkBuilder` | WalkBuilder is a recursive walker; it would force re-architecting BOTH `render_dir` and `build_node` recursions (no-drift regression risk on the frozen JSON node tree). D-20 locks matcher-as-filter. |
| `indicatif` | hand-rolled `\r` spinner | indicatif handles the stderr draw target + non-TTY suppression correctly; hand-rolling re-derives the `--json` contamination guard. |

**Installation (one manifest edit, ideally in the first Phase-8 plan):**
```toml
# Progress bars — hash/flatten. MUST draw to stderr; hidden under --json.
indicatif = "0.18.4"
# tree --gitignore matcher + du --exclude globset (promote from dev-dep to runtime).
ignore = "0.4"
globset = "0.4"
# Win32: GetCompressedFileSizeW (du --on-disk) + GetFileInformationByHandle (dupes hardlink).
# Pin 0.61 to unify with tauri-winrt-notification's transitive windows ^0.61 (D-2).
windows = { version = "0.61", features = [
    "Win32_Storage_FileSystem",   # GetCompressedFileSizeW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION
    "Win32_Foundation",           # GetLastError, HANDLE, INVALID_FILE_SIZE, BOOL
] }
```

**Version verification performed:** `cargo search` confirmed `ignore = "0.4.26"`, `globset = "0.4.18"`, `indicatif = "0.18.4"` current on crates.io (2026-06-27). `cargo tree -i windows` confirmed `windows v0.61.3` already resolved via `tauri-winrt-notification 0.7.2`. `cargo tree -i ignore` / `-i globset` confirmed both present today ONLY as dev-dependencies (assert_fs → globwalk) — they MUST be added to `[dependencies]` to be usable at runtime.

## Package Legitimacy Audit

> slopcheck targets npm/PyPI hallucination vectors and is not applicable to a Rust/cargo project; it was not run. Legitimacy is instead established via crates.io presence + the fact that three of the four crates are already in the resolved dependency graph + their authorship (BurntSushi, console-rs, Microsoft) + their sourcing from the project's OWN CLAUDE.md/STACK.md (authoritative project docs).

| Package | Registry | Age | Author/Source | Already in tree? | Disposition |
|---------|----------|-----|---------------|-----------------|-------------|
| `indicatif` 0.18.4 | crates.io | mature (years) | console-rs | No (first use) | Approved — canonical, named in CLAUDE.md |
| `ignore` 0.4.26 | crates.io | mature | BurntSushi (ripgrep) | Yes (dev-dep) | Approved — promote to runtime |
| `globset` 0.4.18 | crates.io | mature | BurntSushi (ripgrep) | Yes (dev-dep, via ignore) | Approved — promote to runtime |
| `windows` 0.61.3 | crates.io | actively maintained | Microsoft (official) | Yes (transitive, 0.61.3) | Approved — add direct, union features |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none. All four are top-tier, widely-depended-upon crates from known authors; three are already compiled in this project's build graph.

## Architecture Patterns

### System Architecture Diagram

```
box <fs-cmd> [depth flags] [--json] [--clip]
        │
   main.rs: init_config → init_color → init_output(json,clip)   (UNCHANGED by Phase 8)
        │
        └─ Commands::<Cmd>(args).run()
               │
               ├─ parse depth flags off the command's own Args struct  ← clap derive
               │
               ├─ WALK / PLAN  (the single source of truth)
               │     walkdir + core::fs::is_hidden + NEW filters:
               │       ├─ flatten: --extensions / --include-hidden  → build_plan
               │       ├─ tree:    Gitignore matcher + --dirs-only   → read_children  (chokepoint)
               │       ├─ du:      globset --exclude                 → collect_rows / dir_total
               │       └─ dupes:   size → partial-hash → full-hash   → collect_by_size + rayon
               │
               ├─ TRANSFORM (per command)
               │       ├─ hash:        loop paths → digest_reader each
               │       ├─ du:          percentage = size / full_total ; band color ; on-disk via Win32
               │       ├─ bulk-rename: re.replace → {n} expand → --case fold (over SORTED plan)
               │       └─ dupes:       hardlink collapse via GetFileInformationByHandle
               │
               ├─ PRE-FLIGHT (destructive only — abort-all-before-any, pure, I/O-free)
               │       flatten --move / dupes --delete / bulk-rename --backup
               │
               ├─ is_json_on() FORK  (FIRST — before any human chrome / color / progress)
               │       ├─ true:  emit_json(&OneStruct)                ← stdout = one document
               │       └─ false: out_line / println! human render + indicatif progress → stderr
               │
               └─ main.rs: flush_clip()  (UNCHANGED)
```

### Recommended Project Structure (no new modules required)
```
src/commands/
├── hash/mod.rs          # + paths: Vec<String>; loop; progress
├── flatten/mod.rs       # + --extensions/--separator/--include-hidden/--move; progress
├── flatten/rename.rs    # encode_relative/dedupe take a separator param
├── dupes/mod.rs         # + partial-hash stage; hardlink collapse; --delete
├── bulk_rename/mod.rs   # + --case/{n}/--number-width/--start/--step; --backup manifest
├── tree/mod.rs          # + --gitignore/--ignore/--dirs-only/--sort; matcher in read_children
└── du/mod.rs            # + percentage column; band color; --on-disk (Win32); --exclude
src/core/
├── fs.rs                # OPTIONAL: add file_identity() Win32 wrapper here, or keep in dupes
└── (output.rs, input.rs UNCHANGED — consume only)
```
> Localize Win32 FFI to the owning command module (the v1 arboard/winrt pattern), OR add a tiny `core::fs::compressed_size(path)` + `core::fs::file_identity(path)` pair if both `du` and `dupes` want to share the handle-open boilerplate. Either is acceptable; keep `unsafe` minimal and wrapped.

### Pattern 1: New flag folds into the existing single-source walk (no-drift by construction)
**What:** A filter flag must change what BOTH the human render and the JSON path see — so it goes into the ONE function they share, never into the render.
**When to use:** `flatten --extensions/--include-hidden`, `tree --gitignore/--dirs-only/--sort`, `du --exclude`.
**Example (tree, the chokepoint — `tree/mod.rs:279-310`):**
```rust
// read_children is called per-directory by BOTH render_dir (human) and build_node (JSON).
// Layer the matcher + filters INSIDE it so neither recursion can diverge.
fn read_children(dir: &Path, matcher: Option<&Gitignore>, dirs_only: bool) -> anyhow::Result<Vec<Child>> {
    let mut children = Vec::new();
    for entry in WalkDir::new(dir).min_depth(1).max_depth(1)
        .follow_links(false).into_iter()
        .filter_entry(|e| !is_hidden(e))            // D-06 unchanged
    {
        let entry = entry?;
        let is_dir = entry.file_type().is_dir();
        // NEW: gitignore filter (rel path rooted at the tree target), then --dirs-only.
        if let Some(gi) = matcher {
            // matched() returns Match::Ignore / Whitelist / None  (ignore 0.4)
            if gi.matched(entry.path(), is_dir).is_ignore() { continue; }
        }
        if dirs_only && !is_dir { continue; }
        // ... unchanged Child construction ...
    }
    sort_children(&mut children, sort_mode);          // --sort size swaps comparator
    Ok(children)
}
```

### Pattern 2: Destructive flag = dry-run default → `--force` → abort-all-before-any (proven in bulk-rename)
**What:** Compute the FULL plan as a pure I/O-free pass, run a pre-flight that returns every conflict, and refuse the ENTIRE operation if any conflict exists — never partially apply.
**When to use:** `flatten --move`, `dupes --delete`, `bulk-rename --backup` (all three Phase-8 destructive plans).
**Example (the template, `bulk_rename/mod.rs:300-359`):** `Regex::new` → `build_plan` → `preflight_plan` → if conflicts: print plan (only `if !is_json_on()`) + `bail!` (stdout stays empty under `--json`, D-09) → else dry-run preview unless `--force`. Mirror this exact skeleton for `flatten --move` and `dupes --delete`.

### Pattern 3: Win32 FFI localized behind a safe wrapper
**What:** Wrap each Win32 call so the `unsafe` block and the error/last-error handling live in one tiny function; the command logic calls a safe `anyhow::Result`-returning fn.
**Example (du on-disk):**
```rust
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::GetCompressedFileSizeW;
use windows::Win32::Foundation::{GetLastError, INVALID_FILE_SIZE, NO_ERROR};

/// Allocated/compressed on-disk size of one file (NTFS sparse/compressed-aware).
fn compressed_size(path: &Path) -> anyhow::Result<u64> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut high: u32 = 0;
    // SAFETY: wide is null-terminated; high is a valid out-param.
    let low = unsafe { GetCompressedFileSizeW(PCWSTR(wide.as_ptr()), Some(&mut high)) };
    if low == INVALID_FILE_SIZE {
        // INVALID_FILE_SIZE is also a legal low-dword; disambiguate via GetLastError.
        let err = unsafe { GetLastError() };
        if err != NO_ERROR {
            anyhow::bail!("GetCompressedFileSizeW failed for {}: {:?}", path.display(), err);
        }
    }
    Ok(((high as u64) << 32) | (low as u64))
}
```
> Signature verified: `GetCompressedFileSizeW(lpfilename: impl Param<PCWSTR>, lpfilesizehigh: Option<*mut u32>) -> u32` `[VERIFIED: microsoft.github.io/windows-docs-rs]`. A directory has no intrinsic compressed size — for a directory row, RECURSE and sum each descendant file's `compressed_size`, exactly paralleling `dir_total` (`du:216-234`).

### Anti-Patterns to Avoid
- **`std MetadataExt::file_index()` on stable** — does not compile on stable (nightly-only). Use `GetFileInformationByHandle`.
- **`ignore::WalkBuilder` for tree** — re-architects both recursions; D-20 forbids it.
- **Building human text and JSON separately** — the frozen no-drift rule; one struct feeds both.
- **Progress on stdout** — corrupts `--json`; always `ProgressDrawTarget::stderr()`, hidden under `--json`.
- **`fs::rename`/`remove_file` inside a loop with no prior full-plan pre-flight** — the silent-data-loss class; compute the whole plan first.
- **`to_str().unwrap()` on any path field** — non-UTF-8 NTFS names panic; always `to_string_lossy()` (D-4).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| `.gitignore` semantics (negation, nesting, anchoring) | a custom glob/ignore parser | `ignore::Gitignore` + `GitignoreBuilder` | gitignore precedence rules (`!` whitelists, `/`-anchoring, `**`) are subtle; BurntSushi's is the reference impl |
| glob matching for `--exclude` | manual `*`/`?` string matching | `globset::Glob` / `GlobSet` | edge cases (`**`, char classes, path separators) are a footgun |
| on-disk/compressed NTFS size | reading cluster size + doing math | `GetCompressedFileSizeW` | sparse + NTFS-compression + cluster rounding all handled by the API |
| hardlink identity | comparing paths or inode guesses | `GetFileInformationByHandle` `(dwVolumeSerialNumber, nFileIndex*)` | content-equality ≠ same-file; only the file index identifies a shared inode |
| progress bar redraw / TTY detection | `\r` + manual cursor math | `indicatif` + `ProgressDrawTarget::stderr()` | redraw, non-TTY suppression, and rate-limiting are solved |
| case-fold for collision keys | `to_ascii_lowercase` | `to_lowercase()` (already used) | NTFS folds the full Unicode case table (WR-01); ASCII-only misses `RÉSUMÉ` vs `résumé` |
| undo for a rename | copying file bytes | a JSON `{old,new}` manifest (D-22) | a rename changes only the name; the name map IS the reversible state |

**Key insight:** Four of the six commands' depth flags are "thread a well-known matcher/Win32 call into the existing walk." The only genuinely new ALGORITHM is `dupes`' partial-hash stage (a re-bucketing between size and full hash) and `bulk-rename`'s `{n}` counter over the sorted plan — both are small, pure, unit-testable insertions.

## Common Pitfalls

### Pitfall 1: `std::os::windows::fs::MetadataExt` handle fields are nightly-only (CORRECTS project docs)
**What goes wrong:** Following STATE.md:113 / PITFALLS.md Pitfall 5 verbatim, the executor writes `fs::metadata(path).file_index()` and the build fails on stable with "use of unstable library feature 'windows_by_handle'".
**Why it happens:** The project's research docs asserted these were stable std; they are not — issue rust-lang/rust#63010 is OPEN, B-unstable, as of June 2026.
**How to avoid:** Use the `windows` crate: open the file, get its raw handle via `std::os::windows::io::AsRawHandle::as_raw_handle()`, cast to `windows::Win32::Foundation::HANDLE`, call `GetFileInformationByHandle(handle, &mut info)`, read `info.dwVolumeSerialNumber`, combine `info.nFileIndexHigh`/`nFileIndexLow` into a `u64`, and `info.nNumberOfLinks`. Same `Win32_Storage_FileSystem` feature already needed for `du --on-disk` — no extra dep.
**Warning signs:** A compile error mentioning `windows_by_handle`; a CI that only passes on a nightly toolchain.

### Pitfall 2: `--json` stdout contamination from new progress / color / percentage chrome
**What goes wrong:** indicatif draws to stdout, or the du percentage band color emits ANSI under `--json`, breaking `ConvertFrom-Json`.
**How to avoid:** indicatif → `ProgressDrawTarget::stderr()` and `ProgressBar::hidden()` (or skip entirely) when `is_json_on()`; the percentage-band color is gated on `is_color_on()` which `init_output` already forces false under `--json`/`--clip`. The `is_json_on()` fork is checked FIRST in every command (already the case). Copy the existing `json_purity` test (no `0x1B`, no BOM, one parseable value) for each new flag combination (`du --on-disk --json`, `tree --dirs-only --json`, etc.).
**Warning signs:** Progress `\r` fragments in a redirected `> out.json`; ANSI inside JSON string values.

### Pitfall 3: float `NaN`/`Infinity` in the du percentage (divide-by-zero)
**What goes wrong:** An empty directory (full-scan total = 0) makes `size/total` produce `NaN`; if a percentage ever reached a JSON float it would be invalid JSON.
**How to avoid:** Guard `total == 0` → render every row as `0.0%`. (CONTEXT keeps percentage a RENDER value derived at print time, not a stored JSON float — so the JSON channel is unaffected; the consumer derives `size/total_bytes`. Do NOT add a raw `f64` percent field to the JSON struct.)
**Warning signs:** `0 B` directory printing `NaN%`.

### Pitfall 4: nested `.gitignore` not respected (the eza #1086 bug class)
**What goes wrong:** A single root `Gitignore` ignores patterns in a subdirectory's own `.gitignore`, so `tree --gitignore` over-shows or under-shows in nested dirs.
**Why it happens:** `Gitignore::new(root/.gitignore)` only knows the root file. `box`'s `read_children` is called per-directory, so the matcher must accumulate the ancestor chain.
**How to avoid:** Thread a matcher down the recursion that includes every `.gitignore` from the tree root to the current directory. Two viable shapes: (a) maintain a `Vec<Gitignore>` stack (one per ancestor level), checking deepest-first so a nested rule overrides an ancestor; or (b) rebuild a `GitignoreBuilder` per directory adding each ancestor's `.gitignore` via `.add(dir.join(".gitignore"))`. `--ignore <glob>` patterns fold in via `GitignoreBuilder::add_line(None, glob)?`. Use `matched(path, is_dir)` with paths relative to the tree target. Make a 3-level-nested fixture the primary test.
**Warning signs:** `target/` shown despite a root `.gitignore`, or a subdir's `*.tmp` not hidden.
**API verified `[VERIFIED: docs.rs/ignore]`:** `GitignoreBuilder::new<P:AsRef<Path>>(root) -> GitignoreBuilder`; `.add<P:AsRef<Path>>(&mut self, path) -> Option<Error>` (returns the error, doesn't fail — partial success); `.add_line(&mut self, from: Option<PathBuf>, line: &str) -> Result<&mut GitignoreBuilder, Error>`; `.build(&self) -> Result<Gitignore, Error>`; `Gitignore::matched<P:AsRef<Path>>(&self, path, is_dir: bool) -> Match<&Glob>` (and `matched_path_or_any_parents` for the parent-chain check). `Match` has `.is_ignore()` / `.is_whitelist()` / `.is_none()`.

### Pitfall 5: `flatten --move` deleting the source before the copy is confirmed
**What goes wrong:** A naive `fs::rename` fails cross-volume, and a fallback that deletes after a failed copy loses data.
**How to avoid:** copy (via `safe_copy`, which is `create_new` so it never clobbers) → verify dest EXISTS and `metadata().len()` matches the source size → only then `fs::remove_file(source)`. Any failure at any step leaves the source intact. Reuse the containment guard + collision-rename from the copy path verbatim. Snapshot the source tree unchanged on every abort path (containment refusal, dry-run, a mid-batch copy error). Note (locked): empty source directories are left in place (only files relocate).
**Warning signs:** A `--move` that deletes a source whose dest write errored; a cross-drive `--move` that fails.

### Pitfall 6: `dupes --delete` deleting a hardlink alias (frees nothing, destroys a name)
**What goes wrong:** Two paths that are hardlinks to one inode have identical content → same group → `--delete` removes one, freeing zero bytes and destroying a legitimate name.
**How to avoid:** Within each group, collapse paths sharing `(dwVolumeSerialNumber, fileIndex)` (via `GetFileInformationByHandle`, Pitfall 1) BEFORE computing wasted space — count one inode once. `--delete` keeps the first (sorted-order) member and skips any member sharing the kept member's inode. Wasted space = `(distinct_inodes - 1) * size`. Test with a real `std::fs::hard_link` fixture: `--delete` must not reduce the link count to zero and must not report shared-inode space as wasted.
**Warning signs:** "wasted" space that doesn't match `du`; deleting a "duplicate" frees no disk.

### Pitfall 7: `bulk-rename {n}` counter assigned over walk order (non-reproducible)
**What goes wrong:** Assigning `{n}` in `walkdir` order yields different numbers on different machines/runs.
**How to avoid:** SORT the rename plan deterministically (by source path / `src_label`) BEFORE assigning the counter (D-21), then expand `{n}` with `--number-width` zero-padding starting at `--start`, stepping `--step`. Do the `{n}` expansion + `--case` fold BETWEEN `build_plan` and `preflight_plan` so the final names still flow through the unchanged collision/cycle/separator pre-flight. Document the literal-brace escape (e.g. `{{n}}`).
**Warning signs:** `IMG_{n}` files numbered differently across runs; a `{n}` collision reaching the executor (means the counter ran after, not before, pre-flight).

### Pitfall 8: `bulk-rename --backup` manifest written inside the renamed tree
**What goes wrong:** Writing the manifest into the target dir means `--recursive` re-walks it, or renaming the target dir orphans it.
**How to avoid:** Write to `%LOCALAPPDATA%\box\undo\<id>.json` (note **LOCALAPPDATA**, not the config's APPDATA), outside the tree; `create_dir_all` the undo dir; fallback to the target dir only if `LOCALAPPDATA` is unset; echo the path to stderr. Write + `File::sync_all()` (fsync) the FULL manifest (all `applied:false`) BEFORE the first `fs::rename`, then flip each entry `applied:true` (rewrite + fsync) as its rename returns, so a mid-batch I/O error leaves an `applied`-partitioned, reconcilable manifest. `--backup` is a no-op on dry-run.
**Warning signs:** a `*.json` appearing among renamed files under `--recursive`; an orphaned manifest after renaming the target dir.

## Runtime State Inventory

> Phase 8 adds flags to existing read/copy/rename commands; only `bulk-rename --backup` writes new persistent OS state. Each category answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | **`bulk-rename --backup` undo manifests** written to `%LOCALAPPDATA%\box\undo\<id>.json` (NEW persistent files outside any datastore). No database/collection state. | Create `%LOCALAPPDATA%\box\undo\` on first backup; document that manifests accumulate (no auto-cleanup this phase — `--undo` replay is deferred). |
| Live service config | None — `box` is a local CLI; no external service holds the renamed strings. (Verified: no n8n/Datadog/Tailscale/Cloudflare integration in this codebase.) | None. |
| OS-registered state | None — no Task Scheduler / pm2 / systemd registration touched. The `windows` crate calls (`GetCompressedFileSizeW`, `GetFileInformationByHandle`) are read-only queries, register nothing. | None. |
| Secrets/env vars | Reads `%LOCALAPPDATA%` (backup path) and `%APPDATA%` (existing config, unchanged). `BOX_HASH_DEFAULT_ALGO` is the only existing env tier; Phase 8 adds none. | None — `%LOCALAPPDATA%` is read, not created as a secret. |
| Build artifacts | The four new deps change `Cargo.lock`; `windows 0.61` gains the `Win32_Storage_FileSystem` feature (recompiles `windows`). No stale egg-info/global-install equivalent. | `cargo build` regenerates `Cargo.lock`; commit it. |

**The canonical question — after the code ships, what runtime state persists?** Only `bulk-rename --backup` manifests under `%LOCALAPPDATA%\box\undo\`. Everything else (`hash`/`flatten`/`tree`/`du`/`dupes` reads, `flatten --move` file relocation, `bulk-rename` renames) leaves no registered/cached state beyond the filesystem changes the user explicitly requested.

## Code Examples

### Multi-file hash loop (HASH-V2-02) — into the existing `{results,count}` doc
```rust
// hash/mod.rs — HashArgs.path: Option<String> (line 74) becomes paths: Vec<String>.
// The two-space coreutils row is ALREADY emitted at line 281; multi-file just loops it.
// #[arg(...)] pub paths: Vec<String>   // empty Vec => read stdin (label "-")
let targets: Vec<Option<String>> = if self.paths.is_empty() {
    vec![None]                                   // stdin path, label "-"
} else {
    self.paths.iter().cloned().map(Some).collect()
};
let mut rows = Vec::new();
let mut had_error = false;
for t in targets {
    match read_file_or_stdin(t).and_then(|inp| {
        let label = inp.label.clone();
        digest_reader(algo, inp.reader).map(|d| (label, d))
    }) {
        Ok((label, digest)) => {
            if !is_json_on() { out_line(&format!("{digest}  {label}")); }  // TWO spaces (line 281)
            rows.push(HashRow { path: label, algo, digest });
        }
        Err(e) => { eprintln!("error: {e:#}"); had_error = true; }         // best-effort, stderr
    }
}
if is_json_on() { emit_json(&HashOutput { count: rows.len(), results: rows })?; }
if had_error { std::process::exit(1); }   // coreutils parity — see Open Questions for the --json nuance
```
> **coreutils line format note:** GNU `sha256sum` prints `<hex><space><type-marker><filename>` where the type-marker is a space (text mode → two spaces total) or `*` (binary mode). `box` already emits the **two-space text-mode** form (`{digest}  {label}`), which is the locked HASH-V2-02 format. The `*` binary marker is a coreutils convention to document, not adopt, unless the planner adds a `--binary` flag (not in scope).

### du percentage column + band color (DU-V2-01) — replacing `color_size` (du:249-255)
```rust
// total is captured at du:120 BEFORE --top truncation (the percentage basis, D-23).
// When --on-disk is set, total is the on-disk total instead.
fn percent_str(size: u64, total: u64) -> String {
    if total == 0 { return "0.0%".to_string(); }            // divide-by-zero guard (Pitfall 3)
    let pct = size as f64 / total as f64 * 100.0;
    if pct == 0.0 { "0.0%".to_string() }
    else if pct < 0.1 { "<0.1%".to_string() }               // tiny-but-nonzero
    else { format!("{pct:.1}%") }
}
fn band_color(size_token: &str, size: u64, total: u64) -> String {  // REPLACES color_size
    if !is_color_on() { return size_token.to_string(); }
    let pct = if total == 0 { 0.0 } else { size as f64 / total as f64 * 100.0 };
    if pct > 50.0 { size_token.red().to_string() }
    else if pct >= 10.0 { size_token.yellow().to_string() }
    else { size_token.to_string() }                          // default/unaccented
}
// Render: right-aligned percent column BEFORE the size value, both gated; name uncolored.
```

### bulk-rename --case + {n} post-passes (RENM-V2-01) — at re.replace (bulk_rename:513)
```rust
// build_plan (line 456) currently: let new_name = re.replace(&old_name, replacement).into_owned();
// Keep re.replace per-file, but DEFER {n} + --case until after the plan is built & SORTED.
fn apply_number_and_case(name: &str, n: usize, width: usize, case: Option<Case>) -> String {
    let numbered = name.replace("{n}", &format!("{n:0width$}"));   // literal {n}, NOT regex ${n}
    match case {
        Some(Case::Upper) => numbered.to_uppercase(),
        Some(Case::Lower) => numbered.to_lowercase(),
        Some(Case::Title) => title_case_stem(&numbered),           // title on the STEM only
        None => numbered,
    }
}
// In run(): after build_plan, sort plan.items by src path; assign n = start; step by --step;
// width = self.number_width.unwrap_or_else(|| digits(count)); rewrite each item.new_name;
// THEN preflight_plan(&plan) — the unchanged collision/cycle/separator detector (line 185).
```

### dupes partial-hash stage (DUPE-V2-01) — between collect_by_size (181) and rayon full-hash (123)
```rust
const PARTIAL_BYTES: u64 = 16 * 1024;   // taste: enough to split most same-size files cheaply
// After collect_by_size + the >=2 filter, re-bucket candidates by (size, partial_hash):
fn partial_hash(path: &Path) -> anyhow::Result<String> {
    let f = std::fs::File::open(path)?;
    hash_reader_blake3(f.take(PARTIAL_BYTES))     // reuse the existing reader-generic core (dupes:218)
}
// Group candidates by (size, partial); keep (size,partial) buckets of >=2; only those go to
// the existing full-hash par_iter (line 123). For size <= PARTIAL_BYTES the partial covers the
// whole file (the full pass is then redundant — an optional skip-to-full optimization).
```

## State of the Art

| Old Approach (per project docs) | Current Approach (verified) | When Changed | Impact |
|--------------------------------|-----------------------------|--------------|--------|
| `std MetadataExt::file_index()` for hardlinks (STATE.md:113, PITFALLS.md P5) | `GetFileInformationByHandle` via `windows 0.61` | Never stabilized (issue #63010 open since 2019) | dupes hardlink detection MUST use Win32 FFI, not std — no compile on stable otherwise |
| `du` `.cyan()` single accent (du:249) | percentage-band color (red/yellow/default) | Phase 8 D-23 | Replaces, doesn't add; same gated slot |
| `dupes` size → full-hash (2-stage) | size → partial → full (3-stage) | Phase 8 DUPE-V2-01 | Fewer full-file reads on large same-size sets |
| `tree` shows everything-but-dotfiles | opt-in `--gitignore` (default unchanged) | Phase 8 D-20 | Default `box tree` output is byte-identical to v1 |

**Deprecated/outdated:**
- `filesize` crate (on-disk size) — unmaintained since 2020; use `windows` directly.
- The STATE.md note "use `fs::metadata(path)` NOT `DirEntry::metadata()`" is correct in spirit (you do need a handle-based call) but the std methods it points to are nightly — the handle-based call is `GetFileInformationByHandle`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `hash` multi-file under `--json` with a partial failure should emit `{results:[successful rows], count}` AND exit 1 (best-effort), with per-file errors on stderr — a documented refinement of D-09's "empty stdout on failure" (which targets TOTAL failure). | hash / Open Questions | If wrong, the spine's D-09 purity is violated; the conservative alternative is fail-fast-under-`--json` (empty stdout, exit 1 on first error). Planner/discuss should lock this. |
| A2 | `du --json` should NOT add a raw `f64` percent field; the consumer derives percentage from `size`/`total_bytes`. CONTEXT D-23 only specifies `on_disk: bool` + size-carries-on-disk. | du | Low — if a percent field is wanted, it's an additive sibling; but a raw `f64` would reintroduce the NaN-in-JSON risk. |
| A3 | `tree --sort` is spelled as a value flag `--sort size` (vs a bare `--sort-size` bool). CONTEXT says "explicit `--sort size` flag." | tree | Low — a `clap` `ValueEnum` (`name`/`size`) is the natural shape; spelling is cosmetic. |
| A4 | The `{n}` literal-brace escape is `{{n}}` (doubling). CONTEXT marks the exact escape as Claude's Discretion. | bulk-rename | Low — any documented+tested escape satisfies the requirement. |
| A5 | `<id>` for the backup manifest filename is a sortable timestamp (e.g. `box-undo-<unix_millis>.json`), not a UUID. CONTEXT leaves the id scheme open. | bulk-rename | Low — `uuid v4` is also available; either works; timestamp is human-sortable. |
| A6 | `dupes --delete` and `flatten --move` get NEW `Args` bool fields (`delete`/`move_`) — `move` is a Rust keyword, so the field is `r#move` or `move_` with `#[arg(long = "move")]`. | dupes/flatten | Low — purely a naming detail; `#[arg(long = "move")]` is the clean fix. |

## Open Questions (RESOLVED)

> All three resolved during planning (consumers: 08-01/08-02/08-03). Inline `RESOLVED` markers below.

1. **`hash` multi-file `--json` partial-failure semantics (A1).**
   - What we know: human path is best-effort + exit 1 (Claude's Discretion default, coreutils parity). The frozen D-09 says "on failure stdout stays EMPTY."
   - What's unclear: whether a 4-of-5-succeeded run should emit the 4 good rows under `--json` (useful, but stretches D-09) or keep stdout empty (pure, but discards good digests).
   - Recommendation: emit the partial document + exit 1 (errors → stderr), treating "partial success with diagnostics" as distinct from "total failure"; flag for the planner to confirm in 08-01 (or discuss). Conservative fallback: fail-fast under `--json` only.
   - **RESOLVED (08-01 Task 2):** emit the partial `{results,count}` document containing only the successful rows AND exit 1 with per-file errors on stderr — the recommended option, baked in as a deliberate partial-success refinement of D-09.

2. **Nested-`.gitignore` matcher shape for `tree` (Pitfall 4).**
   - What we know: D-20 locks matcher-as-filter inside `read_children`; nested correctness is the main test.
   - What's unclear: `Vec<Gitignore>` ancestor-stack vs. rebuild-per-directory `GitignoreBuilder`.
   - Recommendation: ancestor-stack checked deepest-first (one `Gitignore` per directory level that has a `.gitignore`, plus one for `--ignore` lines); it maps cleanly onto the existing per-directory recursion and is the cheaper allocation. Pin a 3-level-nested fixture.
   - **RESOLVED (08-02 Task 2):** ancestor-stack `Vec<Gitignore>` checked deepest-first (the recommended option); 3-level nested fixture pinned in `tests/tree.rs::gitignore_nested`.

3. **`dupes`/`du` Win32 wrapper location.**
   - What we know: both need `windows 0.61` `Win32_Storage_FileSystem`.
   - What's unclear: localize FFI per-module (v1 arboard/winrt pattern) vs. a shared `core::fs::{compressed_size, file_identity}` pair.
   - Recommendation: a shared `core::fs` pair — both wrappers are tiny, both open a handle, and centralizing the `unsafe` keeps the audit surface in one file. Not load-bearing; planner's call.
   - **RESOLVED (08-02/08-03):** per-module localized FFI — `compressed_size` in `du/mod.rs`, `file_identity` in `dupes/mod.rs` — chosen over the shared `core::fs` pair specifically so no two Wave-1 plans edit `core/fs.rs` (wave-isolation; taken under the "planner's call" latitude).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust stable toolchain (cargo/rustc) | all of Phase 8 | ✓ (project builds today) | stable MSVC | — (nightly NOT required; the windows_by_handle correction keeps it stable) |
| `x86_64-pc-windows-msvc` + `+crt-static` | du on-disk, dupes hardlink | ✓ (project target) | — | — |
| `windows` 0.61.3 | du --on-disk, dupes hardlink | ✓ (already in tree transitively) | 0.61.3 | — |
| `ignore` 0.4.26 | tree --gitignore | ✓ (dev-dep today; promote to runtime) | 0.4.26 | — |
| `globset` 0.4.18 | du --exclude | ✓ (dev-dep today; promote to runtime) | 0.4.18 | — |
| `indicatif` 0.18.4 | hash/flatten progress | crates.io (not yet pulled) | 0.18.4 | progress is optional UX — a missing bar degrades to no bar, never a failure |

**Missing dependencies with no fallback:** none — all four new crates resolve from crates.io and three are already in the build graph.
**Missing dependencies with fallback:** `indicatif` — if the planner defers progress, the commands still function (progress is a stderr affordance, not a correctness requirement).

## Validation Architecture

> `workflow.nyquist_validation` is `true` in `.planning/config.json` — this section is REQUIRED.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (no external runner); dev-deps `assert_cmd 2.2`, `predicates 3.1`, `assert_fs 1.1`, `tempfile 3.27`, `trycmd 1.2`, `insta 1.48` |
| Config file | none (Cargo convention: `src/**` unit tests + `tests/*.rs` integration tests) |
| Quick run command | `cargo test --bin box <name>` (unit; binary-only crate — `--bin box`, NEVER `--lib`, per STATE.md:119) |
| Full suite command | `cargo test` (unit + all integration), then `cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HASH-V2-02 | `box hash a.bin b.bin` → two `digest␣␣filename` lines (two spaces) | integration | `cargo test --test hash multi_file_two_space` | ❌ Wave 0 (extend `tests/hash.rs`) |
| HASH-V2-02 | `--json` → `{results:[…],count:N}` for N files; purity (no 0x1B/BOM) | integration | `cargo test --test hash json_multifile_purity` | ❌ Wave 0 |
| HASH-V2-02 | one unreadable file → stderr error, other digests printed, exit 1 (best-effort) | integration | `cargo test --test hash partial_failure_exit1` | ❌ Wave 0 |
| FLAT-V2-01 | `--extensions jpg,png` (case-insensitive, final-ext) filters the plan | integration | `cargo test --test flatten extensions_filter` | ❌ Wave 0 |
| FLAT-V2-01 | `--separator -` changes the collision-encoding join char | unit | `cargo test --bin box rename::encode_relative_honors_separator` | ❌ Wave 0 (extend `rename.rs` tests) |
| FLAT-V2-01 | `--include-hidden` includes a dotfile the default prunes | integration | `cargo test --test flatten include_hidden` | ❌ Wave 0 |
| FLAT-V2-02 | dry-run default writes nothing; `--force` copies→verifies→deletes source | integration | `cargo test --test flatten move_force_relocates` | ❌ Wave 0 |
| FLAT-V2-02 | **abort: source tree byte-for-byte unchanged** (containment refusal, dry-run, mid-batch copy error) | integration (snapshot-tree) | `cargo test --test flatten move_abort_snapshot_unchanged` | ❌ Wave 0 (use `snapshot_tree` from `tests/dupes.rs:46`) |
| DUPE-V2-01 | size→partial→full splits same-size-different-content; groups identical | integration | `cargo test --test dupes multistage_splits` | ⚠️ extend `tests/dupes.rs` (`dupes_size_then_hash` exists) |
| DUPE-V2-01 | hardlink pair collapsed: not counted as wasted space | integration (hardlink fixture) | `cargo test --test dupes hardlink_not_wasted` | ❌ Wave 0 (`std::fs::hard_link` fixture) |
| DUPE-V2-02 | `--delete` keep-first; dry-run default; `--force` deletes the rest | integration | `cargo test --test dupes delete_keep_first` | ❌ Wave 0 |
| DUPE-V2-02 | **abort/dry-run: tree unchanged**; hardlink alias never deleted | integration (snapshot-tree) | `cargo test --test dupes delete_abort_snapshot_unchanged` | ❌ Wave 0 (`snapshot_tree` exists at line 46) |
| RENM-V2-01 | `--case upper/lower/title` folds the result (title on stem) | unit + integration | `cargo test --bin box apply_number_and_case` ; `cargo test --test bulk_rename case_transforms` | ❌ Wave 0 |
| RENM-V2-01 | `{n}` counter over SORTED plan, `--number-width`/`--start`/`--step`, reproducible | integration | `cargo test --test bulk_rename numbering_sorted_reproducible` | ❌ Wave 0 |
| RENM-V2-02 | `--backup` writes manifest to `%LOCALAPPDATA%\box\undo\`; `--force` only; dry-run no-op | integration | `cargo test --test bulk_rename backup_manifest_written` | ❌ Wave 0 |
| RENM-V2-02 | **mid-batch I/O error: `applied` flags partition done/pending → dir recoverable** | integration (snapshot + manifest) | `cargo test --test bulk_rename backup_partition_recoverable` | ❌ Wave 0 |
| TREE-V2-01 | `--gitignore` respects root AND nested `.gitignore` (eza #1086 class) | integration (3-level fixture) | `cargo test --test tree gitignore_nested` | ❌ Wave 0 (extend `tests/tree.rs`) |
| TREE-V2-01 | `--ignore <glob>` + `--dirs-only` + `--sort size`; default output UNCHANGED | integration | `cargo test --test tree dirs_only_ignore_sort` ; `cargo test --test tree default_output_unchanged` | ⚠️ trycmd `tests/cmd/tree.trycmd` pins default — keep green |
| TREE-V2-01 | `--gitignore --json` recursive node tree honors the filter (no-drift) | integration | `cargo test --test tree gitignore_json_nodrift` | ❌ Wave 0 |
| DU-V2-01 | percentage column from full-scan total (consistent post-`--top`); empty dir → `0.0%` not `NaN` | integration | `cargo test --test du percentage_column` | ❌ Wave 0 (extend `tests/du.rs`) |
| DU-V2-01 | band color gated; `--json` byte-identical-minus-ANSI; `--exclude <glob>` drops children + descendants | integration | `cargo test --test du exclude_glob` ; `cargo test --test du json_no_ansi` | ❌ Wave 0 |
| DU-V2-02 | `--on-disk` reports allocated size; sparse/compressed file < logical; `on_disk:true` in JSON | integration (sparse fixture) | `cargo test --test du on_disk_size` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --bin box <touched-module>` (the unit pure-function tests — fast).
- **Per wave merge:** `cargo test --test <command>` for each command touched + `cargo clippy --all-targets -- -D warnings`.
- **Phase gate:** full `cargo test` green + clippy clean before `/gsd:verify-work`; the three destructive plans (08-04/05/06) additionally require their snapshot-tree-unchanged tests green for EVERY abort path (Code-review gate).

### Wave 0 Gaps
- [ ] `tests/hash.rs` — multi-file two-space, json-multifile purity, partial-failure-exit-1 (REQ HASH-V2-02)
- [ ] `tests/flatten.rs` — extensions/include-hidden/move-force + **move-abort snapshot-unchanged** (FLAT-V2-01/02); reuse `snapshot_tree`
- [ ] `tests/dupes.rs` — multistage-splits, **hardlink-not-wasted** (`std::fs::hard_link` fixture), delete-keep-first, **delete-abort snapshot-unchanged** (DUPE-V2-01/02)
- [ ] `tests/bulk_rename.rs` — case transforms, numbering-sorted-reproducible, **backup-manifest** + **backup-partition-recoverable** (RENM-V2-01/02)
- [ ] `tests/tree.rs` — **gitignore-nested** (3-level fixture), dirs-only/ignore/sort, default-output-unchanged, gitignore-json-nodrift (TREE-V2-01)
- [ ] `tests/du.rs` — percentage-column (+ empty-dir NaN guard), exclude-glob, json-no-ansi, **on-disk** (sparse fixture) (DU-V2-01/02)
- [ ] Unit tests in `src/commands/**`: `rename::encode_relative_honors_separator`, `bulk_rename::apply_number_and_case`, du `percent_str`/`band_color`, dupes partial-bucket re-grouping
- [ ] Shared helpers already exist and are copy-me: `snapshot_tree` (`tests/dupes.rs:46`), `snapshot_names`/`listed_names` (`tests/bulk_rename.rs:63-88`), `json_purity` pattern (`tests/dupes.rs:275`)

*(No framework install needed — the dev-dependency set covers everything.)*

## Security Domain

> `security_enforcement` is not set to `false` in config — included. `box` is a local CLI with no network/auth surface this phase; the relevant categories are input validation (paths/globs) and data-loss safety.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | local CLI, no auth |
| V3 Session Management | no | none |
| V4 Access Control | no | runs as the invoking user |
| V5 Input Validation | yes | path-separator/`..` refusal in rename targets (`bulk_rename::injects`, line 285); `encode_relative` strips separators (`rename.rs:35`); glob/regex compiled, errors → exit 1 not panic; `--separator` must be validated not to contain `/`/`\` |
| V6 Cryptography | partial | BLAKE3 for content equality only (NOT a security boundary — D-13); no crypto secrets handled |

### Known Threat Patterns for {Rust CLI on Windows NTFS}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path-separator / `..` injection in a rename/move target | Tampering | Reuse `bulk_rename::injects` + `flatten::rename::encode_relative` (refuse `/`, `\`, `..`, `.`, dots/spaces-only); abort-all-before-any |
| `std::fs::rename` silently overwrites on Windows (`MoveFileExW`) | Tampering / data loss | The pure pre-flight (`preflight`, line 185) is the entire backstop — no `create_new` for moves; never `rename` without a clean pre-flight |
| `flatten --move` deleting source before dest confirmed | data loss | copy (`safe_copy`, `create_new`) → verify dest exists + size matches → delete source; snapshot-tree-unchanged on abort |
| `dupes --delete` removing a hardlink alias | data loss | collapse by `(volume_serial, file_index)` via `GetFileInformationByHandle` before computing/deleting; keep ≥1 per inode |
| NTFS case-insensitive collision (`README` vs `readme`) | data loss | full-Unicode `to_lowercase()` collision keys (already used in `dedupe`/`fold`) |
| Non-UTF-8 NTFS name panic under `--json` | DoS | `to_string_lossy()` (D-4) — never `to_str().unwrap()`; tested by `tests/dupes.rs:json_lossy_path_name_no_panic` (copy the pattern) |
| `--backup` manifest written inside the renamed tree | data integrity | write to `%LOCALAPPDATA%\box\undo\` outside the tree; fsync before first rename |

## Sources

### Primary (HIGH confidence)
- Project source (read in full, line anchors verified): `src/commands/{hash,flatten,flatten/rename,dupes,du,tree,bulk_rename}/mod.rs`, `src/core/{output,fs,input,config,errors}.rs`, `src/main.rs`, `src/cli.rs`, `Cargo.toml`, `tests/{dupes,bulk_rename}.rs`.
- `cargo tree -i windows` → `windows v0.61.3` via `tauri-winrt-notification 0.7.2`; `cargo tree -i ignore`/`-i globset` → present as dev-deps only (assert_fs→globwalk); `cargo search` → `ignore 0.4.26` / `globset 0.4.18` / `indicatif 0.18.4` current.
- `[VERIFIED: doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt]` — `file_index`/`volume_serial_number`/`number_of_links` are 🔬 nightly-only.
- `[VERIFIED: github.com/rust-lang/rust/issues/63010]` — `windows_by_handle` tracking issue OPEN, B-unstable, as of June 2026.
- `[VERIFIED: docs.rs/ignore]` — `Gitignore`/`GitignoreBuilder` signatures (`matched`, `add`, `add_line(from: Option<PathBuf>, line: &str)`, `build`).
- `[VERIFIED: microsoft.github.io/windows-docs-rs]` — `GetCompressedFileSizeW(lpfilename: impl Param<PCWSTR>, lpfilesizehigh: Option<*mut u32>) -> u32`, `Win32_Storage_FileSystem` feature.
- Planning docs: `08-CONTEXT.md` (D-20…D-23 + carried locks), `06-CONTEXT.md` / `07-CONTEXT.md` (frozen spine), `ROADMAP.md` (Phase 8 SC + 6-plan sketch + Code-review gate), `REQUIREMENTS.md`, `STATE.md`, research `ARCHITECTURE.md`/`PITFALLS.md`/`STACK.md`.

### Secondary (MEDIUM confidence)
- `[CITED: crates.io]` version currency for the four new deps.
- coreutils `sha256sum` line format (two-space text vs `*` binary marker) — standard tool convention.

### Tertiary (LOW confidence)
- WebSearch summary on #63010 stabilization status was self-contradictory; resolved by direct fetch of the std docs + the tracking issue (promoted to HIGH after primary verification).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all four deps verified on crates.io + cargo tree; three already compiled in the graph.
- Architecture / line anchors: HIGH — every insertion point read and line-numbered against current source.
- Pitfalls: HIGH — the windows_by_handle correction verified against std docs + the open tracking issue; all others trace to read source + project pitfall docs.
- One genuine gray area (A1 — hash multi-file `--json` partial failure) flagged for planner/discuss confirmation.

**Research date:** 2026-06-27
**Valid until:** ~2026-07-27 (stable-ecosystem deps; the windows_by_handle status would only change if #63010 stabilizes — re-check if a future toolchain bump is planned)
