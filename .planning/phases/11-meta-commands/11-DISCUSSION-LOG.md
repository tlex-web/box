# Phase 11: Meta-Commands - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-14
**Phase:** 11-meta-commands
**Areas discussed:** config set write strategy, config set safety + keys, show/get view semantics, completions surface
**Mode:** advisor (research-backed comparison tables; 4 parallel gsd-advisor-researcher agents, calibration tier `standard`)

---

## config set write strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Round-trip typed struct | Add Serialize + skip_serializing_if=None to Config; load→mutate→toml::to_string→atomic write. No new crate. Drops user comments. | ✓ |
| toml_edit surgical edit | Comment/format-preserving key edit, but adds toml_edit pulling duplicate winnow 0.7 + toml_datetime 0.7 — against the lean/no-duplicate ethos. | |
| Dynamic toml::Table edit | Zero-dep, keeps Config deserialize-only; edit a toml::Table then re-parse to validate. Fiddlier coercion, still drops comments. | |

**User's choice:** Round-trip typed struct (recommended)
**Notes:** The advisor **empirically verified** (throwaway build against pinned `toml 1.1.2` + `toml_edit 0.23`) that `toml_edit` is NOT free on `toml 1.x` — it pulls a duplicate `winnow 0.7` + `toml_datetime 0.7`. That verified fact flipped the otherwise-obvious "use toml_edit for comment preservation" answer. Config is tiny + fully typed with `deny_unknown_fields`, so surgical editing preserves nothing of value. Empty `[hash]`/`[weather]` table emission on all-None was confirmed harmless (re-parses clean).

---

## config set safety + settable-key surface

| Option | Description | Selected |
|--------|-------------|----------|
| Validate + closed keys, defer color | Round-trip through Config before writing (never brick startup, exit 2 on bad value/unknown key). Closed set: hash.default_algo, weather.location, weather.units. `color` deferred (no schema field). | ✓ |
| Validate + closed keys, add color now | Same validation, but also add a `color` config key + wire a config tier into the color gate this phase. Honors roadmap key list literally; more surface. | |
| Free-form write (git-style) | Write whatever the user types, let startup catch errors. Any typo bricks all 23 commands at exit-2 — unsafe given deny_unknown_fields. | |

**User's choice:** Validate + closed keys, defer color (recommended)
**Notes:** `box` is *stricter at read time* than git/gh/cargo (deny_unknown_fields + typed enums → any stray = exit 2 for all commands at startup), which inverts the usual "just write it" default: the writer must be the strict gate. git defers validation (tolerant reads), gh warns-then-writes unknowns, cargo warns — none can brick, so none apply. The `color` key has no backing schema field, so under the validate-before-write design it *cannot* be a known key without a schema change — deferred cleanly rather than smuggled in.

---

## config show / get view semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Effective everywhere + parity | Human and --json both show resolved values (defaults filled, env applied) — matches SC1. get: unset-with-default→default (exit 0), unset-no-default→empty exit 1, unknown key exit 2. | ✓ |
| Literal file (human) + effective (json) | Human show prints only file-present keys; --json stays effective per SC1. Human/json disagree on the same command. | |
| Effective + --show-origin now | Effective everywhere plus per-key source markers (file/env/default) behind a flag this phase. More surface. | |

**User's choice:** Effective everywhere + parity (recommended)
**Notes:** SC1 already locks `show --json` to "the effective config". `box` owns its config and every command reads through the same `cli.or(env).or(cfg).unwrap_or(builtin)` resolver, so `show`/`get` must display exactly what commands will consume — anything else lets `config show` lie about `box weather`'s behavior. Precedent: gh/cargo resolve (effective); only git echoes the literal file (it treats the file as the user's artifact). Unset-no-default → exit 1 mirrors `git config --get`. `--show-origin` provenance kept in reserve as a later superset.

---

## completions surface

| Option | Description | Selected |
|--------|-------------|----------|
| completions <shell>, all shells | clap_complete Shell ValueEnum (rustup pattern): all shells, free validation + --help. Satisfies `box completions powershell` as a superset. stdout-only; hint as inert PS comment; install.ps1 prints hint, no auto $PROFILE edit. | ✓ |
| completions powershell, PS7-only | Hardcode Shell::PowerShell — exact roadmap wording + charter. Smallest surface; adding a shell later is a breaking CLI-shape change. | |
| completions [shell], default powershell | Shell arg defaulting to powershell — bare command works, still accepts all shells. Silent default hides which shell emitted under redirection. | |

**User's choice:** completions <shell>, all shells (recommended)
**Notes:** `clap_complete::Shell` derives `ValueEnum`, so accepting all shells is literally free and still satisfies CMP-01's `box completions powershell` verbatim (a superset, rustup's exact pattern). Output/registration axis: stdout carries ONLY the script (JSON-purity discipline); registration one-liner as an inert `#`-comment header + in `--help`, never per-run stderr. install.ps1: print a hint by default, do NOT silently mutate `$PROFILE`; opt-in `-RegisterCompletions` with a `# box completions` sentinel guard for idempotency.

---

## Claude's Discretion

- Exact `ConfigArgs`/`ConfigCommand` enum shape; location of the closed settable-key registry.
- Whether to suppress empty `[hash]`/`[weather]` tables via `skip_serializing_if` or accept the harmless scaffolding.
- Exact `config show --json` document shape (nested snake_case effective doc) and whether `get --json` wraps or emits bare.
- Whether config usage errors reuse `BoxError::Config` or a new variant (must route set errors → exit 2, get unset-no-default → exit 1).
- Atomic-write helper location; precise hint/error wording; the PS comment-header lines.

## Deferred Ideas

- **`color` config key** — needs a new schema field + a color-gate config tier (new behavior); own follow-up item.
- **`config unset` / `config edit`** — not in the show/get/set/path SC surface.
- **`config show --show-origin` provenance** — reserved superset of the effective view, behind a flag, later.
- **Interactive config wizard / TUI** — explicitly Out of Scope per REQUIREMENTS.
- **Non-PowerShell completion shells** — supported by D-09 but only PS7 is charter-tested; others best-effort.
