# Phase 11: Meta-Commands - Pattern Map

**Mapped:** 2026-07-14
**Files analyzed:** 8 (2 new, 6 modified)
**Analogs found:** 8 / 8 (every file has a concrete in-repo analog; only `clap_complete::generate` itself is net-new — see No Analog Found)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/commands/config/mod.rs` (NEW) | command (nested-subcommand dispatcher) | CRUD (read/mutate/locate config.toml) + request-response | `src/commands/weather/mod.rs` | role-match (strong) — but config is the FIRST nested subcommand (novel; see D-01 note) |
| `src/commands/completions/mod.rs` (NEW) | command (code generator) | transform (CLI tree → shell script on stdout) | `src/commands/uuid/mod.rs` skeleton + `weather` positional/ValueEnum | role-match (skeleton exact; the `generate()` core has no analog) |
| `src/cli.rs` (MOD) | route / registry | dispatch | its own top-level `#[command(subcommand)]` (lines 34-36) | exact for `Config` variant's nested-subcommand mechanic; flag variants exact for `Completions` |
| `src/main.rs` (MOD) | dispatch + exit-code map | request-response | existing dispatch match (85-109) + downcast (133-144) | exact (add two arms + one downcast entry) |
| `src/core/config.rs` (MOD) | config / model + helper | CRUD + file-I/O | its own `Config`/`load`/`config_path`/`malformed_maps_to_config_error` | exact (extend in place) |
| `src/core/errors.rs` (MOD) | error model | — | existing `Config` / `MissingLocation` variants (45-66) | exact |
| `src/commands/mod.rs` (MOD) | module registry | — | existing `pub mod …;` block (7-29) + `RunCommand` trait (34-36) | exact |
| `install.ps1` (MOD) | config / installer | batch (build→copy→PATH) | its idempotent PATH-guard block (42-58) | role-match (adapt the dedup-guard idiom to a `$PROFILE` sentinel) |
| `Cargo.toml` (MOD) | config | — | existing `clap`/`toml`/`serde` dep lines (13, 23, 102) | exact |

---

## Pattern Assignments

### `src/commands/config/mod.rs` (NEW — command, CRUD + request-response)

**Primary analog:** `src/commands/weather/mod.rs` (config-integrated command: `is_json_on()` fork, `ValueEnum`, config resolver, typed-error → exit-2 mapping).
**JSON-shape analog:** `src/commands/uuid/mod.rs` (the frozen `{results,count}` + `is_json_on()` fork template).
**Write/validate analog:** `src/core/config.rs` `malformed_maps_to_config_error` test (the exact round-trip harness `set` reuses).

**Module skeleton pattern** — every command is a directory with a `mod.rs` exporting one `Args` struct that impls `RunCommand`. Copy the `use` + struct + impl shape from `weather/mod.rs:41-98`:
```rust
use clap::{Args, ValueEnum};                 // ValueEnum only if the enum is a CLI arg
use crate::commands::RunCommand;
// ... command-local structs (Args, output structs) ...
#[derive(Debug, Args)]
pub struct WeatherArgs { /* fields with #[arg(...)] */ }
impl RunCommand for WeatherArgs {
    fn run(self) -> anyhow::Result<()> { /* body */ }
}
```

**Nested-subcommand shape (D-01, the novel bit — NO in-repo command analog).** The ONLY nested-subcommand mechanic in the repo is the top-level parser itself in `src/cli.rs:34-36` and `40-41`. Copy that exact shape one level down: a `ConfigArgs` struct carrying a `#[command(subcommand)]` field whose type is a `#[derive(Subcommand)]` enum. Reference (`src/cli.rs:33-41`):
```rust
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(Subcommand)]
pub enum Commands { /* variants */ }
```
Adapt to:
```rust
use clap::{Args, Subcommand};
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Show,                                   // --json handled via the global flag + is_json_on()
    Get { key: String },
    Set { key: String, value: String },
    Path,
}
impl RunCommand for ConfigArgs {
    fn run(self) -> anyhow::Result<()> {
        match self.command { /* dispatch per variant */ }
    }
}
```
Note: `--json` is a GLOBAL flag on `Cli` (`src/cli.rs:26-28`), already lifted into `is_json_on()` by `init_output` — do NOT add a per-subcommand `--json`; `show`/`get` read the global via `crate::core::output::is_json_on()` exactly like every other command.

