# Architecture Research — v2.0 "Toolbox → Toolkit" Integration

**Domain:** Cross-cutting capability integration into a shipped single-binary multi-subcommand Rust CLI (Windows PowerShell 7)
**Researched:** 2026-06-24
**Confidence:** HIGH (recommendations checked against the real v1 source: `src/cli.rs`, `src/main.rs`, `src/core/output.rs`, `src/core/input.rs`, `src/commands/{uuid,hash,clip,tree,du,weather}/mod.rs`; clap/clap_complete behavior verified via Context7 + docs.rs)

> This document **supersedes** the v1 ARCHITECTURE.md (2026-06-22). The v1 base — single crate, `src/commands/<cmd>/mod.rs`, `RunCommand` trait, ~40-line dispatch `main.rs`, `is_color_on()`-gated styling — is **settled and correct**. v2 does NOT rewrite it; it adds three cross-cutting capabilities (`--json`, `--clip`, config-file defaults) plus two meta-commands (`completions`, `config`) and per-command depth. The whole job here is **how the new spine grafts onto the existing files without per-command churn or divergence**.

---

## The Existing Architecture (verified — integrate WITH this)

| File | Current role | What v2 touches |
|------|-------------|-----------------|
| `src/main.rs` (~123 lines incl. doc) | ANSI bootstrap → `Cli::try_parse()` → `init_color(cli.no_color)` → `match cli.command { … => args.run() }` → 0/1/2 exit map | **Modify:** add `init_config()` + (optionally) a clip-flush after dispatch; add `Completions`/`Config` arms |
| `src/cli.rs` | `#[derive(Parser)] Cli { no_color: bool (global), command: Commands }` + `Commands` enum (23 variants) | **Modify:** add global `--json`/`--clip` args; add `Completions`/`Config` variants |
| `src/core/output.rs` | `COLOR_ON` atomic + `init_color` + `is_color_on()` + `color_enabled()`; `human_size`, `terminal_width`, row/summary helpers | **Modify:** add `emit_json`, the captured-stdout/`--clip` sink, and a JSON-mode atomic gate |
| `src/core/input.rs` | `read_input` / `read_input_bytes` / `read_file_or_stdin` → `ResolvedInput` | Untouched (input layer is orthogonal to output) |
| `src/core/errors.rs` | `BoxError` (`MissingInput`, `UnsupportedHashLength`) → exit-2 downcast in main | **Modify (maybe):** add a `Config`-load variant if a malformed config should be exit 2 |
| `src/commands/<cmd>/mod.rs` × 23 | `Args` struct + `impl RunCommand { fn run(self) }`; prints via `println!` to stdout, color via `is_color_on()` | **Modify per command (incrementally):** branch human vs JSON; read config-derived defaults |
| `src/core/mod.rs` | `pub mod errors/fs/input/output` | **Modify:** add `pub mod config` |

**Load-bearing facts that constrain every recommendation below:**

1. **`RunCommand::run(self)` receives ONLY the command's own `Args` struct** — not `&Cli`, not any context. A global flag declared on the top-level `Cli` (like today's `no_color`) is **invisible inside `run()`** unless it is (a) re-exposed per-command, or (b) lifted into a process-global the way `no_color` already is via `init_color()`. This single fact drives the `--json`/`--clip` design. `no_color` is the proven precedent: it is a `global = true` flag read **once** in `main()` and stashed in the `COLOR_ON` atomic, after which every command consults `is_color_on()` — never the flag.
2. **Every command emits output with bare `println!`/`print!` to the process stdout.** There is no output abstraction layer today; `core::output` only provides *string-returning* helpers (`format_row`, `human_size`, summaries) that commands then `println!` themselves.
3. **The color contract is "byte-identical minus ANSI when piped," proven per-command by a piped-no-ANSI test.** Any v2 output path must preserve this and gain an equivalent per-command JSON test.
4. **`serde` (`derive`) and `serde_json` (`preserve_order`) are already dependencies.** No new crate is needed for `--json`. `arboard` is already a dependency (the `clip` command). The only genuinely new crates are `clap_complete` (completions) and `toml` (config).

---

## System Overview — v2 spine grafted onto v1

```
┌──────────────────────────────────────────────────────────────────────────┐
│  main.rs  (modified — still dispatch-only, no business logic)             │
│    1. enable_ansi_support()                                               │
│    2. Cli::try_parse()                                                     │
│    3. init_config()   ── load %APPDATA%\box\config.toml → process-global   │  NEW
│    4. init_color(cli.no_color)                                            │
│    5. init_output(cli.json, cli.clip)  ── set JSON_ON + CLIP_ON atomics    │  NEW
│    6. match cli.command { … => args.run() }                               │
│    7. flush_clip()  ── if --clip: copy the captured buffer to clipboard    │  NEW
│    8. 0/1/2 exit map (unchanged)                                          │
├──────────────────────────────────────────────────────────────────────────┤
│  cli.rs  (modified)                                                        │
│    Cli { no_color, json (global), clip (global), command }                │  NEW globals
│    Commands { …23 tools…, Completions(..), Config(..) }                    │  NEW variants
├──────────────────────────────────────────────────────────────────────────┤
│  core/output.rs  (modified — the integration hub)                         │
│    COLOR_ON (v1)   JSON_ON (new)   CLIP_ON (new)                           │
│    is_color_on()   is_json_on()    emit_json<T:Serialize>(&T)             │  NEW
│    print helpers now route through a sink that tees to the clip buffer     │  NEW
│  core/config.rs  (NEW)                                                     │
│    Config { default_hash_algo, weather_units, weather_location, color }   │
│    load() → %APPDATA%\box\config.toml  (missing = Config::default())       │
├──────────────────────────────────────────────────────────────────────────┤
│  commands/<cmd>/mod.rs  (modified incrementally, simplest-first)          │
│    run(self):                                                              │
│      if is_json_on() { emit_json(&OutputStruct) }   ──┐ ONE data source    │
│      else            { println!(human render of  ) ───┘ feeds both paths   │
│                         the SAME OutputStruct }                            │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Question 1 — The `--json` contract (the most important decision)

### 1a. Global flag, lifted to a process-global atomic (NOT per-command, NOT threaded through `run`)

**Recommendation: a single `#[arg(long, global = true)] pub json: bool` on the top-level `Cli`, read once in `main()` and stored in a `JSON_ON: AtomicBool` in `core::output` — exactly mirroring the proven `no_color` → `COLOR_ON` pattern.**

