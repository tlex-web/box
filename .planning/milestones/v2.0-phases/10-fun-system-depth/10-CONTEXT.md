# Phase 10: Fun & System Depth - Context

**Gathered:** 2026-07-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Add the deferred depth flags to the four **fun** commands (`cowsay`, `fortune`,
`8ball`, `roast`) and the two **system** commands (`pomodoro`, `weather`) — the
last per-command depth phase before the Phase-11 meta-commands.

Seven requirements, no new tool commands: **COW-V2-01** (cowsay figures +
think-mode), **FORT-V2-01** (fortune categories), **8BAL-V2-01** (8ball ASCII
art + sentiment color), **ROST-V2-01** (roast `--language`), **POMO-V2-01**
(pomodoro session counter / auto-break / `--label`), **POMO-V2-02**
(`pomodoro --sound` via Win32 `MessageBeep`), **WTHR-V2-01** (weather
`--forecast` / response cache / stored default location).

Each feature is additive to an already-shipped command: the `--json` spine
(Phase 7), the config resolver (Phase 6), and the `is_color_on()` gate (v1)
are settled and are grafted onto here, not rebuilt.

</domain>

<decisions>
## Implementation Decisions

### roast (ROST-V2-01)
- **D-01:** `--language <lang>` selects a **programming language**, NOT a spoken
  language. Roasts stay in English; `<lang>` targets a dev ecosystem
  (`python`, `js`/`javascript`, `rust`, …) with a **general/default bucket**
  used when the flag is omitted (preserves today's behavior). New content is
  authored per ecosystem, additive to the existing `src/data/roasts.txt`.
  Content organization (per-language files vs a tagged single file) is
  planner's discretion. An unknown `--language` value → a helpful error listing
  the available languages. JSON gains a `language` field.

### cowsay (COW-V2-01)
- **D-02:** Multiple figures come from a **curated built-in set embedded in
  source** (the classic cow + a small legible roster, e.g. tux / dragon / ghost
  / dog / stegosaurus — exact roster is planner's discretion). `--figure <name>`
  selects; `--list-figures` enumerates. **No external `.cow` file loading** —
  that would add file I/O + parsing and break the Phase-1 pure-ASCII,
  byte-identical, "the glyph is the source of truth" rule. Unknown figure name →
  helpful error / exit 2.
- **D-03:** Think-mode via `--think` — the classic thought bubble: rounded
  `( )` borders and an `o`-dot tether instead of the `\` speech tether. Reuse
  the existing `bubble()` builder with alternate border/tether glyphs.
- JSON stays the flat `{text}` (the bubble + figure is a visual, never
  serialized — A6); optionally add a `figure` field naming the selection.

### fortune (FORT-V2-01)
- **D-04:** Selectable categories with a small taxonomy: **wisdom / tech /
  humor**. `--category <name>` filters; `--list-categories` enumerates; **bare
  `box fortune` draws from ALL categories** (current behavior preserved). The
  corpus is reorganized to carry category membership (per-category files or a
  tagged format — planner's discretion), staying `include_str!`-embedded and
  LF-normalized. Unknown category → helpful error. JSON gains a `category`
  field.

### 8ball (8BAL-V2-01)
- **D-05:** ASCII-art presentation — a **compact ASCII 8-ball** rendered with
  the drawn answer, plus **sentiment color** mapped onto the existing 10/5/5
  tone partition: **affirmative → green, non-committal → yellow, negative →
  red**. The tone boundaries already exist in code
  (`EIGHT_BALL_ANSWERS[0..10] / [10..15] / [15..20]`), so classification is a
  pure index lookup — no new data. Color is **`is_color_on()`-gated** (D-00) so
  piped output is plain and the art stays byte-stable.
- JSON: the ASCII art is a visual (not serialized) — keep the flat `{text}` and
  add a derivable `sentiment: affirmative|non_committal|negative` field
  (scriptable).

### pomodoro (POMO-V2-01, POMO-V2-02)
- **D-06:** Auto-cycling is **opt-in**, not the default. Bare
  `box pomodoro [MINUTES]` stays a single blocking timer that exits (backward
  compatible — nothing scripting `box pomodoro` changes). `--cycles <N>` runs N
  work sessions with 5-min breaks between; `--loop` runs work/break
  indefinitely until Ctrl+C. Classic cadence: **every 4th break is the 15-min
  long break**, else a 5-min short break. A **session counter** shows in the
  countdown line (e.g. `Pomodoro 3/4`, or `Pomodoro #3` under `--loop`).
- **D-07:** `--label <text>` annotates the session — shown in the countdown
  line and carried into the completion toast (title/body).
- **D-08:** The auto-cycle loop **reuses the existing single-`RawGuard` +
  `event::poll` tick model** — one continuous raw-mode session across all
  sub-timers (arm the guard once, loop over work/break segments). Cancel
  (Ctrl+C / q / Esc) at any point restores the terminal and exits 1 with no
  toast/sound; each completed segment fires its toast/sound; completing the full
  cycle set exits 0.
- **D-09 (POMO-V2-02):** `--sound` plays a completion beep via Win32
  `MessageBeep` (the already-committed `windows 0.61` crate — D-2; zero audio
  stack). Fires on each session **completion only** (never on cancel — mirrors
  the toast rule). Composes with the toast (both may fire). The specific
  `MessageBeep` sound constant is Claude's discretion (e.g. `MB_OK`).
- `pomodoro` remains **display-only (SC4)** — no `--json` / `--clip`.

### weather (WTHR-V2-01)
- **D-10:** `--forecast` shows a **7-day daily forecast** (Open-Meteo daily
  endpoint: date, min/max temp, conditions), additive to the current-conditions
  block. Fixed 7-day span (chosen over a parameterized `--forecast [N]`).
- **D-11:** **Response cache** — cached ~**10 minutes** in
  `%LOCALAPPDATA%\box\cache\`, keyed by (location, units, forecast-or-not). A
  hit within TTL serves stored data with no network call. A cache read/parse/IO
  error or a stale entry is treated as a **MISS** (fetch fresh); the cache
  **never errors the command** (mirrors config's missing-file tolerance).
  Transparent (no flag required); a `--no-cache` escape hatch is optional at
  planner's discretion.
- **D-12:** **Stored default location** — the `location` positional becomes
  **optional**. Resolution: CLI positional > `weather.location` config. Bare
  `box weather` with a config location uses it; bare `box weather` with no
  positional AND no config location → a clear usage-style error (exit 2) telling
  the user to pass a location or set `weather.location`. `weather.units` config
  also participates: CLI `--units` > `weather.units` config > metric builtin
  (the settled SPINE-05 precedence shape).
- **D-13 (config schema):** Config moves to **nested TOML tables with dotted
  keys** — `[weather] location`, `[weather] units`, and **migrate** the Phase-6
  flat `default_hash_algo` → `[hash] default_algo`. This matches the dotted keys
  Phase 11's `box config get/set weather.location` will expose. Accept the
  one-time break to any hand-authored `config.toml` — better to converge the
  schema now than have Phase 11 paper over a mixed flat/nested layout. The
  `Config` struct grows nested sub-structs
  (`hash: HashConfig { default_algo }`, `weather: WeatherConfig { location,
  units }`), keeping `#[serde(default, deny_unknown_fields)]` and every field
  `Option<T>`.
- JSON: `--forecast --json` extends `WeatherOutput` with a `forecast:
  [{date, temp_min, temp_max, conditions}]` array; the current-only shape (no
  `--forecast`) is unchanged. Cache/stderr must never contaminate `--json`
  stdout.

### Claude's Discretion
- Exact cowsay figure roster, fortune category corpus split, and 8-ball art
  glyphs (keep small, legible, byte-stable, pure-ASCII).
- roast per-language content organization (files vs tags), the specific
  `MessageBeep` sound constant, pomodoro counter/label wording, and whether
  weather adds a `--no-cache` flag.
- Exact JSON field names for the new fields (`language`, `figure`, `category`,
  `sentiment`, `forecast`) — follow the frozen `snake_case` house style.

### Cross-cutting constraints (carried from prior phases — not re-decided)
- Every new colored/sentiment path stays **`is_color_on()`-gated** (D-00 / SC4):
  8ball sentiment, pomodoro digits + label.
- New data flows into **both** the human and `--json` paths via the frozen
  `is_json_on()` fork; `--json` stdout stays pure (no ANSI / BOM / progress /
  cache chatter). `pomodoro` stays display-only (no `--json`).
- Random picks keep `rand::rng()` + `IndexedRandom::choose` (never `% len`,
  never a fixed seed — D-08 from v1).
- Content stays `include_str!`-embedded, pure-ASCII, LF-normalized
  (`.gitattributes eol=lf`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external ADRs/specs exist beyond the `.planning/` corpus — the decisions
above plus these files fully define the phase.

### Phase scope & requirements
- `.planning/ROADMAP.md` §"Phase 10: Fun & System Depth" — goal, success
  criteria (SC1–SC4), the two provisional plan splits (10-01 fun / 10-02
  system).
- `.planning/REQUIREMENTS.md` §"Fun & system depth" — COW/FORT/8BAL/ROST-V2-01
  + POMO-V2-01/02 + WTHR-V2-01 wording; §"Milestone Decisions" D-2
  (`windows 0.61` GO — both features).
- `.planning/PROJECT.md` §"Key Decisions" + §"Current Milestone" — the toolkit
  ethos and the v2 scope boundary.
- `.planning/STATE.md` §"Accumulated Context" — the D-1..D-38 decision log,
  v2 pitfalls (JSON purity, config precedence, terminal RAII), and the
  architecture graft notes.

### Config schema (D-13 touches this + Phase 11 depends on it)
- `src/core/config.rs` — the current flat `default_hash_algo` `Config` struct,
  `config_path()` (`%APPDATA%` first), the `cli.or(env).or(cfg).unwrap_or()`
  precedence resolver, and the malformed→exit-2 / missing→default behavior to
  preserve under the nested-table migration.

### Command source (extend, don't rewrite)
- `src/commands/cowsay/mod.rs` — `wrap()` + `bubble()` (reuse for `--think`),
  the flat `{text}` JSON fork, the fixed-40-col / clamp-width rules.
- `src/commands/fortune/mod.rs` — `entries()` corpus loader,
  `IndexedRandom::choose` pick, soft-wrap, `{text}` JSON fork.
- `src/commands/roast/mod.rs` — identical shape to fortune (corpus loader +
  pick + `{text}` fork) — the `--language` bucket work mirrors here.
- `src/commands/eight_ball/mod.rs` — the 10/5/5 tone partition
  (`EIGHT_BALL_ANSWERS` index ranges) that sentiment color maps onto.
- `src/commands/pomodoro/mod.rs` — `RawGuard`, `resolve_duration()`,
  `event::poll` tick loop, `is_cancel()`, `MAX_MINUTES` overflow guard, the
  Press-only key filter, and the toast fire-on-completion path.
- `src/commands/weather/mod.rs` — `fetch()` (ureq 3.x status-as-error split),
  `geocode()`, `build_forecast_url()`, `WeatherOutput`, `current_units`
  authoritative-label rule, and the `BOX_WEATHER_BASE_URL` offline test seam.
- `src/data/{fortunes.txt,roasts.txt}` — the LF-normalized embedded corpora to
  reorganize (categories / language buckets).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`cowsay::bubble()` + `wrap()`** — the speech-bubble renderer; `--think`
  parameterizes its border/tether glyphs rather than adding a second renderer.
- **`eight_ball` tone partition** — the 20 answers are already grouped
  10 affirmative / 5 non-committal / 5 negative by index, so sentiment color is
  a free `match index` lookup, no new content.
- **`pomodoro::RawGuard` + `event::poll` tick loop** — the single-guard
  raw-mode + ~1s-tick model extends directly to a multi-segment cycle; arm the
  guard once and loop segments (no per-segment enable/disable).
- **`pomodoro::resolve_duration()`** — pure, unit-tested duration resolver;
  extend for the cycle/long-break cadence while keeping the `MAX_MINUTES`
  overflow guard.
- **`weather::fetch()` + `build_forecast_url()`** — reuse verbatim for the
  daily-forecast GET; `current_units` label-reading rule applies to any daily
  units too. The `BOX_WEATHER_BASE_URL` seam lets the cache + forecast be
  tested offline against a loopback fixture (the Phase-7 pattern).
- **`core::config` resolver** — the `Option<T>` + `.or().or().unwrap_or()`
  precedence chain is the template for `weather.location` / `weather.units`.
- **`core::output` spine** — `is_json_on()` / `emit_json` / `out_line` /
  `is_color_on()` are the frozen forks every command here already threads.

### Established Patterns
- **Scalar `--json` = flat `{…}` object** (A6/D-01): fun commands keep `{text}`
  and add small scalar fields (`language`, `category`, `sentiment`); weather
  extends its existing `WeatherOutput` with a `forecast` array.
- **`is_color_on()` gate** on every colored path (D-00) — piped/`--json` output
  is byte-identical minus ANSI.
- **Terminal RAII** (matrix/pomodoro CR-01 ordering): arm the guard the instant
  raw mode is on, before any fallible setup; `panic = "abort"` means the guard,
  not `.unwrap()`, is the restore path.
- **Config tolerance** (SPINE-05): missing config → silent default; malformed →
  exit 2; a normal command never errors on config. The **cache reuses this
  tolerance** — a bad/absent/stale cache entry is a miss, never an error.
- **Embedded content** via `include_str!` from `src/data/`, LF-forced by
  `.gitattributes`, trimmed defensively on load.

### Integration Points
- **`core/config.rs`** — nested-table migration (D-13) is the one shared-file
  change; `hash`'s compute-default resolver must keep working after
  `default_hash_algo` → `[hash] default_algo`, and `weather` reads
  `[weather] location`/`units`. This is the schema Phase 11's `box config`
  meta-command locks against.
- **`Cargo.toml`** — `windows 0.61` (D-2) is already a committed dep for
  `pomodoro --sound` `MessageBeep`; no new crate needed. Cache = `serde_json`
  + `std::fs` (already present).
- **`cli.rs`** — new per-command flags (`--figure`/`--list-figures`/`--think`,
  `--category`/`--list-categories`, `--language`, `--cycles`/`--loop`/`--label`/
  `--sound`, `--forecast`/optional `location`) all land on the existing arg
  structs; Phase 11 `completions` will generate against this final surface.

</code_context>

<specifics>
## Specific Ideas

- `roast --language python` → roasts targeting the Python ecosystem
  (indentation, `venv`, dependency hell); the general bucket is the no-flag
  default.
- `cowsay --think` mirrors classic `cowthink` — rounded `( )` bubble + `o`
  tether; `cowsay --figure dragon "..."` for the alternates.
- `8ball` renders a small ball with the answer shown, green/yellow/red by tone.
- `pomodoro --cycles 4 --label "deep work" --sound` → 4 work sessions with
  breaks, a beep + labeled toast per completion, single-timer behavior otherwise
  unchanged.
- `box weather` (no args) with `[weather] location = "London"` in config →
  current London conditions; `box weather --forecast` → 7-day outlook.

</specifics>

<deferred>
## Deferred Ideas

- **`box config` meta-command** (CFG-01) — get/set/show/path for the config
  keys, including the new `[weather] location`/`units` and migrated
  `[hash] default_algo`. Phase 11 (already scoped); D-13 here sets the schema it
  locks against.
- **`weather --forecast [N]`** parameterized day count and **hourly forecast** —
  out of scope; D-10 fixes a 7-day daily span for this phase.
- **`--undo` / interactive cache management** and **cowsay external `.cow`
  files** — rejected above to preserve the pure-ASCII / no-file-I/O ethos.
- **Spoken-language roast translation** — considered and rejected (D-01 chose
  the programming-language axis); revisit only if a localization milestone
  appears.

None of the above pulled Phase 10 out of scope — discussion stayed within the
seven requirements.

</deferred>

---

*Phase: 10-fun-system-depth*
*Context gathered: 2026-07-14*
