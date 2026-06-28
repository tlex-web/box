# Phase 08 — Deferred / Out-of-Scope Items

Discoveries logged during execution that fall outside the current plan's scope
(per the executor scope-boundary rule). NOT fixed in-plan.

## Pre-existing repo-wide `cargo fmt --check` drift (discovered during 08-06)

`cargo fmt --check` reports formatting diffs in many committed files that plan
08-06 never touched, e.g.:

- `src/commands/du/mod.rs`, `src/commands/dupes/mod.rs`,
  `src/commands/flatten/mod.rs`, `src/commands/tree/mod.rs`
- `tests/base64.rs`, `tests/bulk_rename.rs`, `tests/cli.rs`, `tests/color.rs`,
  `tests/du.rs`, `tests/dupes.rs`, `tests/dupes_delete.rs`, `tests/epoch.rs`,
  `tests/flatten.rs`, `tests/flatten_move.rs`, `tests/hash.rs`, `tests/json.rs`,
  `tests/passgen.rs`, `tests/qr.rs`, `tests/tree.rs`

These are pre-existing in committed code (clean `git status` at 08-06 start;
unmodified by this plan) — most likely a rustfmt-version difference vs. when
08-01..08-05 were committed. The 08-06 verification gate is `cargo test` +
`cargo clippy --all-targets -- -D warnings` (both clean); `cargo fmt --check` is
not part of the plan's gate. The two files 08-06 authored
(`src/commands/bulk_rename/mod.rs`, `tests/bulk_rename_backup.rs`) ARE
fmt-clean.

**Recommended follow-up:** a dedicated `style: cargo fmt` sweep commit (run
`cargo fmt` once at the repo root) outside any feature plan, so a single
formatting normalization is not entangled with feature diffs.
