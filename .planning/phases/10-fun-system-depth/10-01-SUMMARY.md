---
phase: 10-fun-system-depth
plan: 01
subsystem: cli-commands
tags: [cowsay, eight-ball, ascii-art, owo-colors, serde, clap, sentiment, thiserror]

# Dependency graph
requires:
  - phase: 07-scriptable-spine
    provides: is_json_on()/emit_json/out_line spine + is_color_on() gate forked by both commands
  - phase: 04-terminal-visuals
    provides: cowsay bubble()/wrap() + 8ball tone partition (the reuse seams)
provides:
  - "cowsay --figure/--list-figures curated built-in roster (cow/tux/dragon/ghost/dog/stegosaurus), no external .cow I/O"
  - "cowsay --think thought bubble: parameterized bubble() (rounded ( ) delimiters + o-dot tether)"
  - "cowsay flat --json gains a figure field"
  - "8ball compact ASCII art with sentiment-colored answer (is_color_on()-gated)"
  - "8ball pure sentiment(index) mapper over the 10/5/5 partition + snake_case sentiment JSON field"
  - "BoxError::UnknownFigure typed usage variant (exit 2, lists roster)"
affects: [11-meta-commands, phase-11-completions]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Figure roster as a const name->art table with an @ tether placeholder substituted at render time (speech \\ / think o)"
    - "bubble() parameterized by a think flag (not a second renderer) to keep the speech form byte-identical"
    - "Sentiment derived from the drawn answer's recovered index — pure lookup, no new data"

key-files:
  created: []
  modified:
    - src/commands/cowsay/mod.rs
    - src/commands/eight_ball/mod.rs
    - tests/cowsay.rs
    - tests/eight_ball.rs
    - src/core/errors.rs
    - src/main.rs

key-decisions:
  - "Added a typed BoxError::UnknownFigure variant (not anyhow::bail!) so an unknown --figure maps to exit 2 AND lists the roster — a bail would be exit 1 under main.rs's downcast policy"
  - "Figure templates use an '@' tether placeholder; COW_ART substitutes back to the Phase-1 classic cow byte-for-byte, so default output is unchanged"
  - "8ball draws with the unbiased choose then recovers the index via position() — keeps the module's no-%len discipline while enabling the sentiment lookup"
  - "8ball answer is printed on its own line below the fixed ball art so it stays a findable substring and the icon stays byte-stable"

patterns-established:
  - "Tether-placeholder substitution for think-vs-speech figure rendering"
  - "Parameterized single renderer over duplicated renderers for bubble variants"

requirements-completed: [COW-V2-01, 8BAL-V2-01]

# Metrics
duration: 11min
completed: 2026-07-14
---

# Phase 10 Plan 01: Cowsay Figures/Think + 8ball ASCII/Sentiment Summary

**Grafted additive fun-visual depth onto two shipped commands: a curated built-in cowsay figure roster with `--figure`/`--list-figures`/`--think`, and a compact ASCII 8-ball with tone-mapped sentiment color plus a scriptable `sentiment` JSON field — without disturbing the frozen pure-ASCII, byte-stable, `is_json_on()`/`is_color_on()` contracts.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-07-14T14:13:21Z
- **Completed:** 2026-07-14T14:24:24Z
- **Tasks:** 2 (both TDD: RED → GREEN)
- **Files modified:** 6