**`show --json` / `get --json` emit pattern (dogfood the frozen template).** Copy the `is_json_on()`-FIRST fork verbatim from `uuid/mod.rs:116-131`:
```rust
        // Fork on `is_json_on()` FIRST (Pitfall 1): the ONLY stdout write
        // reachable under `--json` must be `emit_json` — no stray human chrome.
        if crate::core::output::is_json_on() {
            let doc = UuidOutput { count: rows.len(), results: rows };
            crate::core::output::emit_json(&doc)?;
        } else {
            for r in &rows {
                crate::core::output::out_line(&r.uuid);
            }
        }
        Ok(())
```
And the `#[derive(serde::Serialize)]` output struct from `uuid/mod.rs:42-55`. For `config show` D-06 recommends a nested effective doc (`{hash:{default_algo}, weather:{location, units}}`, snake_case) — model the structs on `weather/mod.rs:333-356` (`WeatherOutput` with `#[serde(skip_serializing_if = "Option::is_none")]` on absent-able leaves). `weather/mod.rs:270-283` shows the same fork inside a `render()` helper (`return crate::core::output::emit_json(&doc);`) — a good template for a `show` that has both a JSON and a human branch.

**Effective-config read pattern (D-06/D-07).** `show`/`get` must read through the SAME resolver every command uses. Copy the resolution shape from `weather/mod.rs:100-118` + the pure resolvers `weather/mod.rs:167-176`:
```rust
        let units = resolve_units(self.units, crate::core::config::config().weather.units);
// ...
fn resolve_units(cli: Option<Units>, cfg: Option<Units>) -> Units {
    cli.or(cfg).unwrap_or(Units::Metric)   // the SPINE-05 cli.or(env).or(cfg).unwrap_or(builtin)
}
```
For `config`, there is no CLI/env tier per key — `show`/`get` resolve `config().<table>.<field>.unwrap_or(builtin)`, where the builtins are the same ones the commands use (`Algo::Blake3` per `config.rs:162`; `Units::Metric` per `weather/mod.rs:175`; `weather.location` has NO builtin → the D-07 case-2 "unset, no default → exit 1 / null" carve-out).

**`set` validate-before-write pattern (D-03) — reuse the exact round-trip harness.** The gate is `config.rs`'s `malformed_maps_to_config_error` test (`config.rs:230-256`) turned into production code: reconstruct the resulting `Config`, `toml::to_string` it, then re-parse with the SAME `toml::from_str::<Config>` the startup path uses (`config.rs:115`) before writing. On any parse / invalid-enum / unknown-key error, write NOTHING and return the exit-2 error. Harness excerpt to mirror (`config.rs:231-256`):
```rust
        let map = |toml_src: &str| -> anyhow::Error {
            let parsed: Result<Config, _> = toml::from_str(toml_src);
            let err = parsed.expect_err("expected a parse error");
            anyhow::Error::from(BoxError::Config {
                path: "test/config.toml".to_string(),
                message: err.to_string(),
            })
        };
        // ...
        is_config_err(map("[weather]\nunits = \"kelvin\""), "invalid units value");
```
The typed `Algo`/`Units` enums + `#[serde(deny_unknown_fields)]` (`config.rs:47-80`) already reject bad values and stray keys — `set` just has to RUN the parse before persisting. `box config set weather.units kelvin` fails here (invalid enum), `set nope.key 1` fails here (unknown key), both exit 2, nothing written.

