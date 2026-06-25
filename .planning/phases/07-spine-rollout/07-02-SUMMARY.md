---
phase: 07-spine-rollout
plan: 02
subsystem: cli
tags: [serde, serde_json, json-output, spine, filesystem, du, tree, dupes, flatten, bulk-rename]

# Dependency graph
requires:
  - phase: 07-spine-rollout (plan 01)
    provides: "frozen {Row}/{Output} serde + is_json_on() fork + out_line routing template; json_purity test template; D-3/D-4/D-9/D-11 conventions proven on the pure transforms"
provides:
  - "--json on the 5 Wave-7b filesystem commands (du, tree, dupes, flatten, bulk-rename) — SPINE-02 partial (13 of 16 cumulative)"
  - "du {results:[{name,is_dir,size}],count,total_bytes,total_children} from the existing buffered Row model (D-11)"
  - "tree recursive {name,type,size?,children:[]} via a NEW build_node recursion (A4 resolved, D-17 root-rule exception)"
  - "dupes {results:[{size,paths}],count,wasted_bytes} with paths serialized lossily (D-4, D-17)"
  - "flatten/bulk-rename {results:[{src,dst,action,reason}],count,dry_run,…} D-13 plan projection orthogonal to --force (D-12)"
  - "bulk-rename --force --json emits applied rows (D-12 override) AND its abort path keeps stdout byte-empty (A3/D-09 resolved)"
affects: [07-03-json-qr-weather, 08-filesystem-depth]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Buffered-model → {results,count} projection: a command that already collects+sorts an internal Vec just adds #[derive(Serialize)] (du Row) or a lossy projection row (dupes DupeRow) and forks (D-11)"
    - "Recursive node tree (tree Node + build_node) sharing the printer's read_children/sort_children so JSON order == human order (no-drift, A4)"
    - "Plan projection with dry_run bool: one Plan feeds both the JSON document and the human render; --json is orthogonal to --force/dry-run (D-12/D-13)"
    - "Abort-empty-stdout fork: an error path guards its stdout print behind `if !is_json_on()` so a failed --json run leaves stdout byte-empty, error to stderr, exit 1 (A3/D-09)"
    - "--json suppresses per-row human prints in execute loops via a captured `let json = is_json_on()` guard, emitting one document after the loop"

key-files:
  created: []
  modified:
    - "src/commands/du/mod.rs"
    - "src/commands/tree/mod.rs"
    - "src/commands/dupes/mod.rs"
    - "src/commands/flatten/mod.rs"
    - "src/commands/bulk_rename/mod.rs"
    - "tests/du.rs"
    - "tests/tree.rs"
    - "tests/dupes.rs"
    - "tests/flatten.rs"
    - "tests/bulk_rename.rs"

key-decisions:
  - "du sibling root fields = total_bytes (full-scan sum) + total_children (full child count), both computed BEFORE --top truncation so they stay full-scan; --top truncates .results too (honors the user's intent), but the totals do not"
  - "tree --depth honored in build_node exactly like render_dir: a directory AT the cap depth still appears as a node but its children one level deeper are not descended (descend = depth <= max)"
  - "dupes paths serialized via p.to_string_lossy().into_owned() in a dedicated DupeRow projection (DupeGroup keeps PathBuf for the human render); count = number of groups, wasted_bytes = the existing wasted_space(&groups)"
  - "flatten/bulk-rename action = lowercased RowStatus via a shared action_str() helper that reuses kind.status() (the same source of truth the human glyph derives from); the raw fields are serialized, NEVER format_row output"
  - "bulk-rename --force --json results = the WHOLE plan projection (renames + skips), so it is non-empty even when some rows are skips; the human --force path stays silent-on-success"
  - "A3/D-09 abort: the conflict path's print_plan_with_conflicts is guarded behind `if !is_json_on()`; the bail! error (already → stderr, exit 1 via main.rs) is the only output under --json. No {\"error\":…} envelope on stdout"

