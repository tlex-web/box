# Phase 1: Foundation + Flatten - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-22
**Phase:** 1-foundation-flatten
**Mode:** advisor (research-backed comparison tables; calibration tier: full_maturity)
**Areas discussed:** Install location & PATH, Stub command UX, flatten output format, flatten default scope

---

## Install location & PATH

| Option | Description | Selected |
|--------|-------------|----------|
| `%LOCALAPPDATA%\Programs\box` | Win11 per-user installed-app convention; no admin; doesn't roam; dir = the PATH entry | ✓ |
| `%LOCALAPPDATA%\box\bin` | Same benefits; `\bin` separates exe from future config/cache under one root | |
| `%USERPROFILE%\bin` | Short, Unix-familiar; not a Windows convention; roams with profile | |
| `%USERPROFILE%\.box\bin` | Namespaced dotdir; Unix idiom; roams; not hidden in Explorer | |
| Scoop-style shim dir | Versioned/rollback; over-engineering for one static binary (trimmed) | |

**User's choice:** `%LOCALAPPDATA%\Programs\box`
**Notes:** PATH mechanics locked to research recommendation regardless of dir choice — idempotent dedup-guarded registry write, current-session refresh re-reading both Machine+User scopes, re-install = plain overwrite. `REG_EXPAND_SZ → REG_SZ` regression flagged for the `%VAR%`-in-PATH edge case.

---

## Stub command UX

| Option | Description | Selected |
|--------|-------------|----------|
| Stub=exit 1; bare box=help, exit 2 | Unbuilt = runtime "feature absent" (1); code 2 reserved for clap parse errors; bare box → arg_required_else_help (exit 2) | ✓ |
| Stub=exit 1; bare box=help, exit 0 | Same stub handling; bare box is a friendly zero-exit landing screen | |
| Stub=exit 2; bare box=help, exit 2 | Treat invoking an unbuilt command as a usage error (2) | |

**User's choice:** Stub=exit 1; bare box=help, exit 2
**Notes:** Registration approach was presented as settled (only real clap-derive enum variants returning a `NotImplemented` error satisfy "all 23 in --help with per-command --help"). `external_subcommand`, `hide=true`, feature-gating, and `todo!()` were all disqualified. Approved message form: `error: 'qr' is not yet implemented — coming in a future release`.

---

## flatten output format

| Option | Description | Selected |
|--------|-------------|----------|
| B · glyph + arrow + color | `+`/`~`/`-` glyph = meaning, color = decoration; pipe-safe; collisions scannable; UX template for all commands | ✓ |
| A · plain `src -> dest` list | Simplest, cp -v style; collisions not visually distinct | |
| C · rsync itemize columns | Densest, width-aware codes; cryptic, needs legend | |
| D · grouped-by-status sections | Copies/Renamed/Skipped headers; buffers whole plan (no streaming) | |

**User's choice:** B · glyph + arrow + color
**Notes:** ASCII glyphs chosen over Unicode for PowerShell 7 font reliability. Color gating via `is_terminal()` + `NO_COLOR`. Summary wording locked (dry-run `Plan: …`; real run `Done: …`).

---

## flatten default scope

| Option | Description | Selected |
|--------|-------------|----------|
| Skip hidden + merge safely | Skip dotfiles & Windows-hidden; merge into existing out dir but collision-check vs pre-existing files | ✓ |
| Include all files + merge safely | No hidden concept in v1; floods output with `.git`/cache; makes v2 `--include-hidden` moot | |
| Skip hidden + refuse non-empty out dir | Zero ambiguity but hostile to re-runs; errors on stray `Thumbs.db` | |
| merge but check source-only | Silently clobbers pre-existing output (disqualified — violates core promise) | |

**User's choice:** Skip hidden + merge safely
**Notes:** Locked safety sub-decisions — auto-create output dir; hidden = dot-prefix OR Windows `FILE_ATTRIBUTE_HIDDEN` (pruned in `walkdir filter_entry`); collision prefix from path relative to canonicalized source root, separators→`_`, reserved-name sanitized, numeric-suffix fallback.

---

## Claude's Discretion

- `box --version` source (`Cargo.toml` via clap `#[command(version)]`, start `0.1.0`).
- Exact one-line `about` text per stub command.
- Internal dry-run-planner vs executor module split in `flatten` (share one plan).
- Whether `{size} written` byte count is accumulated during copy.

## Deferred Ideas

None — discussion stayed within phase scope.