## Accomplishments
- cowsay gained a curated, source-embedded figure roster (6 figures) with `--figure <name>`, `--list-figures`, and a `--think` thought bubble — all pure-ASCII, no external `.cow` file I/O (D-02/D-03).
- The classic no-flag `box cowsay` output is byte-identical to Phase 1 (the roster's `cow` template renders back to the exact classic art; the `cowsay-single/multi` trycmd snapshots stay green).
- 8ball now renders a compact ASCII 8-ball with the drawn answer tone-colored (affirmative→green / non_committal→yellow / negative→red), gated strictly on `is_color_on()`, and emits a flat `{text, sentiment}` JSON document (D-05).
- Sentiment is a pure `sentiment(index)` lookup over the existing 10/5/5 partition — zero new answer data.

## Task Commits

Each task was committed atomically (TDD: test → feat):

1. **Task 1 (RED): cowsay figure/list/think tests** — `280cef7` (test)
2. **Task 1 (GREEN): cowsay figures + --figure/--list-figures + --think** — `8f18008` (feat)
3. **Task 2 (RED): 8ball sentiment + ASCII-art tests** — `d334317` (test)
4. **Task 2 (GREEN): 8ball ASCII art + sentiment color + sentiment JSON** — `e8f2c57` (feat)

_TDD tasks produced test → feat commit pairs (no refactor commit was needed)._

## Files Created/Modified
- `src/commands/cowsay/mod.rs` — figure roster const table + `@`-tether placeholder, `find_figure`/`list_figures`/`render_figure`, `--figure`/`--list-figures`/`--think` args, parameterized `bubble(lines, think)`, `figure` JSON field, updated + new unit tests.
- `src/commands/eight_ball/mod.rs` — `Sentiment` enum (snake_case Serialize), pure `sentiment(index)` mapper, `BALL_ART` const, `is_color_on()`-gated answer coloring, `EightBallOutput.sentiment`, index recovery via `position()`, new unit tests.
- `tests/cowsay.rs` — figure-select / list-figures / think / unknown-figure-exit-2 integration coverage.
- `tests/eight_ball.rs` — sentiment-field `--json` purity + ASCII-art multi-line coverage; adapted the existing answer assertions to substring containment (answer now sits inside the art).
- `src/core/errors.rs` — new `BoxError::UnknownFigure { name, available }` typed usage variant.
- `src/main.rs` — added `UnknownFigure` to the exit-2 downcast arm (+ comment update).

## Decisions Made
- **Typed error over bail for exit 2:** `main.rs` only maps specific typed `BoxError` variants to exit code 2; a plain `anyhow::bail!` would surface as exit 1. To satisfy the acceptance criterion (unknown `--figure` → exit 2 with a message listing figures), a new `BoxError::UnknownFigure` variant was added and threaded into the `main.rs` downcast — the "`MissingInput`-style usage error" the plan preferred.
- **Tether placeholder for think-mode:** each figure template carries an `@` marker at its tether positions; `render_figure` substitutes `\` (speech) or `o` (think). This keeps a single figure table serving both bubble modes and guarantees the default cow is byte-identical.
- **Single parameterized `bubble()`:** `--think` is a `think: bool` parameter selecting the delimiter glyphs (`( )` vs `< >` / `|`), not a second renderer, preserving the speech form exactly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added a typed `BoxError::UnknownFigure` variant + main.rs exit-2 mapping**
- **Found during:** Task 1 (cowsay figures)
- **Issue:** The acceptance criterion requires `box cowsay --figure <bad>` to exit 2 AND print a message naming the available figures. `main.rs` maps only specific typed `BoxError` variants to exit 2; a raw `anyhow::bail!` (or reusing `MissingInput`, whose message is fixed and figure-less) could not satisfy both requirements. The plan's `files_modified` scoped only the two command modules + their tests, so touching `errors.rs`/`main.rs` is outside the declared file set.
- **Fix:** Added `BoxError::UnknownFigure { name, available }` (message lists the roster) and extended the `main.rs` exit-2 downcast arm to include it (plus a comment refresh from "three" to "four" variants).
- **Files modified:** `src/core/errors.rs`, `src/main.rs`
- **Verification:** `unknown_figure_exits_2_and_lists_available` asserts exit code 2 and that stderr contains `cow` and `dragon`; the full test suite and `cargo clippy -D warnings` are clean.
- **Committed in:** `8f18008` (Task 1 GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 missing-critical — required to meet the plan's own acceptance criteria).
**Impact on plan:** The extra two files are the minimal, additive surface needed to route the unknown-figure usage error to exit 2 per the threat register (T-10-01-FIG) and acceptance criteria. No scope creep beyond that requirement.

## Issues Encountered
None. Both tasks followed RED → GREEN cleanly; the default cowsay output stayed byte-identical (trycmd green) and 8ball's existing integration assertions were adapted to the new art-wrapped output via substring containment.

## Verification
- `cargo test --bin box cowsay` / `cargo test --test cowsay` — green (10 unit + 5 integration).
- `cargo test --bin box eight_ball` / `cargo test --test eight_ball` — green (5 unit + 6 integration).
- `cargo test` (full suite) — 213 unit + all integration suites pass, 0 failures.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `tests/cmd/cowsay-single.trycmd` + `cowsay-multi.trycmd` + `help.trycmd` — green (default output + top-level help unchanged).
- Manual: `box cowsay --think`, `--figure dragon`, `--list-figures`, `--figure tux --json`; `box 8ball` (art + colored answer) and `box 8ball --json` (`{text, sentiment}`, no ANSI).

## Threat Flags
None. The new surface matches the plan's `<threat_model>`: `--figure` resolves only against the fixed const roster (T-10-01-FIG mitigated via `BoxError::UnknownFigure` exit-2), the 8ball question stays display-only (T-10-01-Q accepted), and sentiment/figure color is emitted only inside `is_color_on()` so `--json` stdout stays ANSI-free (T-10-01-ANSI mitigated, asserted by the no-`0x1B` tests).

## Known Stubs
None. Both features are fully wired (no placeholder data, no TODO/coming-soon paths).

## Next Phase Readiness
- Both commands' final flag surface (`--figure`/`--list-figures`/`--think`, `sentiment` JSON) is settled — Phase 11 `completions` can generate against it.
- No blockers. This plan is Wave-1 (`depends_on: []`) and shares no files with the other Wave-1 plans, so it merges cleanly.

## Self-Check: PASSED

All 6 modified files and the SUMMARY exist on disk; all task commits (`280cef7`, `8f18008`, `d334317`, `e8f2c57`) and the metadata commit (`e6b7c12`) are present in git history. Working tree clean.

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
