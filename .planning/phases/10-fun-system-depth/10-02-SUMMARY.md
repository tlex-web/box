---
phase: 10-fun-system-depth
plan: 02
subsystem: cli
tags: [clap, value-enum, serde, include_str, fortune, roast, taxonomy, json]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "is_json_on()/emit_json()/out_line() spine and the hash::Algo ValueEnum shape"
provides:
  - "box fortune --category <wisdom|tech|humor> filter + --list-categories enumerator"
  - "box fortune --json now emits {text, category} (concrete bucket, even on the union path)"
  - "box roast --language <general|python|javascript|rust> ecosystem buckets"
  - "box roast --json now emits {text, language} (resolved bucket, general when omitted)"
  - "per-bucket embedded corpora under src/data/fortunes/ and src/data/roasts/"
  - ".gitattributes eol=lf glob lock for the new per-bucket data dirs"
affects: [fun-commands, scriptable-json-spine, cowsay-figures, eight-ball-sentiment]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ValueEnum + serde(rename_all=lowercase) taxonomy enum mirrored from hash::Algo (one spelling table for clap args AND JSON output)"
    - "per-bucket include_str! corpus split with a bucket-tagged union for the default draw"
    - "filter-the-slice-first-then-choose (never % len, never a fixed seed)"

key-files:
  created:
    - src/data/fortunes/wisdom.txt
    - src/data/fortunes/tech.txt
    - src/data/fortunes/humor.txt
    - src/data/roasts/general.txt
    - src/data/roasts/python.txt
    - src/data/roasts/javascript.txt
    - src/data/roasts/rust.txt
  modified:
    - src/commands/fortune/mod.rs
    - src/commands/roast/mod.rs
    - .gitattributes
    - tests/fortune.rs
    - tests/roast.rs

key-decisions:
  - "fortune: bare draw = UNION of all buckets (D-04); roast: bare draw = general default bucket only (D-01)"
  - "category/language resolved from the drawn entry's bucket so the JSON scalar is ALWAYS concrete"
  - "unknown --category/--language handled by clap ValueEnum (exit-2 usage error listing valid values) — no custom validation path"
  - "kept the v1 corpora intact: fortunes.txt -> wisdom.txt, roasts.txt -> general.txt (git renames), new buckets are additive"

patterns-established:
  - "Taxonomy enum: derive(ValueEnum, serde::Serialize) + rename_all=lowercase, Option<Enum> arg, None => default/union"
  - "Test-only pub(crate) helpers gated with #[cfg(test)] to stay dead-code clean under clippy -D warnings"

requirements-completed: [FORT-V2-01, ROST-V2-01]

# Metrics
duration: 19min
completed: 2026-07-14
---

# Phase 10 Plan 02: Fortune Categories + Roast Language Buckets Summary

**`box fortune` gains wisdom/tech/humor categories and `box roast` gains general/python/javascript/rust ecosystem buckets, each backed by per-bucket `include_str!` corpora, a `ValueEnum` filter flag, and a new concrete scalar (`category`/`language`) in the `--json` document.**

## Performance

- **Duration:** ~19 min
- **Started:** 2026-07-14T14:07:16Z
- **Completed:** 2026-07-14T14:26:13Z
- **Tasks:** 2 (both TDD: RED → GREEN)
- **Files modified:** 5 modified, 7 created, 2 retired-via-rename

## Accomplishments
- **fortune categories (FORT-V2-01):** split the 70-line corpus into `wisdom` (70) / `tech` (26) / `humor` (20) buckets; added a `Category` `ValueEnum`, `--category` filter, `--list-categories` fast-exit, and a concrete `category` JSON field. Bare `box fortune` still draws from the union of all three (v1 behavior preserved).
- **roast language buckets (ROST-V2-01):** moved the 42-line corpus into `general` (the default) and authored English `python`/`javascript`/`rust` ecosystem buckets (15 each); added a `Language` `ValueEnum`, `--language` filter (`None` → `general`), and a `language` JSON field.
- **shared .gitattributes reorg:** replaced the two single-file `eol=lf` lines with per-bucket-dir globs (`src/data/fortunes/*.txt`, `src/data/roasts/*.txt`) — one owner for the LF lock; `git check-attr` confirms `eol: lf` on the new files.
- **taxonomy validation for free:** an unknown `--category`/`--language` is a clap exit-2 usage error listing the valid values (T-10-02-ENUM mitigated) — no free-form string reaches a filter path.

## Task Commits

Each task was committed atomically (TDD test → feat):

