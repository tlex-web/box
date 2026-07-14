# Phase 8: Filesystem Depth - Context

**Gathered:** 2026-06-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Add the deferred **depth flags** to the six filesystem commands — `hash`, `flatten`, `dupes`, `bulk-rename`, `tree`, `du` — now that every one of them already has a `--json` output struct (Phase 7), so each new field slots into BOTH the human render and the JSON path for free (no-drift by construction). Requirements: **HASH-V2-02, FLAT-V2-01, FLAT-V2-02, DUPE-V2-01, DUPE-V2-02, RENM-V2-01, RENM-V2-02, TREE-V2-01, DU-V2-01, DU-V2-02** (10).

**In scope (per ROADMAP Phase 8 + REQUIREMENTS):**
- `hash` — multiple file args + coreutils `digest␣␣filename` (double-space) per line + stderr progress for large inputs (HASH-V2-02).
- `flatten` — `--extensions`, `--separator`, `--include-hidden`, stderr progress bar (FLAT-V2-01); **DESTRUCTIVE** `--move` = copy→verify→delete, dry-run default + `--force` (FLAT-V2-02).
- `dupes` — multi-stage hashing (size → partial → full BLAKE3) + hardlink-aware grouping (DUPE-V2-01); **DESTRUCTIVE** `--delete` keep-≥1, dry-run default, `--force`, hardlink-safe, abort-all-before-any (DUPE-V2-02).
- `bulk-rename` — case transforms (upper/lower/title) + `{n}` numbering (RENM-V2-01); **DESTRUCTIVE** `--backup` recoverable pre-rename backup (RENM-V2-02).
- `tree` — `.gitignore` respect, `--dirs-only`, `--ignore <glob>`, sort-by-size (TREE-V2-01).
- `du` — in-line percentage column + color-coded size ranges + `--exclude <glob>` (DU-V2-01); `--on-disk` allocated/compressed NTFS size via Win32 (DU-V2-02).

**Out of scope (later phases / deferred):**
- Dev-transform & visual depth (Phase 9), fun & system depth (Phase 10), `config`/`completions` meta-commands (Phase 11).
- An automated `box bulk-rename --undo` replay subcommand — RENM-V2-02 requires only that the backup be *written* and the directory be *recoverable* (see D-22; `--undo` is a clean follow-up, deferred).
- `dupes` interactive review UI, cross-volume dedupe-to-hardlinks (DUPE-V3).
- Any change to the frozen `--json`/`--clip`/config spine — Phase 8 only ADDS fields/flags onto it.

**Code-review gate (ROADMAP):** the three destructive plans (`flatten --move`, `dupes --delete`, `bulk-rename --backup`) each get their OWN plan with mandatory adversarial code review: dry-run default, `--force`, abort-all-before-any pre-flight, and a snapshot-the-tree-unchanged test for every abort path.
</domain>

<decisions>
## Implementation Decisions

> Four gray areas were researched with project-aware advisor tables (standard calibration tier) and decided. All four resolved to the recommended option. The non-discussed depth flags are pre-locked upstream or under Claude's Discretion (both recorded below so the planner has everything in one place).

