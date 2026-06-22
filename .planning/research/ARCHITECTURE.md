# Architecture Research

**Domain:** Single-binary multi-subcommand Rust CLI toolbox (Windows PowerShell 7)
**Researched:** 2026-06-22
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         box (binary entry point)                     │
│  src/main.rs — parse Cli struct, dispatch to Commands enum           │
├─────────────────────────────────────────────────────────────────────┤
│                      Commands Enum (clap Subcommand)                 │
│                                                                      │
│  flatten  qr  passgen  hash  dupes  bulk-rename  tree  clip  uuid   │
│  json  base64  epoch  color  du  ascii  cowsay  lolcat  matrix       │
│  roast  fortune  8ball  pomodoro  weather                            │
│                                                                      │
│  Each variant holds its own Args struct — parsed by clap derive      │
├─────────────────────────────────────────────────────────────────────┤
│                    Shared Core  (src/core/)                          │
│                                                                      │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐              │
│  │  errors.rs  │  │  output.rs   │  │   fs.rs       │              │
│  │  BoxError   │  │  color/print │  │  walk/copy    │              │
│  └─────────────┘  └──────────────┘  └───────────────┘              │
├─────────────────────────────────────────────────────────────────────┤
│                  Per-Command Modules  (src/commands/)                │
│                                                                      │
│  flatten/mod.rs   qr/mod.rs   passgen/mod.rs   hash/mod.rs  ...     │
│  Each module: Args struct + impl RunCommand + unit-testable logic    │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| `src/main.rs` | Parse top-level CLI, match on Commands enum, call `.run()`, translate `anyhow::Error` to exit code | ~40 lines, no business logic |
| `src/cli.rs` | `Cli` struct + `Commands` enum holding all 23 variants | clap `#[derive(Parser, Subcommand)]` |
| `src/core/errors.rs` | Project-wide `BoxError` type; maps to exit codes | `thiserror` enum + `ExitCode` impl |
| `src/core/output.rs` | Shared print helpers — `success()`, `warn()`, `error()`, color detection, NO_COLOR | `owo-colors` + `enable-ansi-support` |
| `src/core/fs.rs` | Shared filesystem helpers — walk dir, safe copy, temp dir | std + `walkdir` |
| `src/commands/<cmd>/mod.rs` | Args struct + `RunCommand` impl for that command | one module per command |
| `tests/` | Integration tests per command via `assert_cmd` + `predicates` | `tests/<cmd>.rs` |

---

## Recommended Project Structure

```
cli-tools/
├── Cargo.toml                  # single-crate binary, [profile.release] tuning
├── install.ps1                 # build → copy → PATH (idempotent)
├── src/
│   ├── main.rs                 # parse Cli, dispatch, exit code mapping
│   ├── cli.rs                  # Cli + Commands (all 23 variants listed)
│   ├── core/
│   │   ├── mod.rs
│   │   ├── errors.rs           # BoxError enum (thiserror) + ExitCode mapping
│   │   ├── output.rs           # color helpers, success/warn/error macros
│   │   └── fs.rs               # walkdir wrapper, safe_copy, collision rename
│   └── commands/
│       ├── mod.rs              # pub use each command module
│       ├── flatten/
│       │   ├── mod.rs          # FlattenArgs + impl RunCommand
│       │   └── rename.rs       # collision-encoding logic (unit-testable)
│       ├── qr/
│       │   └── mod.rs
│       ├── passgen/
│       │   └── mod.rs
│       ├── hash/
│       │   └── mod.rs
│       ├── dupes/
│       │   └── mod.rs
│       ├── bulk_rename/
│       │   └── mod.rs
│       ├── tree/
│       │   └── mod.rs
│       ├── clip/
│       │   └── mod.rs
│       ├── uuid/
│       │   └── mod.rs
│       ├── json/
│       │   └── mod.rs
│       ├── base64/
│       │   └── mod.rs
│       ├── epoch/
│       │   └── mod.rs
│       ├── color/
│       │   └── mod.rs
│       ├── du/
│       │   └── mod.rs
│       ├── ascii/
│       │   └── mod.rs
│       ├── cowsay/
│       │   └── mod.rs
│       ├── lolcat/
│       │   └── mod.rs
│       ├── matrix/
│       │   └── mod.rs
│       ├── roast/
│       │   └── mod.rs
│       ├── fortune/
│       │   └── mod.rs
│       ├── eight_ball/
│       │   └── mod.rs          # `8ball` is an invalid Rust identifier; use eight_ball module
│       ├── pomodoro/
│       │   └── mod.rs
│       └── weather/
│           └── mod.rs
└── tests/
    ├── flatten.rs
    ├── passgen.rs
    ├── hash.rs
    └── ...                     # one integration test file per command
```