Three candidate mechanisms were evaluated:

| Option | How a command sees `--json` | Verdict |
|--------|------------------------------|---------|
| **A. Per-command `--json`** (a `json: bool` field on each of 23 `Args`) | Field on `self` | ✗ 23× duplicated flag + help text; nothing stops one command spelling it `--as-json`; the contract lives in 23 places |
| **B. Global flag + `#[arg(from_global)]` per command** | `self.json` (clap copies the global value down) | ✗ Still requires a `#[arg(from_global)] json: bool` line in all 23 structs (verified: clap's `from_global` is opt-in per consumer; there is no `flatten, from_global`); churns every `Args` struct, and the field is redundant with the atomic |
| **C. Global flag + process-global atomic** ✅ | `is_json_on()` — zero per-command field | **Chosen.** One declaration on `Cli`, one read in `main`, one `JSON_ON` store; commands consult `is_json_on()` the same way they already consult `is_color_on()`. Zero new field on any `Args` struct. |

Option C wins decisively because it **reuses the existing, tested gating idiom** and adds **no field to any of the 23 command structs** — the JSON branch is a function call (`is_json_on()`), not a parsed argument. The `RunCommand::run(self)` signature is **left unchanged**, so there is no churn to the trait, to `main.rs`'s dispatch table, or to any command not yet JSON-enabled.

> Global-flag ergonomics bonus: `global = true` means `box hash --json file` and `box --json hash file` both parse, matching how `--no-color` already works — the user never has to remember flag position.

**`main.rs` change (one new line, paralleling the existing `init_color`):**
```rust
crate::core::output::init_color(cli.no_color);          // existing
crate::core::output::init_output(cli.json, cli.clip);   // NEW — sets JSON_ON + CLIP_ON
```

**`core::output` additions:**
```rust
static JSON_ON: AtomicBool = AtomicBool::new(false);
pub fn is_json_on() -> bool { JSON_ON.load(Ordering::Relaxed) }
pub fn init_output(json: bool, clip: bool) {
    JSON_ON.store(json, Ordering::Relaxed);
    CLIP_ON.store(clip, Ordering::Relaxed);
}
```

### 1b. One serde struct per command, rendered by a shared `emit_json` — the no-divergence guarantee

**Recommendation: each command defines a `#[derive(Serialize)]` output struct, and BOTH the human renderer and the JSON path read from THAT struct.** The shared helper:

```rust
// core::output
pub fn emit_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value)
        .context("serializing --json output")?;
    out.write_all(b"\n")?;          // trailing newline → clean ConvertFrom-Json + pipe hygiene
    Ok(())
}
```

The command shape becomes a thin fork over **one data source**:

```rust
impl RunCommand for HashArgs {
    fn run(self) -> anyhow::Result<()> {
        let result = self.compute()?;            // -> HashOutput { algo, hash, label }
        if is_json_on() {
            emit_json(&result)?;                 // machine path
        } else {
            println!("{}  {}", result.hash, result.label);   // human path — SAME struct
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct HashOutput { algo: &'static str, hash: String, label: String }
```

Because the human `println!` and `emit_json` both read `result`, they **cannot drift** — a field added to the struct is available to both; the JSON can never report a different hash than the text. This is the structural enforcement the question asks for, and it pairs naturally with the v1 "thin orchestrator over a pure function" pattern (Pattern 5 in the v1 doc): the pure function already returns a value — v2 just gives that value a `#[derive(Serialize)]` and a field name.

**Why `to_writer_pretty` (not compact)**: human-readable when eyeballed, and PowerShell's `ConvertFrom-Json` is whitespace-agnostic, so prettiness costs nothing for scripting. (A future `--compact`/NDJSON toggle can be added later without changing the contract — out of scope for v2 unless a command's volume demands it; see 1d.)

### 1c. Interaction matrix — `--json` × `--clip` × NO_COLOR × color gating × piping

