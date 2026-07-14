---
phase: 09-dev-transform-visual-depth
verified: 2026-07-14T15:10:00Z
status: passed
score: 13/13 must-haves verified
overrides_applied: 0
---

# Phase 09: Dev-Transform & Visual Depth Verification Report

**Phase Goal:** Add the deferred dev-transform and visual depth flags, each local to one command and additive to its existing output struct — including the animated lolcat path, which carries a PS7 human-verify gate.
**Verified:** 2026-07-14T15:10:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth (Roadmap Success Criterion) | Status | Evidence |
|---|---|---|---|
| 1 | SC1 — `uuid --v7` time-ordered + `--upper`/`--no-hyphens`/`--braces`/`--urn`; `epoch` relative + `--tz`; `color` CSS names both directions + HSL input; `json --sort-keys` opt-in vs preserved order; `passgen` entropy-bits + `--no-similar` + `--separator` | VERIFIED | `src/commands/{uuid,epoch,color,json,passgen}/mod.rs` read in full. `uuid::Form` enum + `now_v7()`/nibble-14 test (`v7_has_version_nibble_seven`); `--braces`/`--urn` `conflicts_with` (clap exit 2, tested). `epoch::relative_for` ladder feeds both human lines and JSON `relative`; `--tz` validated via `chrono_tz::Tz::from_str`, third `%Z %z` line, unit-tested for `JST`/`+0900` and bad-zone exit 1. `color::parse_hsl`/`hsl_to_rgb` routed before the RGB branch; 148-entry `CSS_NAMES` table; `exact_name`/`nearest_name` (redmean) always-present JSON fields — manually spot-checked `box color "hsl(210,100%,50%)"` → `Hex: #0080FF`, `Name: ~dodgerblue`. `json::sort_value` recursive sort applied before the output fork; manually spot-checked `echo '{"b":1,"a":2}' \| box json --sort-keys --compact` → `{"a":2,"b":1}`. `passgen::entropy_bits`/`drop_similar`/`--separator`; manually spot-checked `box passgen --words 3 --separator - --no-similar` → `blend-sarcasm-creamlike`. Cargo.toml carries `uuid v7` feature + `chrono-tz = "0.10"`. |
| 2 | SC2 — `matrix --color`/`--speed`/`--charset`; `qr --save <file>` PNG/SVG + `--error-correction L|M|Q|H`; `ascii` truecolor + `--braille` (2×4) + `--invert` | VERIFIED | `src/commands/{matrix,qr,ascii}/mod.rs` read in full. `matrix::MatrixColor`/`Speed` ValueEnums + `resolve_charset` pure resolvers feeding the existing `is_color_on()`-gated `.truecolor` sites; unit tests lock the 7 color presets, 3 speed presets, and charset-preset/custom-string resolution. `qr::render_qr(input, ec)` takes an `EcLevel` param feeding BOTH the terminal render and `save_qr`; `--save` writes PNG (`image::Luma<u8>`) or SVG (`svg::Color`), `bail!`s exit 1 on a bad/missing extension, `.with_context`-wraps every write; manually spot-checked `box qr "hello" --save qr_test.png --error-correction H` → wrote a file starting with the PNG magic bytes `89 50 4E 47`. `ascii` gained `owo_colors`/`is_color_on` imports for the first time; `render_ramp`/`render_braille` both truecolor-gated; `braille_glyph`/`DOT_BITS`/`BRAILLE_DOTS` unit-locked; `apply_invert` = `255 - luma` at the single seam; manually spot-checked `box ascii qr_test.png --braille --invert` → valid `U+28xx` braille glyphs. Cargo.toml carries `qrcode` `["image","svg"]` features re-enabled. |
| 3 | SC3 — HUMAN-VERIFY (PS7): `lolcat --animate` smooth rainbow with RAII restore, degrades to static when piped/`--json`, `--freq`/`--seed` visibly change gradient. **Per task instructions, already approved by the user this session — treated as satisfied.** Verified: the code path exists. | VERIFIED (human-approved + code-path confirmed) | 09-03-SUMMARY.md documents human approval 2026-07-14 of all 7 PS7 checks (smoothness, clean q/Esc/Ctrl+C restore, persisted final frame, visible `--freq`/`--seed` effect, `--duration 0` runs to keypress, piped degrade). Code-path confirmed by direct read of `src/commands/lolcat/mod.rs`: the mandatory AND-gate at line 155-160 (`animate && std::io::stdout().is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()`), `RawGuard` armed before the fallible `EnterAlternateScreen` `execute!` (line 240-245), `--freq`/`--seed`/`--duration` args on `LolcatArgs` (line 106-130), and the static-degrade fallthrough (line 164-172). Manually spot-checked `box lolcat "Hello World" --freq 0.5 --seed 10 --animate \| cat` → clean plain text, no hang, no escape bytes. `cargo test --test lolcat` (8 tests) passes, including the `piped_animate_degrades_to_static_byte_identical`, `forced_color_piped_animate_still_degrades_to_static`, and `piped_animate_does_not_hang` SC3-proof tests. |
| 4 | SC4 — new colored paths (`matrix --color`, `ascii` truecolor, animated `lolcat`) stay gated on `is_color_on()`; piped/`--json` output byte-identical minus ANSI | VERIFIED | `matrix`: both `.truecolor` call sites (trail + head) gated on `is_color_on()` (mod.rs:249, :261); `tests/matrix.rs::matrix_redirected_color_preset_emits_no_color_escape` scans for the truecolor SGR introducer `ESC[38;2;` and asserts absence under `--color red` piped — PASS. `ascii`: `is_color_on()` gates both `render_ramp` and `render_braille`; `tests/ascii.rs::piped_render_carries_no_ansi` — PASS. `lolcat`: `is_color_on()` gates the static render's color path AND is a conjunct of the animate AND-gate; `tests/lolcat.rs` proves piped `--animate` (even under `CLICOLOR_FORCE`) never emits the alt-screen escape and degrades byte-identically — PASS. |

