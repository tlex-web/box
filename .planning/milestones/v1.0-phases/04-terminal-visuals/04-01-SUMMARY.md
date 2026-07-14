---
phase: 04-terminal-visuals
plan: 01
subsystem: cli
tags: [json, serde_json, owo-colors, clap, rust, pretty-print, colorizer]

# Dependency graph
requires:
  - phase: 02-pure-transform-utilities
    provides: "core::input::read_input (arg→stdin→exit-2 precedence), core::output::is_color_on (the one color gate), RunCommand trait, color/mod.rs colorize-or-delegate precedent"
provides:
  - "Live `box json` command (JSON-01): 2-space pretty (colored TTY / plain piped), --compact minify, invalid→exit-1 with 1-based line/column"
  - "serde_json 1.0.150 with preserve_order in the dependency tree (first JSON consumer)"
  - "Pure colorize(&Value) walker pattern (6-variant, indent-aware, gated on is_color_on) reusable by lolcat/matrix color paths"
affects: [04-02-lolcat, 04-03-matrix, 04-04-ascii, phase-5-platform]

# Tech tracking
tech-stack:
  added: ["serde_json 1.0.150 (preserve_order; pulls indexmap→hashbrown, serde_core, zmij)"]
  patterns:
    - "parse→validate→colorize-or-delegate: serde_json::from_str::<Value> then --compact→to_string / plain→to_string_pretty / colored→hand-rolled colorize, gated SOLELY on is_color_on() (D-05)"
    - "pure colorize(&Value, depth)->String walker with #[cfg(test)] seam (unit-testable without a terminal)"

key-files:
  created:
    - "src/commands/json/mod.rs (JsonArgs + RunCommand + colorize walker + 4 unit tests)"
    - "tests/json.rs (5 JSON-01 integration tests)"
    - "tests/cmd/json.trycmd (locked 2-space pretty snapshot)"
  modified:
    - "Cargo.toml (serde_json dep), Cargo.lock"
    - "src/cli.rs (Json unit variant → Json(JsonArgs))"
    - "src/main.rs (json not_implemented arm → args.run())"
    - "src/commands/mod.rs (pub mod json;)"

key-decisions:
  - "serde_json preserve_order ON, arbitrary_precision OFF — input key order preserved (IndexMap), avoiding the D-04 Value-round-trip landmine (serde-rs/json #505/#721/#845)"
  - "Coloring is a hand-rolled colorize(&Value) walker gated only on is_color_on() — no colored_json crate, no set_override toggle; non-color/--compact paths delegate to serde_json serializers so piped output is byte-identical minus ANSI (D-05)"
  - "serde_json 1.0.150 transitively pulls `zmij` (dtolnay's Schubfach float-to-string crate, ryu successor) — verified legitimate via the official crates.io sparse-index manifest + on-disk authorship; NOT a substituted/slopsquatted package"

patterns-established:
  - "JSON colorizer: match all 6 Value variants, 2-space INDENT const per depth, IndexMap insertion order on Object, owo-colors per token (key=blue/string=green/number=yellow/bool+null=magenta/punctuation plain)"
  - "color_json_string via serde_json::to_string(Value::String) so colored keys/strings carry correct JSON escaping and match the plain serializer byte-for-byte minus ANSI"

requirements-completed: [JSON-01]

# Metrics
duration: 5min
completed: 2026-06-24
---

# Phase 4 Plan 01: box json Summary

**Live `box json` slice (JSON-01): serde_json `from_str::<Value>` parse with `preserve_order`, 2-space pretty (hand-rolled owo-colors colorizer in a TTY, `to_string_pretty` plain when piped), `--compact` minify, and invalid JSON → exit 1 with a 1-based line/column error — byte-identical-minus-ANSI piped output.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-24T12:26:25Z
- **Completed:** 2026-06-24T12:31:01Z
- **Tasks:** 2 (1 RED test task + 1 TDD GREEN impl task)
- **Files modified:** 8 (3 created, 5 modified)

## Accomplishments
- `box json` is live — the `not_implemented("json")` arm is deleted; the command parses, validates, pretty-prints (2-space), syntax-colors in a TTY, minifies with `--compact`, and reports invalid JSON with a 1-based line/column on stderr (exit 1).
- A pure `colorize(&serde_json::Value, depth) -> String` walker over all 6 `Value` variants — 2-space indent per nesting level, IndexMap insertion order (preserve_order), owo-colors per token type — gated SOLELY on `is_color_on()`, with the non-color and `--compact` paths delegating to serde_json's own serializers so piped output is byte-identical to the colored output minus the ANSI escapes (D-05).
- `serde_json 1.0.150` added with `preserve_order` (arbitrary_precision deliberately OFF), establishing the JSON dependency for the rest of the milestone.
- 5 JSON-01 integration tests + 4 colorizer unit tests + 1 trycmd snapshot, all green; full suite 102 unit + all integration green, clippy `-D warnings` and `fmt --check` clean — no regression in the 98 prior tests.

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 failing tests + json fixture** — `8824d2d` (test)
2. **Task 2: Implement json slice — JsonArgs + colorizer + registry wiring** — `35a1861` (feat)

**Plan metadata:** _(this commit)_ (docs: complete plan)

_TDD gate compliance: `test(04-01)` RED commit precedes the `feat(04-01)` GREEN commit; no refactor commit needed (GREEN was clean)._

