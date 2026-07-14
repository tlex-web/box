---
phase: 02-pure-transform-utilities
plan: 05
subsystem: commands
tags: [rust, clap, rand, thread-rng, include-str, whimsy, fortune, eight-ball, roast, tdd]

# Dependency graph
requires:
  - phase: 02-pure-transform-utilities
    plan: 01
    provides: "rand 0.9 in Cargo.toml; core::output::terminal_width()"
  - phase: 02-pure-transform-utilities
    plan: 04
    provides: "Embedded-content pattern (include_str! + .gitattributes eol=lf LF lock); rand RNG + unbiased IndexedRandom::choose; TDD RED→GREEN commit discipline; command pattern (clap Args + thin run() + pub(crate) parse helpers + #[cfg(test)] units)"
  - phase: 01-foundation-flatten
    provides: "RunCommand trait; main.rs single exit-code owner; cli.rs Commands enum + doc-comment help lock; help.trycmd"
provides:
  - "box fortune — random aphorism from an embedded CC0/public-domain list (70 entries); rand::rng() + IndexedRandom::choose; soft-wrap when wider than the terminal (FORT-01 fits-the-terminal); pub(crate) entries()"
  - "box roast — random programmer one-liner from an embedded self-authored/CC0 list (42 entries); identical shape to fortune (ROST-01)"
  - "box 8ball — random answer from the canonical 20 Magic 8-Ball answers (const &[&str], 10/5/5 by tone, no padding); optional question accepted but ignored for the draw; lives in the eight_ball module with the 8ball CLI name preserved (8BAL-01)"
  - "src/data/fortunes.txt + src/data/roasts.txt (include_str! assets, LF-locked); .gitattributes extended"
  - "Phase 2 COMPLETE: all 9 pure-transform commands functional; zero not_implemented arms remain for any Phase-2 command"