**Typed-error → exit-2 mapping (D-07 / Claude's Discretion).** Reuse `BoxError::Config` (or a new sibling variant) and map it in `main.rs` exactly like `weather` maps `MissingLocation` (`weather/mod.rs:106`):
```rust
        .ok_or(crate::core::errors::BoxError::MissingLocation)?;
```
`get` unset-no-default is NOT this — it prints nothing (JSON: `null`) and returns a plain success path that yields exit 1 via the git-style "not set" signal. Keep "no value" (exit 1) structurally distinct from "no such key" (exit 2).

**`config path` (D-08).** One line: print `core::config::config_path()` via `out_line`, exit 0, whether or not the file exists. Requires making `config_path` `pub` (see the config.rs assignment below).

---

### `src/commands/completions/mod.rs` (NEW — command, transform)

**Skeleton analog:** `src/commands/uuid/mod.rs` (smallest thin-orchestrator command) for the module shape; `src/commands/weather/mod.rs:82-96` for the positional + `ValueEnum` arg.
**Core (`clap_complete::generate`):** NO in-repo analog — this is the one net-new mechanic; planner should lift the exact call from RESEARCH.md / D-09.

**Args pattern — required positional `ValueEnum` (D-09, rustup style).** Copy the positional + `value_enum` shape from `weather/mod.rs:82-96` and the `ValueEnum` derive from `hash/mod.rs:114-116` / `weather/mod.rs:67-74`, but use `clap_complete::Shell` (which already derives `ValueEnum`, so you get validation + `--help` listing for free — no local enum needed):
```rust
use clap::Args;
use clap_complete::Shell;
use crate::commands::RunCommand;

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate a completion script for.
    #[arg(value_enum)]
    pub shell: Shell,          // required positional (no Option) — a bad value → clap exit 2
}
```
(Contrast `weather`'s `pub location: Option<String>` at line 87 which is OPTIONAL; completions' shell is REQUIRED, so drop the `Option`.)

**Core generate pattern (D-09) — stdout-only purity (D-10).** No codebase analog; the canonical call, driven by `clap::CommandFactory` on the live `Cli`:
```rust
impl RunCommand for CompletionsArgs {
    fn run(self) -> anyhow::Result<()> {
        use clap::CommandFactory;
        // D-10: PS comment header (inert) BEFORE the script, powershell only.
        // ... print the `#`-prefixed registration header to stdout ...
        clap_complete::generate(
            self.shell,
            &mut crate::cli::Cli::command(),
            "box",
            &mut std::io::stdout(),
        );
        Ok(())
    }
}
```
Purity rule mirrors `--json` and `emit_json` (`output.rs:143-162`): stdout carries EXACTLY the script (+ inert `#` header) — no stderr chatter, no ANSI/BOM. `completions` still flows through `init_config` harmlessly (see main.rs arm) but its real input is `Cli::command()`. `Cli` is `pub` and derives `Parser` (`cli.rs:14-21`), so `Cli::command()` is available via `CommandFactory`.

---

### `src/cli.rs` (MOD — route/registry)

**Analog:** the existing `Commands` enum (lines 40-123) and its own top-level `#[command(subcommand)]` (34-36).

Add two variants to `Commands` alongside `Weather` (cli.rs:122), following the doc-comment-becomes-help convention (module header lines 3-6). `Config` carries the nested `ConfigArgs` (the first variant whose arg struct itself contains a `#[command(subcommand)]` — novel, but clap handles it transparently):
```rust
    /// Read, edit, and locate the box config file
    Config(crate::commands::config::ConfigArgs),
    /// Generate a shell completion script
    Completions(crate::commands::completions::CompletionsArgs),
```
No `#[command(name = "...")]` needed (both idents are valid CLI names, unlike `8ball`/`bulk-rename` at 59/72).

---

### `src/main.rs` (MOD — dispatch + exit map)

**Analog:** the dispatch match (85-109) and the exit-code downcast (133-144).

Add two arms to the dispatch match (after `Commands::Weather(args) => args.run(),` at line 108), identical in shape to every other arm:
```rust
                Commands::Config(args) => args.run(),
                Commands::Completions(args) => args.run(),
```
Both inherit the existing `init_config → init_color → init_output → dispatch → flush_clip` pipeline (81-111) and the exit downcast for free. If `config set` introduces a NEW `BoxError` variant, add it to the exit-2 downcast set (135-142) beside `Config`/`MissingLocation`:
```rust
            match e.downcast_ref::<crate::core::errors::BoxError>() {
                Some(
                    crate::core::errors::BoxError::MissingInput
                    | crate::core::errors::BoxError::UnsupportedHashLength { .. }
                    | crate::core::errors::BoxError::UnknownFigure { .. }
                    | crate::core::errors::BoxError::Config { .. }
                    | crate::core::errors::BoxError::MissingLocation,
                    // + a new ConfigUsage variant here, if added
                ) => ExitCode::from(2),
                _ => ExitCode::from(1),   // get unset-no-default stays here → exit 1
            }
```
The `get` unset-no-default → exit 1 carve-out needs NO new wiring: return a plain (non-`BoxError`) path and it falls through to the `_ => ExitCode::from(1)` arm — but note that arm only fires on `Err`. For "print nothing, exit 1" you need `run()` to return an `Err` that is NOT a `BoxError` usage variant (a plain `anyhow::bail!` with an empty/suppressed message), mirroring how `json` invalid-data exits 1 via a plain anyhow error (main.rs header 20-25). Planner: confirm the exit-1-with-no-stderr shape (may need `std::process::exit(1)` after a clean stdout, since `eprintln!("error: …")` at 118 always prints — the git-style "not set" signal must be SILENT on stderr).

---

### `src/core/config.rs` (MOD — config/model + helper host)

**Analog:** its own existing code — extend in place.

