---
phase: 02-pure-transform-utilities
plan: 04
subsystem: commands
tags: [rust, clap, rand, osrng, csprng, eff-wordlist, cowsay, passgen, tdd, include-str, trycmd]

# Dependency graph
requires:
  - phase: 02-pure-transform-utilities
    plan: 01
    provides: "core::input::read_input (String reader, arg→stdin→TTY-exit-2); rand 0.9 in Cargo.toml"
  - phase: 02-pure-transform-utilities
    plan: 02
    provides: "Command pattern (clap Args + thin run() + pure helpers + #[cfg(test)] units); TDD RED→GREEN commit discipline"
  - phase: 02-pure-transform-utilities
    plan: 03
    provides: "Latest command pattern + trycmd snapshot conventions (NO_COLOR-deterministic); false-positive-grep-in-comment reword precedent"
  - phase: 01-foundation-flatten
    provides: "RunCommand trait; main.rs single exit-code owner; cli.rs Commands enum + doc-comment help lock; help.trycmd"
provides:
  - "box passgen — CSPRNG password/passphrase generator (OsRng + unbiased IndexedRandom::choose, no % len); --length/--count/--words/--no-symbols; curated paste-safe charset; EFF 7776-word list embedded + attributed; stdout-only (D-02/D-08/D-14)"
  - "box cowsay — classic ASCII speech bubble; greedy 40-col wrap + hard-break; single/multi-line bubble; arg-or-stdin; --width override (D-11)"
  - "src/data/ directory + EFF Large wordlist asset (include_str! embed); LICENSE-THIRD-PARTY.md (CC-BY 3.0 US); .gitattributes (LF lock for the asset)"
  - "A1 closed to fact: rand::TryRngCore resolves under rand 0.9 — no rand_core dep added"