### Structure Rationale

- **Single crate, not a workspace:** For 23 commands targeting one binary, a workspace adds build orchestration complexity with minimal benefit. Workspace shines when crates are independently published or consumed. Here all 23 commands compile into one artifact; a single crate with a `src/commands/` tree gives the same module isolation without the overhead. Incremental compilation within a single crate is fast enough. (Source: Rust workspace performance analysis confirms gains are modest for same-binary targets.)
- **`src/commands/<cmd>/mod.rs` pattern:** Each command is a self-contained module with its own `Args` struct and `RunCommand` impl. Adding a new command means: create the folder, implement the trait, add one variant to `Commands` in `cli.rs`. Removing a command is the reverse. No other files change.
- **`src/core/` for shared infrastructure:** Keeps shared utilities from bleeding into command modules. Commands import from `crate::core::*`. This boundary is enforced by Rust's module system — no circular deps possible.
- **`tests/<cmd>.rs` one-per-command:** Allows running `cargo test flatten` to test only one command's integration suite. Keeps CI feedback fast and isolated.

---

## Architectural Patterns

### Pattern 1: RunCommand Trait Dispatch

**What:** A single trait with a `run(self) -> anyhow::Result<()>` method. Each command's Args struct implements it. `main.rs` matches on the `Commands` enum and calls `.run()`.

**When to use:** Always — this is the core dispatch mechanism for all 23 commands.

**Trade-offs:** Slightly more boilerplate than inlining match arms, but keeps `main.rs` under 50 lines forever regardless of how many commands are added.

**Example:**

```rust
// src/commands/mod.rs — the trait every command implements
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}

// src/main.rs
fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Flatten(args) => args.run(),
        Commands::Qr(args)      => args.run(),
        Commands::Passgen(args) => args.run(),
        // ... 20 more lines, one per command
    };
    match result {
        Ok(())  => ExitCode::SUCCESS,
        Err(e)  => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

// src/commands/passgen/mod.rs
#[derive(Args)]
pub struct PassgenArgs {
    #[arg(short, long, default_value = "16")]
    pub length: usize,
    #[arg(long)]
    pub no_symbols: bool,
}

impl RunCommand for PassgenArgs {
    fn run(self) -> anyhow::Result<()> {
        let pw = generate_password(self.length, !self.no_symbols)?;
        println!("{pw}");
        Ok(())
    }
}
```

**Note:** Do NOT use `Box<dyn RunCommand>` / dynamic dispatch. The enum match is fully statically dispatched by the compiler — zero overhead, and the match exhaustiveness check ensures every command variant is handled.

---

### Pattern 2: Clap Derive with Per-Command Args Structs

**What:** Each command variant in the `Commands` enum holds its own dedicated `Args` struct, not loose fields.

**When to use:** Always — flat inline fields in enum variants cause coupling between commands and make unit-testing individual argument parsing impossible.

**Trade-offs:** One extra struct per command (trivial cost) in exchange for each command's arguments being independently testable and documented.

**Example:**

```rust
// src/cli.rs
#[derive(Parser)]
#[command(name = "box", version, about = "A Rust CLI toolbox")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Flatten a folder tree into a single output directory
    Flatten(FlattenArgs),
    /// Generate a QR code in the terminal
    Qr(QrArgs),
    /// Generate a secure password or passphrase
    Passgen(PassgenArgs),
    // ... all 23
}
```

The `Commands` enum in `cli.rs` is the only file that lists all 23 commands. It serves as the authoritative registry — a new developer can see all commands in one screen.

---

### Pattern 3: Shared Error Type + anyhow for Propagation

**What:** Use `anyhow::Result<()>` everywhere inside command logic for easy `?` propagation and context attachment. Use a thin `BoxError` enum with `thiserror` only for cases where the exit code must differ from generic failure (e.g., network unavailable = exit 2, bad user input = exit 1).