1. **Make `config_path` public** (currently `fn` at line 141) so `config path` + `config set` reuse it verbatim:
```rust
pub fn config_path() -> Option<std::path::PathBuf> {   // was: fn config_path(...)
```

2. **Add `Serialize` derives + `skip_serializing_if` (D-02).** The structs currently derive only `Deserialize` (config.rs:47-80). Add `serde::Serialize` to `Config`, `HashConfig`, `WeatherConfig`, and `#[serde(skip_serializing_if = "Option::is_none")]` on the leaf `Option` fields — mirror the exact attribute weather already uses on an optional serialized field (`weather/mod.rs:354-355`):
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    forecast: Option<Vec<DayForecast>>,
```
Apply to `hash.default_algo`, `weather.location`, `weather.units`. (Discretion: optionally also suppress empty sub-tables — both re-parse clean per D-02.)

3. **Host the validate-before-write + settable-key registry.** The `load()` mapping (config.rs:110-125) and the `malformed_maps_to_config_error` harness (230-256) are the templates. Add a `pub fn set(key, value) -> anyhow::Result<()>` (or place the registry in `commands/config` per Discretion) that: load-or-default → mutate the target leaf → `toml::to_string` → re-parse via `toml::from_str::<Config>` (line 115) → atomic write. The closed key set is `hash.default_algo | weather.location | weather.units` (the D-13 schema, config.rs:51-79).

4. **Atomic temp-write + rename helper (Discretion: `core::fs` vs `core::config`).** Recommend `core::fs` — it already owns the disk-mutation helpers (`safe_copy` at `fs.rs:80`, header lines 1-15) and the `.context(...)`-on-every-fallible-call discipline. Model the new `pub fn atomic_write(path, contents)` on `safe_copy`'s signature/style (`fs.rs:80`): create the `%APPDATA%\box\` parent dir if absent (like `cache` does), write to a temp sibling, `fs::rename` over the target. Reuse the `load()` NotFound-tolerance idiom (config.rs:122) for the parent-dir create.

---

### `src/core/errors.rs` (MOD — error model)

**Analog:** the `Config` variant (45-52) and `MissingLocation` (54-66).

Either reuse `BoxError::Config` for `set`'s validate-fail (its `{path, message}` fields already fit) or add a sibling usage variant following the `MissingLocation` template (a `#[error("…")]` doc-commented variant that main.rs downcasts to exit 2). If added, wire it into the main.rs downcast set (see main.rs assignment). Keep the exit-1 `get` carve-out OUT of `BoxError` — it is a data signal, not a typed usage error (mirrors the deliberate json exit-1 divergence documented at errors.rs usage in main.rs header 20-25).

---

### `src/commands/mod.rs` (MOD — module registry)

**Analog:** the `pub mod …;` block (7-29) + `RunCommand` trait (34-36).

Register the two new modules in the alphabetical `pub mod` list (between `color`/`cowsay` and after `clip`):
```rust
pub mod completions;
pub mod config;
```
The `RunCommand` trait each new Args struct implements is unchanged (lines 31-36):
```rust
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}
```

---

### `install.ps1` (MOD — installer)

**Analog:** the idempotent PATH-guard block (42-58) — reuse its "read → dedup-check → conditional write" idiom for the `$PROFILE` sentinel.

Add (1) a default one-line hint (the D-11 / specifics wording) after the smoke test (~line 67), and (2) an opt-in `-RegisterCompletions` `[switch]` param (declare at top, near `Set-StrictMode` line 20). The idempotency guard mirrors the existing PATH dedup (lines 45-58) — instead of `-inotcontains $BinDir`, guard with `Select-String -Quiet -Pattern '# box completions' $PROFILE` before appending the sentinel-wrapped block. Existing PATH-guard template to adapt (install.ps1:45-58):
```powershell
if ($entries -inotcontains $BinDir) {
    # ... conditional write ...
    Write-Host "Added $BinDir to user PATH"
} else {
    Write-Host "$BinDir already in user PATH — skipped"
}
```
Append `box completions powershell | Out-String | Invoke-Expression` (the live-command form, D-11) wrapped in a `# box completions` sentinel; ensure `$PROFILE` + its parent dir exist first (same `New-Item -ItemType Directory -Force` idiom as line 35).

---

### `Cargo.toml` (MOD — deps, D-12)

**Analog:** the existing `clap`/`toml`/`serde` dep lines (13, 23, 102).