## Files Created/Modified
- `src/commands/json/mod.rs` (created) — `JsonArgs { input, compact }`, `impl RunCommand` (read_input → from_str::<Value> → bail-on-error / compact / colorize / pretty), and the pure `colorize`/`write_value`/`write_array`/`write_object`/`push_indent`/`color_json_string` walker + 4 `#[cfg(test)]` unit tests.
- `tests/json.rs` (created) — 5 integration tests: invalid→exit1+line/column+empty-stdout, valid 2-space pretty, `--compact` single-line, key-order (b before a), piped-no-ANSI byte-scan.
- `tests/cmd/json.trycmd` (created) — locked 2-space pretty layout snapshot for the positional-arg path (auto-discovered by the `tests/cmd/*.trycmd` glob in `tests/cli.rs`).
- `Cargo.toml` / `Cargo.lock` (modified) — `serde_json = { version = "1.0.150", features = ["preserve_order"] }`.
- `src/cli.rs` (modified) — `Json,` → `Json(crate::commands::json::JsonArgs),`.
- `src/main.rs` (modified) — `Commands::Json => Err(not_implemented("json"))` → `Commands::Json(args) => args.run()` (the `not_implemented` import stays for qr/clip/pomodoro/weather).
- `src/commands/mod.rs` (modified) — `pub mod json;` (alpha order, between `hash` and `passgen`).

## Decisions Made
- **preserve_order ON / arbitrary_precision OFF** (D-04): the parsed `Value` object map is an insertion-ordered IndexMap, so `{"b":1,"a":2}` keeps `b` before `a`. `arbitrary_precision` stays off because it breaks `Value` round-tripping/Display for numbers (the documented D-04 landmine).
- **One color path, hand-rolled** (D-05): no `colored_json` crate and no `set_override` toggle — `colorize` always emits ANSI and is reached only when the caller has already checked `is_color_on()`; the plain path delegates to `to_string_pretty`, so the two outputs differ only by escapes. Verified by the `json_piped_no_ansi` byte-scan and the `cat -A` end-to-end check (LF-only, no `^[`).
- **Token color scheme** (RESEARCH OQ-3 discretion): key=blue, string=green, number=yellow, bool+null=magenta, punctuation plain.
- **color_json_string** uses `serde_json::to_string(Value::String)` so colored keys/strings carry the exact JSON escaping serde_json itself would emit.

## Deviations from Plan

None affecting scope. One in-task formatting correction:

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rustfmt-canonical formatting of test assertions**
- **Found during:** Task 2 (GREEN verification — `cargo fmt --check`)
- **Issue:** A multi-line `.stderr(...)` predicate in `tests/json.rs` and two long `assert!` lines in the `json` module's `#[cfg(test)]` block did not match rustfmt's canonical layout, failing the `cargo fmt --check` gate.
- **Fix:** Ran `cargo fmt` to apply canonical formatting (collapsed the predicate to one line; reflowed the two `assert!` calls). Behavior unchanged.
- **Files modified:** `tests/json.rs`, `src/commands/json/mod.rs`
- **Verification:** `cargo fmt --check` exits 0; unit + integration tests still green afterward.
- **Committed in:** `35a1861` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — formatting gate).
**Impact on plan:** Formatting-only; no logic change, no scope creep.

## Issues Encountered

**serde_json 1.0.150 transitively pulls an unfamiliar crate `zmij`** — During Task 2's first build, cargo resolved `serde_json 1.0.150` with a dependency on `zmij v1.0.21` (in place of the historically-expected `ryu`). Treated as a package-legitimacy check (the slopsquat/typosquat concern): verified against the **official crates.io sparse-index manifest** for `serde_json 1.0.150` (checksum `e8014e44…`), which lists `zmij` (req `^1.0`) as a declared normal dependency — i.e. serde_json itself adopted it, I did not substitute it. Confirmed `zmij`'s on-disk `Cargo.toml` shows `authors = ["David Tolnay <dtolnay@gmail.com>"]`, `repository = github.com/dtolnay/zmij`, description "A double-to-string conversion algorithm based on Schubfach and yy" — the same author as serde_json/ryu/anyhow. **Conclusion: legitimate transitive dependency, no checkpoint required.** (`indexmap`→`hashbrown` from `preserve_order` and `serde_core` from serde's recent split are likewise expected.)

## User Setup Required
None — no external service configuration required. `box json` is a local, offline, no-network, no-secrets, no-file-write command (input is stdin/arg only).

## Threat Surface
No new surface beyond the plan's `<threat_model>`: the single trust boundary remains `stdin/arg → serde_json::from_str`. T-04J-02 (malformed→panic) is mitigated — `from_str` returns `Result`, the Err arm `bail!`s (exit 1, no `.unwrap()` on user input), proven by the `{bad`→exit-1 integration test. T-04J-03 (attacker-controlled ANSI) is mitigated — color comes only from owo-colors over parsed `Value` tokens, gated on `is_color_on()`; raw input is never echoed into the colored path. No threat flags.

## Next Phase Readiness
- `box json` slice complete and verified; the `parse→validate→colorize-or-delegate` pattern and the `serde_json` dependency are now established for the remaining Phase-4 slices.
- The pure `colorize(&Value)`-style walker + `is_color_on()` gate is the reusable color-path template for `lolcat` (04-02) and `matrix` (04-03).
- 3 Phase-4 stubs remain: `lolcat`, `matrix`, `ascii`. No blockers.

## Self-Check: PASSED

- Created files exist: `src/commands/json/mod.rs`, `tests/json.rs`, `tests/cmd/json.trycmd`, `04-01-SUMMARY.md`
- Commits exist: `8824d2d` (test RED), `35a1861` (feat GREEN)

---
*Phase: 04-terminal-visuals*
*Completed: 2026-06-24*
