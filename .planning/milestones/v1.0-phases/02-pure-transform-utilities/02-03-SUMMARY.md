---
phase: 02-pure-transform-utilities
plan: 03
subsystem: commands
tags: [rust, clap, chrono, owo-colors, color, epoch, hsl, tdd, exit-codes, color-gate]

# Dependency graph
requires:
  - phase: 02-pure-transform-utilities
    plan: 01
    provides: "core::input::read_input (String reader, arg→stdin→TTY-no-block); chrono (clock,std) in Cargo.toml"
  - phase: 02-pure-transform-utilities
    plan: 02
    provides: "Command pattern to mirror (clap Args + thin run() + pure helpers + #[cfg(test)] units); per-line trim discipline"
  - phase: 01-foundation-flatten
    provides: "RunCommand trait; main.rs single exit-code owner; cli.rs Commands enum + doc-comment help lock; core::output::is_color_on color gate"
provides:
  - "box epoch — Unix timestamp ↔ human date across 3 D-12 modes (no-arg now / integer Local+UTC / date-string→timestamp); EpochArgs impl RunCommand"
  - "box color — bidirectional hex/RGB auto-detect parse (#abc short-expand) → Hex/RGB/Tuple/HSL block + is_color_on()-gated truecolor swatch; ColorArgs impl RunCommand"
  - "First live core::input::read_input (String) consumer (color); strict dead-code gate restored on the String path (#[allow(dead_code)] removed from read_input/resolve)"
  - "First reuse of the Phase-1 core::output::is_color_on() gate by a brand-new styled command — no parallel color path (D-10)"