### Carried forward — LOCKED upstream, NOT re-discussed (recorded for the planner)
- **Spine is frozen** (Phase 6/7 — see `<canonical_refs>`): every new field feeds ONE `#[derive(Serialize)]` struct → both human render and `emit_json` (no-drift); `is_json_on()` fork happens FIRST; progress (indicatif) → stderr, suppressed under `--json`; piped/`--json` output byte-identical-minus-ANSI; multi-row commands keep the `{results,count}` root (+ the locked sibling totals); `tree` keeps its recursive-node root-rule exception.
- **Field policies (D-3/D-4):** large numbers = bare `u64` (document the >2^53 caveat); non-UTF-8 paths = `to_string_lossy()`, never `to_str().unwrap()`.
- **`du --on-disk` dependency (D-2):** GO — `windows 0.61` (`GetCompressedFileSizeW`), pinned to unify with the transitive `windows ^0.61` from `tauri-winrt-notification 0.7.2`. Dependency is settled; only the display/aggregation SEMANTICS were open (→ D-23).
- **`dupes --delete` safety model (D-5):** GO, safe + non-interactive — keep-first, dry-run default, `--force`, hardlink-aware, abort-all-before-any pre-flight, snapshot-the-tree tests, mandatory adversarial review. The keep-selection policy is **keep-first in the sorted (deterministic) path order** — no interactive prompt (Out of Scope), no `--keep <dir>` this phase.
- **`flatten --move` mechanics (ROADMAP SC2):** dry-run default; `--force` runs copy → verify (dest exists + size matches) → delete-source; every abort path leaves the source tree byte-for-byte unchanged. Reuse the existing collision-rename + containment-guard logic from the copy path. **Empty source directories left in place after a move** (flatten operates on files; pruning emptied dirs is NOT in scope — keep the source tree structurally intact, only files relocate).
- **`dupes` multi-stage identity (DUPE-V2-01):** size → partial-hash → full-BLAKE3 cascade, reusing the existing rayon parallel-hash path; hardlink-aware = paths sharing one NTFS file-index are COLLAPSED (counted once, never reported as wasted space).

