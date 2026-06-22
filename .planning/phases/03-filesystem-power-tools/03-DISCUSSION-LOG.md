# Phase 3: Filesystem Power Tools - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-22
**Phase:** 3-filesystem-power-tools
**Mode:** advisor (research-backed comparison tables; calibration tier `full_maturity` from `thorough-evaluator` vendor philosophy)
**Areas discussed:** hash algorithms, Traversal & noise, tree + du display, bulk-rename rules

---

## hash — algorithm policy

| Option | Description | Selected |
|--------|-------------|----------|
| SHA-256 + 4 algos | Default SHA-256 (binding contract); `--algo` = sha256·blake3·sha512·md5; enum-dispatch impl; length-auto-detect verify | ✓ |
| SHA-256 + lean 2 | Default SHA-256; only sha256 + blake3 (no md-5 dep) | |
| SHA-256 + broad 6 | Add sha1, sha512, sha224, sha384 for max interop | |
| BLAKE3 default | Override the binding contract; fails Phase-3 criterion #1 | |

**User's choice:** SHA-256 + 4 algos
**Notes:** Resolves the documented conflict — ROADMAP success-criterion #1 + REQUIREMENTS HASH-01 (binding contract) lock SHA-256; STATE.md/CLAUDE.md "BLAKE3 default" notes are advisory, predate the lock, and are superseded (read as "BLAKE3 via `--algo blake3`"). Research flagged the impl detail: route sha256/sha512/md5 through one RustCrypto `digest::Digest` path, give blake3 its own arm on the **stable native `Hasher`** — its `digest`-trait impl is behind the explicitly-unstable `traits-preview` feature. Verify auto-detects algo by hex length (64-hex ties → sha256), case-insensitive plain `==` (a checksum is public, not a secret), `-` stdin label. `hash` implements the deferred `--file PATH` layer (Phase 2 D-06).

---

## Traversal & noise (shared: tree / du / dupes)

| Option | Description | Selected |
|--------|-------------|----------|
| Hidden-skip only | All three reuse walkdir + `is_hidden` (skip hidden, `--all` later); node_modules/target shown literally; no new deps | ✓ |
| + noise list (dupes) | Tiny built-in skip list scoped to dupes only | |
| + noise list (du & dupes) | Apply that skip list to du as well | |
| Use `ignore` crate | Pull in gitignore-aware `ignore` crate; heavier deps, CWD-dependent | |

**User's choice:** Hidden-skip only
**Notes:** Consistent with flatten (D-12) and the lean-binary goal. `.git`/`.venv` are dot-prefixed so already pruned; `node_modules`/`target` shown as literal truth. The `ignore` crate (CLAUDE.md's dupes suggestion) drags `globset`+`crossbeam` and is non-deterministic by CWD — and TREE-V2/DU-V2/DUPE-V2 already defer gitignore to v2. A dep-free hardcoded skip-list scoped to dupes is reserved as the v2 entry point if needed.

---

## tree + du display

| Option (du row model) | Description | Selected |
|--------|-------------|----------|
| Immediate children | One row per immediate child; dirs=recursive total, files own size; biggest-first; `--depth` caps reporting, `--top` truncates | ✓ |
| Classic GNU `--max-depth` | One row per directory at every level ≤ depth | |
| Flat list of files | List every file by size | |

**User's choice:** Immediate children (dust/ncdu model)
**Notes:** Tree display defaults adopted from research (no objection raised): dirs-first then files (alphabetical); Unicode box-drawing `├── └── │`; `--sizes` per-file only (recursive dir totals are du's job); summary `N directories, M files`; **reuse flatten's tested `human_size`** (1024-based, promoted into `core::output`) rather than the `humansize` crate. Color: dir names (tree) + size values (du) only, through `is_color_on()`. du summary always reflects the full-scan total, not just shown rows; trailing `/` (ASCII) marks dirs so the distinction survives piping.

---

## bulk-rename rules

| Option (scope) | Description | Selected |
|--------|-------------|----------|
| Top-level + `--recursive` | Top-level files by default (matches ./photos), `--recursive` opt-in; full safety model | ✓ |
| Top-level only | Never recurse; max safety | |
| Recursive by default | Walk whole subtree by default; largest blast radius | |

**User's choice:** Top-level + `--recursive`
**Notes:** Safety model adopted from research either way. Decisive fact: `std::fs::rename` **silently overwrites** on Windows (no `create_new` backstop for moves) — so correctness rests on pre-flight, in-memory, abort-all-before-any-rename collision detection (case-folded set per dir; checks targets vs other targets AND out-of-set on-disk files). Files only; full-name regex match (extension protected by pattern discipline); first-match `replace` with `$1`/`${1}` (`--all` deferred to v2); detect-and-abort on swap/cycles (no two-phase temp magic in v1); case-only renames (`foo`→`Foo`) allowed via exact-name compare. Dry-run default + `--force`; reuse flatten's `~`/`-` glyph preview.

---

## Claude's Discretion

- Module layout per command; `core::output` location/signature for the promoted `human_size`; precise per-dir collision-set structure in bulk-rename.
- Whether dupes shows an optional `indicatif` spinner (not required for v1).
- Exact `--algo` value spellings + enum names; `--file` positional-vs-flag for hash.
- Exact color shades, alignment widths, and summary/error wording within the locked families.

## Deferred Ideas

- Reconcile stale "BLAKE3 default" notes in STATE.md / CLAUDE.md (action for planner/transition).
- `--all`/`-a` show-hidden flag (v2; standardize one verb with flatten's `--include-hidden`).
- Noise-dir skipping / `ignore` crate gitignore-awareness (TREE-V2/DU-V2/DUPE-V2).
- Two-phase swap/cycle rename for bulk-rename (v1 detects-and-aborts).
- bulk-rename `--all`/replace_all, case transforms, sequential numbering, `--backup` (RENM-V2).
- hash multi-file/`--json`/progress + sha1/sha224/sha384 (HASH-V2); du percentage bars/apparent-size (DU-V2); tree gitignore/dirs-only/sort-by-size (TREE-V2).
- indicatif progress for dupes/large-file hash (v2).
