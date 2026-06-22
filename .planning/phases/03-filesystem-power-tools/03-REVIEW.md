---
phase: 03-filesystem-power-tools
reviewed: 2026-06-22T21:00:06Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - src/cli.rs
  - src/main.rs
  - src/commands/mod.rs
  - src/commands/hash/mod.rs
  - src/commands/tree/mod.rs
  - src/commands/du/mod.rs
  - src/commands/dupes/mod.rs
  - src/commands/bulk_rename/mod.rs
  - src/commands/flatten/mod.rs
  - src/core/errors.rs
  - src/core/input.rs
  - src/core/output.rs
  - tests/hash.rs
  - tests/tree.rs
  - tests/du.rs
  - tests/dupes.rs
  - tests/bulk_rename.rs
  - tests/cmd/tree.trycmd
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-06-22T21:00:06Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Reviewed the five filesystem subcommands (hash, tree, du, dupes, bulk-rename) plus the dispatch and core (`input`, `output`, `errors`) and the supporting `core::fs`/`flatten::rename` modules they lean on. The phase is generally well-built: the destructive `bulk-rename` pre-flight is largely sound (collisions, swaps/chains, case-fold keys, separator refusal all covered and unit-tested), `dupes` is genuinely read-only (no write path exists), `hash` streams correctly, and the color-gating / piped-output invariants hold.

