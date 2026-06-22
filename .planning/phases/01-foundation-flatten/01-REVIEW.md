---
phase: 01-foundation-flatten
reviewed: 2026-06-22T00:00:00Z
depth: deep
files_reviewed: 14
files_reviewed_list:
  - src/main.rs
  - src/cli.rs
  - src/core/mod.rs
  - src/core/errors.rs
  - src/core/output.rs
  - src/core/fs.rs
  - src/commands/mod.rs
  - src/commands/stub.rs
  - src/commands/flatten/mod.rs
  - src/commands/flatten/rename.rs
  - tests/cli.rs
  - tests/flatten.rs
  - install.ps1
  - Cargo.toml
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: resolved
resolution:
  fixed: [CR-01, WR-01, WR-02]
  fixed_in: e1a8f38
  resolved_at: 2026-06-22
  remaining_advisory: [WR-03, WR-04, IN-02, IN-03]   # IN-01 substantially closed by the CR-01 all-dots fallback
---

# Phase 1: Code Review Report

**Reviewed:** 2026-06-22
**Depth:** deep
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Phase 1 establishes the crate scaffold, the strict 0/1/2 exit-code contract, the
color-gating layer, the shared FS helpers, and the `flatten` anchor command.
`cargo clippy --all-targets -- -D warnings` is **clean** and all 37 tests pass.
The exit-code mapping, color gate, containment guard (`Path::starts_with` is
correctly component-aware, so `proj-flat` is not a false-positive child of
`proj`), symlink skipping, hidden-entry pruning, and timestamp preservation are
all correct.

However, the central safety promise of `flatten` — *"nothing is ever silently
overwritten"* — has a real hole. The reserved-name sanitizer does **not** strip
trailing dots/spaces from the whole filename (only from the stem before the last
dot), and the NTFS-collision keying uses `to_ascii_lowercase()` which does not
fold non-ASCII case the way NTFS does. Both let two distinct in-memory
destination names map to the **same on-disk file**, and `safe_copy` →
`std::fs::copy` overwrites unconditionally. The result is silent data loss for
inputs that the code's own doc-comments claim to defend against. This is the one
must-fix before flatten can be trusted on arbitrary trees.

## Critical Issues

### CR-01: `sanitize_reserved` does not trim trailing dots/spaces from the full name → silent overwrite / data loss

**File:** `src/commands/flatten/rename.rs:56-73`

**Issue:** The doc comment (lines 53-55, 61-63) states the function trims trailing
dots/spaces because *"Windows silently trims these, which would create hidden
collisions."* But the trim is applied only to `stem` — the substring **before the
last `.`** — then the original `ext` is re-appended verbatim. So a trailing dot
or space that sits at the very end of the name (after the final `.`, or when
there is no extension) is preserved in the returned name, while Windows strips it
on write. Verified behavior:

| input          | `sanitize_reserved` returns | Windows writes to disk |
|----------------|-----------------------------|------------------------|
| `"report."`    | `"report."`                 | `report`               |
| `"report. "`   | `"report. "`                | `report`               |
| `"report.txt."`| `"report.txt."`             | `report.txt`           |
| `"report.txt "`| `"report.txt "`             | `report.txt`           |
| `"data."`      | `"data."`                   | `data`                 |
| `"CON."`       | `"CON_."`                   | `CON_`                 |

Because `occupied` is keyed on the *returned* (un-stripped) name, two source
files — e.g. `report` and `report.` — produce two **distinct** keys
(`"report"` and `"report."`), so the collision is never detected. Both then copy
to the same physical file via `safe_copy` → `std::fs::copy`, which overwrites
unconditionally (confirmed: `fs::copy` clobbers an existing destination). The
first file is silently lost — the exact failure D-14 / T-03-reserved /
no_silent_overwrite were designed to prevent. The existing test
`sanitize_trims_trailing_dots_and_spaces` only exercises junk *before* the final
dot (`"report .txt"`, `"report..txt"`), so it never caught this.

