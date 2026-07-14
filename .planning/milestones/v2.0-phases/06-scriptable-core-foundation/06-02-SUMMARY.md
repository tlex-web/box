---
phase: 06-scriptable-core-foundation
plan: 02
subsystem: api
tags: [rust, serde, serde_json, json, clipboard, blake3, sha256, config, spine, uuid, hash]

# Dependency graph
requires:
  - phase: 06-01
    provides: core::output spine (is_json_on/emit_json/out_line/flush_clip + JSON_ON/CLIP_ON/CLIP_BUF), core::config (config()/Config{default_hash_algo}, resolve_algo), BoxError::Config exit-2, global --json/--clip flags, Algo: Deserialize+rename_all=lowercase, reserved BOX_HASH_DEFAULT_ALGO env slot
provides:
  - uuid as first --json/--clip spine consumer (UuidRow{uuid,version} + UuidOutput{results,count}; is_json_on() fork; out_line human path)
  - hash as second spine consumer (HashRow{path,algo,digest} + HashOutput{results,count}; is_json_on() fork; out_line two-space coreutils row)
  - BLAKE3 COMPUTE-default flip (HASH-V2-01/D-04) with full CLI > env (BOX_HASH_DEFAULT_ALGO) > config (default_hash_algo) > builtin Blake3 precedence
  - Algo: Serialize (lowercase) for --json; parse_algo() shared env+config string->Algo via ValueEnum::from_str
  - D-05 BLAKE3-fallback verify probe (decisive when blake3 matches the file, static otherwise; suppressed under --json; exit stays 1)
  - the frozen Phase-7 JSON-purity + clip-capture + BLAKE3-default + config-precedence test template (tests/uuid.rs, tests/hash.rs, tests/config.rs)
affects: [07-spine-rollout, 08-filesystem-depth, 09-dev-transform-visual-depth, 11-meta-commands]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One #[derive(Serialize)] struct ({Row}/{Output}) feeds BOTH human and JSON paths — the pure renderer fills it, is_json_on() forks (no-drift, Pattern 2)"
    - "always-wrapped {results:[...],count:N} even for N=1 (object, never a bare array — Phase-8 multi-item compatible, D-01/D-02)"
    - "compute-default precedence by construction: self.flag.or_else(env).or(config().field).unwrap_or(builtin) (CLI > env > config > builtin, SPINE-05)"
    - "one ValueEnum::from_str parse helper (parse_algo) shared by env + config string->Algo"
    - "single-pass-reader probe: capture the path string BEFORE read_file_or_stdin consumes self.path, re-open for a second hash; degrade to static hint when source is stdin"

key-files:
  created: []
  modified:
    - src/commands/uuid/mod.rs
    - src/commands/hash/mod.rs
    - src/core/output.rs
    - src/core/config.rs
    - tests/uuid.rs
    - tests/hash.rs
    - tests/config.rs

key-decisions:
  - "Env-var spelling: BOX_HASH_DEFAULT_ALGO (the slot reserved in 06-01); parsed via parse_algo() = Algo::from_str(s, true).ok() (clap ValueEnum, case-insensitive), reused for both env and config so the spelling table is single-sourced"
  - "D-05 re-open path-capture mechanism: clone the path string from self.path.as_deref() into path_for_probe BEFORE read_file_or_stdin(self.path) consumes it; Some only for a real path (p != \"-\"), None for stdin"
  - "D-05 hint wording (Claude's Discretion): decisive = 'hint: the digest does not match as sha256, but it MATCHES this file's blake3 — re-run with `--algo blake3`'; static = 'hint: the default hash algorithm is now blake3 — pass `--algo blake3` if this is a blake3 digest'. Both stderr-only; `--algo blake3` styled yellow when is_color_on()"
  - "emit_blake3_probe_hint is a free fn (not inlined) so the re-open + decisive/static branch is self-contained and the run() match stays readable"
  - "Removed the forward-compat #[allow(dead_code)] on is_json_on/emit_json/out_line/config now that uuid+hash consume them live (allow-then-remove, honoring the 06-01 hand-off); resolve_algo KEEPS its allow (still only exercised by its unit test — hash uses an inline .or() chain, not resolve_algo)"
  - "Updated hash/mod.rs module doc + the --algo arg help to announce the BLAKE3-default breaking change loudly (the STATE pitfall 'loud --help note')"

