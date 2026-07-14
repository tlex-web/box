# Phase 8: Filesystem Depth - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-25
**Phase:** 8-Filesystem Depth
**Mode:** advisor (research-backed comparison tables, standard calibration tier, technical framing)
**Areas discussed:** tree gitignore behavior, bulk-rename case + numbering, bulk-rename --backup recovery, du percentage + colors + on-disk

---

## tree gitignore behavior + walker mechanism (TREE-V2-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Opt-in `--gitignore`, matcher over walkdir | `ignore::Gitignore` matcher inside the shared `read_children`; default output unchanged; preserves human/JSON no-drift; `--ignore` folds into the same matcher; `--sort size` explicit | ✓ |
| On by default + `--no-ignore` | fd-style always-on, same matcher; hides target/ node_modules/ out of the box — changes long-standing default | |
| Swap to `ignore::WalkBuilder` | Recursive walker, gitignore for free, but breaks the depth-1 read_children shared by the printer and the frozen --json node tree | |

**User's choice:** Opt-in `--gitignore`, matcher over walkdir (recommended).
**Notes:** Decisive constraint is box-internal — `read_children` is the single source of truth feeding both `render_dir` (human) and `build_node` (frozen `--json` recursive node tree); the matcher-as-filter preserves no-drift by construction. Keeps default `box tree` output unchanged (viewer ≠ finder). → CONTEXT D-20.

---

## bulk-rename case transforms + {n} numbering surface (RENM-V2-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Post-replace flags + `{n}` token | `--case upper\|lower\|title` after the regex; `{n}` literal token, counter over the sorted plan; `--number-width`/`--start`/`--step`; keeps `$1`/`${1}` untouched | ✓ |
| Replacement tokens (`\U$1\E`) | sed/Perl-style case + numbering inside the replacement; per-group case but the regex crate has no `\U`/`\L` escapes → custom expansion layer | |

**User's choice:** Flags + `{n}` token (recommended).
**Notes:** The `regex` crate lacks native case-fold escapes, so token-style case would duplicate regex internals. Post-pass keeps the build-once plan + pre-flight intact; counter over deterministic sorted order; apply order regex → `{n}` → case; `title` on the stem. Mirrors brename. → CONTEXT D-21.

---

## bulk-rename --backup recovery model (RENM-V2-02, destructive)

| Option | Description | Selected |
|--------|-------------|----------|
| Undo manifest / JSON journal | `{old,new}` log in `%LOCALAPPDATA%\box\undo\<id>.json`, written+fsynced before first rename, entries flagged applied; exact reverse; matches brename/f2/PowerRename | ✓ |
| Per-file `.bak` content copies | Copy each file before rename; duplicates bytes for a name-only op; `.bak` siblings become rename/collision noise | |
| Timestamped backup directory | Copy affected files into `./.box-backup-<ts>/`; same byte cost, `--recursive` re-walks it, manual move-back | |

**User's choice:** Undo manifest / JSON journal (recommended).
**Notes:** A pure rename is fully reversible from the name map alone — content copies protect data that was never at risk. `--backup` only with `--force`; write+fsync full manifest before first rename, flag entries `applied` as they succeed (mid-batch failure stays reconcilable). Automated `--undo` subcommand deferred. → CONTEXT D-22.

---

## du percentage + colors + on-disk (DU-V2-01 / DU-V2-02)

| Option (color scheme) | Description | Selected |
|--------|-------------|----------|
| Percentage-of-total buckets | Color size value by share of full-scan total (>50% red / 10–50% yellow / else default); adapts to any dataset; reuses the % column | ✓ |
| Absolute-size thresholds | Color by raw bytes (>1 GB red, >100 MB yellow); useless on small or all-huge targets — no relative signal | |
| Continuous gradient | Truecolor interpolation; polished but reads as noise without bars; legibility risk | |

**User's choice:** Percentage-of-total buckets (recommended), with the sub-decisions riding along.
**Notes:** % basis = full-scan total (captured before `--top`); `<0.1%` for tiny rows; color replaces the lone `.cyan()` accent (gated on `is_color_on()`). `--on-disk` dir = recursive `GetCompressedFileSizeW` sum, switches % basis to on-disk total; `--json` adds `on_disk: bool`. `--exclude` via `globset` (already transitive) on children + descendants. → CONTEXT D-23.

---

## Claude's Discretion

- `hash` multi-file partial-failure policy — default coreutils best-effort (print good digests, stderr error per bad file, exit 1).
- `flatten` `--extensions`/`--separator`/`--include-hidden` exact parsing/spelling.
- "Large input" progress threshold for the indicatif stderr bar (always stderr, suppressed under `--json`).
- `{n}` literal-brace escaping, `title`-case algorithm, exact du color band cutoffs — documented + tested taste calls.

## Deferred Ideas

- `box bulk-rename --undo` automated manifest-replay subcommand (RENM-V2-02 needs only backup-written + dir-recoverable).
- `dupes` interactive review UI / `--keep <dir>` / cross-volume dedupe-to-hardlinks (DUPE-V3).
- `du` continuous gradient / visual size bars (revisit only if du grows bars).
- `tree --gitignore` on-by-default — reconsider as a `tree.gitignore` config-file default once `box config` lands (Phase 11).
