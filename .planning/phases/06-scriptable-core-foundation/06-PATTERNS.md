# Phase 6: Scriptable-Core Foundation - Pattern Map

**Mapped:** 2026-06-25
**Files analyzed:** 8 (1 new src + 1 new test + 6 modified)
**Analogs found:** 8 / 8 (every file has an in-repo analog — this phase invents no new pattern)

> Every spine addition mirrors an existing, tested idiom in the v1 base. The
> single most-reused analog is `src/core/output.rs`'s `COLOR_ON` triad
> (`AtomicBool` + `init_color()` + `is_color_on()`) — clone it three times. The
> second is `src/commands/clip/mod.rs`'s single-shot main-thread `arboard`
> discipline — reuse it verbatim in `flush_clip()`. The planner/executor should
> copy from the cited line ranges, not invent.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/core/config.rs` (NEW) | config / provider (process-global) | file-I/O → load-once | `src/core/output.rs` (COLOR_ON OnceLock-style process-global) | role-match (atomic→OnceLock) |
| `src/core/output.rs` (MOD) | utility / output hub | request-response + transform | itself — the `COLOR_ON` triad (lines 28-59) | exact (self-mirror) |
| `src/cli.rs` (MOD) | config / arg-surface | declarative | the `no_color` global flag (lines 22-24) | exact |
| `src/main.rs` (MOD) | entrypoint / dispatcher | request-response | the `init_color` call (line 72) + exit-2 downcast (lines 114-120) | exact |
| `src/core/errors.rs` (MOD) | model / error enum | n/a | `MissingInput` / `UnsupportedHashLength` variants (lines 18-30) | exact |
| `src/core/mod.rs` (MOD) | config / module registry | declarative | existing `pub mod` lines (8-11) | exact |
| `src/commands/uuid/mod.rs` (MOD) | command / transform | request-response (no input) | itself — the `println!` loop (lines 27-34) | exact (self) + `hash` for the serde-fork shape |
| `src/commands/hash/mod.rs` (MOD) | command / transform | streaming (file/stdin → digest) | itself — `Algo` enum (63-73), compute branch (161-166), verify branch (135-158) | exact (self) |
| `tests/config.rs` (NEW) | test / integration | black-box process spawn | `tests/hash.rs` + `tests/clip.rs` (assert_cmd style) | exact |
| `tests/uuid.rs`, `tests/hash.rs` (MOD) | test / integration | black-box process spawn | `tests/hash.rs` (lines 23-34 helper, 122-139 autodetect) | exact (self) |

---

## Pattern Assignments

### `src/core/output.rs` (utility, MODIFIED) — the integration hub

**Analog:** itself — the `COLOR_ON` triad it already contains. Clone it for `JSON_ON` and `CLIP_ON`.

**The triad to clone** (`src/core/output.rs:28-59`):
```rust
// EXISTING — the exact template. Three pieces: static atomic, is_*_on() reader, init_* writer.
static COLOR_ON: AtomicBool = AtomicBool::new(false);

pub fn is_color_on() -> bool {
    COLOR_ON.load(Ordering::Relaxed)
}

pub fn init_color(no_color_flag: bool) {
    let on = color_enabled(no_color_flag);
    COLOR_ON.store(on, Ordering::Relaxed);
    owo_colors::set_override(on);   // <-- this is the force-color mechanism to reuse in init_output
}
```
Existing imports already present at top of file (`src/core/output.rs:14-17`) — `JSON_ON`/`CLIP_ON` need **no new import** for the atomics; `CLIP_BUF` needs `use std::sync::Mutex;` and `emit_json` needs `use std::io::Write;` (already imported only as `IsTerminal` — add `Write` or fully-qualify).

**What to ADD (mirror the triad ×2 + the clip/json helpers):**
```rust
static JSON_ON: AtomicBool = AtomicBool::new(false);
static CLIP_ON: AtomicBool = AtomicBool::new(false);
static CLIP_BUF: Mutex<String> = Mutex::new(String::new());

pub fn is_json_on() -> bool { JSON_ON.load(Ordering::Relaxed) }

