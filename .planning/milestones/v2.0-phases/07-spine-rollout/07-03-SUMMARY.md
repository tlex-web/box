---
phase: 07-spine-rollout
plan: 03
subsystem: cli
tags: [serde, serde_json, json-output, clipboard, spine, qr, weather, json-passthrough, clip_feed]

# Dependency graph
requires:
  - phase: 07-spine-rollout (plans 01 + 02)
    provides: "frozen {Row}/{Output} serde + is_json_on() fork + out_line routing template; json_purity / clip_roundtrip test templates; D-01..D-17 conventions proven on the 13 pure-transform + filesystem commands"
provides:
  - "--json on the final 3 Wave-7c commands (json, qr, weather) — SPINE-02 COMPLETE (16 of 16)"
  - "--clip on json + qr — SPINE-04 COMPLETE (6 of 6 new this phase)"
  - "NEW core::output::clip_feed(&str): the ONE sanctioned spine addition this phase — tees source text to CLIP_BUF under --clip with NO stdout write (the 'print X, copy Y' split out_line cannot express)"
  - "qr --json emits QrOutput {text, error_correction:\"M\"} metadata (D-14), NOT glyphs; qr --clip copies the SOURCE TEXT via clip_feed (D-15 / A2 resolved)"
  - "json --json is D-16 identity passthrough (emit_json(&value) verbatim, NOT wrapped); json/--compact branches route through out_line for --clip"
  - "weather --json is the D-17 current-only WeatherOutput with unit/wind_unit read from current_units (never hardcoded — imperial wind label is mp/h, Pitfall WTHR-3)"
affects: [08-filesystem-depth, 09-dev-transform-visual-depth, 10-fun-system-depth, 11-meta-commands]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "clip_feed(&str): the 'print X, copy Y' clip-tee primitive (mirrors out_line's tee half, omits the println!) for commands whose stdout payload differs from the clipboard payload (qr glyphs vs source text)"
    - "Identity passthrough (json D-16): the ONE direct-serde command — emit_json(&value) on the parsed Value verbatim; the --json fork wins over --compact (the machine document is always the pretty serde form)"
    - "Offline HTTP fixture server: a one-shot loopback TcpListener serving a fixture body, pointed at via BOX_WEATHER_BASE_URL; a lat,lon location skips geocoding so only the forecast GET runs (network-free, deterministic)"
    - "Authoritative-label-from-response: weather unit/wind_unit serialized from forecast.current_units, never the request param (mp/h vs mph)"

key-files:
  created: []
  modified:
    - "src/core/output.rs"
    - "src/commands/qr/mod.rs"
    - "src/commands/json/mod.rs"
    - "src/commands/weather/mod.rs"
    - "tests/qr.rs"
    - "tests/json.rs"
    - "tests/weather.rs"

key-decisions:
  - "clip_feed gates on CLIP_ON (mirrors out_line's tee) and appends a trailing '\\n' to match out_line's tee shape; documented as the SOLE sanctioned core::output addition of Phase 7 (for qr D-15 only)"
  - "qr --json does NOT call render_qr (D-14 metadata branch needs no glyphs); the human path keeps println! for the glyph block and calls clip_feed(&input) so --clip copies the source text; under --json --clip emit_json's own tee copies the document (no double-feed)"
  - "json --json fork is FIRST and wins over --compact — emit_json(&value) is always the pretty serde document (the decisive discriminator the json_identity_passthrough test pins); the plain to_string_pretty + --compact human branches route through out_line so --clip / --compact --clip tee the printed form; the colored branch stays print! (never reached under --clip, COLOR_ON forced false)"
  - "weather WeatherOutput = {location, temperature, unit, conditions, wind_speed, wind_unit, humidity} (current-only, D-17); unit/wind_unit from forecast.current_units (.clone()); f64 fields straight from forecast.current (finite real API data, no hand-computed NaN/Inf)"
  - "weather json_purity is network-free via a one-shot loopback TcpListener serving forecast_imperial.json; box weather \"51.5,-0.25\" (a lat,lon location) skips geocoding so only the forecast GET runs; asserts unit==\"°F\"/wind_unit==\"mp/h\" to prove the label is read from current_units (not hardcoded)"

patterns-established:
  - "clip_feed_tees_only unit test (mirrors out_line_tees): asserts the tee under --clip + no-op when --clip is off + no stdout write"
  - "json_metadata_not_glyphs (qr): --json yields {text, error_correction} with NO half-block glyph bytes in the document"
  - "json_identity_passthrough (json): the document is the input object verbatim (no results/count wrapper, key order preserved) AND --json beats --compact (proves the emit_json route)"
  - "weather json_purity offline-fixture pattern: spawn_fixture_server(body) + lat,lon location → forecast-only GET → assert current-only shape + authoritative unit labels"

