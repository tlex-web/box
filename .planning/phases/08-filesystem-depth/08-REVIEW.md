---
phase: 08-filesystem-depth
reviewed: 2026-06-28T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - src/commands/hash/mod.rs
  - src/commands/flatten/mod.rs
  - src/commands/flatten/rename.rs
  - src/commands/tree/mod.rs
  - src/commands/du/mod.rs
  - src/commands/dupes/mod.rs
  - src/commands/bulk_rename/mod.rs
  - tests/hash.rs
  - tests/flatten.rs
  - tests/flatten_move.rs
  - tests/tree.rs
  - tests/du.rs
  - tests/dupes.rs
  - tests/dupes_delete.rs
  - tests/bulk_rename.rs
  - tests/bulk_rename_backup.rs
findings:
  critical: 1
  warning: 4
  info: 1
  total: 6
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-06-28
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Phase 8 adds depth flags to the filesystem commands plus three destructive
features (`flatten --move`, `dupes --delete`, `bulk-rename --backup`). The
read-only paths (`tree`, `du`, the `dupes` cascade, `hash` multi-file) are
careful and well-structured, and `--json` stdout purity is correctly preserved
across every command (json fork first, all human chrome behind the `else`,
errors to stderr). `flatten --move` (copy→verify→delete two-phase) and
`dupes --delete` (keep-first, hardlink-safe, abort-all-before-any) hold up to
adversarial tracing: I could not find a data-loss path in either.

The one serious problem is in **`bulk-rename`**, whose module doc correctly
states "The pre-flight check is the ENTIRE safety story" because `std::fs::rename`
silently overwrites on Windows — yet the pre-flight does **not** account for
Windows trailing-dot/space trimming, which the sibling `flatten` code explicitly
fixed (CR-01 in `flatten/rename.rs`). That gap is a silent, irreversible
data-loss vector on the exact platform this tool targets (CR-01 below). The
remaining findings are robustness/consistency issues, most of them concentrated
in `bulk-rename`'s new `--backup` and target-validation surface.

## Critical Issues

### CR-01: `bulk-rename` pre-flight misses Windows trailing-dot/space target collisions → silent data loss

**File:** `src/commands/bulk_rename/mod.rs:347-360` (`injects` / `fold`), executed at `:504`