**Score:** 4/4 roadmap Success Criteria verified (13/13 granular must-haves across the three plans' frontmatter — see Required Artifacts / Key Link tables below).

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `Cargo.toml` | `chrono-tz` dep + `uuid v7` feature + `qrcode ["image","svg"]` | VERIFIED | Line 34 `uuid = {..., features = ["v4","v7"]}`; line 43 `chrono-tz = "0.10"`; line 80 `qrcode = {..., features = ["image","svg"]}`. |
| `src/commands/uuid/mod.rs` | `now_v7` + four wrapping forms | VERIFIED | `Form` enum (Plain/Simple/Braces/Urn), `format_one` single-sources human+JSON, `now_v7()` at line 105, unit tests lock nibble 14 = `'7'`. |
| `src/commands/passgen/mod.rs` | `entropy_bits` + `--no-similar` + `--separator` | VERIFIED | `entropy_bits()` fn (line 238), `drop_similar()` (line 229), `--separator` arg feeds `phrase.join()` (line 155). Entropy on STDERR only (D-14), never stdout — unit + integration tested. |
| `src/commands/epoch/mod.rs` | `relative_for` humanizer + `--tz` line | VERIFIED | `relative_for()` (line 212), `parse_tz`/`format_tz_line` (line 186-205), `EpochOutput.relative`/`tz` fields (line 33-40). |
| `src/commands/json/mod.rs` | recursive `sort_value` key sort | VERIFIED | `sort_value()` (line 115), applied before the `is_json_on()` fork (line 69-73), `preserve_order` still default. |
| `src/commands/color/mod.rs` | `hsl_to_rgb` + CSS name table + redmean nearest | VERIFIED | `hsl_to_rgb()` (line 333), `CSS_NAMES` 148-entry table (line 396), `exact_name`/`nearest_name`/`redmean_sq` (line 357-390). |
| `src/commands/matrix/mod.rs` | `--color`/`--speed`/`--charset` ValueEnum presets | VERIFIED | `MatrixColor`/`Speed` `pub ValueEnum` (line 136-165), `resolve_charset` (line 442). |
| `src/commands/qr/mod.rs` | `--save` PNG/SVG + `--error-correction` + `saved_path` | VERIFIED | `save_qr()` (line 208), `Ec` ValueEnum (line 96), `QrOutput.saved_path` (line 149). Spot-checked file write with correct PNG magic bytes. |
| `src/commands/ascii/mod.rs` | truecolor + braille 2×4 + invert | VERIFIED | `0x2800` braille base (line 74/258), `DOT_BITS`/`BRAILLE_DOTS` unit-locked, `apply_invert` (line 238). |
| `src/commands/lolcat/mod.rs` | `is_terminal()` AND-gate + animate loop | VERIFIED | `is_terminal` consulted at line 156; `RawGuard` (line 223-231); `run_animate`/`render_frame`/`rgb_at(phase,freq)` (line 239-350); `animate_deadline` overflow guard (line 359-363, the BL-01 fix). |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `uuid::format_one` | both `out_line` and `emit_json` | single-source formatted value | WIRED | `UuidRow.uuid` field set from the same `format_one(u, form, self.upper)` call feeding both the human loop and the JSON `results` (mod.rs:110). |
| `epoch::relative_for` | `EpochOutput.relative` AND the integer-branch human suffix | one helper feeds both paths | WIRED | Called identically at mod.rs:110-112 (human) and mod.rs:178 (`epoch_output`, JSON). |
| `passgen::entropy` | `eprintln!` (STDERR) for human, `entropy_bits` for JSON | secret stays sole stdout content | WIRED | mod.rs:190-202 — the human path never writes entropy to stdout; integration test `entropy_never_on_stdout`-style coverage in `tests/passgen.rs` confirms. |
| `qr::render_qr` | both terminal render and `--save` file | one `EcLevel` param feeds both | WIRED | `render_qr(input, ec)` (mod.rs:242) and `save_qr(input, ec, path)` (mod.rs:208) both take the same `ec` resolved once from `self.error_correction.to_level()` (mod.rs:156). |
| `ascii::.truecolor` | `is_color_on()` gate | the SOLE color gate | WIRED | Both `render_ramp` (line 169) and `render_braille` (line 212) gate on `is_color_on()`; no other color path exists in the module. |
| `qr::--save` | `std::fs`/`ImageBuffer::save` with `.with_context` | extension whitelist `.png`/`.svg`, `bail!` on unknown | WIRED | `save_qr` mod.rs:208-233 — `bail!` on any other/missing extension, `.with_context` on every write; unit tests confirm the rejected path is never written. |
| `lolcat` animate entry | `is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()` | mandatory AND-gate before raw mode | WIRED | mod.rs:155-162; `tests/lolcat.rs::forced_color_piped_animate_still_degrades_to_static` proves `is_color_on()` alone is insufficient — `is_terminal()` is the load-bearing conjunct. |
| `lolcat::rgb_at(phase, freq)` | both animated frames and the static one-pass render | one parameterized gradient function | WIRED | `render_gradient` (static, line 204) and `render_frame` (animated, line 313) both call `rgb_at`; unit test `rgb_at_freq_and_seed_change_output` confirms `--freq`/`--seed` affect the shared function. |
| `lolcat::RawGuard` | `Drop` restores cooked mode + leaves alt-screen | armed before the fallible alt-screen `execute!`; panic-free loop | WIRED | `_guard = RawGuard` (line 243) precedes `execute!(EnterAlternateScreen, ...)` (line 245); `animate_deadline` overflow guard (BL-01 fix, commit `8db6691`) keeps the loop panic-free even for an adversarial `--duration`. |

### Data-Flow Trace (Level 4)

Not applicable in the traditional web-app sense (no DB/API layer) — this is a CLI tool where "data flow" is the single-source no-drift pattern verified above (one pure function feeds both the human render and the `--json` document). Each such pairing was traced by direct source read and confirmed with a live spot-check of the built `target/release/box.exe`:

| Computed Value | Source Function | Feeds | Confirmed Real (not static) |
|---|---|---|---|
| `uuid` string | `format_one()` | human line + JSON `results[].uuid` | Spot-checked `box uuid --v7 --braces --upper` → live-generated, version-nibble `7` present. |
| `relative` / `tz` | `relative_for()` / `format_tz_line()` | human lines + JSON fields | Spot-checked `box epoch --tz Asia/Tokyo 1700000000` → `JST +0900`, `2 years ago` (clock-relative, non-static). |
| `name` / `nearest` | `exact_name()` / `nearest_name()` | human Name row + JSON | Spot-checked `box color "hsl(210,100%,50%)"` → `~dodgerblue` (computed via redmean, not hardcoded). |
| sorted JSON | `sort_value()` | `--json`/pretty/compact/colorize | Spot-checked `echo '{"b":1,"a":2}' \| box json --sort-keys --compact` → `{"a":2,"b":1}`. |
| `entropy_bits` | `entropy_bits()` | STDERR text + JSON field | Unit-tested against the formula; charset pool size is live-computed from `--no-similar`. |
| saved QR file | `save_qr()` | filesystem | Spot-checked: PNG magic bytes `89 50 4E 47` present in the written file. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| uuid v7 + braces + upper | `box uuid --v7 --braces --upper` | `{019F60A0-CF3C-7E21-9996-893AA541E1C8}` (v7 nibble `7` at position 14 of the inner hyphenated form) | PASS |
| epoch --tz | `box epoch --tz Asia/Tokyo 1700000000` | Three lines incl. `Asia/Tokyo: 2023-11-15 07:13:20 JST +0900` | PASS |
| color hsl input | `box color "hsl(210, 100%, 50%)"` | `Hex: #0080FF`, `Name: ~dodgerblue` | PASS |
| json --sort-keys | `echo '{"b":1,"a":2}' \| box json --sort-keys --compact` | `{"a":2,"b":1}` | PASS |
| passgen --no-similar --separator | `box passgen --words 3 --separator - --no-similar` | `blend-sarcasm-creamlike` (no `il1Lo0O` chars possible in wordlist path; separator applied) | PASS |
| qr --save --error-correction H | `box qr "hello" --save qr_test.png --error-correction H` | File written, first 4 bytes `89 50 4E 47` (PNG magic) | PASS |
| lolcat --animate piped degrade | `box lolcat "Hello World" --freq 0.5 --seed 10 --animate \| cat` | Plain text, no hang, no escape bytes | PASS |
| ascii --braille --invert | `box ascii qr_test.png --braille --invert` | Valid `U+28xx` braille glyphs rendered | PASS |
| matrix piped via raw Unix pipe | `echo -n "q" \| box matrix --charset digits --color blue` | Hung (2-min bash-tool timeout) | ENV ARTIFACT — see note below |

**Note on the matrix spot-check:** the manual Git Bash pipe invocation hung because Windows' crossterm backend reads keyboard events from the Console Input Buffer (`ReadConsoleInput`), not from redirected stdin bytes — a Unix-pipe `echo "q" |` does not deliver a key event on Windows regardless of the code under test. This is a pre-existing v1 `matrix` characteristic (the loop has always run until an actual console keypress) and is orthogonal to this phase's `--color`/`--speed`/`--charset` additions. The authoritative test is the project's own `assert_cmd`-based `tests/matrix.rs::matrix_starts_and_exits_non_hanging` and `matrix_redirected_color_preset_emits_no_color_escape`, both of which passed in the automated suite (`cargo test --release --test matrix` → 2 passed, 0 failed).

### Probe Execution

N/A — this is not a migration/tooling phase; no `scripts/*/tests/probe-*.sh` files exist in the repository and none are declared in the PLAN/SUMMARY files. Skipped per Step 7c.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| UUID-V2-01 | 09-01 | uuid v7 + format flags | SATISFIED | `Form` enum, `now_v7()`, nibble test, `--braces`/`--urn` conflict (exit 2 tested). |
| EPOC-V2-01 | 09-01 | epoch relative time + `--tz` | SATISFIED | `relative_for`, `chrono_tz::Tz::from_str`, `%Z %z` line, unit + integration tested. |
| COLR-V2-01 | 09-01 | color CSS names + HSL | SATISFIED | `CSS_NAMES` table, `hsl_to_rgb`, `exact_name`/`nearest_name`, spot-checked live. |
| JSON-V2-01 | 09-01 | json `--sort-keys` | SATISFIED | `sort_value` recursive sort, `preserve_order` still default, spot-checked live. |
| PASS-V2-01 | 09-01 | passgen entropy + `--no-similar` + `--separator` | SATISFIED | `entropy_bits`, `drop_similar`, `--separator`, STDERR-only entropy (D-14). |
| MTRX-V2-01 | 09-02 | matrix `--color`/`--speed`/`--charset` | SATISFIED | `MatrixColor`/`Speed` ValueEnums, `resolve_charset`, `is_color_on()`-gated. |
| QR-V2-01 | 09-02 | qr `--save` + `--error-correction` | SATISFIED | `save_qr` PNG/SVG, `Ec` ValueEnum, `saved_path`, spot-checked live PNG write. |
| ASCI-V2-01 | 09-02 | ascii truecolor + `--braille` + `--invert` | SATISFIED | truecolor default-on gated, braille `DOT_BITS` unit-locked, `apply_invert`, spot-checked live. |
| LOL-V2-01 | 09-03 | lolcat `--animate` | SATISFIED | AND-gate, `RawGuard`, `rgb_at(phase,freq)`, `--duration` overflow-safe (BL-01 fixed), PS7 human-approved 2026-07-14. |

**Orphaned requirements check:** REQUIREMENTS.md maps exactly these 9 IDs to Phase 9 (lines 65-69, 73-76) and all 9 appear in the three plans' `requirements:` frontmatter — no orphans, no unclaimed IDs.

**Documentation staleness (non-blocking, INFO):** REQUIREMENTS.md's traceability table (lines 140-148) and the requirement checkboxes (lines 65-69, 73-76) still show all 9 Phase-9 requirements as unchecked `[ ]` / "Pending", even though the code, tests, code review, and human-verify gate are all complete. This appears to be a bookkeeping step not yet run (Phase 8's rows were updated to "Complete" after its verification) — recommend the orchestrator update REQUIREMENTS.md to mark these 9 rows "Complete" as part of closing this phase.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/commands/matrix/mod.rs` | 448-455 (`resolve_charset`) | Custom `--charset` string accepts any character with no width validation (09-REVIEW.md WR-01) | WARNING | A wide/CJK or zero-width `--charset` literal can corrupt the per-column grid math (visual smearing), not a crash or data-integrity issue. Does not block MTRX-V2-01's core claim (preset name OR literal string both resolve). Open, unfixed as of this verification. |
| `src/commands/passgen/mod.rs` | 122-124, 155 | `--separator` accepts an embedded newline with no validation (09-REVIEW.md WR-02) | WARNING | A `--separator` containing `\n`/`\r` could make one `PassgenRow`/`out_line` call print as multiple visible lines, silently breaking a script's `--count N` line-count assumption. Does not block PASS-V2-01's core claim. Open, unfixed as of this verification. |
| `src/commands/lolcat/mod.rs` | 279-289 | `run_animate` explicit teardown + `RawGuard::drop` both run the same leave-alt-screen/show-cursor sequence (09-REVIEW.md WR-03) | WARNING | Relies on the terminal emulator treating a redundant "leave alt screen"/"show cursor" as a no-op (true for Windows Terminal/conhost, unverified for all emulators). Does not affect the human-approved on-screen behavior. Open, unfixed as of this verification. |
| `src/commands/epoch/mod.rs` | 166-181, 196-205 | `epoch_output`/`format_tz_line` duplicate the `DateTime::from_timestamp` derivation (09-REVIEW.md IN-01) | INFO | Maintenance-risk duplication, not a functional gap — both call sites currently agree. |
| `src/commands/matrix/mod.rs`, `src/commands/lolcat/mod.rs` | 174-182/463-469, 223-231/325-331 | `RawGuard`/`is_quit` duplicated near-identically across two modules (09-REVIEW.md IN-02) | INFO | Maintenance-risk duplication (a future teardown fix must be applied twice), not a functional gap. |
| `.planning/REQUIREMENTS.md` | 65-69, 73-76, 140-148 | Phase 9 requirement checkboxes/status still show unchecked/"Pending" | INFO | Documentation staleness — see Requirements Coverage note above. |

**Note:** The one CRITICAL finding from 09-REVIEW.md (CR-01/BL-01 — `lolcat --animate --duration <huge>` panics and strands the terminal in raw mode under `panic = "abort"`) is RESOLVED — confirmed by direct source read of `animate_deadline()` (lolcat/mod.rs:359-363, using `checked_add`) and its locking unit test `animate_deadline_is_overflow_safe`, and by the commit trail (`8db6691` fix + `d0920af` marking it resolved in 09-REVIEW.md frontmatter `critical: 0, critical_resolved: 1`).

### Human Verification Required

None. The phase's sole human-verify gate (Task 3 of 09-03-PLAN.md, LOL-V2-01/SC3) was already presented and approved by the user this session (2026-07-14, all 7 PS7 checks passed per 09-03-SUMMARY.md), and the task instructions for this verification explicitly direct treating SC3 as satisfied while confirming the code path exists — which has been done above.

### Gaps Summary

No gaps. All four ROADMAP Phase 9 Success Criteria are verified against the actual codebase (not just SUMMARY.md claims):
- Full `cargo test --release` run: 207 unit tests (0 failed) + all 29 integration/doc test binaries reporting "test result: ok" with 0 failures — matches the SUMMARY's claimed count exactly.
- `cargo clippy --all-targets -- -D warnings`: clean, no output.
- The one code-review CRITICAL (BL-01, `lolcat --duration` `Instant` overflow) is resolved and locked by a dedicated overflow-safety unit test.
- Live spot-checks against the built `target/release/box.exe` for uuid, epoch, color, json, passgen, qr, lolcat, and ascii all produced correct, non-static output.
- Three code-review WARNING findings (WR-01/WR-02/WR-03) remain open — these are robustness/edge-case gaps (unvalidated `--charset` width, unvalidated `--separator` newline, a redundant-but-likely-idempotent teardown call) that do not contradict any of the four roadmap Success Criteria or the plans' declared must-haves, so they are reported as WARNING-level findings rather than blocking gaps.
- REQUIREMENTS.md's Phase-9 rows are not yet marked "Complete" (documentation bookkeeping, INFO-level, does not affect code delivery).

---

_Verified: 2026-07-14T15:10:00Z_
_Verifier: Claude (gsd-verifier)_