requirements-completed: [SPINE-02, SPINE-04]

# Metrics
duration: 13min
completed: 2026-06-25
---

# Phase 7 Plan 03: Wave-7c json/qr/weather Spine Rollout Summary

**The frozen `--json`/`--clip` spine now spans ALL 16 commands and the final 2 (of 6) `--clip` commands: the ONE sanctioned spine addition `clip_feed(&str)` is live and lets `qr` print glyphs while copying the source text (D-15), `json` is the D-16 identity passthrough (emit the parsed document verbatim, never wrapped), and `weather` is the D-17 current-only shape with its unit labels read from the API's `current_units` (mp/h, never hardcoded) — closing SPINE-02 (16/16) and SPINE-04 (6/6) for the whole phase.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-06-25T14:05:42Z
- **Completed:** 2026-06-25T14:19:18Z
- **Tasks:** 3 (all TDD)
- **Files modified:** 7 (1 spine file + 3 command modules + 3 test files)

## Accomplishments
- **SPINE-02 COMPLETE (16 of 16):** the final 3 commands accept `--json` and emit exactly one parseable JSON document — qr `{text, error_correction}` (D-14 metadata, not glyphs), json identity passthrough (D-16, the parsed Value verbatim), weather current-only `{location, temperature, unit, conditions, wind_speed, wind_unit, humidity}` (D-17).
- **SPINE-04 COMPLETE (6 of 6):** json + qr accept `--clip` — json tees its pretty/compact form via `out_line`; qr copies the SOURCE TEXT (not the glyphs) via the new `clip_feed`.
- **The one sanctioned spine change:** `core::output::clip_feed(&str)` — the "print X, copy Y" primitive `out_line` cannot express. Gated on `CLIP_ON`, no stdout write, unit-tested by `clip_feed_tees_only`. This is the SOLE `core::output` addition of Phase 7 (A2 resolved).
- **D-15 resolved:** `qr --clip` copies the encoded text, not the half-block render (locked by the `#[ignore]`d `clip_copies_source_text` asserting `pasted == input`).
- **Pitfall WTHR-3 held:** weather's `unit`/`wind_unit` are serialized from `forecast.current_units`, proven by the offline `json_purity` test asserting `wind_unit == "mp/h"` (the API's label, not the `mph` request param).
- **Phase-final gate green:** the whole Phase-7 set passes — full `cargo test` (all integration suites, zero failures) + `cargo test --bin box` (158 unit tests) + clippy clean across all targets.

## Task Commits

Each task was committed atomically (TDD: a genuine RED was demonstrated for each before the GREEN, then committed as one feat per task since the spine unit test and qr/json discriminator tests live alongside their impl):

1. **Task 1: clip_feed primitive + qr (D-14 metadata + D-15 clip-copies-text)** — `7daf4f9` (feat) — RED was a compile error (no `clip_feed`) then GREEN.
2. **Task 2: json D-16 identity passthrough + --clip routing** — `cb639d8` (feat) — RED was the strengthened `json_identity_passthrough` failing because the pre-spine human path honored `--compact` even under `--json`.
3. **Task 3: weather D-17 current-only + phase-final gate** — `89d67a1` (feat) — RED was `json_purity` failing to parse the human aligned block as JSON.

**Plan metadata:** (docs commit — this SUMMARY + STATE/ROADMAP/REQUIREMENTS)

## Files Created/Modified
- `src/core/output.rs` — NEW `pub fn clip_feed(&str)` (tees source text to CLIP_BUF under --clip, no stdout write) + `clip_feed_tees_only` unit test (mirrors `out_line_tees`)
- `src/commands/qr/mod.rs` — `QrOutput{text, error_correction}`; is_json_on fork FIRST (emit metadata, no `render_qr` on the json branch); human path keeps `println!` for the glyph block and calls `clip_feed(&input)` (D-15)
- `src/commands/json/mod.rs` — D-16: `is_json_on()` fork emits `emit_json(&value)` verbatim (wins over `--compact`); plain `to_string_pretty` + `--compact` branches route through `out_line` for `--clip`; colored branch unchanged; invalid → bail! (exit 1, empty stdout) unchanged
- `src/commands/weather/mod.rs` — `WeatherOutput` current-only struct (`Serialize`); is_json_on fork FIRST; unit/wind_unit `.clone()`d from `current_units`; human aligned block + stderr echo unchanged
- `tests/qr.rs` — `json_metadata_not_glyphs` + `json_purity` + `#[ignore]`d `clip_copies_source_text` (D-15)
- `tests/json.rs` — `json_identity_passthrough` (no wrapper + key order + --json beats --compact) + `json_purity` + `#[ignore]`d `clip_roundtrip` (pretty + compact variants)
- `tests/weather.rs` — `json_purity` via a one-shot loopback `TcpListener` fixture server (network-free) + `spawn_fixture_server` helper