/// Mirror init_color: lift the two global bools into atomics ONCE in main().
/// MUST run AFTER init_color (Pitfall 7) so its force-off wins.
pub fn init_output(json: bool, clip: bool) {
    JSON_ON.store(json, Ordering::Relaxed);
    CLIP_ON.store(clip, Ordering::Relaxed);
    if json || clip {
        // Reuse the EXACT mechanism init_color uses (line 54 + 58):
        COLOR_ON.store(false, Ordering::Relaxed);
        owo_colors::set_override(false);
    }
}

/// One serializer for every --json command (no-drift guarantee). Raw serde,
/// never .green() — no ANSI ever reaches stdout (Pitfall 1). No BOM (to_writer),
/// single trailing newline (D-03).
pub fn emit_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value)
        .context("serializing --json output")?;     // needs `use anyhow::Context;`
    out.write_all(b"\n")?;
    if CLIP_ON.load(Ordering::Relaxed) {             // D-08: --json --clip tees the doc
        let s = serde_json::to_string_pretty(value)?;
        CLIP_BUF.lock().unwrap().push_str(&s);
    }
    Ok(())
}

/// THE primary-output print primitive (replaces println! in --clip-capable
/// commands). Tees the string to CLIP_BUF when --clip is on.
pub fn out_line(s: &str) {
    println!("{s}");
    if CLIP_ON.load(Ordering::Relaxed) {
        let mut b = CLIP_BUF.lock().unwrap();
        b.push_str(s);
        b.push('\n');
    }
}

/// Called ONCE in main() AFTER a successful dispatch. Single-shot main-thread
/// arboard — reuse clip/mod.rs:108-109 verbatim (see that analog below).
pub fn flush_clip() -> anyhow::Result<()> {
    if !CLIP_ON.load(Ordering::Relaxed) { return Ok(()); }
    let text = CLIP_BUF.lock().unwrap();
    if text.trim_end().is_empty() { return Ok(()); }     // D-08: empty → no-op, no confirmation
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text.trim_end().to_string()).context("write clipboard")?;   // D-07: single trim_end
    if std::io::stderr().is_terminal() {                  // D-08: stderr-only, TTY-gated
        eprintln!("Copied to clipboard");                 // exact wording = planner's discretion (D-08)
    }
    Ok(())
}
```
> **Color-gate-on-stderr note:** the existing `color_enabled` gates on `std::io::stdout().is_terminal()` (line 43). The `flush_clip` confirmation must gate on **`stderr().is_terminal()`** instead (D-08), because the confirmation is a stderr diagnostic — `box uuid --clip 2>log` must not write it into the log.

**Test-serialization gotcha to copy:** the existing test module uses a `static COLOR_LOCK: Mutex<()>` (lines 252-255) to serialize tests that mutate the process-global atomic under the parallel runner. Any new in-file unit test that stores `JSON_ON`/`CLIP_ON` must take the same lock discipline.

---

### `src/core/config.rs` (config provider, NEW)

**Analog:** `src/core/output.rs` for the **process-global "decide once at startup" shape** (the `COLOR_ON` atomic + `init_color` writer + reader-fn), promoted from `AtomicBool` to `OnceLock<Config>`. There is no existing config module — this is the one genuinely new file, but its *shape* is the output-module shape.

**Idiom mapping (output.rs → config.rs):**
| output.rs (analog) | config.rs (new) |
|--------------------|-----------------|
| `static COLOR_ON: AtomicBool` | `static CONFIG: OnceLock<Config>` |
| `pub fn is_color_on() -> bool` | `pub fn config() -> &'static Config` |
| `pub fn init_color(flag)` (called once in main) | `pub fn init_config() -> anyhow::Result<()>` (called once in main, BEFORE init_color) |

