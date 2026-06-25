---
phase: 07-spine-rollout
plan: 01
subsystem: cli
tags: [serde, serde_json, json-output, clipboard, spine, base64, epoch, color, passgen, cowsay]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "core::output spine (is_json_on/emit_json/out_line/flush_clip), global --json/--clip flags, {Row}/{Output} serde pilot (uuid/hash)"
provides:
  - "--json on 8 Wave-7a pure-transform commands (base64, epoch, color, passgen, 8ball, fortune, roast, cowsay) — SPINE-02 partial (8 of 16)"
  - "--clip on 4 Wave-7a commands (base64, color, epoch, passgen) — SPINE-04 partial (4 of 6)"
  - "A1 base64 binary-safe decode policy: --json --decode re-encodes bytes to base64, never from_utf8 unwrap (T-07a-01)"
  - "SC4 display-only omission contract: matrix/pomodoro/lolcat/ascii/clip parse-but-ignore the spine flags"
  - "NEW tests/cowsay.rs + per-command json_purity tests + #[ignore]d clip_roundtrip tests"
affects: [07-02-filesystem-rollout, 07-03-json-qr-weather, 08-filesystem-depth]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Scalar command → flat serde object (base64 {output,mode}, fun-trio {text}, color nested {hex,rgb,hsl}, epoch unified {epoch,utc,local})"
    - "Multi-capable command → always-wrapped {results,count} (passgen, EXACT uuid copy)"
    - "Binary-safe JSON: re-encode arbitrary bytes to base64 rather than lossy String coercion (A1)"
    - "Unified-shape fork: resolve all input modes to one value BEFORE the is_json_on() fork so JSON never branches (epoch)"
    - "Parse-but-ignore SC4: display-only commands never call emit_json/is_json_on"

key-files:
  created:
    - "tests/cowsay.rs"
  modified:
    - "src/commands/base64/mod.rs"
    - "src/commands/epoch/mod.rs"
    - "src/commands/color/mod.rs"
    - "src/commands/passgen/mod.rs"
    - "src/commands/eight_ball/mod.rs"
    - "src/commands/fortune/mod.rs"
    - "src/commands/roast/mod.rs"
    - "src/commands/cowsay/mod.rs"
    - "tests/cli.rs"

key-decisions:
  - "A1 RESOLVED: base64 --decode --json carries decoded bytes re-encoded as base64 in `output` (lossless, round-trippable); decode `mode` documented as base64-of-bytes; binary decode is NOT clip-supported"
  - "color JSON `hex` LOCKED lowercase (#rrggbb); human `Hex` row stays uppercase (#RRGGBB) — only the JSON field is lowercased for a deterministic test"
  - "epoch unified shape: input resolved to a single `epoch: i64` BEFORE the fork; epoch_output() reuses format_timestamp's DateTime math (no-drift)"
  - "passgen row field name = `password`; fun-trio + cowsay + 8ball field name = `text`; cowsay serializes the raw message, not the rendered bubble (A6)"
  - "SC4 mechanism = parse-but-ignore (RESEARCH option a): the 5 display-only modules never consume the spine; test covers runnable subset (clip/ascii/lolcat) live + grep gate for matrix/pomodoro"

patterns-established:
  - "json_purity test per --json command (copied from tests/uuid.rs:135): one JSON value, schema shape, no 0x1B, no BOM"
  - "#[ignore]d clip_roundtrip test per SPINE-04 command (copied from tests/uuid.rs:237)"

requirements-completed: [SPINE-02, SPINE-04]

# Metrics
duration: 19min
completed: 2026-06-25
---

# Phase 7 Plan 01: Wave-7a Pure-Transform Spine Rollout Summary

**The frozen Phase-6 `--json`/`--clip` spine now spans the 8 pure-transform commands (base64/epoch/color/passgen/8ball/fortune/roast/cowsay), the A1 base64 binary-safe-decode surprise is resolved on a pure transform, and the SC4 display-only omission contract is locked and tested.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-06-25T13:15:24Z
- **Completed:** 2026-06-25T13:34:52Z
- **Tasks:** 3
- **Files modified:** 17 (1 created, 16 modified)

## Accomplishments
- 8 Wave-7a commands accept `--json` and emit exactly one parseable JSON document (SPINE-02, 8 of 16): base64 `{output,mode}`, epoch unified `{epoch,utc,local}`, color nested `{hex,rgb:{r,g,b},hsl:{h,s,l}}`, passgen `{results,count}`, and the fun-quartet `{text}`.
- `base64`, `color`, `epoch`, `passgen` route their primary output through `out_line`/`emit_json`, so `--clip` copies the printed result (SPINE-04, 4 of 6).
- **A1 resolved on a pure transform:** `base64 --decode --json` is binary-safe — decoded non-UTF-8 bytes are re-encoded to base64 in `output` (never `String::from_utf8(...).unwrap()`), backed by the `json_decode_non_utf8` regression test (T-07a-01).
- SC4 locked: the 5 display-only commands parse-but-ignore the global flags (a doc note in each module + the `display_only_omit_json` test + a grep gate proving 0 non-comment `emit_json`/`is_json_on` references).
- Full Wave-7a gate green: all integration suites + 157 unit tests; clippy clean.

## Task Commits

Each task was committed atomically (TDD tasks use test → feat):

