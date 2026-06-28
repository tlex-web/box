---
phase: 09-dev-transform-visual-depth
plan: 01
subsystem: cli-commands
tags: [uuid, uuid-v7, chrono-tz, hsl, css-colors, redmean, entropy, serde_json, clap]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "is_json_on()/emit_json/out_line spine + {results,count} serde template"
  - phase: 07-spine-rollout
    provides: "the frozen per-command Serialize output structs (uuid/epoch/color/json/passgen) the new fields ride on"
provides:
  - "uuid --v7 (Uuid::now_v7) + four wrapping forms (--no-hyphens/--braces/--urn) with the --braces/--urn clap conflict (exit 2)"
  - "epoch always-on relative time (hand-rolled relative_for ladder) + validated --tz <IANA> third line (chrono-tz)"
  - "color CSS named-color resolution both directions (148-entry table + redmean nearest) + CSS-functional hsl(...) input"
  - "json opt-in --sort-keys recursive key sort (preserve_order stays default)"
  - "passgen entropy_bits estimate (STDERR/TTY-gated + JSON field) + --no-similar + --separator"
affects: [09-02-visuals, 09-03-lolcat-animate, 11-meta-commands-completions]

# Tech tracking
tech-stack:
  added: ["chrono-tz 0.10 (IANA tz DB)", "uuid v7 feature"]
  patterns:
    - "additive-only field growth on a frozen #[derive(Serialize)] struct → both human + JSON from one source (no-drift)"
    - "pure unit-testable seam per behavior (Form/format_one, relative_for, hsl_to_rgb, nearest_name, sort_value, entropy_bits)"
    - "secret/estimate channel split: entropy → STDERR (TTY-gated) | JSON field, never stdout (D-14)"

key-files:
  created: []
  modified:
    - "Cargo.toml (uuid v7 feature + chrono-tz)"
    - "src/commands/uuid/mod.rs"
    - "src/commands/passgen/mod.rs"
    - "src/commands/epoch/mod.rs"
    - "src/commands/json/mod.rs"
    - "src/commands/color/mod.rs"
    - "tests/{uuid,passgen,epoch,json,color}.rs"
    - "tests/cmd/color.trycmd"

key-decisions:
  - "uuid Form precedence braces>urn>no-hyphens>plain (crate has no simple-braced/simple-urn form); --upper composes as a post-pass on any form"
  - "epoch relative suffix lives ONLY in the integer human branch + JSON relative field; now/date-string stay bare integers; --tz line + JSON tz are integer/JSON only"
  - "passgen entropy is STDERR + TTY-gated (keeps the existing success_writes_nothing_to_stderr contract green under the piped harness); --no-similar shrinks the entropy pool"
  - "color hsl( prefix routed BEFORE the RGB branch (char-boundary-safe .get(..4)); CSS-name lookup AFTER hex, BEFORE bail! (no all-hex 3/6 name collision); exact_name+nearest_name agree on an exact color"

patterns-established:
  - "Hand-rolled pure helpers over crates per CONTEXT.md ethos: relative_for ladder, hsl_to_rgb inverse, redmean nearest, recursive sort_value, entropy_bits"
  - "CSS_NAMES const table (148 CSS Color Module Level 4 keywords incl. rebeccapurple) with deterministic first-match for shared-RGB aliases"

requirements-completed: [UUID-V2-01, EPOC-V2-01, COLR-V2-01, JSON-V2-01, PASS-V2-01]

# Metrics
duration: 28min
completed: 2026-06-28
---

# Phase 9 Plan 01: Dev-Transform Depth Summary

**Five additive depth flags across uuid/epoch/color/json/passgen — uuid v7 + four wrapping forms, epoch always-on relative time + `--tz`, color CSS names (both ways) + HSL input, json `--sort-keys`, passgen entropy + `--no-similar` + `--separator` — each riding the frozen Phase-7 Serialize struct into both human and JSON output (no-drift), plus two additive Cargo.toml edits.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-06-28T19:24:22Z (phase-begin base)
- **Completed:** 2026-06-28T19:52:33Z
- **Tasks:** 3
- **Files modified:** 13 (6 source modules incl. Cargo.toml, 5 integration tests, 1 trycmd snapshot, Cargo.lock)