**Issue:**
On Windows, `std::fs::rename` (→ `MoveFileExW`, no `\\?\` prefix) **strips
trailing dots and spaces from the final path component** before resolving the
destination, and overwrites the destination with `MOVEFILE_REPLACE_EXISTING`.
The bulk-rename pre-flight — which is the *only* thing standing between the user
and a silent clobber — never normalizes for this:

- `injects` (line 347) only rejects names that are **entirely** dots/spaces
  (`name.trim_matches(['.', ' ']).is_empty()`). A target like `keep.`,
  `keep.txt.`, or `keep.txt ` passes (`trim` leaves a non-empty stem).
- `fold` (line 359) is a bare `to_lowercase()` with **no trailing-dot/space
  trim**, so the collision/cycle keys for `keep.` and `keep` differ.

Concrete data-loss scenarios (both clean a pre-flight, then clobber on execute):

1. A file `keep` exists and is not renamed; `src.txt` → `keep.`.
   `fold("keep.") = "keep." != "keep"`, so Rule 2b (clobber-of-existing) misses
   it. `rename(src.txt, parent.join("keep."))` resolves to `keep` on disk and
   **silently overwrites the existing `keep`**.
2. Two sources, `a.txt` → `x` and `b.txt` → `x.` (trailing dot). They fold to
   different keys (`"x"` vs `"x."`), so Rule 2a (two-sources-one-target) misses
   the pair. Both resolve to `x` on disk; the second rename **overwrites the
   first source's content**.

This is exactly the `CR-01` class that `flatten/rename.rs:83` already closes via
`sanitize_reserved` (`trim_end_matches(['.', ' '])` on the whole name), and
`flatten` additionally has `safe_copy`'s create-new backstop. `bulk-rename` has
**neither** the trim nor a create-new backstop, so it is uniquely exposed. No
test exercises a trailing-dot/space target (the suite covers `..`, `.`,
separators, and purely-dots names only), so the gap is uncaught.

**Fix:** Normalize targets for the Windows on-disk name before both the
`injects` refusal and the collision/cycle `fold` key — reuse the existing
`flatten::rename::sanitize_reserved` so the two destructive commands share one
hardened name model. Minimally, fold on the trimmed name and refuse targets that
change under trimming:

```rust
// fold a target to the name Windows will ACTUALLY create on disk.
fn fold(name: &str) -> String {
    // trailing dots/spaces are stripped by Win32 path resolution, so the
    // collision key must be the post-trim, case-folded name.
    name.trim_end_matches(['.', ' ']).to_lowercase()
}

// and in `injects`, also refuse a target whose trimmed form differs from the
// literal (so the user sees the rename was altered) OR route every target
// through `flatten::rename::sanitize_reserved` before planning.
```

Add a regression test: a dir containing `keep` plus `src.txt`, rename
`src.txt` → `keep.` under `--force`, and assert the original `keep` content
survives (it must abort as a collision, not clobber).

## Warnings

### WR-01: `flatten` containment guard folds ASCII-only, inconsistent with the full-Unicode fold used everywhere else

**File:** `src/commands/flatten/mod.rs:228-237`

**Issue:** The output-inside-source guard lowercases with `to_ascii_lowercase()`:

```rust
let src_low = src_root.to_string_lossy().to_ascii_lowercase();
let out_low = out_root.to_string_lossy().to_ascii_lowercase();
if Path::new(&out_low).starts_with(Path::new(&src_low)) { ... }
```

NTFS is case-insensitive over the **full** Unicode table, and the code
deliberately chose `to_lowercase()` for exactly this reason in the occupied seed
(`:250`) and `rename::dedupe`/`fold` (with a WR-01 comment). The guard using
ASCII-only folding means a containment that differs only by a **non-ASCII** case
pair (e.g. source `C:\CAFÉ`, output `C:\café\flat`) is not recognized as
contained, so the guard is bypassed and `--move` will relocate files into a
subdirectory of the source. This is not data loss (the walk completes before any
copy, `safe_copy` is create-new, and the moved files survive in `out`), but it
defeats the guard and is inconsistent with the module's own NTFS-correctness
stance. Note also the inline comment "the disk fills as the walker re-visits its
own output" is inaccurate — `build_plan` fully materializes the plan before any
copy, so there is no re-visit/infinite loop.

**Fix:** Use the same full-Unicode fold as the rest of the module:

```rust
let src_low = src_root.to_string_lossy().to_lowercase();
let out_low = out_root.to_string_lossy().to_lowercase();
```

### WR-02: `bulk-rename --backup` records relative paths when the target dir is relative, breaking the recovery artifact

**File:** `src/commands/bulk_rename/mod.rs:917-933` (`build_manifest`), `:448-475`

**Issue:** The undo manifest is deliberately placed in `%LOCALAPPDATA%\box\undo\`
"OUTSIDE the renamed tree … so it survives renaming the target dir" (D-22). But
`build_manifest` derives entry paths from `item.src` / `item.parent`, which come
from `WalkDir::new(self.dir)`. When the user passes a **relative** `dir`
(e.g. `box bulk-rename . "pat" "rep" --backup --force`), `item.src` and
`item.parent` are relative, so the manifest's `old`/`new` (and the top-level
`dir`) are stored relative to the invocation cwd. A recovery artifact stored in a
cwd-independent location but containing cwd-relative paths is not reconcilable
once the cwd changes — defeating the durability rationale. The doc hedges ("ABSOLUTE
paths … absolute when the target dir is absolute") and every test passes an
absolute `TempDir`, so the relative case is untested.

**Fix:** Canonicalize the target dir once before building the plan (reuse
`crate::core::fs::normalize_path`) so all derived paths are absolute, or
`std::path::absolute(&self.dir)` the manifest entries before serialization.

### WR-03: `write_manifest` truncates in place — a failed flip-write corrupts the entire undo manifest

**File:** `src/commands/bulk_rename/mod.rs:941-949`, called at `:472` and `:513-516`

**Issue:** `write_manifest` does `File::create(path)` (which **truncates** any
existing manifest) and only then serializes + `sync_all`. It is called once per
rename to flip an `applied` flag. If a per-flip rewrite fails *after* truncation
(disk full, transient I/O error) the `?` propagates to exit 1 — but the on-disk
manifest is now truncated/partial, destroying the record of the renames that
already succeeded. That directly defeats the Pitfall-8 guarantee the feature
exists for ("a mid-batch I/O error leaves a manifest whose `applied` flags
EXACTLY partition done-vs-pending → the directory is reconcilable"); the
guarantee holds for a *rename* I/O error (tested) but not for a *manifest-write*
I/O error (untested). A self-corrupting recovery artifact is a real robustness
defect for a destructive command.

**Fix:** Write durably via temp-file-then-atomic-rename so a failed write never
clobbers the last good manifest:

```rust
let tmp = path.with_extension("json.tmp");
{ let f = std::fs::File::create(&tmp)?; serde_json::to_writer_pretty(&f, manifest)?; f.sync_all()?; }
std::fs::rename(&tmp, path)?; // atomic replace; old manifest survives a failed write
```

### WR-04: `bulk-rename` does not neutralize Windows reserved device-name targets (`CON`, `NUL`, `COM1`…)

**File:** `src/commands/bulk_rename/mod.rs:347-360` / `:504`

**Issue:** Unlike `flatten` (which routes every name through
`sanitize_reserved`), bulk-rename never checks reserved device stems. A
replacement yielding `CON`, `nul.txt`, `COM1`, etc. is planned and handed to
`std::fs::rename`. A single-source rename to a reserved name typically fails at
execution (hard exit-1, poor UX vs. flatten's graceful `CON_` rename); the
multi-source case happens to be caught only because the targets fold to the same
literal key. This is the same missing-normalization root cause as CR-01 — fixing
CR-01 by reusing `sanitize_reserved` resolves this consistently.

**Fix:** Route rename targets through `flatten::rename::sanitize_reserved` (or
share the reserved-name list) before planning, matching flatten's behavior.

## Info

### IN-01: Dead/no-op `let _ = is_color_on();` on the bulk-rename success path

**File:** `src/commands/bulk_rename/mod.rs:571`

**Issue:** `let _ = is_color_on();` evaluates and discards the result; the
trailing comment ("color is applied inside format_row's glyph wrap") confirms it
has no effect. It is leftover code that reads as if it were doing something.

**Fix:** Delete the line.

---

_Reviewed: 2026-06-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