patterns-established:
  - "json_purity per Wave-7b command (copied from tests/uuid.rs:135): one JSON value, schema shape, no 0x1B, no BOM"
  - "tree json_recursive_shape: asserts root .type==dir omits .size, a nested file node has .type==file + numeric .size"
  - "flatten/bulk-rename json_dry_run / json_force_run|emits_rows: assert the dry_run bool flips with --force and the real-run counts are captured"
  - "bulk-rename json_abort_empty_stdout: a conflicting plan under --json exits 1 with byte-EMPTY stdout (the D-09 backstop)"

requirements-completed: [SPINE-02]

# Metrics
duration: 12min
completed: 2026-06-25
---

# Phase 7 Plan 02: Wave-7b Filesystem Spine Rollout Summary

**The frozen `--json` spine now spans the 5 filesystem commands — du/dupes project their existing buffered models into `{results,count(,wasted_bytes)}`, tree gained a REAL recursive `build_node` node tree (the A4 surprise), and flatten/bulk-rename carry a `dry_run` boolean with bulk-rename's `--force --json` emitting applied rows while its abort path keeps stdout byte-empty (A3/D-09).**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-25T13:45:10Z
- **Completed:** 2026-06-25T13:57:23Z
- **Tasks:** 3 (all TDD)
- **Files modified:** 10 (5 command modules + 5 test files)

## Accomplishments
- 5 Wave-7b commands accept `--json` and emit exactly one parseable JSON document (SPINE-02, 13 of 16 cumulative): du `{results,count,total_bytes,total_children}`, dupes `{results,count,wasted_bytes}`, tree recursive `{name,type,size?,children}`, flatten/bulk-rename `{results,count,dry_run,…}`.
- **A4 resolved:** tree now builds a genuine recursive `Node` tree via the new `build_node` recursion that reuses the printer's `read_children`/`sort_children` so JSON order can never drift from the human render — the current flat printing recursion was untouched.
- **A3 resolved:** bulk-rename's conflict/abort path under `--json` keeps stdout byte-EMPTY (the `print_plan_with_conflicts` is guarded behind `!is_json_on()`); the `bail!` error reaches the user via stderr, exit 1 — backed by the `json_abort_empty_stdout` regression (T-07b-02 mitigation).
- **D-12 override:** `bulk-rename --force --json` emits the applied rename rows even though the human `--force` path stays silent-on-success.
- Full Wave-7b gate green: all integration suites (`cargo test`) + 157 unit tests (`cargo test --bin box`); clippy clean.

## Task Commits

Each task was committed atomically (TDD: test RED → feat GREEN):

1. **Task 1: du + dupes (buffered models → {results,count[,wasted_bytes]})** — `57dacd4` (test, RED) → `a8a4a75` (feat, GREEN)
2. **Task 2: tree recursive node builder (A4)** — `14cc499` (test, RED) → `6c1dbeb` (feat, GREEN)
3. **Task 3: flatten + bulk-rename plan projection + abort fork (A3) + Wave-7b gate** — `6fccd65` (test, RED) → `5c68af9` (feat, GREEN)

**Plan metadata:** (docs commit — this SUMMARY + STATE/ROADMAP/REQUIREMENTS)

## Files Created/Modified
- `src/commands/du/mod.rs` — `#[derive(Serialize)]` on `Row`; `DuOutput{results,count,total_bytes,total_children}`; is_json_on fork FIRST, all three human writes (rows/blank/summary) behind `else`; bare-u64 size (D-3); `--top` honored on `.results`, totals stay full-scan
- `src/commands/dupes/mod.rs` — `DupeRow`/`DupesOutput{results,count,wasted_bytes}`; paths via `to_string_lossy().into_owned()` (D-4); empty case → `{results:[],count:0,wasted_bytes:0}`
- `src/commands/tree/mod.rs` — NEW `Node{name,type,size?,children}` serde struct (`type` renames `kind` to dir/file, `size` `skip_serializing_if=Option::is_none` for files-only) + NEW `build_node` recursion sharing read_children/sort_children, honoring `--depth`; is_json_on fork (root-rule exception, recursive object)
- `src/commands/flatten/mod.rs` — `FlattenRow{src,dst,action,reason}` + `FlattenOutput{…,dry_run,copied,renamed,skipped,total_bytes}`; `action_str()` lowercases RowStatus; fork on BOTH dry-run (plan) and real (executed) branches
- `src/commands/bulk_rename/mod.rs` — `RenameRow`/`RenameOutput` same shape; D-12 override on the `--force` success path; A3 abort-path guard behind `!is_json_on()`
- `tests/du.rs`, `tests/dupes.rs` — `json_purity` (+ dupes asserts paths are strings, wasted_bytes present)
- `tests/tree.rs` — `json_purity` + `json_recursive_shape`
- `tests/flatten.rs` — `json_purity` + `json_dry_run` + `json_force_run`
- `tests/bulk_rename.rs` — `json_purity` + `json_dry_run` + `json_force_emits_rows` + `json_abort_empty_stdout`

