---
phase: 07-spine-rollout
fixed_at: 2026-06-25T00:00:00Z
review_path: .planning/phases/07-spine-rollout/07-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 7: Code Review Fix Report

**Fixed at:** 2026-06-25
**Source review:** .planning/phases/07-spine-rollout/07-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (the 6 Warnings WR-01..WR-06; 0 Critical; the 7 Info findings were out of scope and untouched)
- Fixed: 6
- Skipped: 0

All in-scope fixes preserve the Phase-7 locked contracts: JSON stdout purity (no
ANSI/BOM/chrome under `--json`), the bulk-rename abort-empty-stdout path, the SC4
display-only omission, and base64 binary-decode safety. After the changes the
project builds and passes `cargo test`, `cargo test --bin box`, and
`cargo clippy --all-targets` clean.

## Fixed Issues

### WR-01: `weather --clip` silently copies nothing (human path bypasses `out_line`)

**Files modified:** `src/commands/weather/mod.rs`
**Commit:** 7130c90
**Applied fix:** Routed the three plain human lines (`Temperature`/`Wind`/
`Humidity`) and the non-color `Conditions` branch through
`crate::core::output::out_line`, so the human render tees to `CLIP_BUF` under
`--clip` like every other spine command. The colored `Conditions` branch stays a
direct `println!` because `is_color_on()` is forced false under `--clip`
(`init_output`), so that branch is unreachable when clip is active and never needs
to tee (mirrors the documented `color` swatch pattern). No JSON-path change.

### WR-02: Filesystem JSON paths are lossy and non-round-trippable for non-UTF-8 NTFS names

**Files modified:** `tests/dupes.rs`
**Commit:** 147dc8b
**Applied fix:** Added the regression guard `json_lossy_path_name_no_panic` that
the review flagged as missing. On Unix it builds a genuinely non-UTF-8 file name
(a lone `0x80` continuation byte via `OsStrExt`) — exactly the case
`to_string_lossy` maps to U+FFFD; on Windows (NTFS UTF-16, where std cannot create
an invalid OS string) it uses a non-ASCII multibyte name to drive the same
projection. The test asserts `box dupes --json` exits 0 without panicking and
still emits one well-formed document whose `.results[].paths` are strings (D-4).
The documented D-4 `to_string_lossy` behavior is deliberately unchanged (a
round-trippable byte/escaped form is a post-v1 consideration).

### WR-03: SC4 omission for `matrix`/`pomodoro` is enforced only by a manual grep, not a test

**Files modified:** `tests/cli.rs`
**Commit:** cc5648c
**Applied fix:** Added the `matrix_pomodoro_have_no_spine_calls` source-level CI
guard. It reads `src/commands/matrix/mod.rs` and `src/commands/pomodoro/mod.rs`
and asserts neither *calls* `emit_json` nor `is_json_on`. The review's suggested
naive `contains` would have false-positived: both modules' doc comments
legitimately MENTION those functions to explain the omission. The implemented
guard strips comment lines (`//`, `//!`, `///`, block-comment `*` rows) before
scanning, so it catches a real spine call without tripping on the documentation —
turning the plan-document grep into a compiled-suite gate.

### WR-04: `du`/`dupes`/`tree` emit unguarded raw `println!` outside the `--json` fork

**Files modified:** `src/commands/du/mod.rs`, `src/commands/dupes/mod.rs`,
`src/commands/tree/mod.rs`
**Commit:** 2b47b21
**Applied fix:** Documented the post-fork invariant at each command's human-render
cluster (the review offered "route through `out_line` OR document the invariant").
Chose documentation deliberately: these three commands are NOT SPINE-04 `--clip`
commands, so routing their human render through `out_line` would silently expand
behavior (teeing their output to the clipboard) and risk the SPINE-04 contract
boundary. The added comments pin that every `println!` below the `is_json_on()`
fork is reachable only when `!is_json_on()` and that hoisting a human write above
the fork would contaminate the JSON channel. No behavior change; JSON purity
preserved.

### WR-05: `dupes` JSON omits the per-file `hash` and the `paths` ordering guarantee

**Files modified:** `src/commands/dupes/mod.rs`, `tests/dupes.rs`
**Commit:** ad89543
**Applied fix:** Took the low-risk documentation + test route the review allowed
("consider" adding the hash; "at minimum add a test asserting `.results[].paths`
is sorted, or document the omission"). Documented in the `DupeRow` struct doc that
the content `hash` is intentionally omitted (the human render omits it too — no
drift — and emitting it would change the locked D-17 group schema that
`json_purity` pins), and that intra-group `paths` are sorted ascending. Added the
`json_paths_sorted_within_group` test that creates three identical files with
non-creation-order names and asserts the emitted `.results[0].paths` is sorted —
giving the ordering guarantee a CI gate. Did NOT add `hash` to the JSON (schema
change, higher risk, out of v1 scope).

### WR-06: `flatten`/`bulk-rename` real-run `count`/`results` derive from the plan, not the executed loop

**Files modified:** `src/commands/flatten/mod.rs`, `src/commands/bulk_rename/mod.rs`
**Commit:** 7e090c1
**Applied fix:** Added a clarifying comment plus `debug_assert_eq!`
reconciliations at both real-run JSON emit sites (the review rated this low
priority and said "a clarifying comment or debug assertion is sufficient"). For
flatten: `copied == to_copy + renamed` and `to_copy + renamed + skipped ==
plan.items.len()`. For bulk-rename: `renamed == to_rename` and `to_rename +
unchanged + skipped == plan.items.len()`. These pin the planner/executor coupling
so a future divergence trips in test/dev builds instead of silently misreporting
the JSON. Verified the assertions hold on every real-run integration test (debug
build → `debug_assert!` active): the full flatten and bulk-rename suites pass.

---

_Fixed: 2026-06-25_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