patterns-established:
  - "JSON-purity test template: serde_json::from_slice over the WHOLE stdout buffer (succeeds only for exactly one JSON value) + .results/.count shape + no 0x1B + first 3 bytes != EF BB BF — the copy-me regression backstop for all 23 Phase-7 commands"
  - "#[ignore]d live-clipboard round-trip mirroring tests/clip.rs (run via `--ignored --test-threads=1`) as the headless-unsafe SPINE-03 proof"
  - "verify-regression backstop: tests/hash.rs::hash_verify_autodetect MUST pass UNCHANGED after the flip; if it needs editing that is the red flag (algo_from_len byte-for-byte unchanged)"

requirements-completed: [SPINE-01, SPINE-03, HASH-V2-01]

# Metrics
duration: ~35 min
completed: 2026-06-25
---

# Phase 6 Plan 02: uuid + hash Spine Pilots Summary

**`uuid` and `hash` become the first `--json`/`--clip` consumers of the wave-1 spine — `{results,count}` serde structs feeding both human and JSON paths, the `hash` COMPUTE default flipped to BLAKE3 with a live CLI>env>config>builtin precedence escape hatch, and the D-05 BLAKE3-fallback verify probe — with the SHA-256 verify length table byte-for-byte intact.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-25T00:30Z (approx)
- **Completed:** 2026-06-25T01:05Z (approx)
- **Tasks:** 2 (both `tdd="true"`)
- **Files modified:** 7 (0 created, 7 modified)

## Accomplishments