1. **Task 1: cowsay test file + scalar fun-trio + cowsay --json + base64 (A1)** — `a813904` (test, RED) → `8283512` (feat, GREEN)
2. **Task 2: epoch + color + passgen (unified/nested/multi-capable)** — `6cead48` (test, RED) → `1ef4b80` (feat, GREEN)
3. **Task 3: SC4 display-only omission contract + Wave-7a full-suite gate** — `5b0999e` (feat)

**Plan metadata:** (docs commit, this SUMMARY + STATE/ROADMAP/REQUIREMENTS)

## Files Created/Modified
- `tests/cowsay.rs` — NEW assert_cmd test file with cowsay `json_purity` (cowsay previously had only `tests/cmd/*.trycmd`)
- `src/commands/base64/mod.rs` — `Base64Output{output,mode}`; is_json_on fork FIRST; encode via `out_line`; A1 binary-safe decode (re-encode to base64)
- `src/commands/epoch/mod.rs` — `EpochOutput{epoch,utc,local}` unified across all modes; `epoch_output()` reuses DateTime math; human path via `out_line`
- `src/commands/color/mod.rs` — `Rgb`/`Hsl`/`ColorOutput{hex,rgb,hsl}` nested; JSON hex lowercase-locked; block tees via `out_line`
- `src/commands/passgen/mod.rs` — `PassgenRow{password}` + `PassgenOutput{results,count}`; rows collected once feed both paths; passwords tee to clip
- `src/commands/eight_ball/mod.rs` — `EightBallOutput{text}`; question never serialized
- `src/commands/fortune/mod.rs`, `src/commands/roast/mod.rs` — `{text}` fork BEFORE soft-wrap; emit UNWRAPPED string; human path via `out_line`
- `src/commands/cowsay/mod.rs` — `CowsayOutput{text}` raw message (A6, bubble not serialized); --json-only, human bubble keeps `println!`/`print!`
- `src/commands/{matrix,pomodoro,lolcat,ascii,clip}/mod.rs` — `# Spine omission (SC4)` doc note (parse-but-ignore)
- `tests/{base64,epoch,color,passgen}.rs` — `json_purity` (+ `json_count_multi` for passgen, `json_decode_non_utf8` for base64) + `#[ignore]`d `clip_roundtrip`
- `tests/{eight_ball,fortune,roast}.rs` — `json_purity`
- `tests/cli.rs` — `display_only_omit_json` SC4 test (runnable subset live + source-state for loopers)

## Decisions Made
- **A1 (base64 binary decode):** Adopted the RESEARCH-recommended base64-string-field policy. The decode-under-json path re-encodes the decoded bytes to base64 in `output`; the human binary-decode path is unchanged (`write_all` raw bytes) and is NOT clip-supported (documented). No `from_utf8` unwrap anywhere → no panic on binary (T-07a-01 mitigation).
- **color hex case:** JSON `hex` locked LOWERCASE (`#{:02x}`) for a deterministic `json_purity` test; the human `Hex` row keeps its existing UPPERCASE spelling. Called out in a doc comment.
- **epoch unification:** Resolved all three input modes (now/int/string) to a single `epoch: i64` BEFORE the `is_json_on()` fork, so the JSON document is byte-shape-identical regardless of input direction (D-17). `epoch_output()` and `format_timestamp()` share the same `DateTime::from_timestamp`/`with_timezone(&Local)` math (no-drift). JSON datetime strings drop the `Local:`/`UTC:` label prefixes the human lines carry, since the JSON key names already convey direction.
- **SC4:** Used the parse-but-ignore mechanism (RESEARCH option a). The runnable subset (clip via piped stdin, ascii via the `tests/cmd/ascii.in/tiny.png` fixture, lolcat via a tiny arg) is asserted live; matrix/pomodoro (which loop/block) are covered by source-state (no `emit_json`) plus the grep gate in the acceptance criteria.

## Deviations from Plan

None — plan executed exactly as written. All locked decisions (A1, A6, D-17, SC4) and discretion field names were applied as specified; the human render paths stayed byte-identical (verified by the unchanged `tests/cmd/color.trycmd` and `cowsay-*.trycmd` snapshots passing).

## Issues Encountered
- `cargo test --test X filter --test Y filter` is not valid (cargo accepts one `--test`/filter pair); ran per-command verification commands sequentially instead. No code impact.

## Authentication Gates
None — Phase 7 installs zero packages and touches no external service. The `#[ignore]`d `clip_roundtrip` tests touch the shared OS clipboard and are run locally with `--ignored --test-threads=1` (a human-verify-adjacent check), never in the automated gate.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The Wave-7a slice is complete and the spine pattern is proven on the simplest commands, so 07-02 (filesystem buffered-rows: du/tree/dupes/flatten/bulk-rename) and 07-03 (json/qr/weather) can copy the same `is_json_on()` fork + `json_purity` test mechanically.
- The one genuine surprise this wave was designed to surface (A1 base64 binary decode) is resolved here — the filesystem commands inherit a known-good binary-safety pattern (`to_string_lossy()` for paths, base64 for bytes).
- No blockers. The SC4 contract is locked so the display-only commands are settled for the rest of v2.0.

## Self-Check: PASSED
- All 6 spot-checked files exist (tests/cowsay.rs + 4 modified modules + tests/cli.rs).
- All 5 task commits exist in git history (a813904, 8283512, 6cead48, 1ef4b80, 5b0999e).

---
*Phase: 07-spine-rollout*
*Completed: 2026-06-25*