**Code to write** (shape verified against RESEARCH Pattern 4 + ARCHITECTURE Q2):
```rust
use std::sync::OnceLock;

/// Config-overridable settings. Every field is Option<T> with NO clap
/// default_value (Anti-Pattern 3) so `Some` = user/file set it, `None` = fall
/// through. #[serde(default, deny_unknown_fields)]: missing key → None (silent),
/// unknown key → loud error (D-10 family, exit 2).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub default_hash_algo: Option<crate::commands::hash::Algo>,
    // Phase-6 LEAN scope (Claude's Discretion): ONLY default_hash_algo.
    // Doc-note: the struct grows one field per command as Phase 7+ adopts the spine.
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get().expect("init_config() must run before config()")
}

/// Load once in main() BEFORE dispatch (and before init_color, per main ordering).
/// Missing file = silent Config::default() (SPINE-05); malformed = BoxError::Config → exit 2 (D-10).
pub fn init_config() -> anyhow::Result<()> {
    let _ = CONFIG.set(load()?);
    Ok(())
}

fn load() -> anyhow::Result<Config> {
    let Some(path) = config_path() else { return Ok(Config::default()) };
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).map_err(|e| {
            crate::core::errors::BoxError::Config {
                path: path.display().to_string(),
                message: e.to_string(),
            }
            .into()
        }),
        // TOCTOU-free missing-file handling (Don't-Hand-Roll: match NotFound, never exists()-then-read).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))   // %APPDATA%\box\config.toml
}
```
> **`Algo` must round-trip:** `Config.default_hash_algo: Option<Algo>` requires `Algo` to be `Deserialize` (config read) — wired in the `hash/mod.rs` task below. The lowercase TOML value `default_hash_algo = "sha256"` deserializes via `#[serde(rename_all = "lowercase")]`.
> **New deps:** `toml = "1.1.2"`, `dirs = "6.0.0"` in `Cargo.toml` (the ONLY manifest change this phase). `dirs::config_dir()` is the locked form; `std::env::var_os("APPDATA")` is the documented fallback only.

---

### `src/cli.rs` (arg-surface, MODIFIED)

**Analog:** the existing `no_color` global flag on `Cli` (`src/cli.rs:22-24`).

**The flag to mirror** (`src/cli.rs:21-28`):
```rust
pub struct Cli {
    /// Disable colored output (also honors NO_COLOR and non-TTY stdout)
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}
```

**What to ADD** (two sibling global bools, same `#[arg(long, global = true)]` form):
```rust
    /// Emit machine-readable JSON on stdout (suppresses color/progress)
    #[arg(long, global = true)]
    pub json: bool,

    /// Copy the command's output to the clipboard (also prints it)
    #[arg(long, global = true)]
    pub clip: bool,
```
> **Scope guard:** do NOT add `Completions`/`Config` variants to the `Commands` enum — those are Phase 11. Only the two global flags land here.

---

### `src/main.rs` (entrypoint, MODIFIED)

**Analog:** the existing `init_color(cli.no_color)` call (`src/main.rs:72`) and the typed-`BoxError`→exit-2 downcast (`src/main.rs:114-120`).