- **`uuid` as the first spine consumer** — `UuidRow{uuid,version}` + `UuidOutput{results,count}` serde structs; `run()` builds rows once (the pure `format_one` renderer feeding both paths — no drift), then forks on `is_json_on()` FIRST: one `{results,count}` document via `emit_json`, else one UUID per line via `out_line` (so `--clip` tees). Always-wrapped object even for N=1.
- **`hash` as the second spine consumer + BLAKE3-default flip** — `HashRow{path,algo,digest}` + `HashOutput{results,count}`; the no-verify branch now resolves `self.algo.or_else(BOX_HASH_DEFAULT_ALGO).or(config().default_hash_algo).unwrap_or(Blake3)` (D-04 breaking flip + SPINE-05 + the env tier wired live), forks on `is_json_on()`, and keeps the two-space coreutils `<hash>  <label>` row through `out_line`.
- **`Algo` round-trips both ways** — added `serde::Serialize` (Deserialize landed 06-01) with the lowercase rename, plus `parse_algo()` (one `ValueEnum::from_str` helper shared by env + config).
- **D-05 BLAKE3-fallback verify probe** — on a 64-hex `--verify` mismatch with no explicit `--algo` (and not under `--json`), the probe re-opens the file (the streaming reader is single-pass) and computes BLAKE3: a DECISIVE `--algo blake3` hint when blake3 matches the file, a STATIC transitional hint otherwise (and for piped stdin, which cannot be re-read). stderr-only, exit STAYS 1.
- **The verify backstop held** — `algo_from_len` (64→sha256) is byte-for-byte unchanged and `tests/hash.rs::hash_verify_autodetect` passes UNCHANGED; no stored SHA-256 baseline silently breaks (D-04, the #1 v2 data-risk).
- **The frozen Phase-7 test template** — `tests/uuid.rs` (`json_purity`, `json_count_multi`, `human_output_unchanged`, `#[ignore]`d `clip_roundtrip`), `tests/hash.rs` (`default_is_blake3`, `json_shape`, `json_error_empty_stdout`, `verify_blake3_probe_hint`), `tests/config.rs` (full `hash_default_override` precedence round-trip). Full suite green across 25 binaries; `clippy --all-targets -D warnings` + `fmt --check` clean.

## Task Commits

Each task was committed atomically (TDD: failing test + minimal implementation form one compiling unit per task):

1. **Task 1: uuid as first --json/--clip spine consumer** — `498dc8f` (feat)
2. **Task 2: hash BLAKE3-default flip + Algo serde + --json/--clip + D-05 probe** — `4a13d77` (feat)

**Plan metadata:** appended in the final docs commit.

## Files Created/Modified

- `src/commands/uuid/mod.rs` — `UuidRow`/`UuidOutput` structs; `run()` builds rows then forks on `is_json_on()` (emit_json | out_line); pure `format_one` feeds both paths.
- `src/commands/hash/mod.rs` — `Algo: Serialize`; `parse_algo` (env+config); `HashRow`/`HashOutput`; compute-default precedence chain flipped to BLAKE3; `is_json_on()` fork; `emit_blake3_probe_hint` (D-05 re-open probe); module doc + `--algo` help announce the breaking default. `algo_from_len` UNCHANGED.
- `src/core/output.rs` — removed forward-compat `#[allow(dead_code)]` on `is_json_on`/`emit_json`/`out_line` (now live consumers).
- `src/core/config.rs` — removed forward-compat `#[allow(dead_code)]` on `config()` (now read live by `hash`); `resolve_algo` keeps its allow (test-only).
- `tests/uuid.rs` — `json_purity` (the copy-me JSON-purity template), `json_count_multi`, `human_output_unchanged`, `#[ignore]`d `clip_roundtrip`.
- `tests/hash.rs` — flipped `hash_default_sha256` → `default_is_blake3` (+ explicit `--algo sha256` → SHA-256), flipped `hash_stdin_dash_label` to BLAKE3; added `json_shape`, `json_error_empty_stdout`, `verify_blake3_probe_hint`. `hash_verify_autodetect` UNCHANGED.
- `tests/config.rs` — replaced the 06-01 placeholder `hash_default_override` with the full precedence round-trip (config sha256 → SHA-256 row; CLI `--algo blake3` → BLAKE3 row).

## Decisions Made

- **Env-var spelling = `BOX_HASH_DEFAULT_ALGO`** (the 06-01-reserved slot), parsed via `parse_algo(s) = Algo::from_str(s, true).ok()` — clap's `ValueEnum` parser, case-insensitive, reused for BOTH env and config so the `sha256`/`blake3`/`sha512`/`md5` spelling table is single-sourced. An unrecognized env value returns `None` and falls through (never errors a normal `box hash`).
- **D-05 path-capture mechanism = clone `self.path` into `path_for_probe` BEFORE `read_file_or_stdin(self.path)` consumes it.** `path_for_probe` is `Some(p)` only for a real path (`p != "-"`), `None` for stdin — which is exactly the "probe only when a real path is available" precondition. The re-open uses `read_file_or_stdin(Some(path))` + `digest_reader(Algo::Blake3, ...)`.
- **D-05 hint wording (Claude's Discretion):**
  - decisive (blake3 matches the file): `hint: the digest does not match as sha256, but it MATCHES this file's blake3 — re-run with `--algo blake3``
  - static (no match / stdin): `hint: the default hash algorithm is now blake3 — pass `--algo blake3` if this is a blake3 digest`
  - Both stderr-only; the `--algo blake3` token is styled `.yellow()` when `is_color_on()`, plain otherwise (byte-identical minus ANSI). Suppressed entirely under `--json` (D-09).
- **`emit_blake3_probe_hint` is a free fn**, not inlined, so the re-open + decisive/static branch stays self-contained and the `run()` match remains readable.
- **Removed forward-compat allows** on `is_json_on`/`emit_json`/`out_line`/`config` (allow-then-remove, honoring the 06-01 hand-off) — they are now live consumers. `resolve_algo` KEEPS its allow because `hash` resolves with an inline `.or()` chain, not `resolve_algo` (the inline form is what the plan specified at line 162).
- **Loud breaking-change doc** — updated `hash/mod.rs` module doc and the `--algo` arg help to announce the BLAKE3 COMPUTE-default flip and the `--algo sha256` / config escape hatch (the STATE "loud `--help` note" pitfall). No `tests/cmd/*.trycmd` snapshot captures `box hash --help`, so no snapshot churn.

### Final D-02 pilot JSON literals (as emitted — the verified shape Phase 7 copies)

`box uuid --json`:
```json
{
  "results": [
    {
      "uuid": "89303d1f-73e0-4e26-a578-3d0527810e94",
      "version": "v4"
    }
  ],
  "count": 1
}
```

`box hash <file> --json` (no `--algo`, no config → BLAKE3 default):
```json
{
  "results": [
    {
      "path": "C:/Users/tim/AppData/Local/Temp/box.bin",
      "algo": "blake3",
      "digest": "095dfefdedb7f0870e801730da35823caaa8e969078e53b6e262c66f1a5b1c1e"
    }
  ],
  "count": 1
}
```

(`box uuid -n N --json` repeats the `{uuid,version}` element N times with `"count": N`. `--clip`/`--json --clip` route through `out_line`/`emit_json` respectively, teeing into `CLIP_BUF` for `flush_clip`.)

### Verify-backstop confirmation

`tests/hash.rs::hash_verify_autodetect` passed **UNCHANGED** (not edited). `algo_from_len` is byte-for-byte unchanged (64→sha256). A bare `box hash --verify <64-hex sha256> file` still verifies as SHA-256 (exit 0); a 64-hex BLAKE3 value verified with no `--algo` mismatches as sha256 (exit 1) with the decisive D-05 hint.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated stale `hash` documentation to announce the BLAKE3 breaking change**
- **Found during:** Task 2 (BLAKE3-default flip)
- **Issue:** The `hash/mod.rs` module doc opened with "SHA-256 (default)" and the `--algo` arg help said "Unset means sha256 when computing" — both stale and actively misleading after the D-04 flip. STATE's v2 pitfall mandates a "loud `--help`/PROJECT note" for the breaking change so users discover the `--algo sha256` / config escape hatch.
- **Fix:** Rewrote the module doc to lead with the BLAKE3-default + the full precedence + the verify-table-unchanged invariant + the D-05 hint; updated the `--algo` help to state the BLAKE3 default and name both escape hatches. No behavior change.
- **Files modified:** src/commands/hash/mod.rs
- **Verification:** No `tests/cmd/*.trycmd` snapshot captures `box hash --help` (only top-level `box --help`); full suite + clippy + fmt clean.
- **Committed in:** `4a13d77` (Task 2 commit)

**2. [Rule 1 - Bug] Flipped `hash_stdin_dash_label` expectation to BLAKE3**
- **Found during:** Task 2 (BLAKE3-default flip)
- **Issue:** The plan's RED list named `hash_default_sha256` as the one test whose expectation flips, but `hash_stdin_dash_label` (piped stdin, no `--algo`) ALSO asserted the SHA-256 vector and would break under the new default — it is a second legitimate default-path assertion, not a regression.
- **Fix:** Updated its expected row from `BOX_SHA256  -` to `BOX_BLAKE3  -` and refreshed its doc to note the v2 default flip.
- **Files modified:** tests/hash.rs
- **Verification:** `cargo test --test hash` fully green (11 passed).
- **Committed in:** `4a13d77` (Task 2 commit)

**3. [Rule 3 - Blocking] `clippy::doc_lazy_continuation` on the rewritten `hash_default_override` doc**
- **Found during:** Task 2 (clippy `-D warnings` gate)
- **Issue:** The new doc comment ended a markdown list with a non-indented "Runnable via …" line, which clippy parses as a lazy list continuation — a `-D warnings` failure that blocked the build.
- **Fix:** Inserted a blank `///` line before the "Runnable via" line (paragraph break).
- **Files modified:** tests/config.rs
- **Verification:** `clippy --all-targets -D warnings` exits 0.
- **Committed in:** `4a13d77` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 Rule 2 missing-critical doc, 1 Rule 1 second-default-test flip, 1 Rule 3 clippy-gate). 
**Impact on plan:** All three are mechanical correctness fixes flowing directly from the planned BLAKE3 flip — the stale doc and the second default-path test both HAD to move with the default, and the clippy nit is a CI-gate formality. No scope creep; the spine shape and the four ROADMAP success criteria are exactly as specified.

## Known Stubs

None. Both pilots are fully wired: `uuid`/`hash` emit real `{results,count}` documents from live data, `--clip` tees through `out_line`/`emit_json`, and the BLAKE3 default + config/env precedence is end-to-end functional. `resolve_algo` retains a forward-compat `#[allow(dead_code)]` because `hash` deliberately uses the inline `.or()` chain (per the plan's line-162 spec) rather than calling `resolve_algo`; it remains exercised by its `precedence_matrix` unit test and is available for any future caller — this is a documented forward-compat allow, not a stub.

## Threat Flags

None. No new security surface beyond the plan's `<threat_model>`. The four mitigations are in place: T-06-05 (CLI `--algo` beats config — `hash_default_override` proves CLI blake3 beats config sha256), T-06-06 (64→sha256 verify table unchanged — `hash_verify_autodetect` UNCHANGED), T-06-07 (D-05 probe stderr-only, suppressed under `--json`, re-reads only a real path, never echoes contents), T-06-08 (the `path` field is a serde-serialized String via `emit_json`, never raw-printed). T-06-09 (streaming, no whole-file buffering) preserved — the D-05 re-open streams a second pass via `read_file_or_stdin` + `digest_reader`.

## Issues Encountered

- `cargo test --test hash a b c` (multiple positional filters, as the plan's `<verify>` block lists them space-separated) is rejected by cargo — only one positional `TESTNAME` substring is accepted. Ran the whole test binary (`cargo test --test hash`) per filter group instead; no behavior impact.

## User Setup Required

None — no external service configuration required. (Optional: a user may set `default_hash_algo = "sha256"` in `%APPDATA%\box\config.toml`, or export `BOX_HASH_DEFAULT_ALGO=sha256`, to restore the v1 SHA-256 default — but no setup is required for the tool to work.)

## Next Phase Readiness

- **Phase 6 is COMPLETE** (both waves). The spine is built (06-01) and proven end-to-end on the two cheapest commands (06-02). SPINE-01, SPINE-03, and HASH-V2-01 are established/contract-tested on the pilots.
- **Phase 7 (Spine Rollout, SPINE-02/SPINE-04) is ready.** The frozen copy-me template is in place:
  - the `{Row}/{Output}` serde-struct + `is_json_on()` fork + `out_line` routing pattern (uuid is the cleanest reference, hash adds the `path`-bearing variant);
  - the JSON-purity test (`tests/uuid.rs::json_purity`) + the `#[ignore]`d clip round-trip — copy per command;
  - `emit_json`/`out_line`/`is_json_on` are live (allows removed); `config().<field>` is the live config-tier accessor; `resolve_algo`/the inline `.or()` chain are both available for any new config-overridable flag.
- **No blockers.** Full test suite green (25 binaries), clippy `-D warnings` clean, fmt clean.

## Self-Check: PASSED

- FOUND: .planning/phases/06-scriptable-core-foundation/06-02-SUMMARY.md
- FOUND: commit 498dc8f (Task 1 — uuid spine consumer)
- FOUND: commit 4a13d77 (Task 2 — hash BLAKE3 flip + spine + D-05 probe)

---
*Phase: 06-scriptable-core-foundation*
*Completed: 2026-06-25*