**When to use:** Default to `anyhow::Result` in all command `run()` methods. Promote to a typed `BoxError` variant only when a distinct exit code matters to a calling script.

**Trade-offs:** `anyhow` makes happy-path development fast. `thiserror` enums are more verbose but allow `match e.downcast_ref::<BoxError>()` in `main` to set exit codes.

**Example:**

```rust
// src/core/errors.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BoxError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Invalid input: {0}")]
    BadInput(String),
    #[error("File not found: {path}")]
    NotFound { path: std::path::PathBuf },
}

// In main, map typed errors to exit codes:
fn exit_code_for(e: &anyhow::Error) -> ExitCode {
    if e.downcast_ref::<BoxError>().is_some() {
        // could inspect variant for different codes
        ExitCode::from(2)
    } else {
        ExitCode::FAILURE  // generic = 1
    }
}
```

---

### Pattern 4: Shared Output Helpers

**What:** A `src/core/output.rs` module exposing `print_success`, `print_warn`, `print_error`, and color-enabling initialization. Commands import these instead of calling `println!` directly.

**When to use:** Always — ensures consistent styling across all 23 commands and makes NO_COLOR / FORCE_COLOR handling happen in one place.

**Example:**

```rust
// src/core/output.rs
use owo_colors::OwoColorize;

pub fn init_color() {
    // enable ANSI on Windows — must be called once in main() before any output
    let _ = enable_ansi_support::enable_ansi_support();
    // owo-colors respects NO_COLOR automatically via set_override
    if std::env::var("NO_COLOR").is_ok() {
        owo_colors::set_override(false);
    }
}

pub fn print_success(msg: &str) {
    println!("{}", msg.green());
}
pub fn print_error(msg: &str) {
    eprintln!("{}", msg.red());
}
```

---

### Pattern 5: Extracting Testable Logic from run()

**What:** The `run()` method should be a thin orchestrator. Real logic lives in pure functions that take no I/O and return values.

**When to use:** Any command with non-trivial logic. Commands that are pure transformations (epoch, base64, color, uuid) are especially easy to unit-test this way.

**Example:**

```rust
// src/commands/epoch/mod.rs

// Pure, unit-testable:
pub fn unix_to_human(ts: i64) -> String { /* ... */ }
pub fn human_to_unix(s: &str) -> anyhow::Result<i64> { /* ... */ }

// Thin orchestrator:
impl RunCommand for EpochArgs {
    fn run(self) -> anyhow::Result<()> {
        match self.mode {
            EpochMode::ToHuman { ts } => println!("{}", unix_to_human(ts)),
            EpochMode::ToUnix  { s  } => println!("{}", human_to_unix(&s)?),
        }
        Ok(())
    }
}
```

---

## Data Flow

### Command Execution Flow

```
User types: box flatten ./src ./out

main()
  └── Cli::parse()            // clap parses argv into Cli { command: Commands::Flatten(FlattenArgs { src, out, .. }) }
        └── match cli.command
              └── Commands::Flatten(args) => args.run()
                    └── FlattenArgs::run(self)
                          ├── core::output::init_color()   // ANSI enable (idempotent)
                          ├── core::fs::walk_dir(src)      // collect files
                          ├── for each file: resolve_collision() -> copy
                          └── Ok(()) -> ExitCode::SUCCESS (0)

On error: Err(e) -> eprintln!("{e:#}") -> ExitCode::FAILURE (1)
```

### Exit Code Convention

| Code | Meaning | When |
|------|---------|------|
| 0 | Success | Command completed as expected |
| 1 | General failure | Unexpected error, logic failure, panic |
| 2 | Bad user input / usage error | Invalid args not caught by clap, bad format |
| Network failures | Map to 1 for simplicity in v1; can graduate to distinct code later | |

Clap itself exits with code 2 for argument parse errors (missing required arg, unknown flag). This is consistent with standard POSIX convention and requires no custom handling.

---

## Command Groupings for Parallel Phase Development

Group commands by their external dependency footprint. Commands in the same group can be built in parallel because they share crate dependencies and do not block on each other.

### Group A — Zero External Deps (Pure Transforms)

No external crates beyond std + clap + anyhow. Highest confidence, build first.