**Fix:** Trim trailing dots/spaces from the **whole reconstructed name**, after
re-attaching the extension, and re-test for emptiness:

```rust
pub fn sanitize_reserved(name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (name, None),
    };
    let mut stem = stem.trim_end_matches(['.', ' ']).to_string();
    let is_reserved = RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&stem));
    if is_reserved {
        stem.push('_');
    }
    let rebuilt = match ext {
        Some(e) => format!("{stem}.{e}"),
        None => stem,
    };
    // Windows strips trailing dots/spaces from the ENTIRE name on write; strip
    // them here so the occupied-set key matches the real on-disk name.
    let trimmed = rebuilt.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        // Name was nothing but dots/spaces — give it a stable, writable name
        // so it can never collapse to "" / "." / ".." as a copy target.
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}
```

Add regression cases for `"report."`, `"report.txt "`, `"data."`, and an
all-dots name like `"..."`.

## Warnings

### WR-01: NTFS collision keying uses `to_ascii_lowercase()` — non-ASCII case collisions clobber

**File:** `src/commands/flatten/rename.rs:81,91`, `src/commands/flatten/mod.rs:129,240,246,259`

**Issue:** Every collision/occupied key is computed with
`to_ascii_lowercase()`, and the doc comment (rename.rs:77-78) explicitly justifies
this as keying *"on `to_ascii_lowercase()` to catch `README.TXT` vs
`readme.txt`."* NTFS, however, is case-insensitive over the **full Unicode** case
table, not just ASCII A–Z. So `RÉSUMÉ.txt` and `résumé.txt` produce **different**
keys (`"rÉsumÉ.txt"` vs `"résumé.txt"`), the collision is missed, and the second
file overwrites the first via `std::fs::copy`. Same for Greek, Cyrillic, German
ß/SS, Turkish dotted-I edge cases, etc. Lower frequency than CR-01 (needs
non-ASCII filenames) but the same silent-data-loss class.

**Fix:** Use full Unicode case folding for the collision key. Replace
`to_ascii_lowercase()` with `to_lowercase()` at the keying sites (the
`occupied` inserts in `mod.rs`, and the `key`/candidate comparisons in
`dedupe`). Keep the *returned* file name in its original case; only the
comparison key should be folded:

```rust
// rename.rs dedupe()
let key = name.to_lowercase();
...
if !occupied.contains(&cand.to_lowercase()) { return cand; }
```

```rust
// mod.rs build_plan()
let base_key = base_safe.to_lowercase();
...
occupied.insert(chosen.to_lowercase());
```

Note: NTFS uses an OS-version-specific uppercase table, so `to_lowercase()` is a
close-but-imperfect match. It is strictly safer than ASCII-only and removes the
common-case loss. Document the residual limitation.

### WR-02: Real-run copy has no defense-in-depth against an unexpected existing destination

**File:** `src/core/fs.rs:71`, `src/commands/flatten/mod.rs:167-169`

**Issue:** `safe_copy` calls `std::fs::copy(src, dst)`, which overwrites any file
already at `dst` with no error. The *only* thing preventing an overwrite is the
in-memory `occupied` set being perfectly in sync with the on-disk reality. CR-01
and WR-01 are two ways that invariant breaks; a TOCTOU race (a file appearing in
`out` between the `read_dir` seed and the copy) is a third. Given that "never
silently overwrite" is the headline guarantee, the executor should not rely
solely on the name bookkeeping.

**Fix:** For the destination open, prefer a create-new semantics so an unexpected
collision fails **loudly** instead of clobbering. Either pre-check
`dst.try_exists()` and `bail!` if present, or copy via an explicit open:

