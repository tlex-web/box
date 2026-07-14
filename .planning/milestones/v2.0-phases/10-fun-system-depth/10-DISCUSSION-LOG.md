# Phase 10: Fun & System Depth - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-14
**Phase:** 10-fun-system-depth
**Areas discussed:** roast --language axis, pomodoro auto-cycle, weather forecast + cache + stored location, fun content & presentation

---

## roast --language axis

| Option | Description | Selected |
|--------|-------------|----------|
| Programming language | `--language python\|js\|rust\|…` selects roasts targeting that ecosystem; stays English, additive to a general/default bucket | ✓ |
| Spoken language | `--language de\|fr\|es` translates the 42 roasts; higher authoring cost, humor translates poorly | |
| Both | Programming ecosystem buckets AND a couple of spoken languages; doubles the content surface | |

**User's choice:** Programming language
**Notes:** Matches the "programmer roast" identity, no translation-quality risk, extends the existing English corpus by ecosystem. General bucket remains the no-flag default. → CONTEXT D-01.

---

## pomodoro auto-cycle

| Option | Description | Selected |
|--------|-------------|----------|
| Opt-in `--cycles N` / `--loop` | Bare `box pomodoro` stays a single timer; `--cycles N` runs N work sessions with breaks, `--loop` runs until Ctrl+C; every 4th break is the 15-min long break; counter + `--label` shown | ✓ |
| Auto-cycle by default | Bare `box pomodoro` loops until Ctrl+C — changes existing single-timer behavior | |
| Fixed classic set | Runs 4 pomodoros + long break then exits — rigid | |

**User's choice:** Opt-in `--cycles N` / `--loop`
**Notes:** Backward compatible; cycling is an explicit choice. Reuses the existing single-`RawGuard` + `event::poll` loop. `--sound` (POMO-V2-02) fires on completion only, via already-committed `windows 0.61` `MessageBeep`. → CONTEXT D-06/D-07/D-08/D-09.

---

## weather forecast + cache + stored location

### Forecast span & cache freshness

| Option | Description | Selected |
|--------|-------------|----------|
| 7-day, ~10-min cache | 7-day daily forecast; ~10-min cache in `%LOCALAPPDATA%\box\cache` keyed by location+units | ✓ |
| `--forecast [N]`, 1-16 days | Optional day count, capped at Open-Meteo's 16; same cache | |
| 3-day, ~10-min cache | Shorter horizon | |

**User's choice:** 7-day, ~10-min cache → CONTEXT D-10/D-11.

### Config-key structure

| Option | Description | Selected |
|--------|-------------|----------|
| Dotted tables + migrate hash key | `[weather] location/units` + migrate flat `default_hash_algo` → `[hash] default_algo`; matches Phase-11 `box config get weather.location` | ✓ |
| Dotted for new keys only | `[weather] location` but leave `default_hash_algo` flat — mixed schema | |
| Flat keys | `weather_location` flat, matching Phase-6 style — diverges from roadmap dotted keys | |

**User's choice:** Dotted tables + migrate hash key
**Notes:** One-time break to any hand-authored config.toml, accepted to converge the schema before Phase 11 locks it. `location` positional becomes optional (CLI > `weather.location` config; no-location + no-config → exit 2). → CONTEXT D-12/D-13.

---

## fun content & presentation

### cowsay figures + think-mode

| Option | Description | Selected |
|--------|-------------|----------|
| Curated built-in set + `--figure`/`--think` | Small ASCII roster embedded in source; `--figure`/`--list-figures`; `--think` = rounded bubble + `o` tether; pure-ASCII / byte-identical | ✓ |
| Minimal — 2-3 figures + `--think` | Smallest content scope | |
| External `.cow` files | File I/O + parsing; breaks byte-identical ethos | |

**User's choice:** Curated built-in set + `--figure`/`--think` → CONTEXT D-02/D-03.

### fortune categories

| Option | Description | Selected |
|--------|-------------|----------|
| wisdom + tech + humor, default = all | 3 named categories; `--category`/`--list-categories`; bare command draws from all | ✓ |
| Add a 'tech'/'dev' category only | Minimal, thin notion of categories | |
| Broader set | wisdom/tech/humor/motivation/classic — five categories | |

**User's choice:** wisdom + tech + humor, default = all → CONTEXT D-04.

### 8ball ASCII presentation + sentiment color

| Option | Description | Selected |
|--------|-------------|----------|
| ASCII 8-ball + sentiment color | Compact ASCII ball with the answer; green/yellow/red by the existing 10/5/5 tone split; `is_color_on()`-gated | ✓ |
| Sentiment color only | Color the text, no ball art | |
| Large multi-line art | Bigger, fussier to align | |

**User's choice:** ASCII 8-ball + sentiment color → CONTEXT D-05.

---

## Claude's Discretion

- Exact cowsay figure roster, fortune category corpus split, 8-ball art glyphs (small/legible/byte-stable).
- roast per-language content organization (files vs tags), the `MessageBeep` sound constant, pomodoro counter/label wording, optional weather `--no-cache` flag.
- Exact `snake_case` JSON field names for the new fields (`language`, `figure`, `category`, `sentiment`, `forecast`).

## Deferred Ideas

- `box config` meta-command (CFG-01, Phase 11) — locks against the D-13 nested schema.
- Parameterized `--forecast [N]` and hourly forecast — out of scope (7-day daily fixed).
- Spoken-language roast translation — considered, rejected in favor of the programming-language axis.
- External `.cow` file support / interactive cache management — rejected to keep the pure-ASCII, no-file-I/O ethos.