## Decisions Made
- **clip_feed shape (A2):** mirrors the tee half of `out_line` (`CLIP_BUF.push_str(s); push('\n')`) gated on `CLIP_ON`, but omits the `println!`. Documented as the SOLE sanctioned `core::output` addition this phase. A no-op when `--clip` is off.
- **qr --json needs no render (D-14):** the json branch returns `emit_json(&QrOutput{...})` before `render_qr` is ever called — the glyphs are a visual, not data. The human path is unchanged (`println!` the glyph block) and adds `clip_feed(&input)` so `--clip` copies the source text. Under `--json --clip`, `emit_json`'s own tee copies the document (no double-feed from clip_feed, which the json branch never reaches).
- **json --json beats --compact (D-16):** the `is_json_on()` fork is FIRST and returns `emit_json(&value)` (always the pretty serde document), so `box json --json --compact` yields the pretty passthrough — this is the decisive behavioral discriminator the `json_identity_passthrough` test pins (the pre-spine human path would have minified). The plain and `--compact` human branches route through `out_line` for `--clip`; the colored `print!` branch is left as-is because `init_output` forces `COLOR_ON=false` under `--clip`/`--json`, so it is never reached on a clip path.
- **weather offline fixture (discretion):** rather than the unbound-port offline seam (which the existing `offline_exits_1_clean` already covers), `json_purity` spins up a one-shot loopback `TcpListener` serving `forecast_imperial.json` and uses a `lat,lon` location so only the forecast GET runs (geocoding is skipped). Asserting `wind_unit == "mp/h"` is the concrete proof the label comes from `current_units`, not a hardcoded `mph`.
- **weather field set (D-17 discretion):** `{location, temperature, unit, conditions, wind_speed, wind_unit, humidity}` — snake_case, current-only, all built from the already-parsed `forecast`. No forecast/daily/hourly fields (Phase 10).

## Deviations from Plan

None — plan executed exactly as written. All locked decisions (D-14, D-15, D-16, D-17, A2) and the discretion field names were applied as specified; the human render paths stayed byte-stable (verified by the unchanged qr/json/weather human integration tests passing).

One lint-hygiene touch on NEW code (not a behavioral change, folded into the Task-3 commit): a clippy `doc_lazy_continuation` warning on the new `weather` `json_purity` doc comment (a list followed by trailing lines) was resolved by inserting blank lines so the trailing sentences are their own paragraph. No source logic affected.

## Known Stubs

None — every `--json` document is wired to the command's real computed data: qr's source input + the fixed `"M"` EC level, json's parsed `serde_json::Value` (verbatim), weather's parsed `forecast` (`current` + `current_units`). No hardcoded empties, placeholders, or unwired data sources were introduced.

## Issues Encountered
- The `json_identity_passthrough` test initially passed in the RED phase because the pre-spine human path already emits valid passthrough JSON for plain pretty output. Per the TDD fail-fast rule I strengthened the test with a decisive discriminator (`--json` must beat `--compact`, i.e. force the pretty `emit_json` document) — that produced a genuine RED, and the D-16 implementation turned it GREEN. No code impact beyond the stronger assertion.
- The weather `json_purity` test needed a real (offline) HTTP response since `--json` runs the full fetch path; a one-shot loopback `TcpListener` fixture server plus a `lat,lon` location (skipping geocoding) made it network-free and deterministic.

## Authentication Gates
None — Phase 7 installs zero packages and touches no external service. The weather `json_purity` test serves a local fixture over loopback (never the live Open-Meteo API). The `#[ignore]`d `clip_*` round-trip tests touch the shared OS clipboard and are run locally with `--ignored --test-threads=1` (a human-verify-adjacent check), never in the automated gate.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- **Phase 7 is COMPLETE:** all 16 commands carry `--json` (SPINE-02), the 6 new `--clip` commands are wired (SPINE-04), the SC4 display-only omission contract is locked, and `clip_feed` is the only spine addition. The depth phases (8 filesystem / 9 dev+visual / 10 fun+system) inherit a fully-rolled, frozen spine — they add fields/flags to the existing `{Output}` structs, never the fork mechanism.
- `clip_feed` is available for any future "print X, copy Y" command (none currently planned beyond qr).
- The offline HTTP fixture-server pattern is available for any future network-touching `--json` test (weather's `--forecast` in Phase 10 can reuse it).
- No blockers.

## Self-Check: PASSED
- All 7 modified files exist on disk (src/core/output.rs + 3 command modules + 3 test files).
- All 3 task commits exist in git history (7daf4f9, cb639d8, 89d67a1).

---
*Phase: 07-spine-rollout*
*Completed: 2026-06-25*