**The init call-site to mirror** (`src/main.rs:69-72`):
```rust
crate::core::output::init_color(cli.no_color);
```
**What to ADD around it (ORDER IS LOAD-BEARING — Pitfall 7):**
```rust
// 1. config FIRST (its Err must reach the exit-2 downcast; see below).
crate::core::config::init_config()?;   // or fold into `result` — see ordering note
// 2. color (unconditional store).
crate::core::output::init_color(cli.no_color);
// 3. output LAST so its json||clip force-off overwrites color's TTY decision.
crate::core::output::init_output(cli.json, cli.clip);
```
> **`init_config()` Err must reach exit-2 (D-10):** `init_config()` returns `anyhow::Result` BEFORE the dispatch `match`. To map a malformed-config error to exit 2 via the existing downcast, fold its `Err` into the same `result` value the dispatch produces (recommended: keep `main`'s `match result` the single exit-code owner). Verified structure: `src/main.rs:74-122`.

**The exit-2 downcast to extend** (`src/main.rs:114-120`):
```rust
// EXISTING
match e.downcast_ref::<crate::core::errors::BoxError>() {
    Some(
        crate::core::errors::BoxError::MissingInput
        | crate::core::errors::BoxError::UnsupportedHashLength { .. },
    ) => ExitCode::from(2),
    _ => ExitCode::from(1),
}
```
**What to CHANGE — add `Config` to the exit-2 arm:**
```rust
        | crate::core::errors::BoxError::UnsupportedHashLength { .. }
        | crate::core::errors::BoxError::Config { .. },   // NEW (D-10)
```

**The `flush_clip()` call** — add AFTER a successful dispatch, BEFORE the exit-code map. The existing dispatch result is `let result = match cli.command { … };` (lines 74-98) and `Ok(()) => ExitCode::SUCCESS` (line 101). `flush_clip()` runs only on the `Ok` branch (D-08 — a failed command must never clobber the clipboard).

---

### `src/core/errors.rs` (error enum, MODIFIED)

**Analog:** the existing `MissingInput` / `UnsupportedHashLength` variants (`src/core/errors.rs:18-30`) — both are `#[error("…")]` thiserror variants that `main()` downcasts to exit 2.

**The variant pattern to mirror** (`src/core/errors.rs:21-30`):
```rust
    #[error("unsupported --verify hash length: {len} (expected 32/64/128 hex)")]
    UnsupportedHashLength { len: usize },
```

**What to ADD** (a struct-variant carrying `path` + `message`, matching the `config.rs` constructor above):
```rust
    /// A corrupt/unparseable %APPDATA%\box\config.toml (or an unknown key under
    /// deny_unknown_fields). Aborts BEFORE the operation runs → exit 2 (D-10),
    /// joining MissingInput/UnsupportedHashLength in the main() downcast. A
    /// MISSING file is NOT this error — it falls back to Config::default() silently.
    #[error("config error in {path}: {message}")]
    Config { path: String, message: String },
```

---

### `src/core/mod.rs` (module registry, MODIFIED)

**Analog:** the existing `pub mod` lines (`src/core/mod.rs:8-11`).
```rust
pub mod errors;
pub mod fs;
pub mod input;
pub mod output;
```
**What to ADD** (one line, keep alphabetical — `config` goes first):
```rust
pub mod config;
```

---

### `src/commands/uuid/mod.rs` (command, MODIFIED) — first --json/--clip consumer

**Analog:** itself — the `println!`-per-line loop (`src/commands/uuid/mod.rs:27-34`) — plus the serde-fork shape from `hash` below. The pure `format_one` helper (lines 40-47) already exists and stays the human renderer.

**The loop to fork** (`src/commands/uuid/mod.rs:27-34`):
```rust
fn run(self) -> anyhow::Result<()> {
    for _ in 0..self.count {
        println!("{}", format_one(uuid::Uuid::new_v4(), self.upper));
    }
    Ok(())
}
```

**What to CHANGE — build the struct once, fork on `is_json_on()`, route human path through `out_line`** (D-01/D-02: always-wrapped `{results:[…],count:N}` even for N=1):
```rust
#[derive(serde::Serialize)]
struct UuidRow { uuid: String, version: &'static str }     // version: "v4" (D-03 field names)
#[derive(serde::Serialize)]
struct UuidOutput { results: Vec<UuidRow>, count: usize }

fn run(self) -> anyhow::Result<()> {
    let rows: Vec<UuidRow> = (0..self.count)
        .map(|_| UuidRow {
            uuid: format_one(uuid::Uuid::new_v4(), self.upper),   // SAME pure fn feeds both paths
            version: "v4",
        })
        .collect();
    if crate::core::output::is_json_on() {                  // is_json_on() checked FIRST (Pitfall 1)
        let doc = UuidOutput { count: rows.len(), results: rows };
        crate::core::output::emit_json(&doc)?;
    } else {
        for r in &rows {
            crate::core::output::out_line(&r.uuid);        // out_line, not println! (clip tee)
        }
    }
    Ok(())
}
```
> **Pilot literal to hit exactly (D-02):** `box uuid` → `{"results":[{"uuid":"…","version":"v4"}],"count":1}`; `box uuid -n 5` → same `{uuid,version}` element ×5, `"count":5`.

---

### `src/commands/hash/mod.rs` (command, MODIFIED) — second consumer + BLAKE3 flip + verify probe

**Analog:** itself. Four precise edit points, all verified against the current source.

**1. `Algo` enum — add Serialize/Deserialize + lowercase rename** (`src/commands/hash/mod.rs:63-73`):
```rust
// CURRENT
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Algo { Sha256, Blake3, Sha512, Md5 }
```
```rust
// CHANGE TO — round-trips config (Deserialize) + --json (Serialize); spellings already match ValueEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algo { Sha256, Blake3, Sha512, Md5 }
```

**2. Compute-default flip (D-04 breaking change)** — the no-verify branch (`src/commands/hash/mod.rs:161-166`):
```rust
// CURRENT (line 162)
let algo = self.algo.unwrap_or(Algo::Sha256);
let computed = digest_reader(algo, input.reader)?;
println!("{computed}  {label}");
```
```rust
// CHANGE TO — config tier inserted, builtin flipped to Blake3; fork human/JSON; out_line for clip tee
let algo = self.algo
    .or_else(|| std::env::var("BOX_HASH_DEFAULT_ALGO").ok().and_then(|s| /* parse → Algo */))
    .or(crate::core::config::config().default_hash_algo)    // config file tier
    .unwrap_or(Algo::Blake3);                               // NEW builtin (D-04)
let computed = digest_reader(algo, input.reader)?;
if crate::core::output::is_json_on() {
    let doc = HashOutput { count: 1, results: vec![HashRow { path: label.clone(), algo, digest: computed }] };
    crate::core::output::emit_json(&doc)?;
} else {
    crate::core::output::out_line(&format!("{computed}  {label}"));   // keeps the two-space coreutils row (D-01)
}
```
With the output structs (D-02/D-03 — `path`, `algo`, `digest`; always-wrapped `{results,count}`):
```rust
#[derive(serde::Serialize)]
struct HashRow { path: String, algo: Algo, digest: String }   // algo serializes lowercase
#[derive(serde::Serialize)]
struct HashOutput { results: Vec<HashRow>, count: usize }
```
> **Env-tier spelling = Claude's Discretion:** `BOX_HASH_DEFAULT_ALGO` (uppercase `BOX_<SECTION>_<KEY>`) is the suggested convention. The env value is a string needing the same lowercase→`Algo` parse the config uses — reuse ONE parse fn (e.g. a `FromStr`/`ValueEnum::from_str` helper) for both env and config.

**3. Verify table `algo_from_len` — UNCHANGED (the #1 v2 data-risk backstop, D-04/Pitfall 5)** (`src/commands/hash/mod.rs:78-85`):
```rust
// LEAVE BYTE-FOR-BYTE AS-IS — 32→md5, 64→sha256, 128→sha512. Do NOT add a blake3 arm.
fn algo_from_len(len: usize) -> Result<Algo, BoxError> {
    match len {
        32 => Ok(Algo::Md5),
        64 => Ok(Algo::Sha256),     // 64 stays sha256 — never blake3
        128 => Ok(Algo::Sha512),
        len => Err(BoxError::UnsupportedHashLength { len }),
    }
}
```
> `tests/hash.rs::hash_verify_autodetect` (lines 122-139) pins this. It MUST pass unchanged after the flip — if it needs editing, that is the regression red flag.

**4. D-05 BLAKE3-probe on the verify-mismatch `bail!`** (`src/commands/hash/mod.rs:153-157`):
```rust
// CURRENT — the mismatch branch
} else {
    bail!("hash mismatch for {label}: expected {expected}, got {computed}");
}
```
```rust
// CHANGE TO — probe blake3 when 64-hex + no explicit --algo + not --json; stderr-only hint, exit STAYS 1
} else {
    if expected.len() == 64 && self.algo.is_none() && !crate::core::output::is_json_on() {
        // Re-read caveat: input.reader is single-pass (Box<dyn Read>, consumed by digest_reader).
        // For a real PATH, re-open via read_file_or_stdin(Some(path)) to compute blake3.
        // For piped stdin (label == "-") there is NO second read → degrade to the static hint.
        let hint = /* probe_blake3_matches(path) ? decisive hint : static --algo blake3 hint */;
        eprintln!("{hint}");   // stderr-only; styling gated on is_color_on() (D-05)
    }
    bail!("hash mismatch for {label}: expected {expected}, got {computed}");
}
```
> **Re-read caveat (load-bearing):** `ResolvedInput.reader` is `Box<dyn Read>` (verified `src/core/input.rs:121-127`) and is consumed by `digest_reader` (line 149). The probe needs the bytes a SECOND time — re-open the path via `read_file_or_stdin(Some(path.clone()))`. When the source is stdin (`label == "-"`), it cannot be re-read → the probe degrades to the static `--algo blake3` hint. Make the "probe only when a real path is available" condition explicit. Exit code stays **1** on mismatch (the existing `bail!` → plain anyhow → exit 1, unchanged).

---

### `tests/config.rs` (integration test, NEW)

**Analog:** `tests/hash.rs` and `tests/clip.rs` — black-box `assert_cmd` style.

**Helper + assertion shape to copy** (`tests/hash.rs:23-34` + `tests/clip.rs:124-148`):
```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

// Mirror tests/hash.rs:24-34: build the binary, force NO_COLOR for determinism.
// For config tests, point the config dir at a temp location. dirs::config_dir()
// reads %APPDATA% on Windows, so set APPDATA to a TempDir to isolate the test
// (deterministic, no real user config interference).
```

**Tests to write (the reusable Phase-7 template, D-10 + SPINE-05):**
1. **Precedence — CLI beats config:** `--algo sha256` + a config with `default_hash_algo = "blake3"` → SHA-256 digest (Pitfall 3). Mirror `tests/hash.rs::hash_algo_blake3` (62-70) for the digest-row assertion.
2. **Precedence — config beats builtin:** config `default_hash_algo = "sha256"`, no `--algo` → SHA-256 (recovers v1 behavior).
3. **Missing config → silent default:** no config file present → `box uuid` still prints a UUID, exit 0, no stderr (SPINE-05). Mirror `tests/clip.rs` exit-code + empty-stderr assertions (131-147).
4. **Malformed config → exit 2:** invalid TOML (or an unknown key under `deny_unknown_fields`) → `box uuid` exits **2** with a stderr `error:` (D-10). Mirror `tests/hash.rs::hash_verify_bad_len_exit2` (180-186) `.failure().code(2)`.

> **APPDATA isolation:** set `cmd.env("APPDATA", temp_dir)` so `dirs::config_dir()` resolves into the test's TempDir — never the developer's real `%APPDATA%\box\config.toml`. This is the env-override discipline `tests/hash.rs` already uses for `NO_COLOR` (line 32).

---

### `tests/uuid.rs` & `tests/hash.rs` (integration tests, MODIFIED)

**Analog:** `tests/hash.rs` — the existing helper (23-34), the autodetect regression test (122-139), and the bad-len exit-2 test (179-187).

**JSON-purity test (the copy-me template for all 23 Phase-7 commands):**
```rust
// Mirror RESEARCH "pilot JSON literals as test vectors". Run `box uuid --json`,
// then assert stdout is EXACTLY one JSON value + the D-01/D-02 shape + purity.
let out = /* Command::cargo_bin("box").args(["uuid","--json"]).env("NO_COLOR","1").output() */;
let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is one JSON value");
assert!(v.get("results").is_some() && v.get("count") == Some(&serde_json::json!(1)));
assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --json stdout");      // Pitfall 1
assert_ne!(&out.stdout[..3.min(out.stdout.len())], b"\xEF\xBB\xBF", "no UTF-8 BOM");  // Pitfall 2
```
**hash-specific tests to ADD:**
- **BLAKE3-default** (D-04): `box hash <file>` with no `--algo` and no config → 64-hex BLAKE3 digest (use `BOX_BLAKE3` vector, line 21). Currently `tests/hash.rs::hash_default_sha256` (51-59) asserts SHA-256 — that test must be UPDATED to expect BLAKE3 (or pair with an explicit config override). This is the one existing test whose expectation legitimately changes.
- **verify-stays-sha256** (D-04/Pitfall 5): `hash_verify_autodetect` (122-139) MUST still pass unchanged — it is the regression backstop.
- **probe-hint** (D-05): a 64-hex BLAKE3 digest verified with no `--algo` against a file whose BLAKE3 matches → exit 1 AND a stderr hint mentioning `--algo blake3`.

---

## Shared Patterns

### Process-global flag → atomic → `is_*_on()` (the spine's load-bearing idiom)
**Source:** `src/core/output.rs:28-59` (the `COLOR_ON` triad).
**Apply to:** `core::output` (`JSON_ON`, `CLIP_ON`), and analogously `core::config` (`OnceLock<Config>` + `config()` + `init_config`).
```rust
static COLOR_ON: AtomicBool = AtomicBool::new(false);
pub fn is_color_on() -> bool { COLOR_ON.load(Ordering::Relaxed) }
pub fn init_color(no_color_flag: bool) { /* store once in main */ }
```
**Why:** zero per-command field, zero `RunCommand::run(self)` churn — commands consult a fn, never a parsed arg. Mirrors the proven `no_color` precedent exactly.

### Single-shot main-thread arboard discipline
**Source:** `src/commands/clip/mod.rs:108-109` (+ the doc comment at lines 22-36).
**Apply to:** `flush_clip()` in `core::output` — verbatim.
```rust
let mut cb = arboard::Clipboard::new().context("open clipboard")?;  // `let mut` — set_text takes &mut self
cb.set_text(text).context("write clipboard")?;                      // synchronous; persists past exit on Windows
```
**Why:** satisfies the "arboard main-thread only" pitfall — `flush_clip()` runs in `main` on the main thread, after `Ok(())`, never on a worker thread. Reuses the exact Win32 flow proven in `clip`.

### One `#[derive(Serialize)]` struct feeds human + JSON (no-drift)
**Source:** v1 "thin orchestrator over a pure function" — `uuid::format_one` (40-47) and `hash::digest_reader` (119-126) already return the values; just give them named-field structs.
**Apply to:** `uuid` (`UuidRow`/`UuidOutput`) and `hash` (`HashRow`/`HashOutput`), and every Phase-7 command.
**Why:** the human render and `emit_json` read the SAME struct → the JSON can never report a different digest than the human row (Anti-Pattern 2).

### Black-box integration-test scaffold
**Source:** `tests/hash.rs:23-34` (the `hash_cmd` helper + `NO_COLOR` determinism) and `tests/clip.rs:124-148` (exit-code + empty-stdout + no-panic assertions).
**Apply to:** `tests/config.rs` (new), `tests/uuid.rs`/`tests/hash.rs` (the JSON-purity + clip-capture additions).
```rust
let mut cmd = Command::cargo_bin("box").unwrap();
cmd.arg("hash").env("NO_COLOR", "1");      // determinism: ANSI must never leak into the asserted string
cmd.assert().success().code(0).stdout(predicate::str::contains(/* … */));
```

### Typed `BoxError` variant → exit-2 downcast
**Source:** `src/core/errors.rs:18-30` (variants) + `src/main.rs:114-120` (downcast).
**Apply to:** `BoxError::Config` (new) joins the exit-2 arm; `core::config::load` constructs it on malformed TOML.
**Why:** one exit-code owner (`main`), one downcast list. A plain `anyhow::bail!` (e.g. hash mismatch) stays exit 1; only the listed typed variants are exit 2.

---

## No Analog Found

None. Every Phase-6 file maps to an in-repo analog (`config.rs` is the only new module, but its process-global "decide once at startup" shape is `output.rs`'s shape promoted from `AtomicBool` to `OnceLock`). The two new crates (`toml`, `dirs`) are mechanical: `toml::from_str` is the serde deserialize already used by `serde_json`, and `dirs::config_dir()` is a one-call path resolver.

## Metadata

**Analog search scope:** `src/core/` (output, errors, mod, input), `src/cli.rs`, `src/main.rs`, `src/commands/{uuid,hash,clip}/mod.rs`, `tests/{hash,clip}.rs`.
**Files read in full:** 10 source/test files + 3 planning docs (CONTEXT, RESEARCH, ARCHITECTURE).
**Key verified line refs:** output.rs:28-59 (COLOR_ON triad), 252-255 (test lock); clip/mod.rs:108-109 (arboard), 22-36 (discipline doc); hash/mod.rs:63-73 (Algo), 78-85 (algo_from_len UNCHANGED), 161-166 (compute flip), 153-157 (verify bail!); cli.rs:22-24 (no_color global); main.rs:72 (init_color), 114-120 (exit-2 downcast); errors.rs:18-30 (variants); input.rs:121-127 (single-pass reader → D-05 caveat); tests/hash.rs:23-34, 122-139, 180-186.
**Pattern extraction date:** 2026-06-25