```rust
// In safe_copy, replace fs::copy with a create_new open + io::copy so an
// already-present destination errors instead of being overwritten.
let mut reader = std::fs::File::open(src)
    .with_context(|| format!("opening source {}", src.display()))?;
let mut writer = std::fs::File::options()
    .write(true)
    .create_new(true) // fails with AlreadyExists rather than clobbering
    .open(dst)
    .with_context(|| format!("creating destination {}", dst.display()))?;
let bytes = std::io::copy(&mut reader, &mut writer)
    .with_context(|| format!("copying {} -> {}", src.display(), dst.display()))?;
```

(Keep the existing pre-existing-output seeding so legitimate collisions still
rename; `create_new` is the backstop for the cases the name set misses.)

### WR-03: `install.ps1` empty-string PATH-entry filter can drop a meaningful trailing entry and is order-sensitive

**File:** `install.ps1:43-58`

**Issue:** Two smaller PATH-mutation concerns:
1. `$entries = $rawPath -split ';' | Where-Object { $_ -ne '' }` strips empty
   segments. That is fine for dedup, but the rebuilt `$newPath` is then written
   back **without** those empties — silently rewriting the user's PATH layout on
   every first install. Usually harmless, but it mutates state the installer
   claims only to append to.
2. When `%VARS%` are present the code writes via `Set-ItemProperty -Type
   ExpandString` (correct, preserves REG_EXPAND_SZ), but when they are absent it
   writes via `[Environment]::SetEnvironmentVariable(..., 'User')`. These two
   code paths can disagree on whitespace/empty handling on re-run, so the
   "idempotent" claim depends on the `%`-branch choice staying stable across
   runs. It does today, but it is fragile.

**Fix:** Preserve the raw entries verbatim for the dedup test, and only append:

```powershell
$entries = $rawPath -split ';'
if (($entries | ForEach-Object { $_.TrimEnd('\') }) -inotcontains $BinDir.TrimEnd('\')) {
    $newPath = if ($rawPath -eq '') { $BinDir } else { "$rawPath;$BinDir" }
    if ($rawPath -match '%') {
        Set-ItemProperty -Path $key -Name 'Path' -Value $newPath -Type ExpandString
    } else {
        Set-ItemProperty -Path $key -Name 'Path' -Value $newPath -Type String
    }
}
```