| Command | Logic |
|---------|-------|
| `uuid` | `uuid` crate v1/v4 generation |
| `base64` | std or `base64` crate encode/decode |
| `epoch` | `chrono` timestamp conversion |
| `color` | hex ↔ RGB string parsing (pure math) |
| `roast` | static string array, random pick (`rand`) |
| `fortune` | static string array, random pick (`rand`) |
| `8ball` | static string array, random pick (`rand`) |
| `cowsay` | string formatting, ASCII border draw |
| `passgen` | `rand` + optional wordlist |

**Shared dep introduced:** `rand`, `chrono`, `uuid`, `base64`

---

### Group B — Filesystem Operations

Depend on `walkdir` and `std::fs`. Build after Group A (shares `rand`, adds `walkdir`).

| Command | Logic |
|---------|-------|
| `flatten` | `walkdir` + copy + collision rename (anchor command) |
| `tree` | `walkdir` + size formatting |
| `dupes` | `walkdir` + `sha2` hash comparison |
| `bulk-rename` | `walkdir` + `regex` rename |
| `du` | `walkdir` + size aggregation |
| `hash` | `sha2` / `md5` / `blake3` on single file |

**Shared dep introduced:** `walkdir`, `sha2`, `regex`

---

### Group C — Terminal / Output Effects

Depend on terminal rendering crates. Can be built in parallel with Group B.

| Command | Logic |
|---------|-------|
| `lolcat` | ANSI rainbow colorize stdin lines |
| `matrix` | Terminal size + `crossterm` animation loop |
| `ascii` | `image` crate decode + pixel-to-char |
| `json` | `serde_json` pretty-print + validation |

**Shared dep introduced:** `crossterm`, `image`, `serde_json`

---

### Group D — System Integration (Windows-Specific)

Depend on Windows APIs or external services. Higher research risk; build last.

| Command | Logic |
|---------|-------|
| `clip` | `arboard` crate (Windows clipboard read/write) |
| `qr` | `qrcode` crate + terminal render |
| `pomodoro` | Timer loop + `win-toast-notify` Windows toast |
| `weather` | HTTP client (`reqwest`) + weather API |

**Shared dep introduced:** `arboard`, `qrcode`, `reqwest`, `win-toast-notify` (or `winrt-notification`)

---

## install.ps1 Design

### Goals