affects: [fortune, eight_ball, roast, hash, json, lolcat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "OsRng CSPRNG via OsRng.unwrap_err() (rand::TryRngCore resolves through rand 0.9 — A1 fact, no rand_core); every char/word picked unbiased with IndexedRandom::choose — NEVER % len (D-02/D-08, T-V6 code-review gate)"
    - "Large bundled lists embedded with include_str! (path relative to the SOURCE file: src/commands/passgen/mod.rs → ../../data/...); parsed via .lines() + split_whitespace().last() so the loader is dice-code-agnostic; 7776-count asserted by a Rust unit test, not a shell line-count"
    - "EFF asset stored words-only with LF forced via .gitattributes (eol=lf) so no \\r leaks into include_str! on a CRLF (autocrlf=true) checkout; loader still trims defensively"
    - "cowsay fixed-40 wrap (NOT terminal width) keeps pipe-vs-TTY output reproducible (D-11); hard-break over-long words into width chunks BEFORE greedy wrap; width=0 clamped to 1 (no infinite loop/panic, T-02-09)"
    - "trycmd normalizes literal \\ to / in captured transcripts (Windows path handling, RESEARCH A4); the TRUE byte-exact bubble (real backslashes) is locked by the bubble unit tests, the trycmd files are the end-to-end render lock"
    - "EFF CC-BY 3.0 US attribution attached via clap #[command(after_help = ...)] so it shows in `box passgen --help` without disturbing the help.trycmd-locked variant one-liner"

key-files:
  created:
    - "src/data/eff_large_wordlist.txt — 7776-word EFF Large (Diceware) list, words-only, LF"
    - "LICENSE-THIRD-PARTY.md — EFF wordlist CC-BY 3.0 US attribution note (D-09)"
    - ".gitattributes — forces eol=lf on the embedded EFF asset"
    - "src/commands/passgen/mod.rs — PassgenArgs + build_charset + eff_wordlist + 5 unit tests"
    - "src/commands/cowsay/mod.rs — CowsayArgs + pure wrap() + pure bubble() + COW const + 6 unit tests"
    - "tests/passgen.rs — PASS-01 integration (length, charset membership, --no-symbols, --words 4, --count 10 distinct, stdout-only)"
    - "tests/cmd/cowsay-single.trycmd — single-line bubble snapshot"
    - "tests/cmd/cowsay-multi.trycmd — multi-line wrap bubble snapshot"
  modified:
    - "src/commands/mod.rs — pub mod passgen; pub mod cowsay;"
    - "src/cli.rs — Passgen/Cowsay unit variants swapped for Args-carrying variants (doc comments byte-identical, help.trycmd lock preserved)"
    - "src/main.rs — Passgen/Cowsay dispatch arms swapped from not_implemented to args.run()"

key-decisions:
  - "A1 resolved to FACT: a one-line cargo example probe confirmed `use rand::TryRngCore;` + `OsRng.unwrap_err()` + `IndexedRandom::choose` + `random_range` all compile under rand 0.9 via re-exports — so Cargo.toml is UNCHANGED, no rand_core added; passgen RNG path is OsRng.unwrap_err() (D-08 literal reading)"
  - "EFF asset stored words-only (dice codes stripped at fetch time) — makes the include_str! parse trivial; the loader's split_whitespace().last() still strips a dice prefix were the asset ever stored verbatim, so the contract (7776 clean words) holds either way"
  - ".gitattributes added (eol=lf for the wordlist) because the repo has core.autocrlf=true — without it a CRLF checkout would embed \\r into every word via include_str!; the loader also trims defensively (belt-and-suspenders)"
  - "passphrase separator is a DOT (.), not a hyphen: some EFF words are hyphenated (t-shirt, yo-yo, drop-down, felt-tip) so a hyphen separator yields ambiguous word boundaries; a dot is paste-safe in PS7 and never appears inside an EFF word (Rule-1 fix of a latent ambiguity)"
  - "passgen samples EVERY char/word with IndexedRandom::choose (the canonical unbiased primitive) — random_range is NOT used, so its import was removed (clippy -D warnings); both are valid per the plan, choose is cleaner"
  - "EFF attribution wired via clap after_help (not the struct long-about) — the Commands::Passgen variant doc-comment is the about and is locked byte-identical by help.trycmd, so after_help is the non-disruptive place for the CC-BY footer"
  - "cowsay multi-line bubble = a `/   \\` top decoration line + one `| line |` per content line (padded to the longest) + a `\\   /` bottom decoration; single content line uses the `< text >` form with `_`/`-` borders sized to text+2 (matches the CONTEXT single-line target byte-for-byte)"

patterns-established:
  - "Security-critical RNG is gated by code review (grep for OsRng presence + grep for absence of modulo-indexed selection), NOT a flaky statistical/chi-square test (T-V6) — properties are asserted (length, membership, distinctness, stdout-only), never seeded values"
  - "When trycmd's path normalization corrupts a literal backslash (cow art), accept the normalized snapshot as the end-to-end render lock and rely on a pure-function unit test for the byte-exact assertion — do not fight the harness"

requirements-completed: [PASS-01, COW-01]

# Metrics
duration: 11min
completed: 2026-06-22
---

# Phase 2 Plan 04: passgen + cowsay Commands Summary

**The phase's one real threat surface plus a text-layout slice: `passgen` (CSPRNG passwords + EFF-wordlist passphrases — OsRng via `OsRng.unwrap_err()`, every character/word picked unbiased with `IndexedRandom::choose`, never `% len`; curated paste-safe charset; the 7776-word EFF Large list embedded with `include_str!` and attributed CC-BY 3.0 US; stdout-only) and `cowsay` (greedy fixed-40-column word-wrap with pre-wrap hard-break of over-long words, the classic single/multi-line speech bubble, arg-or-stdin, `--width` override). passgen's bias-freedom + CSPRNG-source guarantee is satisfied as a code-review grep gate (T-V6), not a statistical test; cowsay's wrap/bubble math is locked by pure-function unit tests plus two trycmd snapshots. Plan assumption A1 is closed to a checked fact — `rand::TryRngCore` resolves under rand 0.9, so no `rand_core` dependency was added.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-06-22T16:25:24Z
- **Completed:** 2026-06-22
- **Tasks:** 3 (Task 1 chore/data; Tasks 2 & 3 TDD RED→GREEN — 5 commits total)
- **Files:** 11 (8 created, 3 modified)

## Accomplishments

- **`box passgen`** generates cryptographically secure secrets. Bare `box passgen` prints one 16-character password from the four-class curated set (lowercase + uppercase + digits + a curated paste-safe symbol subset). `--length N` controls length, `--count N` prints N distinct passwords (one per line), `--words N` switches to an EFF passphrase of N dot-joined words, and `--no-symbols` drops the symbol class. Randomness comes from `OsRng.unwrap_err()` (the OS CSPRNG, getrandom → ProcessPrng on Windows) and every character/word is selected with the uniform `IndexedRandom::choose` — there is **no** modulo-indexed selection anywhere (D-02), **no** fixed/manual seed, and all output goes to stdout only, never logged to stderr (D-14 information-disclosure mitigation). The EFF Large wordlist (7776 words) is embedded via `include_str!` and the CC-BY 3.0 US attribution appears in `box passgen --help`.
- **`box cowsay`** renders the classic cow + speech bubble. `box cowsay "Hello, box!"` produces the single-line `< Hello, box! >` bubble (matching the CONTEXT target byte-for-byte); a longer string wraps greedily at a fixed 40 columns into the multi-line `/ \`…`\ /` bubble with every middle line space-padded to the longest. A single word longer than the width is hard-broken into width-sized chunks **before** wrapping, so no rendered line ever exceeds the width (Pitfall 5), and `--width 0` is clamped to 1 so it cannot loop forever or panic (T-02-09). Text is read from an argument or piped stdin via `core::input::read_input` (no-arg interactive TTY → exit 2). The width default is fixed at 40 (NOT the terminal width) so pipe-vs-TTY output is reproducible (D-11).
- **EFF wordlist asset + attribution:** created `src/data/eff_large_wordlist.txt` (the authentic 7776-word EFF Large list, words-only with dice codes stripped, LF line endings), `LICENSE-THIRD-PARTY.md` (CC-BY 3.0 US notice), and a `.gitattributes` rule forcing `eol=lf` on the asset so no `\r` leaks into the embedded words on a CRLF checkout (`core.autocrlf=true`).
- **A1 closed to fact:** a one-line `cargo build --example` probe confirmed the entire RNG import chain (`rand::rngs::OsRng`, `rand::TryRngCore`, `OsRng.unwrap_err()`, `IndexedRandom::choose`, `random_range`) compiles under rand 0.9 via re-exports — so **Cargo.toml was left unchanged and no `rand_core` dependency was added**. The passgen RNG construction is `OsRng.unwrap_err()` (D-08 literal reading).
- **Stubs removed:** both `not_implemented("passgen")` and `not_implemented("cowsay")` arms are gone; both modules are registered and dispatched. The `box --help` trycmd snapshot still passes — the `///` variant doc comments were preserved byte-identical.
- **Tests:** PASS-01 (6 integration + 5 unit), COW-01 (6 unit + 2 trycmd snapshots) all green; full suite 98 tests (58 unit + 40 integration/snapshot); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean; T-V6 grep gate satisfied (OsRng present, no modulo-indexed selection); cowsay `terminal_width` grep returns 0.

## Task Commits

1. **Task 1 (EFF asset)** — `3c45bde` (chore): EFF Large wordlist + LICENSE-THIRD-PARTY.md + .gitattributes; A1 resolved to fact.
2. **Task 2 (passgen) RED** — `a38c82d` (test): 6 failing PASS-01 integration tests against the stub.
3. **Task 2 (passgen) GREEN** — `4b19229` (feat): PassgenArgs + OsRng + curated charset + EFF embed + cli/main swap; 6/6 integration + 5/5 unit green.
4. **Task 3 (cowsay) RED** — `0af9ffe` (test): 6 failing wrap/hard-break/bubble unit tests (todo!() stubs).
5. **Task 3 (cowsay) GREEN** — `4b66aac` (feat): pure wrap() + bubble() + COW + cli/main swap + 2 trycmd snapshots; 6/6 unit + 7/7 trycmd green.

## Files Created/Modified

- `src/data/eff_large_wordlist.txt` (created) — 7776-word EFF Large list, words-only, LF.
- `LICENSE-THIRD-PARTY.md` (created) — EFF CC-BY 3.0 US attribution note.
- `.gitattributes` (created) — `eol=lf` for the EFF asset.
- `src/commands/passgen/mod.rs` (created) — `PassgenArgs`, `build_charset`, `eff_wordlist`, curated `SYMBOLS`/`LOWER`/`UPPER`/`DIGITS` consts, 5 unit tests.
- `src/commands/cowsay/mod.rs` (created) — `CowsayArgs`, pure `wrap`, pure `bubble`, `COW` const, 6 unit tests.
- `tests/passgen.rs` (created) — PASS-01 integration tests.
- `tests/cmd/cowsay-single.trycmd`, `tests/cmd/cowsay-multi.trycmd` (created) — bubble snapshots (auto-registered by the `tests/cmd/*.trycmd` glob in `tests/cli.rs:101-103`).
- `src/commands/mod.rs` (modified) — `pub mod passgen;` + `pub mod cowsay;`.
- `src/cli.rs` (modified) — `Passgen(PassgenArgs)` + `Cowsay(CowsayArgs)` variant swaps (doc comments byte-identical).
- `src/main.rs` (modified) — `Commands::Passgen(args) => args.run()` + `Commands::Cowsay(args) => args.run()`.

## Decisions Made

- **A1 → fact (Cargo.toml unchanged):** `rand::TryRngCore` resolves through rand 0.9, verified by a compiled probe — no `rand_core` dependency needed; passgen uses `OsRng.unwrap_err()`.
- **Dot passphrase separator:** EFF words can be hyphenated (`t-shirt`, `yo-yo`, `drop-down`, `felt-tip`), so a hyphen separator would make word boundaries ambiguous. A dot is paste-safe in PowerShell 7 and never appears inside an EFF word — chosen instead (a Rule-1 fix of a latent ambiguity in the obvious hyphen choice).
- **`choose` everywhere (no `random_range`):** every character and word is sampled with the canonical unbiased `IndexedRandom::choose`; `random_range` was not used, so its `rand::Rng` import was removed to satisfy clippy `-D warnings`. Both are valid per the plan.
- **EFF attribution via `after_help`:** the `Commands::Passgen` variant doc comment is the `about` and is locked byte-identical by `help.trycmd`, so the CC-BY 3.0 US footer was attached with `#[command(after_help = ...)]` rather than the struct long-about — non-disruptive to the snapshot.
- **`.gitattributes` LF lock:** with `core.autocrlf=true`, a CRLF checkout would embed `\r` into each EFF word via `include_str!`; forcing `eol=lf` on the asset is the root-cause fix (the loader also `.trim()`s defensively).
- **cowsay bubble shape:** single content line → `< text >` with `_`/`-` borders sized to text+2 (byte-for-byte the CONTEXT target); multiple → a `/   \` top decoration, `| line |` middles padded to the longest, a `\   /` bottom decoration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] CRLF checkout would corrupt the embedded EFF wordlist**
- **Found during:** Task 1 (staging the asset; Git warned `LF will be replaced by CRLF`).
- **Issue:** The repo has `core.autocrlf=true`. `include_str!` embeds the *working-copy* bytes at compile time, so on a CRLF checkout every EFF word would carry a trailing `\r`, breaking exact matches.
- **Fix:** Added `.gitattributes` with `src/data/eff_large_wordlist.txt text eol=lf`, re-added the asset to apply it; the loader also `.trim()`s defensively.
- **Files modified:** .gitattributes (created), src/data/eff_large_wordlist.txt (re-staged).
- **Committed in:** `3c45bde` (Task 1).

**2. [Rule 1 - Bug] Hyphen passphrase separator is ambiguous with hyphenated EFF words**
- **Found during:** Task 2 GREEN (the `--words 4` integration test split on the separator and saw one token).
- **Issue:** The initial implementation joined words with `-`, but the EFF list contains hyphenated words (`t-shirt`, `yo-yo`, `drop-down`, `felt-tip`), so the rendered phrase had ambiguous word boundaries — a latent usability/correctness bug, not just a test artifact.
- **Fix:** Switched the separator to a dot (`.`) — paste-safe in PS7 and never present inside an EFF word; updated the test's tokenizer to split on the dot.
- **Files modified:** src/commands/passgen/mod.rs, tests/passgen.rs.
- **Committed in:** `4b19229` (Task 2 GREEN).

**3. [Rule 3 - Blocking] Unused `rand::Rng` import failed clippy `-D warnings`**
- **Found during:** Task 2 GREEN (clippy gate).
- **Issue:** `use rand::Rng;` (imported for `random_range`) was unused because the final implementation samples words with `choose`, not `random_range`.
- **Fix:** Removed the unused import (kept `IndexedRandom` and `TryRngCore`).
- **Files modified:** src/commands/passgen/mod.rs.
- **Committed in:** `4b19229` (Task 2 GREEN).

**4. [Rule 1 - Bug] clippy `doc_lazy_continuation` on the passgen module doc**
- **Found during:** Task 2 GREEN (clippy gate).
- **Issue:** A doc paragraph continuation line beginning with `+` was parsed as a markdown list item, tripping `doc_lazy_continuation` (4 errors under `-D warnings`).
- **Fix:** Reworded the paragraph so no continuation line starts with `+`/`-`/`*`.
- **Files modified:** src/commands/passgen/mod.rs.
- **Committed in:** `4b19229` (Task 2 GREEN).

**5. [Rule 1 - Bug] T-V6 `% len` acceptance grep tripped by explanatory comments**
- **Found during:** Task 2 acceptance-criteria check (`grep -nE '\[[^]]*%[^]]*\]'` must return no modulo-indexed selection).
- **Issue:** Two comments contained the bracketed literals `[x % len]` / `[x % charset.len()]` while *forbidding* that pattern — a false positive identical to the [02-03] `set_override`-in-comment case. The code never uses modulo selection.
- **Fix:** Reworded both comments to keep the guidance without the bracketed-modulo literal (the established [02-03] reword-not-weaken precedent).
- **Files modified:** src/commands/passgen/mod.rs.
- **Committed in:** `4b19229` (Task 2 GREEN).

**6. [Rule 3 - Blocking] trycmd normalizes literal `\` to `/`, breaking the hand-written cowsay snapshots**
- **Found during:** Task 3 GREEN (first `cargo test --test cli` run — 2 of 4 trycmd cases failed).
- **Issue:** trycmd applies Windows path normalization to captured transcripts, converting the cow art's backslashes (`\`, `\_`, `)\/\`) to forward slashes (RESEARCH A4). Hand-written snapshots with real backslashes never matched the normalized capture.
- **Fix:** Regenerated both snapshots with `TRYCMD=overwrite` so they store trycmd's own normalized form (forward slashes). The REAL command output is byte-exact (verified with `cat -A`); the byte-exact bubble — including true backslashes — is locked by the `bubble` unit tests, with the trycmd files serving as the end-to-end render lock. Documented in the cowsay module doc.
- **Files modified:** tests/cmd/cowsay-single.trycmd, tests/cmd/cowsay-multi.trycmd, src/commands/cowsay/mod.rs (doc note).
- **Committed in:** `4b66aac` (Task 3 GREEN).

**7. [Rule 1 - Bug] cowsay `terminal_width` acceptance grep tripped by a doc comment**
- **Found during:** Task 3 GREEN (`grep -c 'terminal_width' src/commands/cowsay/mod.rs` must return 0).
- **Issue:** The module doc said "NOT `terminal_width()`" while *forbidding* its use (D-11) — the same false-positive-in-comment pattern.
- **Fix:** Reworded the doc to "deliberately NOT the detected terminal width" (no literal token). The code never calls `terminal_width`.
- **Files modified:** src/commands/cowsay/mod.rs.
- **Committed in:** `4b66aac` (Task 3 GREEN).

**8. [Rule 3 - Blocking] `cargo fmt --check` drift (passgen & cowsay)**
- **Found during:** Task 2 and Task 3 verification (the `cargo fmt --check` quality gate).
- **Issue:** The `after_help` attribute line, an `assert!`/`assert_eq!` chain, and the multi-line bubble test exceeded rustfmt's width.
- **Fix:** Ran `cargo fmt` (cosmetic re-wrap only — whitespace/line-breaks, no logic change); re-verified the suites green afterward.
- **Files modified:** src/commands/passgen/mod.rs, tests/passgen.rs, src/commands/cowsay/mod.rs.
- **Committed in:** `4b19229` (Task 2), `4b66aac` (Task 3).

---

**Total deviations:** 8 auto-fixed (2 latent correctness bugs — dot separator, CRLF embed; 3 false-positive-grep/lint comment rewords; 3 blocking import/fmt/trycmd-normalization fixes). No scope creep — every fix was needed to pass the plan's own gates or to remove a real correctness/usability defect; the locked command behavior matches the plan and the CONTEXT targets.

## Threat Model Compliance

- **T-V6-rng (Info Disclosure / passgen RNG construction):** mitigated — randomness is `OsRng.unwrap_err()` (OS CSPRNG, getrandom → ProcessPrng); there is no fixed/manual seed. Code-review gate: `grep -c 'OsRng' src/commands/passgen/mod.rs` returns 6. No statistical test is used as the gate (T-V6 explicitly).
- **T-V6-bias (Info Disclosure / passgen selection):** mitigated — every character and word is selected with `IndexedRandom::choose` (uniform). Code-review gate: `grep -nE '\[[^]]*%[^]]*\]' src/commands/passgen/mod.rs` returns no modulo-indexed selection. No `% len` anywhere.
- **T-V6-leak (Info Disclosure / passgen output channel):** mitigated — all passwords/passphrases are printed to stdout only; nothing is written to stderr on success (no `--clip` in v1). Integration test `success_writes_nothing_to_stderr` asserts empty stderr on success.
- **T-02-08 (passgen symbol set):** mitigated — the curated `SYMBOLS` const excludes backtick, single/double quotes, `$`, `\`, `;`, `|`, `&`, angle brackets, parens, and space; unit test `symbols_exclude_shell_hostile_chars` asserts each banned char is absent.
- **T-02-09 (DoS-self / cowsay wrap):** mitigated — over-long words are hard-broken into width chunks before wrapping (no line exceeds the width); `width == 0` is clamped to 1 so the loop always terminates. Unit-tested (`wrap_hard_breaks_overlong_word`, `wrap_width_zero_does_not_panic`).
- **T-02-SC (Tampering / crates.io install of rand_core):** N/A — `rand::TryRngCore` resolved under rand 0.9 (A1 fact), so `rand_core` was never added; no new dependency, no install-time supply-chain surface.
- No new security surface beyond the plan's threat register. No threat flags.

## Known Stubs / Forward-compat Surface

- No stubs introduced by this plan: both passgen and cowsay are fully implemented; no `not_implemented`, `todo!`, `unimplemented!`, or placeholder data in either module.
- The three remaining Phase-2 whimsy commands (`fortune`, `8ball`, `roast`) plus the Phase-3/4/5 commands remain `not_implemented(...)` stubs by design — they are the remaining waves.

## User Setup Required

None — no external service or configuration. The EFF wordlist is embedded in the binary at compile time.

## TDD Gate Compliance

Tasks 2 and 3 followed RED→GREEN: a `test(...)` commit (failing — passgen integration tests against the stub; cowsay unit tests against `todo!()` bodies) precedes each `feat(...)` commit (implementation). Verified in git log: `a38c82d` (test) → `4b19229` (feat) for passgen; `0af9ffe` (test) → `4b66aac` (feat) for cowsay. Task 1 is a data/config asset (no `<behavior>` source files), exempt from the RED gate; its 7776-count contract is owned by the Task-2 `eff_wordlist_has_exactly_7776_entries` unit test. No REFACTOR commits — both implementations were minimal and clean as written.

## Self-Check: PASSED

- FOUND: src/data/eff_large_wordlist.txt
- FOUND: LICENSE-THIRD-PARTY.md
- FOUND: .gitattributes
- FOUND: src/commands/passgen/mod.rs
- FOUND: src/commands/cowsay/mod.rs
- FOUND: tests/passgen.rs
- FOUND: tests/cmd/cowsay-single.trycmd
- FOUND: tests/cmd/cowsay-multi.trycmd
- FOUND: .planning/phases/02-pure-transform-utilities/02-04-SUMMARY.md
- FOUND commit: 3c45bde (Task 1 — EFF asset)
- FOUND commit: a38c82d (Task 2 RED)
- FOUND commit: 4b19229 (Task 2 GREEN)
- FOUND commit: 0af9ffe (Task 3 RED)
- FOUND commit: 4b66aac (Task 3 GREEN)

---
*Phase: 02-pure-transform-utilities*
*Completed: 2026-06-22*