Two correctness defects stand out. **(BLOCKER)** the `bulk-rename` pre-flight refuses `/` and `\` but does NOT refuse a target of exactly `..` or `.` — `parent.join("..")` escapes the target directory, defeating the stated path-injection guard on the one destructive command. **(WARNING)** `box hash --algo sha256 --verify <hash>` cannot distinguish an *explicit* `--algo sha256` from the default, so length auto-detection silently overrides the user's explicit choice: `--algo sha256 --verify <32-hex>` verifies as MD5, not SHA-256.

The remaining items are robustness and quality issues (file-vs-directory arguments produce silent empty output, `du --depth 0` / `--top 0` degenerate inputs, a brittle trycmd fixture).

## Critical Issues

### CR-01: bulk-rename pre-flight does not refuse `..` / `.` targets — path escape on the destructive command

**File:** `src/commands/bulk_rename/mod.rs:185-198` (detection) and `:342-345` (execution)
**Issue:**
The pre-flight's separator rule only refuses targets containing `/` or `\`:

```rust
if r.new.contains('/') || r.new.contains('\\') {
    conflicts.push(Conflict::Separator { ... });
}
```

A replacement that produces a target of exactly `..` or `.` contains no path separator, so it passes pre-flight as "safe", clears the collision/cycle checks (its fold key is `..` / `.`, matching nothing on disk), and reaches the executor:

```rust
let dst = item.parent.join(new_name);          // parent.join("..") -> the PARENT dir
std::fs::rename(&item.src, &dst) ...            // MoveFileExW, silently overwrites
```

`PathBuf::join("..")` does not collapse the component — it yields `<parent>\..`, which resolves to the grandparent directory; `join(".")` yields the parent directory itself. This is exactly the traversal class the module's own doc comment (`:8-13`) and the `Conflict::Separator` rule claim to defend against ("a path-separator-injecting replacement ABORTS the whole batch"). Because `bulk-rename` is the phase's only destructive command and `std::fs::rename` silently overwrites on Windows, a target that escapes the intended directory is a data-loss / unexpected-clobber risk, not merely a refused no-op.

A pattern like `box bulk-rename ./dir ".+" ".." --force` (or any replacement yielding `..`/`.`) reaches the rename call. Even where the OS rejects renaming a file onto an existing directory, the guard is supposed to *abort the batch in memory before any rename*, and here it does not — the contract ("any separator/injection aborts the whole batch") is violated.

**Fix:** Treat `..` and `.` (and any name that is purely dots/spaces, which Windows trims) as refused targets in the same pure pre-flight pass, mirroring `flatten::rename`'s handling. For example, extend the rule-4 predicate:

```rust
fn injects(name: &str) -> bool {
    name.contains('/')
        || name.contains('\\')
        || name == ".."
        || name == "."
        || name.trim_matches(['.', ' ']).is_empty()
}
// rule 4:
for r in renames {
    if injects(&r.new) {
        conflicts.push(Conflict::Separator { source: r.old.clone(), target: r.new.clone() });
    }
}
// and the `safe` filter:
let safe: Vec<&Rename> = renames.iter().filter(|r| !injects(&r.new)).collect();
```

Add a unit test alongside `refuses_path_separators` asserting `rn("a.txt", "..")` and `rn("a.txt", ".")` each yield a `Conflict::Separator`.

## Warnings

### WR-01: `hash --algo sha256 --verify` silently ignores the explicit algorithm

**File:** `src/commands/hash/mod.rs:72-78` and `:144-151`
**Issue:**
`Algo::is_default()` is `self == Algo::Sha256`, and `--algo` uses `default_value_t = Algo::Sha256`. There is therefore no way to tell an *explicit* `--algo sha256` apart from the unset default. In the verify path:

```rust
let algo = if self.algo.is_default() {
    algo_from_len(expected.len())?   // length auto-detect WINS
} else {
    self.algo
};
```

So `box hash --algo sha256 --verify <32-hex>` ignores the user's explicit `sha256` and auto-detects length 32 → **MD5**, computing an MD5 digest and comparing it to the 32-hex value. Likewise `--algo sha256 --verify <128-hex>` verifies as SHA-512. The user asked for SHA-256 and got a different algorithm with no warning — a silent correctness violation of the `--verify` contract. (The non-verify path is unaffected: `digest_reader(self.algo, ...)` uses the value directly.)

**Fix:** Don't infer "explicit vs default" from the value. Either make the algo field an `Option<Algo>` (so `None` = auto-detect, `Some(a)` = honor it), or read clap's explicit-source flag (`ArgMatches::value_source`). Minimal change:

```rust
#[arg(long, value_enum)]
pub algo: Option<Algo>,
// ...
let algo = match self.algo {
    Some(a) => a,                          // explicit choice always wins
    None => algo_from_len(expected.len())?, // only auto-detect when truly unset
};
// non-verify path: let algo = self.algo.unwrap_or(Algo::Sha256);
```

### WR-02: tree/du/dupes silently produce empty output when given a FILE path

**File:** `src/commands/tree/mod.rs:79-97`, `src/commands/du/mod.rs:74-125`, `src/commands/dupes/mod.rs:58-97`
**Issue:**
All three normalize the path and then `WalkDir::new(root).min_depth(1).max_depth(1)` (tree/du) or a full walk (dupes). `normalize_path` succeeds on a regular file, and a depth-1 walk over a file yields zero children. The result: `box tree somefile.txt` prints the filename and `0 directories, 0 files`; `box du somefile.txt` prints `0 of 0 entries shown. 0 B total.`; `box dupes somefile.txt` prints `No duplicate files found.` Each exits 0 with misleading, empty output instead of telling the user the argument is not a directory.

**Fix:** After `normalize_path`, verify the target is a directory and return a clear error otherwise, e.g.:

```rust
if !root.is_dir() {
    anyhow::bail!("{} is not a directory", self.path.display());
}
```

(`du`/`tree` are directory-analysis tools; treating a file argument as "an empty directory" is a usability defect, and for `du` a single-file `du` arguably *should* report that file's size — either way the current silent-empty behavior is wrong.)

### WR-03: `normalize_path` on a missing path yields a raw OS error, not a friendly message

**File:** `src/commands/tree/mod.rs:83-84`, `src/commands/du/mod.rs:78-79`, `src/commands/dupes/mod.rs:62-63`
**Issue:**
`normalize_path` is `dunce::canonicalize`, which fails on a non-existent path. The call sites wrap it with `.with_context(|| format!("resolving {}", self.path.display()))`, so the user sees `error: resolving ./nope: The system cannot find the path specified. (os error 3)`. That is acceptable but inconsistent with the project's stated convention of clear, path-naming errors (FOUND-05/06) — "resolving X" is less clear than "no such directory: X". Lower severity because it does not misbehave, only reads poorly.

**Fix:** Special-case `NotFound` to a friendlier message, or change the context to `format!("no such directory: {}", self.path.display())` after a pre-check (`self.path.exists()`), so the common typo path is self-explanatory.

### WR-04: `du --depth 0` and `--top 0` are silently degenerate rather than rejected

**File:** `src/commands/du/mod.rs:97-99` (`--top`) and `:162-168` (`--depth`)
**Issue:**
`--depth 0` makes `dir_total` build `WalkDir::new(dir).min_depth(1).max_depth(0)`, an empty walk, so every directory child reports a recursive total of `0` and the summary total collapses to just the file children — a confusing result with no diagnostic. `--top 0` calls `rows.truncate(0)`, printing no rows but a `0 of N entries shown.` summary. Neither input is rejected or explained. `tree --depth 0` has the analogous behavior (shows only the root). These are edge-case inputs, but a `0` depth/top almost certainly indicates user error and currently produces output that looks like a bug.

**Fix:** Reject `0` for `--depth`/`--top` with a usage error, or document and clamp (`depth.max(1)`). Clap supports `value_parser = clap::value_parser!(usize).range(1..)` to reject `0` at parse time with a clear message.

### WR-05: `--recursive` bulk-rename can rename a file into a directory it has already descended

**File:** `src/commands/bulk_rename/mod.rs:372-457`
**Issue:**
Under `--recursive`, `build_plan` walks with `WalkDir` and renames files in place per parent directory. Directories themselves are skipped (`:409-421`), and the per-directory pre-flight catches in-directory clobbers/cycles. However, the pre-flight only seeds its occupied set from `read_dir_names(dir)` of that one parent (`:504-507`) and only considers renames whose `parent` equals that dir. A rename target is validated solely as a base name within its own parent. That is correct for the documented model, but the walk order under `--recursive` combined with renaming files *as the executor iterates* means the executor's `plan.items` ordering (walk order) is relied upon to be safe. Since pre-flight already aborts on any chain/cycle/collision, executed renames are mutually independent and order-insensitive — so this is not a data-loss bug — but there is no test exercising `--recursive` across *multiple* directories with same-named targets in sibling dirs, leaving the cross-directory scoping unverified by the suite. Treat as a test-coverage gap on the destructive command.

**Fix:** Add an integration test: two sibling subdirectories each containing `a.txt`, a pattern renaming `a.txt`→`b.txt` in both, asserting both succeed independently and neither directory's pre-flight bleeds into the other. This locks the per-directory scoping that the safety argument depends on.

## Info

### IN-01: Magic exit codes use bare `ExitCode::from(2)` / `from(1)` without named constants

**File:** `src/main.rs:47,104,105`
**Issue:** The 0/1/2 exit policy is central (documented in the module header), but the codes appear as bare literals. A named constant (`const EXIT_USAGE: u8 = 2;`) would make the policy self-documenting and prevent drift between the two exit-2 sites.
**Fix:** Introduce `const` names for the three exit codes and use them at every `ExitCode::from` site.

### IN-02: `let _ = is_color_on();` is dead and misleading

**File:** `src/commands/bulk_rename/mod.rs:364`
**Issue:** `let _ = is_color_on();` immediately after the rename loop has no effect — its comment even says color is applied inside `format_row`. It reads as a leftover and may confuse a maintainer into thinking it primes state.
**Fix:** Delete the line (and the now-unused `is_color_on` import on `:44` if nothing else in the module uses it).

### IN-03: `tree.trycmd` fixture is brittle on Windows / casing-sensitive renderers

**File:** `tests/cmd/tree.trycmd:1-11`
**Issue:** The golden output hard-codes `project` as the root label and a specific glyph layout. `tree` prints `self.path.to_string_lossy()` as the root label verbatim (`src/commands/tree/mod.rs:88`), so the fixture only matches when invoked exactly as `box tree project` from the fixture's working directory. It also assumes color-off (true under trycmd's pipe) and the dirs-first/case-insensitive sort. This is fine today but is the most fragile test in the phase; any change to the root-label rendering or sort tiebreak silently breaks it with an opaque diff.
**Fix:** Add a short comment in the trycmd (or the harness) documenting the working-directory and no-TTY assumptions, so a future breakage is diagnosable.

### IN-04: `dupes` does not de-duplicate hardlinks / same-inode candidates

**File:** `src/commands/dupes/mod.rs:104-127`
**Issue:** Two hardlinks to one inode have identical size and content, so they are reported as a "duplicate group" wasting space, even though they occupy a single physical allocation. This over-reports wasted space. Not incorrect for a content-equality tool, but worth a documented caveat. (Out of v1 scope to fix; noted for completeness.)
**Fix:** Optionally compare `(volume, file index)` on Windows to collapse hardlinks, or document that hardlinks are counted as duplicates.

---

_Reviewed: 2026-06-22T21:00:06Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