Add `clap_complete` pinned to match `clap 4.6` (line 13). No other new crate — `config` reuses `toml = "1.1.2"` (line 23) + `serde` derive (line 102). This is the ONE sanctioned new dep (D-12), consistent with the lean-dep / no-duplicate-crate ethos that drove the `toml_edit` rejection (D-02).

---

## Shared Patterns

### `is_json_on()` fork FIRST (JSON purity — SPINE-01 / Pitfall 1)
**Source:** `src/commands/uuid/mod.rs:116-131` (canonical) + `src/core/output.rs:109-162` (spine).
**Apply to:** `config show` and `config get` (both `--json`-capable).
The ONLY stdout write reachable under `--json` must be `emit_json`. Build the data once, fork on `is_json_on()`, `emit_json(&doc)` on true, `out_line(...)` on false. `emit_json` (output.rs:152-162) writes one pretty serde doc, no BOM, no ANSI, single trailing `\n`, and tees to `--clip`. `completions` follows the same PURITY rule (stdout = exactly the script) without using `emit_json`.

### `RunCommand` static dispatch
**Source:** `src/commands/mod.rs:34-36`; impl example `src/commands/uuid/mod.rs:81-133`.
**Apply to:** both new commands. Each Args struct impls `fn run(self) -> anyhow::Result<()>`; `main()` matches the `Commands` variant and calls `.run()`. No `Box<dyn>`.

### Config-precedence resolver (SPINE-05)
**Source:** `src/core/config.rs:160-163` (`resolve_algo`) + `src/commands/weather/mod.rs:167-176` (`resolve_units`/`resolve_location`).
**Apply to:** `config show` / `config get` effective-value computation.
`cli.or(env).or(cfg).unwrap_or(builtin)` — for `config` the per-key form is `config().<table>.<field>.unwrap_or(<builtin>)`. `show` MUST display exactly what real commands consume so it can't lie (D-06). `weather.location` has no builtin → the D-07 exit-1 carve-out.

### Typed-error → exit-2 downcast (0/1/2 contract)
**Source:** `src/core/errors.rs:45-66` (variants) + `src/main.rs:133-144` (downcast).
**Apply to:** `config set` validate-fail / unknown-key (exit 2). Reuse `BoxError::Config` or add a sibling; wire into the downcast set. The `get` unset-no-default → exit 1 must NOT be a `BoxError` usage variant (git-style data signal, silent stderr).

### Validate-before-persist round-trip (D-03)
**Source:** `src/core/config.rs:110-125` (`load` mapping) + `src/core/config.rs:230-256` (`malformed_maps_to_config_error` harness).
**Apply to:** `config set`. Re-parse the reconstructed doc through the SAME `toml::from_str::<Config>` startup uses; write nothing on any error. Makes a self-inflicted exit-2 lockout structurally impossible.

### Typed `ValueEnum` + lowercase serde rename
**Source:** `src/commands/hash/mod.rs:114-125` (`Algo`) + `src/commands/weather/mod.rs:67-74` (`Units`).
**Apply to:** `completions` shell arg reuses `clap_complete::Shell` (already a `ValueEnum`); `config set`'s value validation leans on the existing `Algo`/`Units` `Deserialize` derives — no new enum, the round-trip validates the string.

---

## No Analog Found

| Symbol / File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `clap_complete::generate(...)` call in `completions/mod.rs` | code generator | transform | No command in the repo generates shell completions; this is the one net-new mechanic (D-09). Skeleton (Args + `RunCommand`) is well-covered, but the `generate` + `CommandFactory` core comes from RESEARCH.md, not a codebase analog. |
| nested `#[command(subcommand)]` on an Args struct (`ConfigArgs`) | route | dispatch | No existing COMMAND uses a nested subcommand — `config` is the first (D-01). The only in-repo precedent is the TOP-LEVEL parser (`cli.rs:34-36`), which is the pattern to copy one level down. Flag its novelty in the plan. |
| atomic temp-write + rename helper | fs helper | file-I/O | `fs.rs` has `safe_copy` (create-new copy) but no write-then-rename-over-existing helper. Closest style analog is `safe_copy` (`fs.rs:80`) + `cache`'s parent-dir-create; the atomic rename itself is new (recommend `core::fs`). |

---

## Metadata

**Analog search scope:** `src/commands/**` (25 command modules), `src/core/**` (`config`, `output`, `errors`, `fs`, `cache`, `input`, `mod`), `src/cli.rs`, `src/main.rs`, `install.ps1`, `Cargo.toml`.
**Files scanned:** 12 read in full/targeted; directory listing of `src/commands` (25 modules) + `src/core` (7 modules).
**Pattern extraction date:** 2026-07-14