affects: [cowsay, passgen, hash, json, lolcat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "epoch self-resolves input (no-arg = 'print now', NOT the exit-2 missing-input case), so it does NOT call core::input::read_input; color DOES (no-arg interactive TTY → exit 2)"
    - "DST safety: chrono Local.from_local_datetime(&naive).single() collapses LocalResult::None/Ambiguous to an error — never .unwrap() (T-02-05)"
    - "Color swatch = single is_color_on() gate (truecolor glyphs when on, plain ██ when off) — byte-identical minus ANSI, no set_override / no background-SGR swatch (D-10, Pattern C)"
    - "rgb_to_hsl closed-form (CSS Color Module L3): h 0-359, s/l 0-100, integer-rounded; unit-tested within ±1 of the reference"

key-files:
  created:
    - "src/commands/epoch/mod.rs — EpochArgs + parse_date (3 modes) + format_timestamp + local_naive_to_timestamp + resolve_value + 6 unit tests"
    - "src/commands/color/mod.rs — ColorArgs + parse_color (auto-detect) + parse_hex (short-expand) + parse_rgb + rgb_to_hsl + 5 unit tests"
    - "tests/epoch.rs — EPOC-01 integration (no-arg now, integer Local+UTC, bad-format exit 1, RFC3339 round-trip)"
    - "tests/color.rs — COLR-01 integration (hex/RGB agree, #abc expand, malformed/out-of-range → exit 1 clean)"
    - "tests/cmd/color.trycmd — locked Hex/RGB/Tuple/HSL block + plain ██████████ swatch snapshot (NO_COLOR)"
  modified:
    - "src/commands/mod.rs — pub mod epoch; pub mod color;"
    - "src/cli.rs — Epoch/Color unit variants swapped for Args-carrying variants (doc comments byte-identical, help.trycmd lock preserved)"
    - "src/main.rs — Epoch/Color dispatch arms swapped from not_implemented to args.run()"
    - "src/core/input.rs — removed #[allow(dead_code)] from read_input + resolve (color is now the live String-path caller)"

key-decisions:
  - "epoch does NOT use core::input::read_input: for epoch a no-arg interactive TTY means 'print the current timestamp', not the exit-2 missing-input case, so it has its own resolve_value (arg → piped stdin → None=now). This means epoch is NOT what makes the String path live."
  - "color IS the first live read_input (String) consumer; the forward-compat #[allow(dead_code)] came off read_input + resolve, restoring the strict dead-code gate on the String path (mirrors the 02-02 byte-path removal and the [01-03] allow-then-remove precedent)"
  - "Color swatch is the ONLY color path: gated on is_color_on() (truecolor when on, plain ██ when off). No owo_colors::set_override, no background-SGR fill — those strip to a blank line (D-10 rejects them; Pattern C reuse-not-parallel)"
  - "epoch labels chosen as 'Local: …' / 'UTC:   …' with format %Y-%m-%d %H:%M:%S (+%z for local, ' UTC' suffix) — two labeled lines satisfying D-12 integer mode"
  - "color block layout uses fixed 6-char label padding ('Hex   : ', 'RGB   : ', 'Tuple : ', 'HSL   : ') with a 2-space indent, matching the D-10 target byte-for-byte; locked by color.trycmd"

patterns-established:
  - "Per-command input policy is explicit: a command that has a meaningful no-arg behavior (epoch=now) self-resolves; a command that requires input (color) delegates to core::input::read_input and inherits the exit-2-on-TTY contract"
  - "False-positive grep acceptance gates (literal token in an explanatory comment) are resolved by rewording the comment, not weakening the gate — same as the [02-01] bail!-in-comment fix"

requirements-completed: [EPOC-01, COLR-01]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 2 Plan 03: epoch + color Commands Summary

**Two transform slices on the Plan-01/Phase-1 foundation: `epoch` (Unix timestamp ↔ human date across the three D-12 modes, with chrono `Local`/DST handled via `.single()` — never `.unwrap()`) and `color` (bidirectional hex/RGB auto-detect parse incl. `#abc` short-expand → the locked Hex/RGB/Tuple/HSL block + a truecolor swatch gated on the single Phase-1 `is_color_on()` decision so it degrades to plain `██` when piped). Each TDD-driven RED→GREEN, with integration + unit tests and a locked trycmd snapshot; the strict dead-code gate is now restored on the `core::input` String path.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-22T16:14:39Z
- **Completed:** 2026-06-22
- **Tasks:** 2 (both TDD, 4 commits total: 2 RED + 2 GREEN)
- **Files:** 9 (5 created, 4 modified)

## Accomplishments

- **`box epoch`** covers all three D-12 modes: bare `box epoch` prints the current Unix timestamp (single integer); `box epoch 1700000000` prints two labeled human-date lines (`Local:` + `UTC:`); a date string (RFC 3339, `YYYY-MM-DD HH:MM:SS` local, or `YYYY-MM-DD` local midnight) prints the resulting timestamp. `parse_date` tries the three formats in order; an unrecognized string `bail!`s with a hint naming the accepted formats (no `MM/DD/YYYY`). All chrono `LocalResult` DST cases are collapsed via `.single()` to an error — there is no `.unwrap()` on any user-input parse path (T-02-05), and an out-of-range timestamp errors instead of panicking.
- **`box color`** parses input bidirectionally and auto-detected (D-13): hex (`#3b82f6`, `3b82f6`, short `#abc` → `(170,187,204)` via nibble-doubling) and RGB (`"59,130,246"` and `"59 130 246"`), then prints the locked D-10 block — uppercase `#RRGGBB`, CSS `rgb(r, g, b)`, `r g b` tuple, `hsl(h, s%, l%)` — a blank line, then a `██████████` swatch. The swatch is the **only** color path: gated on `core::output::is_color_on()`, it renders truecolor in a TTY and degrades to plain glyphs when piped/`NO_COLOR`, byte-identical minus ANSI. No `owo_colors::set_override`, no background-SGR fill (D-10/Pattern C). Out-of-range RGB components and malformed hex error cleanly to stderr (exit 1) with no panic and no array-index overflow (T-02-06).
- **First live `core::input::read_input` (String) consumer:** `color` acquires its input via `read_input` (arg → piped stdin → no-arg interactive TTY → exit 2), so the forward-compat `#[allow(dead_code)]` came off `read_input` + the inner `resolve`, restoring the strict dead-code gate on the String path. (`epoch` deliberately does NOT use `read_input` — a no-arg interactive TTY means "print now", not the missing-input/exit-2 case — so it self-resolves and is not the String-path's first caller.)
- **First reuse of the Phase-1 color gate by a new styled command:** `color` composes cleanly with `core::output::is_color_on()` — proving the locked gate (D-10) generalizes beyond `flatten`'s row glyphs.
- **Stubs removed:** both `not_implemented("epoch")` and `not_implemented("color")` arms are gone; both modules registered and dispatched. The `box --help` trycmd snapshot still passes — the `///` doc comments were preserved byte-identical.
- **Tests:** EPOC-01 (4 integration + 6 unit) and COLR-01 (5 integration + 5 unit + 1 trycmd snapshot) all green; full suite 81 tests; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.

## Task Commits

Each task was committed atomically following the TDD RED→GREEN cycle:

1. **Task 1 (epoch) RED** — `7a9a78c` (test): 4 failing EPOC-01 integration tests against the stub.
2. **Task 1 (epoch) GREEN** — `af743de` (feat): EpochArgs + parse_date (3 modes) + module register + cli/main swap; 4/4 integration + 6/6 unit green.
3. **Task 2 (color) RED** — `d7b8a49` (test): 5 failing COLR-01 integration tests against the stub.
4. **Task 2 (color) GREEN** — `7af1545` (feat): ColorArgs + parse/HSL helpers + gated swatch + trycmd snapshot + read_input allow-removal; 5/5 integration + 5/5 unit + snapshot green.

## Files Created/Modified

- `src/commands/epoch/mod.rs` (created) — `EpochArgs { value }`, `parse_date`, `format_timestamp`, `local_naive_to_timestamp`, `resolve_value`, 6 unit tests.
- `src/commands/color/mod.rs` (created) — `ColorArgs { input }`, `parse_color`/`parse_hex`/`parse_rgb`, `rgb_to_hsl`, 5 unit tests.
- `tests/epoch.rs` (created) — no-arg now, integer Local+UTC, bad-format exit 1, RFC3339 round-trip.
- `tests/color.rs` (created) — hex/RGB agree, `#abc` expand, malformed hex + out-of-range RGB → exit 1 clean.
- `tests/cmd/color.trycmd` (created) — locked Hex/RGB/Tuple/HSL block + plain `██████████` swatch (NO_COLOR), auto-registered by the `tests/cmd/*.trycmd` glob in `tests/cli.rs:101-103`.
- `src/commands/mod.rs` (modified) — `pub mod epoch;` + `pub mod color;`.
- `src/cli.rs` (modified) — `Epoch(EpochArgs)` + `Color(ColorArgs)` variant swaps (doc comments byte-identical).
- `src/main.rs` (modified) — `Commands::Epoch(args) => args.run()` + `Commands::Color(args) => args.run()`.
- `src/core/input.rs` (modified) — removed `#[allow(dead_code)]` from `read_input` + `resolve`; updated comments to note color is the live String-path caller.

## Decisions Made

- **epoch self-resolves input, color delegates:** epoch's no-arg-on-TTY is "print now" (a feature), so it has its own `resolve_value` and does NOT call `core::input::read_input`; this means epoch is NOT what makes the String path live. color requires input, so it calls `read_input` and inherits the exit-2-on-no-arg-TTY contract — making color the first live String-path consumer.
- **Strict dead-code gate restored (String path):** `#[allow(dead_code)]` came off `read_input` + `resolve` now that color is a live caller — the surgical, per-item allow-then-remove pattern from [01-03]/[02-02]. The byte path was already live (base64, 02-02); the String path is now live too, so `core::input` carries no forward-compat allows.
- **Single color path (D-10):** the swatch is gated only on `is_color_on()`. No `set_override`, no `\x1b[48;2…m` background fill — those would strip to a blank line and need a banned parallel color path. Reuse-not-parallel (Pattern C).
- **DST safety (T-02-05):** `Local.from_local_datetime(&naive).single()` returns `None` for both the spring-forward (nonexistent) and fall-back (ambiguous) cases; both map to a clean error — never a panic.
- **Layout locked by snapshot:** the color block uses a 2-space indent + 6-char label column; the exact bytes are locked in `color.trycmd`, so any future layout drift fails the snapshot test.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo fmt --check` drift in tests/epoch.rs**
- **Found during:** Task 1 verification (the plan's `cargo fmt --check` quality gate).
- **Issue:** an `assert!` macro call in `tests/epoch.rs` exceeded rustfmt's line width, so `cargo fmt --check` reported a diff and would block the gate.
- **Fix:** ran `cargo fmt` (cosmetic re-wrap only — whitespace/line-breaks, no logic change); re-verified the epoch suite green afterward.
- **Files modified:** tests/epoch.rs (also reformatted src/commands/epoch/mod.rs's naive-datetime line)
- **Verification:** `cargo fmt --check` exits 0; epoch tests still 4/4 + 6/6.
- **Committed in:** `af743de` (Task 1 GREEN).

**2. [Rule 1 - Bug] `set_override`/`48;2` acceptance grep tripped by explanatory comments**
- **Found during:** Task 2 acceptance-criteria check (`grep -c 'set_override\|48;2' src/commands/color/mod.rs` must return 0).
- **Issue:** the criterion is meant to prove there is no parallel/background color path in the *code*; it returned 2 because two doc/inline comments contained the literal tokens `set_override` and "background-ANSI" while explaining we deliberately do NOT use them — a false positive identical to the [02-01] `bail!`-in-comment case.
- **Fix:** reworded both comments ("owo-colors global-override call" / "background-color swatch built from raw SGR escapes") to keep the semantic guidance while removing the literal tokens. The code never used either pattern.
- **Files modified:** src/commands/color/mod.rs
- **Verification:** `grep -c 'set_override\|48;2' src/commands/color/mod.rs` now returns 0; `grep -c 'is_color_on'` returns 3; all color tests green.
- **Committed in:** `7af1545` (Task 2 GREEN).

**3. [Rule 3 - Blocking] `cargo fmt --check` drift in tests/color.rs + color/mod.rs**
- **Found during:** Task 2 verification (same `cargo fmt --check` gate).
- **Issue:** an `assert!` call in `tests/color.rs` and a `bail!`/`split` chain in `src/commands/color/mod.rs` exceeded rustfmt's width.
- **Fix:** ran `cargo fmt` (cosmetic re-wrap only); re-verified the full suite green.
- **Files modified:** tests/color.rs, src/commands/color/mod.rs
- **Verification:** `cargo fmt --check` exits 0; full suite 81 tests green.
- **Committed in:** `7af1545` (Task 2 GREEN).

---

**Total deviations:** 3 auto-fixed (2 blocking fmt re-wraps, 1 false-positive-grep comment reword). No scope creep — all three were needed to pass the plan's own quality gates; no command behavior changed.

## Threat Model Compliance

- **T-02-05 (DoS-self / epoch parse panic):** mitigated — every chrono parse result is matched (`if let Ok`), `DateTime::from_timestamp` is `ok_or_else`-guarded for out-of-range, and the `LocalResult` DST cases are collapsed via `.single()` to an error. No `.unwrap()` on any user-input path. Unit-tested (`junk_string_is_err_with_hint`, `out_of_range_timestamp_is_err`) + integration-tested (`bad_format_exits_1_with_hint` asserts `.code(1)`, empty stdout, format-hint stderr).
- **T-02-06 (DoS-self / color parse panic):** mitigated — `parse_color`/`parse_hex`/`parse_rgb` all return `Result`; out-of-range RGB components parse to `u8` (rejecting `>255`), malformed/short hex is length+digit validated before slicing (no index overflow), and channel parses use `?`/context (no `.unwrap()` on user-derived values). Unit-tested (`parse_color_rejects_bad_input`) + integration-tested (`malformed_hex_exits_1_clean`, `out_of_range_rgb_exits_1_clean` assert `.code(1)`, empty stdout, non-empty stderr).
- **T-02-07 (Tampering / color swatch output integrity):** mitigated — the swatch is gated on the single Phase-1 `is_color_on()` decision, so piped output is byte-identical minus ANSI (the `██` glyphs survive as a meaningful line). There is no parallel/background color path; the `set_override`/`48;2` acceptance grep returns 0 and the `color.trycmd` snapshot locks the exact plain bytes.
- No new security surface beyond the plan's threat register. No threat flags.

## Known Stubs / Forward-compat Surface

- `core::input` now carries **no** forward-compat `#[allow(dead_code)]`: the byte path went live with base64 (02-02) and the String path went live with color (this plan). Both readers are fully live and unit-tested.
- The five Phase-2 commands not in this plan (passgen, cowsay, fortune, 8ball, roast) remain `not_implemented(...)` stubs by design — the remaining Phase-2 waves.

## User Setup Required

None — no external service or configuration.

## TDD Gate Compliance

Both tasks followed RED→GREEN: a `test(...)` commit (failing against the `not_implemented` stub) precedes each `feat(...)` commit (implementation). Verified in git log: `7a9a78c` (test) → `af743de` (feat) for epoch; `d7b8a49` (test) → `7af1545` (feat) for color. No REFACTOR commits — both implementations were minimal and clean as written.

## Self-Check: PASSED

- FOUND: src/commands/epoch/mod.rs
- FOUND: src/commands/color/mod.rs
- FOUND: tests/epoch.rs
- FOUND: tests/color.rs
- FOUND: tests/cmd/color.trycmd
- FOUND: .planning/phases/02-pure-transform-utilities/02-03-SUMMARY.md
- FOUND commit: 7a9a78c (Task 1 RED)
- FOUND commit: af743de (Task 1 GREEN)
- FOUND commit: d7b8a49 (Task 2 RED)
- FOUND commit: 7af1545 (Task 2 GREEN)

---
*Phase: 02-pure-transform-utilities*
*Completed: 2026-06-22*