## Decisions Made
- **du sibling totals (discretion):** `total_bytes` and `total_children` are the already-computed full-scan `total`/`rows.len()` captured before `--top`, so they stay full-scan exactly like the human summary line; `--top` still truncates `.results` to honor the user's intent.
- **tree `--depth` semantics:** `build_node` mirrors `render_dir`'s boundary — `descend = depth <= max` — so a directory AT the cap depth is present as a node with empty `children`, never an over-deep descent.
- **dupes projection:** a dedicated `DupeRow` carries lossy `Vec<String>` paths; `DupeGroup` keeps its `PathBuf` for the human `path.display()` render (both lossy-safe, no drift). `count` = group count, `wasted_bytes` = the existing `wasted_space(&groups)`.
- **action enum spelling (discretion):** a shared `action_str()` maps `kind.status()` → `"copy"`/`"rename"`/`"skip"` in each of flatten and bulk-rename, reusing the same `RowStatus` source of truth the human glyph derives from — no separate serde enum was needed since the projection is a `&'static str`.
- **bulk-rename `--force --json` rows:** the emitted `.results` is the whole plan projection (renames + skips), guaranteeing non-empty output for an applied run while leaving the human `--force` path silent-on-success.
- **A3 abort under --json:** stdout stays byte-empty; the conflict detail rides the existing `bail!` → stderr → exit-1 path (no new error envelope). This satisfies D-09 and is locked by `json_abort_empty_stdout` (tested under both dry-run and `--force`).

## Deviations from Plan

None — plan executed exactly as written. All locked decisions (D-11, D-12, D-13, D-17, A3, A4) and the discretion field names were applied as specified; the human render paths stayed byte-stable (verified by the unchanged du/dupes/tree/flatten/bulk-rename human integration tests passing). The one clippy `doc_lazy_continuation` warning was in a test doc comment I added during Task 3 and was reworded in the same Task-3 GREEN commit (lint hygiene on new code, not a behavioral change).

## Known Stubs

None — every `--json` document is wired to the command's real computed data (du's buffered+sorted rows, dupes' grouped output, tree's live recursion, flatten/bulk-rename's actual Plan). No hardcoded empties, placeholders, or unwired data sources were introduced.

## Issues Encountered
None. The four surprises this wave was designed around (A3 bulk-rename abort, A4 tree recursion) were anticipated by RESEARCH/PATTERNS and resolved as planned; the buffered-model commands (du/dupes/flatten/bulk-rename) projected cleanly because the data models already existed.

## Authentication Gates
None — Phase 7 installs zero packages and touches no external service. The filesystem commands read/write only the user-supplied temp fixtures in tests; no clipboard (`--clip`) work was in scope for this wave (none of these 5 are SPINE-04).

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- 13 of 16 commands now carry `--json`; the remaining 3 (json, qr, weather) are 07-03 — the odd-fits (json identity passthrough D-16, qr metadata + the `clip_feed` primitive D-14/D-15, weather f64/unit-from-API). The buffered/recursive patterns proven here do not block them.
- The two genuine Wave-7b surprises (A3, A4) are resolved, so 07-03 inherits a clean spine with no outstanding filesystem-command questions.
- No blockers.

## Self-Check: PASSED
- All 5 modified command modules + 5 test files exist on disk.
- All 6 task commits exist in git history (57dacd4, a8a4a75, 14cc499, 6c1dbeb, 6fccd65, 5c68af9).

---
*Phase: 07-spine-rollout*
*Completed: 2026-06-25*
