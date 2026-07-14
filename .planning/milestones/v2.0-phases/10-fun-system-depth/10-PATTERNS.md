# Phase 10: Fun & System Depth - Pattern Map

**Mapped:** 2026-07-14
**Files analyzed:** 11 (7 command/core source, 1 CLI registry, 2 data corpora, 1 new cache module)
**Analogs found:** 11 / 11 (all in-repo — this phase extends already-shipped commands, so every file's best analog is *itself* plus one cross-cutting spine source)

> **Framing:** Phase 10 grafts depth flags onto six shipped commands. There is no
> greenfield here — the *closest analog for each modified file is its own current
> body*, and the load-bearing patterns (JSON fork, color gate, config resolver,
> RawGuard) already live in the tree. The excerpts below are the exact seams the
> new flags plug into. `--skip-research` was intentional: CONTEXT.md D-01..D-13 is
> the source of truth, and the grounding is the live `src/` code, not RESEARCH.md.

---

## File Classification

| File | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `src/commands/cowsay/mod.rs` | command handler | human render + `--json` fork | itself (`bubble`/`wrap`) | exact (self-extend) |
| `src/commands/fortune/mod.rs` | command handler | random-pick + `--json` fork + embedded-data read | itself (`entries`) + `hash::Algo` enum | exact (self-extend) |
| `src/commands/roast/mod.rs` | command handler | random-pick + `--json` fork + embedded-data read | `fortune/mod.rs` (identical shape) | exact |
| `src/commands/eight_ball/mod.rs` | command handler | random-pick + `--json` fork + `is_color_on` gate | itself (tone partition) + `output::format_row` glyph-color | exact (self-extend) |
| `src/commands/pomodoro/mod.rs` | command handler | raw-mode tick loop (display-only, no JSON) | itself (`RawGuard`/`resolve_duration`/`is_cancel`) | exact (self-extend) |
| `src/commands/weather/mod.rs` | command handler | network GET + `--json` fork + config-read | itself (`fetch`/`build_forecast_url`) + `config` resolver | exact (self-extend) |
| `src/core/config.rs` | core resolver | config file read → process-global | itself (flat `Config` → nested tables) | exact (self-migrate) |
| `src/core/cache.rs` **(new)** | new core module | file-I/O read/write with miss-tolerance | `config.rs` (`load`/`config_path` tolerance) | role-match |
| `src/cli.rs` | CLI registry | dispatch only | itself | doc-only / likely no change |
| `src/data/fortunes.txt` (+split) | embedded data | `include_str!` corpus | itself + `.gitattributes eol=lf` | exact |
| `src/data/roasts.txt` (+split) | embedded data | `include_str!` corpus | `fortunes.txt` | exact |

> **cli.rs accuracy note for the planner:** the per-command `--figure`/`--category`/
> `--language`/`--cycles`/`--forecast` flags do **NOT** land in `cli.rs`. Each `Args`
> struct lives in its own `command/mod.rs` (e.g. `CowsayArgs` at
> `cowsay/mod.rs:52`); `cli.rs` only holds the `Commands` enum that *references*
> those structs (`cli.rs:41-120`). `cli.rs` changes only if a `///` long-help doc
> comment is updated (e.g. the pomodoro/weather help blocks at `cli.rs:102-119`).
> Treat `cli.rs` as no-code-change unless help text is revised.

---

## Shared Patterns (apply to every command file below)

### The `--json` fork (SPINE-01) — check FIRST, before any human-render logic
**Source:** `src/core/output.rs:109` (`is_json_on`), `:152` (`emit_json`)
**Apply to:** cowsay, fortune, roast, 8ball, weather (NOT pomodoro — display-only, SC4)

```rust
// The frozen fork every fun command already threads (cowsay/mod.rs:71-77):
if crate::core::output::is_json_on() {
    let doc = CowsayOutput { text: raw.trim().to_string() };
    crate::core::output::emit_json(&doc)?;
    return Ok(());
}
// ... human render below, never reached under --json
```
**How to extend:** add the new scalar field (`language`/`category`/`sentiment`/
`figure`) to the existing `*Output` struct; the visual (bubble, 8-ball art) is
NEVER serialized (A6). New fields follow frozen `snake_case`.

### The `is_color_on()` gate — every colored/sentiment path
**Source:** `src/core/output.rs:34` (`is_color_on`), live use at `output.rs:285-293`
(`format_row` colors ONLY the glyph) and `pomodoro/mod.rs:163-170` (colors digits)
**Apply to:** 8ball sentiment color, pomodoro digits+label

```rust
// output.rs:285 — the canonical "glyph is source of truth, color is decoration"
let glyph_str = if is_color_on() {
    match status {
        RowStatus::Copy => glyph.green().to_string(),
        RowStatus::Rename => glyph.yellow().to_string(),
        RowStatus::Skip => glyph.red().to_string(),
    }
} else {
    glyph.to_string()
};
```
**How to extend (8ball):** the plain answer text is the source of truth; wrap it in
`.green()/.yellow()/.red()` ONLY inside an `if is_color_on()` arm. `init_output`
(`output.rs:134-141`) already forces color OFF under `--json`/`--clip`, so the
byte-stable art needs no second guard.

### The config-precedence resolver — `cli.or(env).or(cfg).unwrap_or(builtin)`
**Source:** `src/core/config.rs:128-130` (`resolve_algo`), live chain at
`hash/mod.rs:301-308`
**Apply to:** `weather.location`, `weather.units`

```rust
// config.rs:128 — the pure resolver shape (CLI > env > config > builtin)
pub fn resolve_algo(cli: Option<Algo>, env: Option<Algo>, cfg: Option<Algo>) -> Algo {
    cli.or(env).or(cfg).unwrap_or(Algo::Blake3)
}
// hash/mod.rs:301 — the LIVE chain reading config().default_hash_algo
let algo = cli_algo
    .or_else(|| std::env::var("BOX_HASH_DEFAULT_ALGO").ok().and_then(|s| parse_algo(&s)))
    .or(crate::core::config::config().default_hash_algo)
    .unwrap_or(Algo::Blake3);
```
**How to extend (weather):** `--units` (CLI `Option`) `.or(weather.units config)`
`.unwrap_or(Units::Metric)`; location = `cli_positional.or(weather.location config)`
then a usage error (exit 2) if still `None` (D-12). NOTE: to make units resolve
through this shape, `WeatherArgs.units` must drop its clap `default_value_t` and
become `Option<Units>` (Anti-Pattern 3 — a `default_value` breaks the `None =
fall-through` contract; see the config.rs module doc `config.rs:11-15`).

### Embedded content via `include_str!` + LF lock
**Source:** `fortune/mod.rs:39` (`include_str!`), `fortune/mod.rs:85-91` (`entries`),
`.gitattributes` (`src/data/*.txt text eol=lf`)
**Apply to:** any reorganized/added `src/data` corpus

```rust
const FORTUNES_RAW: &str = include_str!("../../data/fortunes.txt");
pub(crate) fn entries() -> Vec<&'static str> {
    FORTUNES_RAW.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
}
```
**How to extend:** every new corpus file (per-category / per-language split) needs a
matching `.gitattributes` `eol=lf` line; the loader always trims defensively.

### Random pick — never `% len`, never a fixed seed
**Source:** `fortune/mod.rs:51-52`, `eight_ball/mod.rs:82-85`
```rust
let mut rng = rand::rng();               // OS-seeded ThreadRng (rand 0.9)
let chosen = *list.choose(&mut rng).expect("list is non-empty");
```
**Apply to:** category/language-filtered fortune/roast picks — filter the slice
FIRST, then `choose` over the filtered subset.

---

## Pattern Assignments

### `src/commands/cowsay/mod.rs` (COW-V2-01 — `--figure`/`--list-figures`/`--think`)

**Analog:** itself — `bubble()` and `wrap()` are the exact reuse targets (D-02/D-03).

**The bubble builder to parameterize** (`cowsay/mod.rs:142-176`):
```rust
fn bubble(lines: &[String]) -> String {
    let inner = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    if lines.len() <= 1 {
        let text = lines.first().map(String::as_str).unwrap_or("");
        let border: String = "_".repeat(inner + 2);
        let dashes: String = "-".repeat(inner + 2);
        format!(" {border}\n< {text} >\n {dashes}")   // <-- speech delimiters
    } else {
        // ... "/ \" top, "| line |" middles, "\ /" bottom
    }
}
```
**Embedded figure roster** — mirror the `COW` const shape (`cowsay/mod.rs:42-46`):
```rust
const COW: &str = r"        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||";
```
**How to extend:**
- `--think` (D-03): parameterize `bubble()` to take border/tether glyphs — rounded
  `( )` delimiters and the `o`-dot tether instead of `< >` and `\`. Do NOT add a
  second renderer; pass glyphs into the existing one.
- `--figure <name>` (D-02): a `const &[(&str, &str)]` name→ASCII-art table (same
  pure-ASCII, byte-stable rule the `COW` const already obeys). Select by name;
  unknown name → helpful error listing figures + exit 2. NO external `.cow` file
  I/O (rejected — D-02 / deferred).
- `--list-figures`: enumerate the roster names and return (mirror the fast-exit
  shape of the `--json` early-return at `cowsay/mod.rs:71-77`).
- JSON stays flat `{text}` (`CowsayOutput`, `cowsay/mod.rs:34-37`); optionally add a
  `figure` field.

---

### `src/commands/fortune/mod.rs` (FORT-V2-01 — `--category`/`--list-categories`)

**Analog:** itself (`entries` loader + `choose` pick + `{text}` fork) and the
`hash::Algo` `ValueEnum` (`hash/mod.rs:114-125`) for a `--category` enum.

**Corpus loader to extend** (`fortune/mod.rs:85-91`, shown in Shared Patterns).

**The `--json` fork to add a field to** (`fortune/mod.rs:57-63`):
```rust
if crate::core::output::is_json_on() {
    let doc = FortuneOutput { text: chosen.to_string() };
    crate::core::output::emit_json(&doc)?;
    return Ok(());
}
```
**ValueEnum shape for `--category`** (copy from `hash/mod.rs:114-125`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Algo { Sha256, Blake3, Sha512, Md5 }   // -> Category { Wisdom, Tech, Humor }
```
**How to extend:**
- Reorganize the corpus to carry category membership: either per-category files
  (`src/data/fortunes/{wisdom,tech,humor}.txt`, each `include_str!`-embedded) or a
  tagged single file (planner discretion). Add `.gitattributes eol=lf` for any new
  file.
- `entries()` gains a category filter; bare `box fortune` unions ALL categories
  (current behavior preserved — D-04). Filter the slice, THEN `choose`.
- `--list-categories` enumerates the taxonomy (fast-return like the JSON fork).
- Unknown `--category` → helpful error. Add a `category` field to `FortuneOutput`.

---

### `src/commands/roast/mod.rs` (ROST-V2-01 — `--language` programming-language buckets)

**Analog:** `fortune/mod.rs` — roast is byte-for-byte the same shape (`entries` at
`roast/mod.rs:75-81`, JSON fork at `:53-59`, `soft_wrap` at `:86-108`). Whatever the
category work does in fortune, the language-bucket work mirrors here.

**How to extend:**
- `--language <lang>` selects a **programming ecosystem** bucket (python/js/rust/…),
  NOT a spoken language; roasts stay English (D-01). A **general/default bucket** is
  used when the flag is omitted (preserves today's no-flag behavior).
- Reorganize `src/data/roasts.txt` into per-language buckets (files vs tags — planner
  discretion), additive to existing content. `.gitattributes eol=lf` per new file.
- Unknown `--language` → helpful error listing available languages. Add a `language`
  field to `RoastOutput` (`roast/mod.rs:28-31`).

---

### `src/commands/eight_ball/mod.rs` (8BAL-V2-01 — ASCII 8-ball art + sentiment color)

**Analog:** itself — the tone partition is already in code, so sentiment is a pure
index lookup with NO new data (D-05).

**The tone partition to map onto** (`eight_ball/mod.rs:38-62` is the 20-answer const;
boundaries pinned by the test at `:127-134`):
```rust
let affirmative = &EIGHT_BALL_ANSWERS[0..10];   // -> green
let non_committal = &EIGHT_BALL_ANSWERS[10..15]; // -> yellow
let negative = &EIGHT_BALL_ANSWERS[15..20];      // -> red
```
**The draw + JSON fork to extend** (`eight_ball/mod.rs:82-98`):
```rust
let answer = *EIGHT_BALL_ANSWERS.choose(&mut rng).expect("...non-empty");
if crate::core::output::is_json_on() {
    let doc = EightBallOutput { text: answer.to_string() };
    crate::core::output::emit_json(&doc)?;
} else {
    crate::core::output::out_line(answer);
}
```
**How to extend:**
- Recover the drawn answer's INDEX (use `choose` over indices, or find the position)
  so `match index { 0..10 => affirmative, 10..15 => non_committal, _ => negative }`
  gives the sentiment — a free lookup, no new content.
- Render a compact ASCII 8-ball around the answer (pure-ASCII, byte-stable, small —
  discretion). Color the answer/art green/yellow/red ONLY inside `if is_color_on()`
  (copy the glyph-color arm from `output.rs:285-293`), so piped output is
  byte-identical minus ANSI and `--json` stays plain.
- JSON: keep flat `{text}`, add a derivable `sentiment: affirmative|non_committal|
  negative` field to `EightBallOutput` (`eight_ball/mod.rs:30-33`).

---

### `src/commands/pomodoro/mod.rs` (POMO-V2-01/02 — `--cycles`/`--loop`/`--label`/`--sound`)

**Analog:** itself — the single-`RawGuard` + `event::poll` tick model extends
directly to a multi-segment cycle (D-08); `resolve_duration` extends for the
cadence.

**RawGuard RAII — arm ONCE, loop segments inside** (`pomodoro/mod.rs:114-122` +
`:139-144`):
```rust
struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, cursor::Show);
        let _ = disable_raw_mode();
    }
}
// run():
enable_raw_mode()?;
let _guard = RawGuard;          // armed the INSTANT raw mode is on (CR-01 ordering)
crossterm::execute!(out, cursor::Hide)?;
```
**The tick loop + cancel + completion spine** (`pomodoro/mod.rs:151-213`):
```rust
let cancelled = loop {
    let remaining = end.saturating_duration_since(Instant::now());
    if remaining.is_zero() { break false; }     // COMPLETION
    // ... redraw MM:SS in place, ONE flush per tick (color gated on is_color_on)
    let tick = remaining.min(Duration::from_secs(1));
    if event::poll(tick)? {
        if let Event::Key(key) = event::read()? {
            if is_cancel(&key) { break true; }  // CANCEL — exit 1, NO toast/sound
        }
    }
};
drop(_guard);                                   // restore BEFORE toast/message
if cancelled { eprintln!("cancelled"); std::process::exit(1); }
// COMPLETION path: fire toast (never fails the run)
```
**The cancel predicate (Press-only filter)** (`pomodoro/mod.rs:249-255`):
```rust
fn is_cancel(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press { return false; }   // Windows double-fire filter
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}
```
**The pure duration resolver + overflow guard** (`pomodoro/mod.rs:228-242`, consts at
`:66-77`):
```rust
const MAX_MINUTES: u64 = 24 * 60 * 366;    // overflow guard on mins*60
fn resolve_duration(minutes: Option<u64>, break_: bool, long_break: bool) -> Duration {
    let mins = match minutes { Some(m) => m, None if long_break => 15, None if break_ => 5, None => 25 };
    let mins = mins.min(MAX_MINUTES);      // keep this clamp
    Duration::from_secs(mins * 60)
}
```
**How to extend:**
- `--cycles <N>` / `--loop` (D-06): wrap the existing single-timer loop in an OUTER
  segment loop (work → break → work …), arming `RawGuard` ONCE before the outer loop
  (D-08). Bare `box pomodoro [MINUTES]` stays a single blocking timer (backward
  compatible). Classic cadence: every 4th break = 15-min long break, else 5-min.
- Session counter in the countdown line (`Pomodoro 3/4` or `#3` under `--loop`) and
  `--label <text>` (D-07) — both feed the same in-place redraw at
  `pomodoro/mod.rs:161-171` (color-gate the label like the digits).
- Cancel at any segment restores the terminal and exits 1 with NO toast/sound; each
  COMPLETED segment fires its toast/sound; full set → exit 0 (D-08).
- `--sound` (D-09/POMO-V2-02): Win32 `MessageBeep(MB_OK)` on completion ONLY (mirror
  the toast rule — after `drop(_guard)`, never on cancel). **Cargo.toml action:** the
  `windows 0.61` dep (`Cargo.toml:129`) currently enables only
  `Win32_Storage_FileSystem` + `Win32_Foundation`; `MessageBeep` lives in
  `windows::Win32::UI::WindowsAndMessaging`, so add the
  `Win32_UI_WindowsAndMessaging` feature. No new crate (D-2 already committed the
  windows dep).
- Stays display-only (SC4): `run()` NEVER calls `is_json_on`/`emit_json` — do not add
  a JSON path.

---

### `src/commands/weather/mod.rs` (WTHR-V2-01 — `--forecast` + cache + config-default location)

**Analog:** itself — `fetch()` is reused verbatim for the daily GET; `WeatherOutput`
extends with a `forecast` array; the `BOX_WEATHER_BASE_URL` seam tests it offline.

**The GET helper (ureq 3.x status-as-error split) — reuse as-is** (`weather/mod.rs:208-217`):
```rust
fn fetch<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    match ureq::get(url).call() {
        Ok(resp) => serde_json::from_reader(resp.into_body().into_reader())
            .context("parse weather response"),
        Err(ureq::Error::StatusCode(code)) => anyhow::bail!("weather service returned {code}"),
        Err(_) => anyhow::bail!("could not reach weather service (offline?)"),
    }
}
```
**The URL builder + offline test seam** (`weather/mod.rs:268-278`, seam at `:49-54`,
`:227-229`):
```rust
const BASE_URL_ENV: &str = "BOX_WEATHER_BASE_URL";   // overrides origin for offline tests
fn forecast_origin() -> String {
    std::env::var(BASE_URL_ENV).unwrap_or_else(|_| FORECAST_ORIGIN.to_string())
}
fn build_forecast_url(lat: f64, lon: f64, units: Units) -> String {
    let mut url = format!("{}/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m", forecast_origin());
    if matches!(units, Units::Imperial) {
        url.push_str("&temperature_unit=fahrenheit&wind_speed_unit=mph");
    }
    url
}
```
**The `--json` doc to extend** (`weather/mod.rs:160-176`) and the authoritative-label
rule (`weather/mod.rs:104-107` — read unit labels from `current_units`, NEVER
hardcode; the imperial wind label is `"mp/h"`, not `mph`).

**The args struct to make location optional** (`weather/mod.rs:74-81`):
```rust
pub struct WeatherArgs {
    pub location: String,        // -> Option<String> (D-12: CLI positional > weather.location config)
    #[arg(long, value_enum, default_value_t = Units::Metric)]
    pub units: Units,            // -> Option<Units> + drop default_value_t (so config can win, SPINE-05)
}
```
**How to extend:**
- `--forecast` (D-10): add `&daily=temperature_2m_max,temperature_2m_min,weather_code`
  to a forecast URL (mirror `build_forecast_url`), deserialize a new `daily` +
  `daily_units` struct pair (mirror `ForecastResp`/`CurrentUnits` at
  `weather/mod.rs:335-358`), and reuse `fetch()`. Fixed 7-day span. Read daily unit
  labels from `daily_units` (same authoritative-label rule). Add a `forecast:
  [{date, temp_min, temp_max, conditions}]` array to `WeatherOutput` — the current-
  only shape (no `--forecast`) stays unchanged. Add a matching daily fixture under
  `tests/fixtures/weather/` (existing: `forecast_metric.json`, `forecast_imperial.json`,
  `geocode_hit.json`, `geocode_no_match.json`).
- Optional location + config default (D-12): make `location` an `Option<String>`;
  resolve `cli.or(weather.location config)` then a usage-style error (exit 2, reuse
  `BoxError::MissingInput` shape or a new variant) if still `None`. Make `units` an
  `Option<Units>` resolving `cli.or(weather.units config).unwrap_or(Units::Metric)`
  via the config resolver shape (see Shared Patterns).
- Cache/stderr must NEVER contaminate `--json` stdout (the resolved-location echo is
  already stderr-only at `weather/mod.rs:94-97` — follow that discipline).

---

### `src/core/config.rs` (D-13 — nested TOML tables, migrate flat `default_hash_algo`)

**Analog:** itself — grow the flat `Config` into nested sub-structs while preserving
every tolerance behavior.

**The struct to migrate** (`config.rs:40-47`):
```rust
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub default_hash_algo: Option<Algo>,     // FLAT (Phase 6)
}
```
Target (D-13): nested sub-structs, every field `Option<T>`, `deny_unknown_fields`
kept:
```rust
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hash: HashConfig,        // [hash] default_algo
    pub weather: WeatherConfig,  // [weather] location, units
}
// HashConfig { default_algo: Option<Algo> }, WeatherConfig { location: Option<String>, units: Option<Units> }
```
**Tolerance behavior to PRESERVE** (`config.rs:77-92` `load`, `:108-117`
`config_path`): missing file → silent `Config::default()` (match `NotFound`, never
`exists()`-then-read); malformed/unknown-key → `BoxError::Config` → exit 2. The
`%APPDATA%`-env-var-FIRST path resolution (`config.rs:109-116`) stays.

**The live consumer to update** (`hash/mod.rs:307`):
```rust
.or(crate::core::config::config().default_hash_algo)   // -> config().hash.default_algo
```
**How to extend:**
- Add `HashConfig`/`WeatherConfig` sub-structs, each `#[derive(Default,
  Deserialize)]` + `#[serde(default, deny_unknown_fields)]`, all fields `Option<T>`.
- Migrate `hash`'s live read (`hash/mod.rs:307`) from `config().default_hash_algo` to
  `config().hash.default_algo`. This is the one shared-file change; accept the
  one-time break to hand-authored `config.toml` (D-13). Update the config unit tests
  (`config.rs:132-206`) to the nested TOML form (`[hash]\ndefault_algo = "sha256"`).
- This is the schema Phase 11's `box config get/set weather.location` locks against.

---

### `src/core/cache.rs` **(new)** — weather response cache (D-11)

**Analog:** `config.rs` — the cache reuses config's missing-file tolerance exactly: a
bad/absent/stale entry is a MISS (fetch fresh), NEVER an error (D-11).

**The tolerance pattern to copy** (`config.rs:77-92`):
```rust
match std::fs::read_to_string(&path) {
    Ok(s) => toml::from_str(&s).map_err(/* -> BoxError::Config */),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
    Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
}
```
**The path-resolution pattern to copy** (`config.rs:108-117` — env-var-FIRST, `dirs`
fallback):
```rust
fn config_path() -> Option<std::path::PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("box").join("config.toml"));
    }
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))
}
```
**How to extend:**
- Cache dir is `%LOCALAPPDATA%\box\cache\` (D-11): mirror `config_path` but read the
  `LOCALAPPDATA` env var first, `dirs::cache_dir()` as fallback. Env-var-first is
  load-bearing for per-process test isolation (see the `config_path` rationale
  `config.rs:98-107`).
- Serialize entries with `serde_json` + `std::fs` (already present — no new crate,
  CONTEXT Integration Points). Key by (location, units, forecast-or-not).
- TTL ~10 min: an entry older than TTL is a MISS. On ANY read/parse/IO error →
  treat as MISS and fetch fresh; the cache NEVER errors the command (unlike
  config's malformed→exit-2 — the cache is strictly best-effort, D-11).
- Cache chatter must never touch `--json` stdout. `--no-cache` escape hatch is
  optional (planner discretion).

---

### `src/data/{fortunes.txt, roasts.txt}` (corpus reorg)

**Analog:** the files themselves + the `.gitattributes eol=lf` locks already present
for both (`.gitattributes` pins `src/data/fortunes.txt` and `src/data/roasts.txt`).

Current: `fortunes.txt` = 70 lines (loader range-checks [50,150],
`fortune/mod.rs:127-133`); `roasts.txt` = 42 lines (range [30,80],
`roast/mod.rs:116-122`). One aphorism/roast per LF line, trimmed on load.

**How to extend:**
- Reorganize to carry category (fortune) / language bucket (roast) membership: either
  split into per-bucket files (`src/data/fortunes/wisdom.txt`, `.../fortunes/tech.txt`,
  `.../roasts/python.txt`, …) each `include_str!`-embedded, or one tagged file
  (planner discretion, D-01/D-04).
- **Every new data file needs a `.gitattributes` `text eol=lf` line** — this is the
  root-cause fix for the `\r`-on-CRLF-checkout hazard (the loaders trim defensively,
  but eol=lf is the real guard). Keep content pure-ASCII, additive to existing.
- If per-bucket files are used, the count-range unit tests
  (`fortune/mod.rs:127-133`, `roast/mod.rs:116-122`) must be updated to the new
  per-bucket or total counts.

---

## No Analog Found

None. Every file in scope extends existing, shipped code — the closest analog is
always in-repo (usually the file itself). The one net-new file (`src/core/cache.rs`)
has a strong role-match analog in `src/core/config.rs` (file-read-with-tolerance +
`%APPDATA%`-style path resolution), so the planner does NOT need to invent a pattern.

---

## Metadata

**Analog search scope:** `src/core/` (output, config, errors), `src/commands/`
(cowsay, fortune, roast, eight_ball, pomodoro, weather, hash), `src/cli.rs`,
`src/main.rs`, `src/data/`, `tests/fixtures/weather/`, `Cargo.toml`,
`.gitattributes`.
**Files scanned:** 13 source/config files read in full + data corpora inspected.
**Pattern extraction date:** 2026-07-14

## PATTERN MAPPING COMPLETE