1. Build release binary with `cargo build --release`
2. Copy `target\release\box.exe` to a dedicated, stable bin dir (not cargo's bin)
3. Add that dir to the user PATH — idempotently (no duplicate entries)
4. Verify the binary is reachable and print a success message

### Concrete Pattern (idempotent PATH update)

```powershell
#Requires -Version 7
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$BinDir = "$env:LOCALAPPDATA\box\bin"
$ExeSrc = ".\target\release\box.exe"
$ExeDst = "$BinDir\box.exe"

# 1. Build
Write-Host "Building box..."
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 2. Copy
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
Copy-Item -Force $ExeSrc $ExeDst
Write-Host "Installed to $ExeDst"

# 3. Idempotent PATH update (user scope only)
$CurrentPath = [Environment]::GetEnvironmentVariable('Path', [System.EnvironmentVariableTarget]::User)
$PathEntries  = $CurrentPath -split ';' | Where-Object { $_ -ne '' }

if ($PathEntries -inotcontains $BinDir) {
    $NewPath = ($PathEntries + $BinDir) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $NewPath, [System.EnvironmentVariableTarget]::User)
    Write-Host "Added $BinDir to user PATH"
} else {
    Write-Host "$BinDir is already in user PATH — skipped"
}

# 4. Refresh current session PATH so box is immediately usable
$env:Path = [Environment]::GetEnvironmentVariable('Path', [System.EnvironmentVariableTarget]::User) + ';' +
            [Environment]::GetEnvironmentVariable('Path', [System.EnvironmentVariableTarget]::Machine)

# 5. Verify
$ver = & box --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "box is available: $ver"
} else {
    Write-Warning "box was installed but could not be invoked. Open a new PowerShell session."
}
```

### Why this approach

- **`$env:LOCALAPPDATA\box\bin`** — user-writable, survives across Cargo versions, no admin required. Clear namespace (`box`) avoids collisions.
- **`-split ';' | -inotcontains`** — splits on semicolon and does case-insensitive membership check before writing, making the update safe to run multiple times. Avoids the `Contains()` substring trap (e.g., `C:\foo` would false-match inside `C:\foobar`).
- **`[System.EnvironmentVariableTarget]::User`** — writes to `HKCU\Environment`, not the machine hive. No admin rights needed.
- **Refresh `$env:Path` in current session** — PowerShell 7 does not automatically pick up registry changes in the running session. The script manually merges user + machine PATH so `box` is immediately available without opening a new terminal.
- **`$LASTEXITCODE` check after `cargo build`** — strict mode + early exit prevents copying a stale binary on build failure.

### Verification in PowerShell 7

After install, the user can confirm availability:

```powershell
Get-Command box                # shows full path
box --version                  # prints version string
box --help                     # lists all 23 subcommands
box flatten --help             # per-command help
```

---

## Testing Strategy

### Layer 1 — Unit Tests (in-module `#[cfg(test)]`)

Test pure functions inside each command module. Use `#[cfg(test)]` blocks within `src/commands/<cmd>/mod.rs` or companion `*_test.rs` files.

Targets: epoch conversions, base64 encode/decode, collision rename logic, hex-to-RGB parsing, password strength rules, hash computation.

### Layer 2 — Integration Tests (`tests/<cmd>.rs` via `assert_cmd`)

Test the compiled binary as a black box. Uses `assert_cmd` + `predicates` + `assert_fs` for temp files.

```rust
// tests/passgen.rs
use assert_cmd::Command;
use predicates::str::is_match;

#[test]
fn default_length_is_16_chars() {
    Command::cargo_bin("box").unwrap()
        .arg("passgen")
        .assert()
        .success()
        .stdout(is_match(r"^\S{16}\n$").unwrap());
}

#[test]
fn explicit_length() {
    Command::cargo_bin("box").unwrap()
        .args(["passgen", "--length", "32"])
        .assert()
        .success()
        .stdout(is_match(r"^\S{32}\n$").unwrap());
}
```

### Layer 3 — Snapshot Tests (`insta` / `trycmd`)

For commands with complex or multi-line output where exact formatting matters: `tree`, `cowsay`, `ascii`, `json` pretty-print. Use `insta` for inline snapshots or `trycmd` for file-based command transcript tests.

`trycmd` is well-suited for documenting expected CLI behavior — tests live as `.toml` or `.md` files listing commands and expected output. Running `cargo test` automatically verifies them and fails with a diff on regression.

```toml
# tests/cmd/cowsay.toml
[[test]]
bin.name = "box"
args = ["cowsay", "hello"]
status.code = 0
stdout = """
 _______
< hello >
 -------
        \\   ^__^
         ...
"""
```

### Test Command for CI

```bash
cargo test              # all unit + integration tests
cargo test flatten      # only flatten integration suite
cargo test --doc        # any doc examples
```

---

## Anti-Patterns

### Anti-Pattern 1: Business Logic in main.rs Match Arms

**What people do:** Put command logic directly inside the `match cli.command { ... }` block in `main.rs`.

**Why it's wrong:** `main.rs` grows with every command. By command 10 it is unreadable. Changing one command risks breaking another because all logic is in one scope. Unit testing is impossible without invoking the whole program.

**Do this instead:** Keep each match arm as a single call `args.run()`. All logic lives in the command's own module.

---

### Anti-Pattern 2: Using a Workspace for a Single Output Binary

**What people do:** Create a Cargo workspace with a crate per command, all compiled into one binary.

**Why it's wrong:** Workspaces shine when crates produce separate artifacts or are published independently. Forcing 23 intra-binary modules into workspace crates adds `Cargo.toml` files per command, complicates inter-command shared code (requires a published or path dependency), and provides no meaningful compile-time benefit over a module tree in one crate.

**Do this instead:** Single crate with `src/commands/<cmd>/` modules. Same isolation, zero workspace overhead.

---

### Anti-Pattern 3: Using `process::exit()` Directly Inside Commands

**What people do:** Call `std::process::exit(1)` inside command logic when something goes wrong.

**Why it's wrong:** Bypasses Rust's Drop machinery (file handles, temp dirs not cleaned up). Makes commands untestable — `assert_cmd` cannot catch `process::exit`.

**Do this instead:** Return `Err(anyhow::anyhow!("message"))` from `run()`. Let `main()` be the only place that calls `process::exit` (or returns `ExitCode`).

---

### Anti-Pattern 4: Checking `args.len()` for Color Support

**What people do:** Check `std::io::stdout().is_terminal()` manually and wrap every `println!` in an `if color_enabled` branch.

**Why it's wrong:** Duplicates color-detection logic across 23 commands. Misses NO_COLOR, FORCE_COLOR, CI detection.

**Do this instead:** Call `core::output::init_color()` once in `main()` before dispatch. Use `owo-colors` + `enable-ansi-support`. The `if_supports_color()` method handles detection automatically at the call site.

---

### Anti-Pattern 5: Duplicating PATH Entry Without Idempotency Check

**What people do:** `install.ps1` always appends the bin dir to PATH, every time it runs.

**Why it's wrong:** After 3 installs the user's PATH has the same dir 3 times. This bloats the registry value (Windows PATH has a ~2048-char registry limit for REG_SZ) and can cause confusing diagnostics.

**Do this instead:** Use the `-inotcontains` split-and-check pattern shown above. The entry is only written if it is absent.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Weather API (OpenMeteo or wttr.in) | `reqwest` blocking HTTP GET, JSON parse | wttr.in has a JSON API requiring no API key — research phase should confirm stability |
| Windows Clipboard | `arboard` crate | Windows-only; `arboard` wraps Win32 clipboard APIs cleanly |
| Windows Toast Notifications | `win-toast-notify` or `winrt-notification` | `pomodoro` only; verify crate maintenance status in stack research |
| QR Terminal Render | `qrcode` crate → `image` → Unicode blocks | `qrcode` outputs a QrCode matrix; render as Unicode half-blocks for terminal display |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `main.rs` → command module | Direct function call via `RunCommand::run(args)` | Statically dispatched through match |
| command module → `core::fs` | Direct function call | No trait; plain utility functions |
| command module → `core::output` | Direct function call | `init_color()` called once in main, not per command |
| command module → `core::errors` | `BoxError` variants returned via `?` | Promotes `anyhow::Error` chain automatically |

---

## Scaling Considerations

This is a CLI tool, not a server. "Scaling" here means maintainability as commands are added, not user load.

| Scale | Architecture Adjustment |
|-------|-------------------------|
| 23 commands (v1 target) | Single crate, modules per command — current design handles this comfortably |
| 50+ commands | Consider splitting into logical workspace crates (file-tools, fun, converters) each producing a lib, unified by a thin binary crate. Not needed for v1. |
| Cross-platform (future) | Isolate Windows-specific code in `#[cfg(target_os = "windows")]` blocks from day 1 so `clip` and `pomodoro` can be feature-gated later |

---

## Sources

- Rain's Rust CLI Recommendations — Handling Arguments: https://rust-cli-recommendations.sunshowers.io/handling-arguments.html
- Rain's Rust CLI Recommendations — Managing Colors: https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html
- Command Line Applications in Rust (official book) — Testing: https://rust-cli.github.io/book/tutorial/testing.html
- Command Line Applications in Rust (official book) — Exit Codes: https://rust-cli.github.io/book/in-depth/exit-code.html
- clap official docs — Subcommands with Derive: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
- Ideas for crafting CLI in Rust (RunCommand trait pattern): https://dev.to/64bit/ideas-for-crafting-cli-in-rust-5b0d
- owo-colors crate (zero-alloc ANSI, Windows-compatible): https://lib.rs/crates/owo-colors
- insta snapshot testing: https://github.com/mitsuhiko/insta
- trycmd CLI transcript testing: https://docs.rs/trycmd/latest/trycmd/
- assert_cmd integration testing: https://rust-cli.github.io/book/tutorial/testing.html
- PowerShell idempotent PATH update pattern: https://abdus.dev/posts/update-path-powershell/
- PowerShell install script PATH registry pattern: https://github.com/PowerShell/PowerShell/blob/master/tools/install-powershell.ps1
- Rust Performance Book — Build Configuration: https://nnethercote.github.io/perf-book/build-configuration.html

---
*Architecture research for: box — Rust CLI toolbox (Windows PowerShell 7)*
*Researched: 2026-06-22*