## Accomplishments
- **uuid (UUID-V2-01):** `--v7` (`Uuid::now_v7`) + `--no-hyphens`/`--braces`/`--urn` via a private `Form` enum; `--braces` conflicts_with `--urn` (clap exit 2); `format_one(u, form, upper)` single-sources the human line and JSON `uuid`; `version` is `"v4"`/`"v7"`.
- **epoch (EPOC-V2-01):** hand-rolled `relative_for` threshold ladder (just now / N min|hr|day|week|month|year, past `ago` + future `in N`) appended to the integer Local+UTC lines; validated `--tz` (chrono-tz `Tz::from_str`, bad zone → exit 1 + IANA hint) adds a third `%Z %z` line; JSON gains always-present `relative` + conditional `tz`.
- **color (COLR-V2-01):** `hsl(H,S%,L%)` (comma + space forms) routed before the RGB branch; 148-entry CSS table both directions; `exact_name`/`nearest_name` (redmean) feed always-present JSON `name`(exact|null) + `nearest`; human Name row (bare exact | `~nearest`); hex stays lowercase-locked (D-19).
- **json (JSON-V2-01):** opt-in `--sort-keys` recursive `sort_value` applied before the `is_json_on()` fork (arrays keep order); `preserve_order` stays the default.
- **passgen (PASS-V2-01):** `entropy_bits` helper → top-level JSON field + STDERR (TTY-gated) human summary; `--no-similar` prunes `il1Lo0O` and shrinks the pool; `--separator` overrides the passphrase join. RNG (`OsRng` + `.choose`) untouched (T-V6).
- **Manifest:** `uuid` `v7` feature + `chrono-tz = "0.10"` — the only manifest edits.

## Task Commits

Each task was committed atomically:

1. **Task 1: uuid v7 + wrapping forms; passgen entropy + --no-similar + --separator** - `167815a` (feat)
2. **Task 2: epoch relative time + --tz (chrono-tz); json --sort-keys** - `67e93be` (feat)
3. **Task 3: color CSS named-color (both directions) + HSL input** - `3a1d21f` (feat)

_TDD process per task: pure seams + tests authored together with the implementation and verified RED→GREEN via `cargo test --bin box <seam>`; committed atomically per task once green (Rust inline `#[cfg(test)]` tests share the source file, so a separate non-compiling test commit was not introduced)._

## Files Created/Modified
- `Cargo.toml` - uuid `["v4","v7"]` feature; added `chrono-tz = "0.10"`
- `src/commands/uuid/mod.rs` - `Form` enum, `--v7`/`--no-hyphens`/`--braces`/`--urn`, widened `format_one`
- `src/commands/passgen/mod.rs` - `--no-similar`/`--separator`, `drop_similar`/`entropy_bits` helpers, `entropy_bits` field, STDERR entropy
- `src/commands/epoch/mod.rs` - `relative_for`/`parse_tz`/`format_tz_line`, `--tz`, `relative`/`tz` fields
- `src/commands/json/mod.rs` - `sort_value` recursive sort, `--sort-keys`
- `src/commands/color/mod.rs` - `parse_hsl`/`hsl_to_rgb`, `CSS_NAMES` table, `name_to_rgb`/`exact_name`/`nearest_name`, `name`/`nearest` fields, Name row
- `tests/{uuid,passgen,epoch,json,color}.rs` - per-flag unit + integration coverage
- `tests/cmd/color.trycmd` - updated for the new `Name  : ~dodgerblue` row

## Decisions Made
- **uuid Form precedence** `braces > urn > no-hyphens > plain` — the crate exposes no "simple braced"/"simple urn" form, so a wrapping form wins over `--no-hyphens`; `--upper` is an orthogonal post-pass that composes with any form.
- **epoch relative scope** — relative suffix and the `--tz` third line live in the integer human branch only; `now`/date-string modes stay bare integers (scripting-clean), while the unified JSON always carries `relative` (and `tz` under `--tz`). `--tz` is validated once, before any output, so a bad zone fails identically in both modes.
- **passgen entropy on STDERR is TTY-gated** (exactly like the clip confirmation). Under the piped `assert_cmd` harness stderr is not a TTY, so the existing `success_writes_nothing_to_stderr` contract stays green; the integration coverage asserts the load-bearing property (entropy never on stdout) + the `--json` `entropy_bits` field.
- **color routing/ordering** — `hsl(` checked before the RGB separator branch (char-boundary-safe `.get(..4)`, case-insensitive); CSS-name lookup after the hex check and before the final `bail!` (no all-hex length-3/6 name collides). `exact_name` and `nearest_name` both first-match in table order so they agree on an exact color.

## Deviations from Plan

None - plan executed exactly as written. (One in-scope housekeeping the plan implied: the `tests/cmd/color.trycmd` snapshot was updated to reflect the new human Name row — the plan adds the row, so the snapshot regenerate is part of that change, not a scope deviation.)

## Issues Encountered
None. All pure seams passed RED→GREEN on first implementation; the full suite (190 unit + every integration file) and `cargo clippy --all-targets -- -D warnings` are green.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The five dev-transform requirements (UUID/EPOC/COLR/JSON/PASS-V2) are complete, tested, and clippy-clean.
- Plan 09-02 (visuals: matrix/qr/ascii) and 09-03 (lolcat `--animate`, human-verify) are independent of these modules and unblocked.
- No blockers. The phase gate for this plan (`cargo test` green for uuid/epoch/color/json/passgen + clippy `--all-targets -D warnings`) is satisfied.

## Self-Check

- Created files: none (plan modifies existing modules only).
- Commits verified present: `167815a`, `67e93be`, `3a1d21f` (see below).

---
*Phase: 09-dev-transform-visual-depth*
*Completed: 2026-06-28*