1. **Task 1 (RED): fortune category tests** - `a5d5690` (test)
2. **Task 1 (GREEN): fortune categories + corpus split** - `f8b7832` (feat)
3. **Task 2 (RED): roast language tests** - `c686376` (test)
4. **Task 2 (GREEN): roast language buckets + corpus split** - `0b56124` (feat)

_No REFACTOR commits were needed — both GREEN implementations landed clean._

## Files Created/Modified
- `src/commands/fortune/mod.rs` - `Category` ValueEnum, per-bucket `include_str!` consts, `--category`/`--list-categories`, bucket-tagged union loader, `{text,category}` JSON
- `src/commands/roast/mod.rs` - `Language` ValueEnum, per-bucket `include_str!` consts, `--language` (None→general), `{text,language}` JSON
- `src/data/fortunes/{wisdom,tech,humor}.txt` - the three fortune buckets (wisdom = the preserved v1 corpus; tech/humor new, pure-ASCII CC0/original)
- `src/data/roasts/{general,python,javascript,rust}.txt` - the four roast buckets (general = the preserved v1 corpus; python/js/rust new English ecosystem roasts)
- `.gitattributes` - per-bucket-dir `eol=lf` globs replacing the two single-file lines
- `tests/fortune.rs` - category filter / list-categories / bare-concrete-category / unknown-exit-2 / bucket-membership cases; union-membership base updated to read the three bucket files
- `tests/roast.rs` - language filter / bare-general / unknown-exit-2 / bucket-membership cases; base membership updated to read the general bucket

## Decisions Made
- **Asymmetric default draw (per CONTEXT D-01/D-04):** fortune bare = union of all buckets; roast bare = the `general` bucket only. This preserves each command's exact v1 behavior while adding the taxonomy.
- **Concrete JSON scalar:** the union path tags every entry with its bucket, so `category` is always a real name (never null) even without `--category`.
- **No custom validation:** clap's `ValueEnum` is the single gate for both the arg and the JSON spelling (`rename_all="lowercase"`), so the arg value and the serialized field can never drift.
- **Preserve curated content:** the v1 corpora were kept verbatim as `wisdom.txt` / `general.txt` (tracked as git renames); the new buckets are strictly additive.

## Deviations from Plan

None on the implementation — both tasks executed exactly as written (data split + ValueEnum + filter flag + list flag + new scalar + `.gitattributes` reorg + extended tests, all per the plan's `<action>` blocks).

## Issues Encountered

**Worktree spawned from a stale base commit (resolved at startup).**
- **Symptom:** this agent's worktree branch (`worktree-agent-a879b4086ed9f2bde`) was created from `986c841` (end of Phase 06), which predates the fortune/roast `--json` spine that plan 10-02 assumes (the plan's line-anchored interfaces reference `is_json_on()` forks + `FortuneOutput`/`RoastOutput` that did not exist at that commit). All three sibling wave-1 agents (`...a0f6e3`, `...a67c37`, `...ac339b`) were correctly based on `main` (`d2802e3`).
- **Resolution:** the working tree was clean and `986c841` is a strict ancestor of `main`, so I fast-forwarded the branch onto `main` (`git reset --hard main`) at agent startup — the sanctioned worktree base-alignment — before writing any code. Post-reset the base carries the JSON spine and the phase-10 plan files, matching the plan's assumptions and the sibling agents' base.
- **Merge impact:** my branch is now based on `d2802e3` like the siblings, so the orchestrator's merge-back sees only the 4 commits above (no phase 7-9 regression). Had I executed on the stale base, the merge would have reverted phases 7-9.

## Threat surface
No new trust boundaries introduced. The plan's threat register is satisfied: T-10-02-ENUM (ValueEnum rejects unknown values → exit 2, verified), T-10-02-IO (buckets are compile-time `include_str!`, no runtime file read), T-10-02-EOL (`.gitattributes eol=lf` glob + defensive `str::trim`, `git check-attr` confirms `eol: lf`). No stubs, no placeholder data.

## Verification
- `cargo test` full suite green (fortune: 5 unit + 8 integration; roast: 4 unit + 7 integration; all other suites unaffected).
- `cargo clippy --all-targets -- -D warnings` clean.
- `git check-attr text eol` reports `eol: lf` on the new bucket files.
- Manual smoke test of all 7 acceptance behaviors passed (list-categories, category/language filters, bare concrete scalar, exit-2 usage errors with value lists).

## Next Phase Readiness
- The `{text, category}` / `{text, language}` shapes are the schema Phase 11's `box config`/JSON consumers can rely on.
- Taxonomy-enum pattern (ValueEnum + serde lowercase + Option-default) is reusable for the remaining fun-command depth flags (cowsay `--figure`, 8ball sentiment).

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