| Combination | Behavior | Rationale |
|-------------|----------|-----------|
| `--json` alone | Emit pretty JSON to stdout, **no ANSI ever** | JSON is data, not display. `emit_json` writes raw serde output; it never calls `.green()`. |
| `--json` + color-on TTY | Still **no ANSI** in the JSON | JSON must be a clean parse target. `is_json_on()` short-circuits BEFORE any styling decision. **`init_output` should also force `COLOR_ON = false` whenever `json` is set**, so any stray `.green()` in a not-yet-audited code path can't corrupt the document. (One line: `if json { COLOR_ON.store(false) }`.) |
| `--json` + `NO_COLOR` | No ANSI (already true), JSON unaffected | Orthogonal; NO_COLOR only ever suppressed color, which JSON never had. |
| `--json` + piped | Identical to `--json` on a TTY | JSON output is TTY-independent by design — this is the *point* of `--json` (deterministic for `| ConvertFrom-Json`). The v1 "byte-identical minus ANSI" test generalizes to "JSON byte-identical regardless of TTY." |
| `--json` + `--clip` | JSON document is **both** printed to stdout **and** copied to the clipboard | The clip sink (Q3) captures whatever was emitted — for a JSON command that is the JSON text. `box weather London --json --clip` puts parseable JSON on the clipboard. |
| `--clip` alone (no `--json`) | The **human** rendered output (minus ANSI — see Q3) is copied | Clipboard gets clean text, never escape codes. |

**Precedence rule to encode:** `is_json_on()` is checked **first**, before any color or layout logic, in every command's `run()`. JSON mode is an output *mode*, not a decoration toggle.

### 1d. Streaming / large commands — buffer-then-serialize a single document (default), with an NDJSON escape hatch only where volume demands

The commands that produce many rows: `hash` (could take multiple paths in v2), `dupes`, `du`, `tree`, `flatten`.

**Recommendation: emit ONE JSON document per invocation (buffer the result struct, serialize once).** Reasoning:

- **PowerShell ergonomics decide this.** `box du --json | ConvertFrom-Json` should yield ONE object with a `.entries` array the user can pipe into `Sort-Object`/`Where-Object`. NDJSON (one object per line) forces `Get-Content | ForEach-Object { $_ | ConvertFrom-Json }` — awkward and non-idiomatic in PS7. A single document is the right default for a *toolkit aimed at PS7*.
- **These outputs are bounded by what the human view already buffers.** `du`/`tree`/`dupes` already `collect()` their rows and `sort_by` them *before* printing (verified in `du/mod.rs`: "rows are `collect`ed and then `sort_by` … BEFORE printing"). The full result set is already in memory for the human path; serializing it costs nothing extra. There is no command that streams unbounded rows it doesn't already buffer.
- **`flatten`** is the one genuinely large *action*; but its JSON summary is a fixed-shape object (`{copied, renamed, skipped, bytes, rows:[…]}`) — again already assembled for the human summary.

**NDJSON is explicitly NOT the v2 default.** Reserve it for a hypothetical future per-command `--ndjson` flag if a command ever streams millions of rows (none does today). Document this so the roadmapper doesn't over-engineer a streaming serializer.

**Single-document shape convention (lock this so all commands agree):**
```jsonc
// box du . --json
{ "path": ".", "total_bytes": 12345, "shown": 3, "entries": [ {…}, {…}, {…} ] }
// box hash a b --json   (multi-file)
{ "results": [ { "algo":"blake3", "hash":"…", "path":"a" }, {…} ] }
```
Top-level object (never a bare array at the root for multi-row commands) so the document is extensible (adding a sibling key never breaks consumers).

### 1e. Keeping `--json` free of stray stdout (progress bars / logs → stderr)

This is a **discipline + one guard**, not new infrastructure:

1. **Progress/spinners (`indicatif`, the in-place `pomodoro`/`dupes`/`hash` progress) MUST render to stderr.** They are already non-stdout-data conceptually; v2 must ensure any `indicatif::ProgressBar` is built with `ProgressDrawTarget::stderr()`. In JSON mode, **suppress progress entirely** (`if is_json_on() { /* no bar */ }`) — a parser doesn't want even stderr noise interleaving, and progress is a TTY affordance.
2. **All diagnostics already go to stderr** (verified: `weather` echoes its resolved geocode to stderr; `main` prints `error:` to stderr). v2 keeps this — the existing FOUND-03 "data→stdout, messages→stderr" contract is exactly what makes `--json` clean.
3. **The one guard:** stdout in JSON mode must carry *only* the `emit_json` document. Concretely, audit each JSON-enabled command so the *only* `println!`/`print!` reachable when `is_json_on()` is the `emit_json` call. The per-command JSON test asserts this: `stdout` parses as a single JSON value (`serde_json::from_slice` succeeds and consumes all of stdout). That test is the regression backstop — the v2 analogue of v1's piped-no-ANSI test.

---

## Question 2 — Config-file layering (precedence without breaking clap)

### The problem, precisely

`clap`'s `default_value`/`default_value_t` are **compile-time constants**; they cannot read a file at runtime. And clap fills a non-`Option` field with its default *before* you can know whether the user actually typed the flag — so you can't distinguish "user passed `--algo sha256`" from "clap defaulted it." That distinction is the whole precedence problem.

### Recommendation: `Option` fields + an explicit merge step (the "clap parses presence, config fills absence" pattern)

**Make every config-overridable flag an `Option<T>` in its `Args` struct (no `default_value`), then resolve the effective value with a 4-tier merge inside `run()` (or a tiny per-command resolver).** Precedence falls out naturally because **`Some` means "the user typed it," `None` means "fall through to the next tier."**

```
effective = cli_flag            // Some(x)  → user typed it on the CLI   (highest)
        .or(env_var_value)      // BOX_HASH_ALGO etc. (optional tier)
        .or(config.field)       // loaded from config.toml
        .unwrap_or(BUILTIN);    // the old default_value constant        (lowest)
```