affects: [hash, json, lolcat, tree, du, dupes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Whimsy random-pick: rand::rng() (OS-seeded ThreadRng, ChaCha12 — NOT the pre-0.9 free-function spelling) + IndexedRandom::choose over an embedded slice; never modulo-indexed selection, never a fixed seed (D-08); a fresh process reseeds from the OS so repeated calls differ"
    - "Non-determinism tested by PROPERTY only (RESEARCH Pattern F): membership in the source list + N=10-runs-yield-≥2-distinct; never a seeded/exact value"
    - "Membership-test exposure: each list command exposes pub(crate) fn entries() (fortune/roast) or a const (8ball); integration tests parse the SAME embedded asset (include_str! across the crate boundary) so test and binary share one source of truth"
    - "include_str! text assets get a .gitattributes `eol=lf` rule (CRLF-leak root-cause fix on autocrlf=true) + a defensive .trim() in the loader (belt-and-suspenders) — the Plan 04 EFF-wordlist pattern reused"
    - "Fits-the-terminal soft-wrap breaks only between words (an over-long single word is left whole), so wrapped output whitespace-collapses back to the source entry — the membership test stays valid"
    - "8ball→eight_ball module-name pitfall: Rust identifiers can't start with a digit, so the module dir is eight_ball/ while the CLI name stays 8ball via #[command(name = \"8ball\")] (STATE.md pitfall confirmed)"

key-files:
  created:
    - "src/data/fortunes.txt — 70 public-domain/CC0/original aphorisms, one per line, LF"
    - "src/data/roasts.txt — 42 self-authored/CC0 programmer one-liners, one per line, LF"
    - "src/commands/fortune/mod.rs — FortuneArgs + entries() + soft_wrap() + 3 unit tests"
    - "src/commands/roast/mod.rs — RoastArgs + entries() + soft_wrap() + 3 unit tests"
    - "src/commands/eight_ball/mod.rs — EightBallArgs { question } + const EIGHT_BALL_ANSWERS (canonical 20) + 3 unit tests"
    - "tests/fortune.rs — FORT-01 membership + varies-across-runs (N=10)"
    - "tests/roast.rs — ROST-01 membership + varies-across-runs (N=10)"
    - "tests/eight_ball.rs — 8BAL-01 membership (with + without a question) + varies-across-runs (N=10)"
  modified:
    - "src/commands/mod.rs — pub mod fortune; pub mod roast; pub mod eight_ball;"
    - "src/cli.rs — Fortune/EightBall/Roast unit variants swapped for Args-carrying variants (doc comments byte-identical; #[command(name = \"8ball\")] preserved; help.trycmd lock intact)"
    - "src/main.rs — Fortune/EightBall/Roast dispatch arms swapped from not_implemented to args.run()"
    - ".gitattributes — eol=lf lock added for fortunes.txt and roasts.txt"

key-decisions:
  - "Whimsy RNG is rand::rng() (OS-seeded ThreadRng), NOT OsRng — these are decorative, not security; no CSPRNG requirement (D-08). Unbiased IndexedRandom::choose is still used (over % len) as a distribution-quality choice, not a security gate."
  - "fortune soft-wraps at the terminal width only when the chosen line exceeds it (FORT-01 fits-the-terminal, Open Question 3), and breaks only between words — so a wrapped render whitespace-collapses back to its source entry, keeping the membership test valid. roast reuses the same helper for symmetry (low cost, consistent UX)."
  - "8ball's optional question is accepted but explicitly discarded for the draw (classic 8-ball) — `let _ = self.question;` makes the no-injection-surface contract (T-02-10) self-documenting; the answer is drawn uniformly regardless."
  - "Integration tests include_str! the SAME asset the binary embeds and parse it identically, so membership is asserted against one source of truth (no hardcoded duplicate of the 70/42 lists). 8ball's 20 answers are duplicated in the test (a const can't be re-exported across the crate boundary cheaply); the in-module tone-split + count unit tests guard the const's shape."
  - "fortune/roast counts chosen at 70 and 42 (within D-09 [50,150] and [30,80]) — comfortable mid-range, no licensing risk (public-domain proverbs / self-authored roasts, no BSD datfiles bundled)."

patterns-established:
  - "Whimsy random pick = rand::rng() + IndexedRandom::choose over an embedded slice, tested by membership + N-runs-differ properties (never a seeded value) — the canonical decorative-randomness shape for the toolbox"
  - "A list command's source-of-truth list is shared between the binary (include_str!) and its integration test (include_str! of the same path), so membership tests cannot drift from the shipped data"

requirements-completed: [FORT-01, 8BAL-01, ROST-01]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 2 Plan 05: fortune + 8ball + roast Commands Summary

**The three Phase-2 whimsy random-pick commands — `fortune` (random aphorism from 70 embedded CC0/public-domain entries, soft-wrapped to the terminal when over-wide), `roast` (random programmer one-liner from 42 self-authored/CC0 entries), and `8ball` (one of the canonical 20 Magic 8-Ball answers, optional question ignored for the draw) — each built on `rand::rng()` (OS-seeded ThreadRng) + unbiased `IndexedRandom::choose`, with non-determinism proven by membership + N=10-runs-differ property tests (never a seeded value). `fortune`/`roast` embed `include_str!` LF-locked text assets; `8ball` lives in the `eight_ball` module (digit-leading-identifier pitfall) with its `8ball` CLI name preserved. This is the final plan of Phase 2 — all 9 pure-transform commands are now functional and every Phase-2 `not_implemented` arm is gone.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-22T16:43:11Z
- **Completed:** 2026-06-22
- **Tasks:** 2 (both TDD RED→GREEN — 4 commits total)
- **Files:** 12 (8 created, 4 modified)

## Accomplishments

- **`box fortune`** prints one random aphorism from an embedded list of 70 public-domain / CC0 / original proverbs. Randomness is `rand::rng()` (OS-seeded ThreadRng, ChaCha12) + the uniform `IndexedRandom::choose` — no fixed seed, no modulo-indexed selection. For FORT-01's "fits the terminal" criterion (Open Question 3), a line wider than the current terminal is greedy soft-wrapped at word boundaries (an over-long single word is left whole); shorter lines print verbatim. The parsed list is exposed as `pub(crate) fn entries()` so unit tests assert the count is in the D-09 range [50, 150] and no entry is empty.
- **`box roast`** prints one random programmer roast from an embedded list of 42 self-authored / CC0 one-liners — identical shape to `fortune` (same RNG path, same `entries()` exposure, same soft-wrap helper), count in the D-09 range [30, 80].
- **`box 8ball`** answers a yes/no question with one of the canonical 20 Magic 8-Ball answers, held as a `const EIGHT_BALL_ANSWERS: &[&str]` grouped 10 affirmative / 5 non-committal / 5 negative (no padding). The optional `question` argument is accepted for the ritual but deliberately discarded for the draw (classic 8-ball) — it is never interpreted, so there is no injection surface (T-02-10 accept). The Rust module is `eight_ball` (identifiers can't start with a digit) while the CLI name stays `8ball` via the preserved `#[command(name = "8ball")]` attribute.
- **Embedded data assets:** created `src/data/fortunes.txt` and `src/data/roasts.txt` (one entry per line, LF), and extended `.gitattributes` with `eol=lf` rules for both so no `\r` leaks into the `include_str!`-embedded lines on a CRLF checkout (`core.autocrlf=true`) — the Plan 04 EFF-wordlist root-cause fix reused; the loaders also `.trim()` defensively.
- **Stubs removed — Phase 2 complete:** the `not_implemented("fortune")`, `not_implemented("8ball")`, and `not_implemented("roast")` arms are all gone; all three modules are registered and dispatched. A grep confirms zero `not_implemented(...)` arms remain for any of the 9 Phase-2 commands. The `box --help` trycmd snapshot still passes (the `///` variant doc comments and the `8ball` name were preserved byte-identical).
- **Tests:** FORT-01 (2 integration + 3 unit), ROST-01 (2 integration + 3 unit), 8BAL-01 (3 integration + 3 unit) all green; full suite green (67 binary-unit + 47 integration/snapshot across 10 test files); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean. Source acceptance greps pass: `thread_rng|gen_range` returns 0 for fortune/roast, `include_str!` returns exactly 1 in fortune, the 8ball module path is `eight_ball/`, and the dispatch arm `Commands::EightBall(args) => args.run()` is present once.

## Task Commits

Each task followed TDD RED→GREEN (a failing `test(...)` commit before the `feat(...)` implementation):

1. **Task 1 (fortune + roast) RED** — `603284c` (test): failing FORT-01/ROST-01 integration tests + the two CC0 data assets + `.gitattributes` LF locks.
2. **Task 1 (fortune + roast) GREEN** — `ea6f144` (feat): FortuneArgs/RoastArgs + `rand::rng()` + `IndexedRandom::choose` + soft-wrap + `entries()` + cli/main swap; integration + unit suites green.
3. **Task 2 (8ball) RED** — `9cc8e38` (test): failing 8BAL-01 integration tests (membership + varies-across-runs) against the stub.
4. **Task 2 (8ball) GREEN** — `cf23c8a` (feat): EightBallArgs + the canonical-20 const + `rand::rng()` + cli/main swap (`#[command(name = "8ball")]` preserved); integration + unit + cli trycmd green.

**Plan metadata:** (docs commit — this SUMMARY + STATE.md + ROADMAP.md + REQUIREMENTS.md)

## Files Created/Modified

- `src/data/fortunes.txt` (created) — 70 public-domain/CC0/original aphorisms, LF.
- `src/data/roasts.txt` (created) — 42 self-authored/CC0 programmer one-liners, LF.
- `src/commands/fortune/mod.rs` (created) — `FortuneArgs`, `pub(crate) entries()`, `soft_wrap()`, `FORTUNES_RAW` embed, 3 unit tests.
- `src/commands/roast/mod.rs` (created) — `RoastArgs`, `pub(crate) entries()`, `soft_wrap()`, `ROASTS_RAW` embed, 3 unit tests.
- `src/commands/eight_ball/mod.rs` (created) — `EightBallArgs { question }`, `const EIGHT_BALL_ANSWERS` (canonical 20), 3 unit tests.
- `tests/fortune.rs` (created) — FORT-01 membership + varies-across-runs.
- `tests/roast.rs` (created) — ROST-01 membership + varies-across-runs.
- `tests/eight_ball.rs` (created) — 8BAL-01 membership (with/without question) + varies-across-runs.
- `src/commands/mod.rs` (modified) — registered `fortune`, `roast`, `eight_ball`.
- `src/cli.rs` (modified) — `Fortune(FortuneArgs)` / `EightBall(EightBallArgs)` / `Roast(RoastArgs)` variant swaps (doc comments byte-identical; `#[command(name = "8ball")]` preserved).
- `src/main.rs` (modified) — three dispatch arms swapped from `not_implemented` to `args.run()`.
- `.gitattributes` (modified) — `eol=lf` rules for `fortunes.txt` and `roasts.txt`.

## Decisions Made

- **Whimsy RNG = `rand::rng()`, not `OsRng`:** these commands are decorative, not security — no CSPRNG requirement (D-08). Unbiased `IndexedRandom::choose` is still used over `% len` as a distribution-quality choice (avoids modulo bias), not a security gate.
- **fortune fits-the-terminal soft-wrap (FORT-01 / Open Question 3):** wrap only when the chosen line exceeds the terminal width, breaking only between words (an over-long single word stays whole). This keeps wrapped output whitespace-equal to its source entry, so the membership test remains valid. `roast` reuses the same helper for a consistent UX at near-zero cost.
- **8ball question is discarded for the draw:** `let _ = self.question;` documents the classic-8-ball no-injection contract (T-02-10); the answer is drawn uniformly regardless of the question.
- **One source of truth for membership:** the integration tests `include_str!` the same `fortunes.txt`/`roasts.txt` the binary embeds and parse them identically, so the membership assertion can never drift from the shipped data. The 8ball 20 are duplicated in the test (a `const` doesn't re-export cheaply); the in-module count + tone-split + non-empty unit tests guard the const's shape.
- **Counts 70 / 42:** comfortable mid-range within the D-09 windows, with zero licensing risk (public-domain proverbs and self-authored roasts; no BSD `fortune` datfiles bundled).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Acceptance grep false positives in fortune doc/comments (`thread_rng` and `include_str!`)**
- **Found during:** Task 1 GREEN (acceptance-criteria source greps).
- **Issue:** `grep -c 'thread_rng\|gen_range' src/commands/fortune/mod.rs` returned 1 and `grep -c 'include_str!'` returned 3, both because the literal tokens appeared in doc/comments (a comment said "NOT the removed `thread_rng()`"; two doc lines referenced `include_str!`). The code itself uses only `rand::rng()` and exactly one `include_str!` macro call — identical to the [02-03]/[02-04] grep-in-comment false positives.
- **Fix:** Reworded the comment to "the pre-0.9 free-function spelling is gone" and the two doc lines to "embedded at compile time" / "The macro below embeds…" (the established reword-not-weaken precedent — guidance preserved, forbidden literal removed). No code change.
- **Files modified:** src/commands/fortune/mod.rs.
- **Verification:** `thread_rng|gen_range` → 0 (fortune + roast); `include_str!` → 1 (fortune); clippy + fmt clean; all suites still green.
- **Committed in:** `ea6f144` (Task 1 GREEN).

---

**Total deviations:** 1 auto-fixed (a false-positive-grep comment reword — the established [02-03]/[02-04] precedent).
**Impact on plan:** No behavior change and no scope creep. The locked command behavior matches the plan and the CONTEXT/RESEARCH guidance; the only edit was prose so the acceptance greps measure code, not comments.

## Threat Model Compliance

- **T-02-10 (fortune / roast / 8ball — accept):** honored. These are pure random picks over embedded CC0/public-domain lists — no network, auth, storage, or untrusted input reaching a sink. The 8ball question is accepted but discarded (`let _ = self.question;`), never interpreted — no injection surface. Output is decorative text to stdout.
- **Embedded data licensing (mitigate):** honored. `fortunes.txt` is public-domain/CC0 proverbs and `roasts.txt` is self-authored/CC0 one-liners (D-09) — no mixed-license BSD `fortune` datfiles bundled, so no third-party attribution gate (unlike the EFF wordlist in Plan 04).
- **No CSPRNG requirement:** `rand::rng()` (OS-seeded ChaCha12) is correct and sufficient for decorative randomness (D-08). Unbiased `choose` is a quality choice, not a security gate. No threat flags — no new security surface beyond the plan's register.

## Known Stubs / Forward-compat Surface

- No stubs introduced by this plan: `fortune`, `roast`, and `8ball` are fully implemented — no `not_implemented`, `todo!`, `unimplemented!`, or placeholder data in any of the three modules.
- All 9 Phase-2 commands are now functional. The remaining `not_implemented(...)` arms belong only to the Phase-3/4/5 commands (`hash`, `tree`, `du`, `dupes`, `bulk-rename`, `lolcat`, `matrix`, `ascii`, `json`, `qr`, `clip`, `pomodoro`, `weather`) — the future waves, by design.

## User Setup Required

None — no external service or configuration. Both text assets are embedded in the binary at compile time.

## TDD Gate Compliance

Both tasks followed RED→GREEN: a `test(...)` commit (failing — integration tests against the `not_implemented` stub, exit 1) precedes each `feat(...)` commit (implementation). Verified in git log: `603284c` (test) → `ea6f144` (feat) for fortune + roast; `9cc8e38` (test) → `cf23c8a` (feat) for 8ball. No REFACTOR commits — both implementations were minimal and clean as written. (Note: the per-command in-module unit tests landed with the GREEN commit alongside the integration RED tests, mirroring the Plan 04 split.)

## Next Phase Readiness

- **Phase 2 is complete.** All 9 pure-transform commands (`uuid`, `base64`, `epoch`, `color`, `passgen`, `cowsay`, `fortune`, `8ball`, `roast`) are functional, tested, and dispatched; zero `not_implemented` arms remain for any Phase-2 command. The phase gate is green: full `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` all clean; `box --help` still lists all 23 commands with their locked one-liners.
- **For Phase 3 (Filesystem Power Tools):** the whimsy random-pick pattern and the `include_str!` + `eol=lf` embedded-asset pattern are established and reusable; `hash`/`json` will be the first `core::input` `--file` consumers (deferred extension point from D-06). No blockers.
- Carried-over phase-transition todos (unchanged): spike `winrt-notification 0.5` before Phase 5 planning; decide `pomodoro` blocking model and `weather` default units; add `strip-ansi-escapes` for `lolcat` in Phase 4 planning.

## Self-Check: PASSED

- FOUND: src/data/fortunes.txt
- FOUND: src/data/roasts.txt
- FOUND: src/commands/fortune/mod.rs
- FOUND: src/commands/roast/mod.rs
- FOUND: src/commands/eight_ball/mod.rs
- FOUND: tests/fortune.rs
- FOUND: tests/roast.rs
- FOUND: tests/eight_ball.rs
- FOUND: .planning/phases/02-pure-transform-utilities/02-05-SUMMARY.md
- FOUND commit: 603284c (Task 1 RED)
- FOUND commit: ea6f144 (Task 1 GREEN)
- FOUND commit: 9cc8e38 (Task 2 RED)
- FOUND commit: cf23c8a (Task 2 GREEN)

---
*Phase: 02-pure-transform-utilities*
*Completed: 2026-06-22*