### D-20 — `tree` gitignore: opt-in `--gitignore`, matcher layered over `walkdir` (TREE-V2-01)
- **Default tree output is UNCHANGED.** `.gitignore` respect is **opt-in via a `--gitignore` flag**, NOT on-by-default. Rationale: `box tree` has always shown everything-except-dotfiles; GNU `tree` (the reference users expect) has no gitignore notion; `eza`/`exa` made it explicit opt-in precisely to avoid silently hiding `target/`/`node_modules/`. A *viewer* must not change what it shows out of the box. (Only `fd`, a *finder*, defaults it on.)
- **Mechanism:** keep the `walkdir` depth-1 traversal in `read_children` and layer an **`ignore::gitignore::Gitignore` matcher as a filter** inside it — do NOT swap to `ignore::WalkBuilder`. **Decisive box-internal constraint:** `read_children` is the single source of truth that BOTH `render_dir` (human) and `build_node` (the frozen `--json` recursive node tree) call per directory; the matcher-as-filter preserves no-drift by construction, whereas `WalkBuilder` is a recursive walker that would force re-architecting both recursions (no-drift regression risk on a frozen JSON shape) or re-parsing the ignore stack per node. `core::fs::is_hidden` (D-06) and the D-08 dirs-first comparator stay untouched.
- **Flag composition:** `--ignore <glob>` folds into the **same** `Gitignore` matcher (`GitignoreBuilder::add_line`) so both compose in one filter pass; `--dirs-only` is a trivial `child.is_dir` filter applied in `read_children` AFTER the ignore filter (drops files identically in human + JSON paths); **sort-by-size is an explicit `--sort size` flag** that REPLACES the D-08 dirs-first-then-alpha comparator only when set (tie-break alpha for equal sizes; dirs carry `size: None` → sort to a defined end), rather than silently augmenting the default order.
- **New dep:** `ignore = "0.4"` (BurntSushi, pure-Rust, `+crt-static`/PS7-clean — no DLL). Nested-`.gitignore` correctness is the main test target (the eza #1086 class of bug); use `matched(rel_path, is_dir)` with paths rooted relative to the tree target.

### D-21 — `bulk-rename` case + numbering: post-replace flags + literal `{n}` token (RENM-V2-01)
- **Case transforms are flags** — `--case upper|lower|title` — applied to the whole resulting name AFTER `re.replace`, NOT replacement-string escapes. **Decisive constraint:** box uses the `regex` crate, which has **no `\U`/`\L` case-fold escapes** (unlike Perl/sed; the Rust-based `rnr` is the same) — a `\U$1\E` style would require a custom replacement-expansion layer re-implementing `$`/`${}` group substitution, duplicating regex internals and risking drift from the documented `$1`/`${1}` foot-gun rules. Keeping case as a separate post-pass leaves the regex semantics 100% untouched.
- **`{n}` is a literal token** in the `replacement` string, expanded in a small post-replace pass (no clash with the regex crate's `${n}` group syntax — `{n}` ≠ `${n}`; document escaping a literal brace). The **counter is assigned over the deterministic SORTED plan order** (reproducible), not walk order.
- **Numbering controls:** `--number-width` for zero-pad width (or auto-width from the file count), plus `--start` / `--step` for the starting value and increment — mirroring `brename`'s proven `{nr}` + `--nr-width` + `--start-num` design.
- **Apply order:** `re.replace` → `{n}` expansion → `--case` fold. `title` operates on the **stem** so the extension isn't title-cased mid-word.
- **Pre-flight intact:** every generated name still flows through the existing per-directory ABORT-ALL collision/cycle/separator pre-flight unchanged — two files both producing `IMG_{n}` are disambiguated by distinct `n`, never collide; a user-forced true collision is caught exactly as today. No new dep.

### D-22 — `bulk-rename --backup`: undo manifest (JSON journal), NOT content copies (RENM-V2-02, DESTRUCTIVE)
- **Write a JSON undo manifest, not file copies.** A pure rename (`std::fs::rename` = `MoveFileExW`) changes only the NAME, so the entire reversible state is the `{old_path → new_path}` mapping — copying bytes protects data that was never at risk. This is the prevailing pattern: `brename` (`.brename_detail.txt`), `f2` (JSON backup map), PowerRename all journal the name map and replay in reverse; none copy content for a rename.
- **Format/location:** one `{old, new}` (absolute paths) record per affected file in `%LOCALAPPDATA%\box\undo\<id>.json` (OUT of the renamed tree, so `--recursive` never re-walks it and it survives renaming the target dir); echo the manifest path to stderr so the user can find it. Fallback to the target dir only if `LOCALAPPDATA` is unset. It is a direct serde projection of the existing build-once `Plan` (already collision-cleared by pre-flight) — zero drift.
- **Mid-batch reconciliation:** write + fsync the FULL manifest BEFORE the first `fs::rename`, then flip each entry `applied: true` as its rename returns. An I/O error mid-loop (the existing `?`-propagation → exit 1) leaves a manifest whose `applied` flags exactly partition done-vs-pending → the directory is reconcilable by reversing only the applied entries.
- **`--backup` is orthogonal to and only meaningful with `--force`** — it is a property of the executed run; on a dry-run it is a no-op (clean "nothing to back up"). Snapshot-the-tree-unchanged tests apply to every abort path (code-review gate).
- **Deferred:** an automated `box bulk-rename --undo` replay subcommand (which must itself re-run pre-flight on the reverse map) — RENM-V2-02 requires only the backup be written + the dir recoverable, both satisfied by the manifest + a one-line documented manual reverse. No new dep (serde already present).

### D-23 — `du` percentage + colors + on-disk + exclude (DU-V2-01 / DU-V2-02)
- **Color scheme = percentage-of-total bucketing.** Color the size VALUE by its share of the full-scan total — `>50%` red, `10–50%` yellow, else default/unaccented — which adapts to ANY dataset (a tiny source tree vs a multi-TB drive both light up the dominant rows), reusing the percentage already computed as the bucket key. Matches the relative-to-the-set model of `dust`/`ncdu`; none of the surveyed tools use fixed absolute byte thresholds. This **REPLACES** the lone `.cyan()` accent (still size-value-only; name stays uncolored; all gated on `is_color_on()` → byte-identical-minus-ANSI when piped/`--json`).
- **Percentage basis = the full-scan TOTAL** (the value the summary already captures BEFORE `--top` truncation), so per-row % is consistent with the printed total and the shown rows correctly sum to <100% after `--top` (honest). Format right-aligned `42.3%` in its own fixed-width column placed BEFORE the size value; render a tiny-but-nonzero row as `<0.1%` (reserve `0.0%` for genuine zero-byte rows).
- **`--on-disk` directory aggregation = recursive sum of each descendant file's `GetCompressedFileSizeW`** allocated size, exactly paralleling the current apparent-size recursion in `dir_total` (directories have no intrinsic compressed size). When `--on-disk` is set, the **percentage basis switches to the on-disk total** so the column stays internally consistent.
- **`--json` composition:** the `size` field carries on-disk bytes when `--on-disk` is set, plus a top-level **`on_disk: bool`** marker — keeps the one-row-feeds-both no-drift invariant (human + JSON read the same `size`) without doubling the schema.
- **`--exclude <glob>` = `globset`** (BurntSushi standard, already present transitively via the `ignore` ecosystem — one `Cargo.toml` line, no new compile cost), matched against the path relative to the target root, applied to BOTH immediate children (excluded child → no row) AND recursive descendants (excluded file never counts toward any dir total).

### Claude's Discretion (planner/executor latitude — sensible defaults pre-stated)
- **`hash` multi-file partial-failure policy (HASH-V2-02):** lean **coreutils best-effort** — print the digest for each readable file (one `digest␣␣filename` line each), emit a stderr error per unreadable file, exit 1 if any failed (matches `sha256sum`). Planner may instead choose fail-fast-abort if a snapshot/test argues for it; default is best-effort-continue.
- **`flatten` filter semantics (FLAT-V2-01):** `--extensions <list>` = comma-separated, case-insensitive, match on the final extension; `--separator <str>` = the collision-encoding join char (currently `_`); `--include-hidden` flips the `is_hidden` prune for the walk. Exact spellings/parsing are the planner's call within these intents.
- **"Large input" progress threshold (HASH-V2-02 / FLAT-V2-01):** the byte/file-count threshold at which the indicatif stderr bar appears — planner picks a sensible cutoff (e.g. only show for files/totals above some size, and never under `--json`). Progress is always stderr, suppressed under `--json` (spine rule).
- **`{n}` literal-brace escaping, `title`-case algorithm, exact band cutoffs** for du color (the 50% / 10% boundaries) — taste calls, keep documented and tested.
- **New deps to add this phase:** `indicatif 0.18.4` (progress — HASH/FLAT), `windows 0.61` (on-disk — D-2, locked), `ignore 0.4` (tree gitignore — D-20), `globset` (du exclude — D-23, already transitive). `clap_complete`/`chrono-tz`/`uuid v7` belong to later phases.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & locked contracts
- `.planning/ROADMAP.md` § "Phase 8: Filesystem Depth" — goal, 5 success criteria, the 6-plan sketch (08-01 hash+flatten / 08-02 tree+du / 08-03 dupes+bulk-rename / 08-04 flatten --move / 08-05 dupes --delete / 08-06 bulk-rename --backup), and the **Code-review gate** wording for 08-04/05/06.
- `.planning/REQUIREMENTS.md` — the 10 Phase-8 requirements (HASH-V2-02, FLAT-V2-01/02, DUPE-V2-01/02, RENM-V2-01/02, TREE-V2-01, DU-V2-01/02) + milestone decisions **D-2** (`windows 0.61` on-disk) and **D-5** (`dupes --delete` safety model) + the Out-of-Scope table (no interactive prompts, no NDJSON, no per-command bespoke JSON schemas).
- `.planning/STATE.md` § "Accumulated Context" — locked v2 decisions + critical spine pitfalls.
- `.planning/PROJECT.md` — milestone scope, "deepening not rewrite" framing, v1 Key Decisions.

### The frozen spine (READ FIRST — every new field rides on these)
- `.planning/phases/06-scriptable-core-foundation/06-CONTEXT.md` — the authoritative spine contract: D-01 JSON root rule, D-03 field-naming policy, D-07/D-08 `--clip`, D-09/D-10 error/exit contract, D-3/D-4 field policies.
- `.planning/phases/07-spine-rollout/07-CONTEXT.md` — the per-command `{results,count}` + `dry_run` projections already shipped on all six filesystem commands (D-11 uniform root, D-13 action rows, D-17 field schemas incl. `dupes {size,paths}` and `tree` recursive node).

### Source files this phase touches (per command)
- `src/commands/hash/mod.rs` — single-file → multi-file: loop `HashRow`s into the existing `{results,count}` doc; coreutils `digest␣␣filename` per line; reuse the streaming `digest_reader` + `read_file_or_stdin`.
- `src/commands/flatten/mod.rs` (+ `src/commands/flatten/rename.rs`) — add filters to `build_plan`'s walk; `--move` is a NEW plan reusing `safe_copy` + the containment guard + collision-rename, then verify + delete; `FlattenOutput` already has `dry_run`.
- `src/commands/dupes/mod.rs` — add partial-hash stage + hardlink-index collapse to the size-bucket cascade; `--delete` keep-first over the already-sorted groups.
- `src/commands/bulk_rename/mod.rs` — add `--case`/`{n}` post-passes at the `re.replace` site (line ~513), counter over the sorted plan; `--backup` manifest as a serde projection of `Plan` written before the executor loop (lines ~366–410).
- `src/commands/tree/mod.rs` — layer the `ignore::Gitignore` matcher + `--dirs-only` filter inside `read_children` (the shared chokepoint at ~279–310); `--sort size` swaps `sort_children`.
- `src/commands/du/mod.rs` — percentage column + percentage-band color in the render loop; `--on-disk` adds a Win32 per-file size path paralleling `dir_total`; `--exclude` globset filter in `collect_rows`/`dir_total`.

### Shared infra (reuse VERBATIM — do not re-implement)
- `src/core/output.rs` — `emit_json`, `out_line`, `is_json_on`, `is_color_on`, `human_size`, `format_row`, `RowStatus`, `terminal_width`, `dry_run_summary`/`real_run_summary`. (Add nothing to the spine; consume it.)
- `src/core/fs.rs` — `is_hidden` (the shared hidden-prune, D-06), `normalize_path` (dunce, no `\\?\` leak), `safe_copy` (create-new + timestamp-preserving copy, for `flatten --move`).
- `src/core/input.rs` — `read_file_or_stdin` (hash's streaming input layer).
- `Cargo.toml` — current deps; add `indicatif`, `windows 0.61`, `ignore`, `globset` this phase.

### Research blueprints (HIGH-confidence)
- `.planning/research/ARCHITECTURE.md` — the New-vs-Modified file ledger + per-command change lists.
- `.planning/research/PITFALLS.md` — `--json` contamination, color/progress leakage, walk determinism (rayon sort-before-group), symlink-loop safety, NTFS case-fold collisions.
- `.planning/research/STACK.md` — dependency versions/rationale.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Every filesystem command already has its `--json` output struct** (Phase 7) — new depth fields are ADDITIONS to existing `#[derive(Serialize)]` structs feeding both renders, so no-drift is free.
- **`safe_copy` (`core::fs`)** — create-new + timestamp-preserving copy; the verify-half of `flatten --move`'s copy→verify→delete can read back dest metadata after it.
- **`bulk_rename`'s build-once `Plan` + per-directory ABORT-ALL `preflight`** — the load-bearing safety detector is already pure/unit-tested; `--backup` projects the same `Plan`, and `--case`/`{n}` only reshape the `new_name` BEFORE pre-flight, so the safety logic is untouched.
- **`dupes`' size-bucket → rayon parallel BLAKE3 cascade** — the partial-hash stage inserts between size-bucketing and full-hash; the deterministic `(hash, path)` sort + group fold already give stable groups for keep-first `--delete`.
- **`du`'s `dir_total` recursion + full-scan-total-before-`--top` capture** — the percentage basis and the `--on-disk` per-file path both parallel existing recursion; the total is already captured pre-truncation.
- **`tree`'s shared `read_children` (feeds both `render_dir` and `build_node`)** — the single chokepoint for the gitignore matcher, `--dirs-only` filter, and `--sort size`.

### Established Patterns
- **`is_json_on()` fork FIRST**, then human writes below it (every command already does this — new flags must keep all human chrome below the fork; progress to stderr, suppressed under `--json`).
- **Dry-run-default → `--force` → abort-all-before-any** is the destructive template (proven in `bulk-rename`; `flatten --move` and `dupes --delete` follow it). Snapshot-the-tree-unchanged test for every abort path.
- **Single `.cyan()`/`.yellow()`/`.blue()` accent gated on `is_color_on()`** — du's new percentage-band color replaces the cyan accent in the same gated slot.
- **Deterministic sort BEFORE printing/grouping** (rayon order is arbitrary) — bulk-rename's `{n}` counter and dupes `--delete` keep-first both ride the existing sorted order.

### Integration Points
- **New deps in `Cargo.toml`:** `indicatif` (progress), `windows 0.61` (Win32 `GetCompressedFileSizeW`), `ignore` (tree gitignore matcher), `globset` (du exclude — already transitive).
- **No `cli.rs`/`main.rs` spine plumbing changes** — global `--json`/`--clip` + `init_output`/`flush_clip` already wired; Phase 8 is per-command args + per-command logic + per-command tests.
- **Win32 surface (`du --on-disk`):** `GetCompressedFileSizeW` returns per-file allocated size; a dir row aggregates its descendants. `windows 0.61` pinned to the existing transitive version (D-2).
</code_context>

<specifics>
## Specific Ideas

- **The destructive-flag asymmetry is the headline:** of the three, `dupes --delete` (D-5) and `flatten --move` (SC2) are mechanically pre-locked upstream — only `bulk-rename --backup` had a genuinely open design, resolved by D-22 (undo manifest, not content copies; a rename is reversible from the name map alone).
- **`bulk-rename --backup` manifest location is deliberate:** `%LOCALAPPDATA%\box\undo\<id>.json`, OUTSIDE the renamed tree, so `--recursive` never re-walks it and it survives renaming the target dir. Echo the path to stderr.
- **`tree --gitignore` is opt-in on purpose** — keeping the default `box tree` output unchanged is the decisive UX choice; the matcher-over-`walkdir` mechanism (not `WalkBuilder`) is the decisive no-drift choice for the frozen recursive `--json` node tree.
- **`du` color = relative, not absolute:** percentage-of-total bucketing is the one scheme that signals "biggest offenders" across both a 4 KB repo and a 4 TB drive — pin the bands (>50% / 10–50% / else) and keep them documented + tested.
- **Pre-planning doc check:** unlike Phases 6/7, no doc-amendment action item surfaced — ROADMAP Phase-8 SC + REQUIREMENTS wording are consistent with these decisions (the `entries`→`results` and verify-default amendments were already applied in Phases 6/7).
</specifics>

<deferred>
## Deferred Ideas

- **`box bulk-rename --undo` automated replay subcommand** — replays a D-22 manifest in reverse (re-running pre-flight on the reverse map). RENM-V2-02 needs only the backup written + the dir recoverable (manifest + documented manual reverse satisfy it). Clean follow-up; candidate for a future phase or a fast-follow.
- **`dupes` interactive review UI + `--keep <dir>` selection / cross-volume dedupe-to-hardlinks** — DUPE-V3 (already deferred beyond v2 in REQUIREMENTS); this phase is non-interactive keep-first only.
- **`du` continuous truecolor gradient / visual size bars (à la `dust`)** — rejected for the flat row list now; revisit only if `du` grows bars. Tracks toward VIS-V3-style polish.
- **`tree --gitignore` on-by-default** — explicitly NOT chosen (would change long-standing default output); could be reconsidered as a config-file default key (`tree.gitignore = true`) once `box config` lands in Phase 11.

None of these are in Phase 8 scope — captured so they are not lost.
</deferred>

---

*Phase: 8-Filesystem Depth*
*Context gathered: 2026-06-25*