This keeps the existing string intact (no empty-segment rewrite) and uses a
consistent `Set-ItemProperty` write on both branches. Also consider trailing-`\`
normalization in the dedup compare so `...\box` and `...\box\` aren't double-added.

### WR-04: `install.ps1` smoke test can report a false negative due to PowerShell command caching

**File:** `install.ps1:64-69`

**Issue:** Step 4 updates `$env:Path` in the live session, then step 5 runs `&
box --help`. PowerShell caches command-name resolution; in a session where `box`
was *not* previously on PATH the new `$env:Path` is honored, but if a different
`box` (or none) was cached, `&  box` may resolve stale or fail, producing a
spurious "did not run in this session" warning even though the install
succeeded. Minor UX-correctness issue, not a functional defect.

**Fix:** Invoke the freshly installed binary by absolute path for the smoke test,
which is unambiguous and matches what the install actually produced:

```powershell
& $ExeDst --help | Out-Null
```

## Info

### IN-01: `encode_relative` / `sanitize_reserved` can return `""`, `"."`, or `".."`, yielding a degenerate copy target

**File:** `src/commands/flatten/rename.rs:35-50,56-73`; consumed at `src/commands/flatten/mod.rs:167`

**Issue:** `encode_relative(Path::new(".."))` returns `""` and
`sanitize_reserved("...")` returns `"."`. If such a value ever became a
`dst_name`, `out_root.join(dst_name)` resolves to the output dir itself (`""`) or
`out/.` / `out/..`, and `fs::copy` into a directory fails with a confusing error.
Reachability is marginal — a walked *file* always contributes a non-dot final
component to `rel` — but the functions are public, unit-tested in isolation, and
make no guarantee they return a non-empty, non-dot name. The CR-01 fix (mapping
an all-dots/spaces name to `"_"`) closes most of this; mirror it in
`encode_relative` for defense in depth.

**Fix:** After cleaning/joining in `encode_relative`, guard the result:
`if joined.is_empty() { return "_".to_string(); }` before calling
`sanitize_reserved`, and rely on the hardened `sanitize_reserved` from CR-01.

### IN-02: `dry_run_summary` ignores the byte total but `real_run_summary` reports it — minor asymmetry, and unused `_` patterns elsewhere

**File:** `src/core/output.rs:208-212`

**Issue:** Purely cosmetic: the dry-run summary cannot report bytes (nothing is
read), which is correct, but a reader comparing the two summary builders may
expect symmetry. No action required beyond a one-line note in the doc comment
that the dry run intentionally omits a size because it performs no I/O. Flagged
only so it is a conscious choice, not an omission.

**Fix:** None required; optionally add `// dry run performs no I/O, so no byte
total is available` to the doc comment.

### IN-03: `arrow_col` / `print_plan` recompute terminal width and column independently of the real-run path

**File:** `src/commands/flatten/mod.rs:143-144,302-304`

**Issue:** `print_plan` (dry-run) computes `arrow_col(plan)` and
`terminal_width()` internally, while the real-run path computes the same two
values at lines 143-144 and passes them per row. The logic is duplicated rather
than shared, so a future change to alignment must be made in two places or the
dry-run preview and the real run can drift in layout (the plan is shared, but the
*rendering* is not). Maintainability only — output is identical today.

**Fix:** Extract a single `render_rows(plan, &mut impl FnMut(...))` or have
`print_plan` take the precomputed `arrow_col`/`width`, so dry-run and real-run
share one rendering path.

---

## Resolution (2026-06-22)

Fixed in commit `e1a8f38` (`fix(01-03): close flatten silent-data-loss holes…`):

- **CR-01 (critical) — FIXED.** `sanitize_reserved` now trims trailing dots/spaces
  from the whole reconstructed name (not just the stem), with an all-dots/spaces →
  `"_"` fallback. Regression tests `sanitize_trims_whole_name_trailing_junk` and
  `trailing_dot_collides_with_bare_name` added. This fallback also substantially
  closes **IN-01** (no more `""`/`"."`/`".."` copy target).
- **WR-01 (warning) — FIXED.** Collision keying switched from `to_ascii_lowercase()`
  to full-Unicode `to_lowercase()` in `dedupe` and at all three `occupied` keying
  sites in `flatten/mod.rs`. Regression test `dedupe_is_case_insensitive_unicode`.
- **WR-02 (warning) — FIXED.** `safe_copy` now opens the destination with
  `create_new(true)` via `io::copy`, so any name-set↔disk drift fails loudly with
  `AlreadyExists` instead of clobbering. Regression test
  `safe_copy_refuses_to_overwrite_existing`.

Verification after fix: `cargo test` 41/41 pass, `cargo clippy --all-targets -- -D
warnings` clean, `cargo fmt` clean, release binary rebuilt and `flatten` re-verified
end-to-end (collision rename + timestamp preservation + source intact).

### Remaining (advisory, non-blocking — tracked as follow-ups)

- **WR-03** — `install.ps1` empty-segment PATH rewrite + dual write-path consistency.
- **WR-04** — `install.ps1` smoke test should invoke `$ExeDst` by absolute path to
  avoid PowerShell command-cache false negatives.
- **IN-02 / IN-03** — cosmetic: dry-run summary byte-total note; share one render path
  between `print_plan` and the real-run loop.

These touch `install.ps1` (already human-verified working) and rendering
maintainability — no correctness or data-safety impact. Deferred to a polish pass.

---

_Reviewed: 2026-06-22_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Resolved: 2026-06-22 (CR-01, WR-01, WR-02 in e1a8f38; advisory items deferred)_