This is the standard, robust Rust-CLI pattern (it's what `figment`/`config`-style layering does, but hand-rolled here — no new dependency needed, and a 4-line `.or().or().unwrap_or()` is clearer than a config framework for ~4 settings). It **does not fight clap**: clap still parses, still validates `ValueEnum` values, still produces `--help`; the only change is the field type goes from `T` (with `default_value`) to `Option<T>` (without).

> `hash` is **already** shaped for this: `pub algo: Option<Algo>` with the default applied as `self.algo.unwrap_or(Algo::Sha256)` (verified in `hash/mod.rs`). v2 just inserts the env/config tiers into that existing `.unwrap_or`:
> ```rust
> let algo = self.algo
>     .or_else(|| config().default_hash_algo)
>     .unwrap_or(Algo::Blake3);     // BLAKE3 is the new v2 builtin (HASH-V2-01)
> ```
> This is the cleanest possible proof the pattern fits: the precedence machinery is a one-line change to an existing `unwrap_or`.

**Rejected alternatives:**
- *`default_value_if` / inspecting `ArgMatches` `ValueSource`* — clap *can* tell you whether a value came from the CLI vs a default (`matches.value_source(id) == Some(ValueSource::CommandLine)`), but using it means dropping out of the derive API into `ArgMatches` plumbing in `main`, threading match results into commands, and re-deriving exit semantics. It's strictly more complex than `Option` + `.or()` and buys nothing. **Not recommended.**
- *A config crate (`config`, `figment`)* — overkill for ~4 settings; adds a dependency and a learning surface for a 4-line merge. **Not recommended.**

### The shared `Config` struct + load path

```rust
// core/config.rs  (NEW)
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]   // unknown key → clear error, not silent ignore
pub struct Config {
    pub default_hash_algo: Option<crate::commands::hash::Algo>,  // ties to Q5
    pub weather_units: Option<Units>,
    pub weather_location: Option<String>,
    pub color: Option<bool>,             // a config-level color default (CLI --no-color still wins)
    // add fields as commands opt in — start tiny
}

/// %APPDATA%\box\config.toml ; missing file = Config::default() (NOT an error).
pub fn load() -> anyhow::Result<Config> {
    let Some(path) = config_path() else { return Ok(Config::default()) };
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s)
            .with_context(|| format!("parsing {}", path.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("box").join("config.toml"))
}
```

**Where it loads:** once in `main()`, **before `init_color`/dispatch**, into a process-global `OnceLock<Config>` (the same "decide once at startup" shape as color), exposed via `core::config::config() -> &'static Config`. Commands read `config()` only for the fields they support.

- **Missing file = silently default.** A first-time user with no config file gets v1 behavior exactly. This is essential — config is purely additive.
- **Malformed file** → a clear `error: parsing …\config.toml: …` and **exit 2** (a usage/config error, same family as clap's bad-flag). Add `BoxError::Config` and downcast it in `main` alongside `MissingInput`/`UnsupportedHashLength`. (If the roadmapper prefers exit 1, that's a minor call — but exit 2 is the more honest "your invocation environment is wrong" signal.)
- **`%APPDATA%` is the correct Windows location** (roaming user config), no admin, survives reinstalls — consistent with the v1 `%LOCALAPPDATA%\box\bin` install convention (config roams, binary doesn't).
- **`deny_unknown_fields`** so a typo'd key (`weather_unit` vs `weather_units`) is a loud error, not a silent no-op — matching the project's "nothing silently lost" ethos (the flatten-collision philosophy applied to config).

### How `box config` fits

A `Config(ConfigArgs)` meta-command variant with three sub-modes:
- `box config show` / `box config path` — print the loaded config (respects `--json`! `emit_json(&config())`) and the file path.
- `box config get <key>` — print one resolved value.
- `box config set <key> <value>` — read-modify-write the TOML (create `%APPDATA%\box\` if absent), validating the value against the same types the merge uses.

`config` is itself a `RunCommand` and a perfect **`--json` dogfood**: `box config show --json` returns the effective config as an object. Keep `set` minimal (whole-file rewrite via `toml::to_string_pretty`; comments are not preserved — acceptable for a 4-key file; document it).

---

## Question 3 — `--clip` placement (capture-then-tee in core, not per-command)

### Recommendation: a shared output sink in `core::output` that **tees stdout to an in-memory buffer when `--clip` is set**, flushed to `arboard` once after dispatch in `main`. Commands do NOT re-implement capture.

The challenge: commands print with bare `println!`. To copy "what would have been printed" without rewriting 23 commands, you need to **intercept the bytes**. Two viable approaches:

| Approach | Mechanism | Verdict |
|----------|-----------|---------|
| **A. Route all output through a `core::output` print API** (`out_line!`/`emit()`), which tees to stdout + a `CLIP_BUF` when `CLIP_ON` | Replace `println!` with the helper in each command | ✓ Clean, explicit, no magic — but **touches every `println!` in 23 commands** (high churn). Worth it only because v2 is already editing each command for `--json`. |
| **B. Capture real stdout via an OS-level redirect** (swap the fd) | `gag`/`os_pipe` redirect of fd 1 around dispatch | ✗ Fragile on Windows, fights `crossterm`/raw-mode commands (`matrix`, `pomodoro`), and would capture progress noise. **Rejected** — too clever, regression risk against the v1 raw-mode terminal handling. |

**Choose A, but scope it to the commands `--clip` actually makes sense for.** `--clip` is meaningful for *value-producing* commands (`uuid`, `hash`, `passgen`, `qr`→the text, `json`, `du`, `tree`, `weather`, `base64`, `epoch`, `color`, `8ball`/`fortune`/`roast`/`cowsay`). It is **meaningless** for the full-screen animations (`matrix`) and the clipboard command itself (`clip` — copying the clipboard to the clipboard). Gate `--clip` to opt-in commands; for non-applicable commands either ignore it or error clearly.

**The mechanism (single capture point, the same struct that emits JSON):**

```rust
// core::output
static CLIP_ON: AtomicBool = AtomicBool::new(false);
static CLIP_BUF: Mutex<String> = Mutex::new(String::new());

/// The ONE print primitive v2 commands use for their primary stdout output.
/// Tees to the clip buffer when --clip is set; ALWAYS goes to real stdout.
pub fn out_line(s: &str) {
    println!("{s}");
    if CLIP_ON.load(Ordering::Relaxed) {
        let mut b = CLIP_BUF.lock().unwrap();
        b.push_str(s);
        b.push('\n');
    }
}

/// Called once in main() after dispatch succeeds. No-op unless --clip was set.
pub fn flush_clip() -> anyhow::Result<()> {
    if !CLIP_ON.load(Ordering::Relaxed) { return Ok(()); }
    let text = CLIP_BUF.lock().unwrap();
    if text.is_empty() { return Ok(()); }
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text.trim_end_matches('\n').to_string()).context("write clipboard")?;
    Ok(())
}
```

- **Clip content is always ANSI-free.** Because the clip path captures the *string* passed to `out_line` (which, in human mode, is built with the `is_color_on()`-gated styling that no-ops when not a TTY) — and critically, **`--clip` capture should force the no-color path** so the clipboard never gets escape codes even on a TTY. Simplest rule: `init_output` sets `COLOR_ON = false` when `clip` is set, identical to the JSON rule. (You rarely want both colored terminal output *and* a clean clipboard from one run; clean clipboard wins.)
- **`emit_json` also tees to `CLIP_BUF`** so `--json --clip` puts the JSON on the clipboard (just have `emit_json` write through the same sink, or append its serialized string to `CLIP_BUF` when `CLIP_ON`).
- **Reuses the v1 `clip` command's proven arboard discipline** (single-shot, main-thread, `set_text` synchronous, persists past exit on Windows — verified in `clip/mod.rs`). `flush_clip` runs on the main thread in `main()`, satisfying the "arboard main-thread only" pitfall.
- **Flush happens only on success** (after `Ok(())` from dispatch, before exit-code mapping) so a failed command doesn't clobber the clipboard with partial output.

**Why not fully per-command:** per-command `--clip` would re-implement arboard 15+ times and re-derive the trailing-newline policy each time. Centralizing in `core::output` + `main` means one capture rule, one arboard call, one newline policy — and the per-command cost is just *using `out_line` instead of `println!`* for the primary output (an edit you're already making for `--json`).

---

## Question 4 — Shell completions (`box completions <shell>`)

**Recommendation: a `Completions(CompletionsArgs)` subcommand that calls `clap_complete::generate` against the built `Cli` command, writing the script to stdout.** Confirmed it composes with the global `--json`/`--clip`/config args.

```rust
// commands/completions/mod.rs   (NEW)
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (powershell is the v2 target).
    #[arg(value_enum)]
    pub shell: Shell,
}

impl RunCommand for CompletionsArgs {
    fn run(self) -> anyhow::Result<()> {
        let mut cmd = crate::cli::Cli::command();   // the SAME Cli, incl. global --json/--clip
        let name = cmd.get_name().to_string();
        generate(self.shell, &mut cmd, name, &mut std::io::stdout());
        Ok(())
    }
}
```

**Composition is automatic and verified:**
- `clap_complete::generate(gen, &mut Command, bin_name, &mut Write)` (clap_complete 4.6.x, matching clap 4.6) introspects the **entire** `Cli` command tree built by `Cli::command()` (via `CommandFactory`). Because `--json`/`--clip`/`--no-color` are declared on `Cli` with `global = true`, and `Completions`/`Config` are normal `Commands` variants, **they all appear in the generated grammar with no extra work** — global args are emitted as completable on every subcommand, which is exactly the PS7 UX you want (`box hash --<TAB>` offers `--json`).
- **PowerShell target:** `clap_complete::Shell::PowerShell` produces a `Register-ArgumentCompleter`-based `.ps1`. Usage in PS7: `box completions powershell | Out-String | Invoke-Expression` (or persist into `$PROFILE`).
- **Static (AOT) generation, not dynamic.** Use the stable `clap_complete::aot::generate` path — *not* the `unstable-dynamic` feature (verified gated behind `unstable-dynamic`; avoid unstable in a shipping tool). A static `.ps1` the user sources is the right, durable choice; regenerate after adding commands.
- **New crate:** `clap_complete = "4.6"` (pin to the clap minor line). One dependency, no transitive runtime weight in the hot path (only the `completions` command links its generators).

**Edge note for the roadmapper:** completions should be added **after** the global `--json`/`--clip` args land, so the generated script includes them. This orders `completions` *late* (Q6).

---

## Question 5 — BLAKE3 as the new `hash` default (breaking change)

**Recommendation: the default lives where it already does — in `hash/mod.rs`'s `unwrap_or` — flipped from `Sha256` to `Blake3`, with config able to override it via the Q2 merge.**

Current (v1): `let algo = self.algo.unwrap_or(Algo::Sha256);` (verified).

v2: insert the config tier and change the builtin:
```rust
let algo = self.algo
    .or_else(|| config().default_hash_algo)   // config tier (Q2)
    .unwrap_or(Algo::Blake3);                  // NEW builtin (HASH-V2-01) — the breaking change
```

- **Precedence:** explicit `--algo` (CLI) > `default_hash_algo` (config) > **BLAKE3** (builtin). A user who needs SHA-256-by-default for interop sets `default_hash_algo = "sha256"` in `config.toml` and gets the old behavior back — config makes the breaking change *recoverable per-user* without a flag on every invocation. This is a strong argument for shipping config **before or with** the BLAKE3 flip.
- **`--verify` auto-detect is unaffected:** the length→algo table (32→md5, 64→sha256, 128→sha512) is independent of the *compute* default and stays as-is (BLAKE3 has no unambiguous length slot; 64 already maps to sha256 — verified). Document that `--verify <64-hex>` still means sha256, not blake3, even though blake3 is now the compute default; an explicit `--algo blake3 --verify` is the blake3 verify path.
- **`Algo` gains `Serialize` + `ValueEnum`-string round-trip** so config can deserialize `"blake3"` and `--json`/`config show` can serialize it. (`Algo` is already `ValueEnum`; add `#[derive(Serialize, Deserialize)]` with `#[serde(rename_all = "lowercase")]` to match the clap spellings.)
- **The breaking change needs its own test + doc line**: a test asserting `box hash file` (no `--algo`) now emits a 64-hex BLAKE3 digest, and the `hash --help`/PROJECT note flagging the v1→v2 behavior change.

---

## Question 6 — Suggested build order (risk-ordered; phases continue from v1's Phase 5 → start at Phase 6)

The v1 retrospective's **#1 lesson** is *"order phases by integration risk — find the architecture problem on the simplest command."* For v2 the risk is concentrated in **the shared spine** (`--json`/`--clip`/config core), not in any one command. So the order is: **prove the spine on the two simplest commands, then roll it across, then add per-command depth, then the meta-command polish.** This minimizes per-command churn (the core is settled before 21 commands adopt it) and front-loads the only real architecture risk.

> This is a **recommendation**; the roadmapper makes the final phase cut. Phase numbers continue v1's sequence (v1 ended at Phase 5).

### Phase 6 — Scriptable-core foundation, proven on the two simplest commands

**Goal:** Build the entire shared spine and prove it on `uuid` (zero input, single value) and `hash` (single value, already `Option`-shaped, and the BLAKE3-default home). Find any architecture problem here, on the cheapest commands — before 21 others depend on it.

- Add global `--json` + `--clip` to `Cli`; add `JSON_ON`/`CLIP_ON` atomics + `init_output` + `is_json_on()` to `core::output`.
- Add `emit_json`, `out_line`, `CLIP_BUF`, `flush_clip` to `core::output`; wire `init_output` + `flush_clip` into `main.rs`.
- Add `core::config` (`Config`, `load`, `OnceLock`, `config()`) + `init_config()` in `main`; add `BoxError::Config` (exit-2).
- **First consumers:** `uuid` (define `UuidOutput`, branch human/JSON, support `--clip`); `hash` (define `HashOutput`, branch, support `--clip`, **flip default to BLAKE3** with the config-override tier — Q5).
- **Tests:** per-command JSON test (`stdout` is a single parseable JSON value), `--clip` capture test, config-precedence unit test (CLI > config > builtin), BLAKE3-default test. These become the **template** every later command copies.

*Integration risk: HIGH (all new shared machinery) — deliberately on the 2 cheapest commands.*

### Phase 7 — Roll `--json`/`--clip` across the value-producing commands

**Goal:** Apply the now-proven Phase-6 template to the rest, simplest→nested, in waves with zero file overlap (the v1 sequencing discipline).

- **Wave 7a (pure transforms, trivial structs):** `base64`, `epoch`, `color`, `passgen`, `uuid`(done), `8ball`, `fortune`, `roast`, `cowsay`.
- **Wave 7b (filesystem, the buffered-rows commands):** `du`, `tree`, `dupes`, `flatten`, `bulk-rename` — each gets a single-document JSON shape (`{…, entries:[…]}` per Q1d). These are the ones that validate the "buffer-then-serialize, top-level object" decision.
- **Wave 7c (remaining value commands):** `json` (already serde-native — JSON-of-JSON or pass-through), `qr` (JSON = the encoded text + EC level; `--clip` = the text), `weather` (rich object — a flagship `--json` demo), `clip`(N/A — skip), `matrix`/`pomodoro`/`ascii`/`lolcat` (display-only — `--json` N/A or a minimal status object; mostly skip `--clip`).
- Each command: define its `#[derive(Serialize)]` output struct, fork on `is_json_on()`, route primary output through `out_line`, add the two tests.

*Integration risk: LOW per command (template is fixed) — the volume is the cost, not the risk. Order simplest→nested so any surprise surfaces on `base64`, not `flatten`.*

### Phase 8 — Per-command depth (the deferred-V2 feature set)

**Goal:** The comprehensive depth flags, now that every command already has its output struct (new fields slot into both human + JSON paths for free).

- Filesystem: flatten/hash/dupes/bulk-rename/tree/du depth flags.
- Dev transforms: uuid v7, epoch tz, color CSS+HSL, json `--sort-keys`, passgen entropy/`--no-similar`/`--separator`.
- Visuals: lolcat `--animate`/`--freq`/`--seed`, matrix color/speed/charset, qr `--save`/EC, ascii color/braille/invert.
- Fun/system: cowsay figures/think, fortune categories, 8ball art, roast `--language`, pomodoro counter/auto-break/`--label`, weather `--forecast`/cache/stored-location.

*Integration risk: LOW (each flag is local to one command; JSON struct already exists so a new field is additive). Can interleave with Phase 7's later waves if the roadmapper wants — but keeping depth after the spine keeps each command edited once for the spine, once for depth, not thrashed.*

### Phase 9 — Meta-command polish: `config` + `completions`

**Goal:** The frictionless-PS7 surface, built **last** because it depends on the finished arg surface.

- `box config` (show/get/set/path) — dogfoods `--json` (`config show --json`).
- `box completions powershell` — generates the script including all globals **and** all Phase-8 flags (must come after the arg surface is final, per Q4).
- Final pass: `install.ps1` optional completion-registration hint; help-text/PROJECT note for the BLAKE3 breaking change.

*Integration risk: LOW (both are read-only over a settled `Cli`). Completions intrinsically must be last so the generated grammar is complete.*

### Why this order minimizes churn (the explicit retrospective tie-in)

- **The spine is built once, on 2 commands (Phase 6), before 21 adopt it** — so an architecture flaw costs 2 commands of rework, not 23. This is the v1 "find the problem on `uuid`, not `flatten`" lesson applied to *cross-cutting code* instead of *per-command code*.
- **Each command is edited for the spine once (Phase 7) and for depth once (Phase 8)** — no command is touched, reverted, and re-touched as the contract evolves, because the contract is frozen in Phase 6.
- **Completions/config last (Phase 9)** because they *consume* the final arg surface; building them earlier guarantees regenerating them later.

---

## Data Flow — v2 request flow (showing the new touch points)

```
box hash report.bin --json --clip
        │
   main: enable_ansi → Cli::try_parse()
        │                ├─ cli.json = true (global)   ─┐
        │                └─ cli.clip = true (global)    │
        ├─ init_config()  → load %APPDATA%\box\config   │ once, at startup
        ├─ init_color(false)  → COLOR_ON forced false   │ (json/clip imply no-ANSI)
        ├─ init_output(json=true, clip=true) → JSON_ON, CLIP_ON
        │
        └─ Commands::Hash(args).run()
               ├─ algo = args.algo .or(config.default_hash_algo) .unwrap_or(Blake3)   ← Q2/Q5
               ├─ result = HashOutput { algo, hash, label }     ← ONE data source
               └─ if is_json_on() {                              ← Q1 (checked first)
                      emit_json(&result)   → stdout  + tee→CLIP_BUF   ← Q1b/Q3
                  } else {
                      out_line(&format!("{}  {}", result.hash, result.label))
                  }
        │
   main: flush_clip()   → arboard.set_text(CLIP_BUF)            ← Q3 (main-thread, on success)
        └─ Ok(()) → ExitCode 0
```

---

## New-vs-Modified file ledger (for the roadmapper / plan-phase)

| File | New / Modified | Change |
|------|----------------|--------|
| `src/core/config.rs` | **NEW** | `Config` struct, `load()`, `OnceLock` + `config()`, `init_config()` |
| `src/commands/completions/mod.rs` | **NEW** | `CompletionsArgs` + `RunCommand` (clap_complete generate) |
| `src/commands/config/mod.rs` | **NEW** | `ConfigArgs` (show/get/set/path) + `RunCommand` |
| `src/core/output.rs` | **Modified** | `JSON_ON`/`CLIP_ON` atomics, `CLIP_BUF`, `init_output`, `is_json_on`, `emit_json`, `out_line`, `flush_clip`; force `COLOR_ON=false` under json/clip |
| `src/core/mod.rs` | **Modified** | `pub mod config;` |
| `src/core/errors.rs` | **Modified** | `BoxError::Config { … }` (exit-2 downcast) |
| `src/cli.rs` | **Modified** | global `--json`/`--clip` on `Cli`; `Completions`/`Config` variants in `Commands` |
| `src/main.rs` | **Modified** | `init_config()` + `init_output()` before dispatch; `flush_clip()` after; `Completions`/`Config` dispatch arms; `BoxError::Config`→exit-2 |
| `Cargo.toml` | **Modified** | `+ clap_complete = "4.6"`, `+ toml = "0.8"` (serde/serde_json/arboard already present) |
| `src/commands/<cmd>/mod.rs` × ~17 value commands | **Modified (incremental, Phases 6–8)** | `#[derive(Serialize)]` output struct; `is_json_on()` fork; `out_line` for primary output; config-tier merge where applicable; depth flags |
| `tests/<cmd>.rs` × value commands | **Modified** | per-command JSON-parse test + `--clip` capture test |
| `tests/config.rs`, `tests/completions.rs` | **NEW** | meta-command integration tests |

---

## Anti-Patterns (v2-specific)

### Anti-Pattern 1: Per-command `--json` field duplicated 23×
**What people do:** add `#[arg(long)] json: bool` to every `Args` struct.
**Why it's wrong:** 23 copies of the flag + help text; nothing enforces identical spelling/semantics; a new command can silently omit it. Contract lives in 23 places.
**Do this instead:** one `global = true` flag on `Cli` → `JSON_ON` atomic → `is_json_on()`. Mirrors the proven `no_color`/`COLOR_ON` idiom; zero per-command field.

### Anti-Pattern 2: Separate code paths building human text vs JSON
**What people do:** the JSON branch re-derives the values independently of the `println!` branch.
**Why it's wrong:** the two drift — JSON reports a stale/divergent value the human view never showed (the classic `--json` bug). Untestable as "same data, two renderings."
**Do this instead:** one `#[derive(Serialize)]` struct is the single source; human render formats *that* struct, `emit_json` serializes *that* struct. They cannot diverge.

### Anti-Pattern 3: `default_value` on a config-overridable flag
**What people do:** keep `#[arg(default_value_t = …)]` and try to layer config on top.
**Why it's wrong:** clap fills the default *before* you can tell "user typed it" from "clap defaulted it," so config can never win over the built-in default — precedence is unimplementable.
**Do this instead:** `Option<T>` field, no `default_value`; resolve `cli.or(env).or(config).unwrap_or(builtin)`. `Some` = user-typed (highest); `None` = fall through.

### Anti-Pattern 4: ANSI escapes leaking into `--json` or `--clip`
**What people do:** let the normal color path run and serialize/copy the styled string.
**Why it's wrong:** corrupts the JSON parse target and pastes escape codes into the clipboard.
**Do this instead:** `init_output` forces `COLOR_ON=false` whenever `json` or `clip` is set; `emit_json` never styles; `is_json_on()` is checked before any styling decision.

### Anti-Pattern 5: Progress bars / logs on stdout in JSON mode
**What people do:** `indicatif` bar or a status `println!` shares stdout with the JSON document.
**Why it's wrong:** breaks `| ConvertFrom-Json` — stdout is no longer a single JSON value.
**Do this instead:** all progress/diagnostics → stderr (FOUND-03, already the convention); suppress progress entirely under `is_json_on()`; the per-command JSON test asserts stdout parses as one value.

### Anti-Pattern 6: OS-level fd redirection to capture `--clip` output
**What people do:** redirect fd 1 around dispatch (`gag`/`os_pipe`) to grab "whatever was printed."
**Why it's wrong:** fragile on Windows, fights `crossterm` raw-mode (`matrix`/`pomodoro`), captures progress noise, regression risk against v1's terminal-restore handling.
**Do this instead:** route primary output through `out_line`, which tees the *string* to `CLIP_BUF`; `flush_clip` copies once in `main`.

---

## Integration Points

### Internal boundaries (new in v2)
| Boundary | Communication | Notes |
|----------|---------------|-------|
| `main.rs` → `core::output::init_output` | direct call, once at startup | sets JSON_ON/CLIP_ON; mirrors existing `init_color` |
| `main.rs` → `core::config::init_config` | direct call, once at startup | loads `%APPDATA%\box\config.toml` into `OnceLock`; missing = default |
| `main.rs` → `core::output::flush_clip` | direct call, after successful dispatch | single main-thread arboard write (satisfies arboard-main-thread pitfall) |
| command `run()` → `is_json_on()` / `emit_json` / `out_line` | direct calls | no `Args` field added; same idiom as `is_color_on()` |
| command `run()` → `config()` | direct read of `&'static Config` | only for fields the command supports; tier in the `.or().unwrap_or()` merge |
| `Completions`/`Config` arms → `Cli::command()` / `config()` | introspect the built command / read config | read-only over a settled arg surface |

### External integrations (new/reused)
| Service | Integration | Notes |
|---------|-------------|-------|
| Windows clipboard (write) | `arboard` `set_text`, main-thread, single-shot, on success | **reuses** the exact discipline proven in v1's `clip` command (persists past exit on Windows) |
| PowerShell completion | `clap_complete::Shell::PowerShell` static `.ps1` | user sources via `$PROFILE`; regenerate after adding commands; **avoid** `unstable-dynamic` |
| Config file | `toml` + `serde(Deserialize)` over `%APPDATA%\box\config.toml` | roaming user scope; `deny_unknown_fields`; missing = default |

---

## Sources

- v1 source (authoritative for the existing architecture): `src/cli.rs`, `src/main.rs`, `src/core/output.rs`, `src/core/input.rs`, `src/core/errors.rs`, `src/commands/{uuid,hash,clip,tree,du,weather}/mod.rs` — read in full (HIGH).
- `.planning/PROJECT.md` (v2.0 Current Milestone scope) and `.planning/RETROSPECTIVE.md` (the #1 "order by integration risk" + "promote-on-second-consumer" lessons) (HIGH).
- clap derive — `global = true` and `from_global` semantics: https://docs.rs/clap/latest/clap/_derive/index.html (`from_global` = "Read a `Arg::global()` argument, regardless of what subcommand you are in") — verified via Context7 `/websites/rs_clap` + docs.rs (HIGH).
- clap `global` + `from_global` is opt-in per consumer (no `flatten, from_global`): https://github.com/clap-rs/clap/issues/5525, https://github.com/clap-rs/clap/issues/3269 (MEDIUM — open feature requests confirm the current limitation).
- clap_complete 4.6.x — `generate(generator, &mut Command, bin_name, &mut Write)`, static AOT path; dynamic completion gated behind `unstable-dynamic`: https://docs.rs/clap_complete/latest/clap_complete/index.html, https://docs.rs/clap_complete/latest/clap_complete/env/struct.Powershell.html (HIGH).
- Config-precedence (`Option` + merge, CLI > env > file > default) — the standard layered-config pattern; Rain's Rust CLI recommendations on argument handling: https://rust-cli-recommendations.sunshowers.io/handling-arguments.html (MEDIUM).
- arboard Windows persistence + main-thread discipline: encoded in v1 `src/commands/clip/mod.rs` doc comment (verified against shipped code) (HIGH).

---
*Architecture research (v2 integration) for: box — Rust CLI toolkit (Windows PowerShell 7)*
*Researched: 2026-06-24 — supersedes the 2026-06-22 v1 foundation architecture*
