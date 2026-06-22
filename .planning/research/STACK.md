# Stack Research

**Domain:** Rust single-binary multi-subcommand CLI toolbox (Windows PowerShell 7)
**Researched:** 2026-06-22
**Confidence:** HIGH (all versions verified against docs.rs / official sources)

---

## Recommended Stack

### Core Framework

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| clap | 4.6.1 | CLI parsing + subcommand dispatch | Unanimous community standard. Derive API turns each subcommand into a struct/enum — zero boilerplate, automatic `--help`, shell completions. The `#[command(subcommand)]` enum pattern is exactly the right architecture for 23 commands in one binary. |
| anyhow | 1.0.102 | Error propagation in binaries | The correct pairing with CLI apps: wrap any error in `anyhow::Error`, add `.context("...")` at every boundary, print to stderr at `main()`. No need to define custom error types in application code. |
| thiserror | 2.0.18 | Structured errors in library-level modules | When individual command modules need typed errors (e.g., hash mismatch vs. file not found), thiserror derives `std::error::Error` cleanly. Pair with anyhow at the binary boundary. |

### Terminal Output & Colors

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| crossterm | 0.29.0 | Raw terminal control: cursor, clear screen, ANSI sequences | Best Windows ANSI story — handles `ENABLE_VIRTUAL_TERMINAL_PROCESSING` via the `windows` API internally. Required for matrix rain (timed screen redraws) and any command that moves the cursor or reads key events. Actively maintained. |
| owo-colors | 4.3.0 | Inline text colorization (lolcat, tree, du, cowsay) | Zero-allocation, no runtime dependencies. Works as a trait on `&str`: `"text".green()`. For lolcat and matrix, compute RGB values manually and use `truecolor(r, g, b)`. Complements crossterm: owo-colors builds ANSI escape strings, crossterm handles the Windows API plumbing. |
| enable-ansi-support | 0.3.1 | One-time Windows ANSI bootstrap | PowerShell 7 itself supports ANSI but child processes inherit the console mode. Call `enable_ansi_support::enable_ansi_support()` once at startup to set `ENABLE_VIRTUAL_TERMINAL_PROCESSING` so raw ANSI escapes render instead of printing literally. Costs ~3 lines of code. |

**Why NOT termcolor?** It targets the legacy Windows Console API (WriteConsoleW with attributes) rather than ANSI escape codes. The API is more complex, requires a `BufferedStandardStream` wrapper, and produces different code paths per OS. PowerShell 7 fully supports VT codes — there is no reason to use the old API.

**Why NOT colored?** The `colored` crate does not initialize ANSI support on Windows itself and has historically been unreliable in non-Unix terminals. owo-colors with enable-ansi-support is the correct 2025 approach.

### QR Code

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| qr2term | 0.3.3 | Render QR code as Unicode blocks in the terminal | Purpose-built for exactly this use case: `qr2term::print_qr("https://...")`. Uses half-block Unicode characters (▄ ▀) for compact output. Wraps the `qrcode` crate internally. No need to reach for the lower-level `qrcode` crate directly unless you need SVG/PNG output, which this project does not. |

### Clipboard

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| arboard | 3.6.1 | Read and write Windows clipboard (text and image) | Actively maintained by 1Password — the highest-credibility Rust clipboard crate. Windows backend uses the Win32 clipboard API correctly. `Clipboard::new()?.get_text()` and `set_text()` cover the full `box clip` use case. copypasta is an older fork with less active maintenance; arboard is the current community default. |

### Windows Toast Notifications (pomodoro)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| winrt-notification | 0.5.1 | Windows 10/11 toast notifications | The most established Rust wrapper for the WinRT Toast API. Simple API: `Toast::new(Toast::POWERSHELL_APP_ID).title("Pomodoro").text1("Break time!").show()`. Requires PowerShell App ID or a registered AUMID — using the PowerShell AUMID (`{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe`) works without any registration step, which is ideal for a CLI tool. **NOTE:** This crate is not actively developed. If it fails to compile against a future MSRV, fall back to `winrt-toast` (0.1.1) which wraps the `windows` crate directly. |

**Alternative — winrt-toast (0.1.1):** Thinner wrapper built on the `windows` crate (0.62.2). More verbose but will track the `windows` crate releases. Use if `winrt-notification` stalls.

**Avoid:** `notify-rust` is primarily a Linux/dbus crate; Windows support is an afterthought and has known issues.

### File Hashing (hash command)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| blake3 | 1.8.5 | BLAKE3 hashing — the primary algorithm | Faster than SHA-256 on modern CPUs, cryptographically secure, auto-detects AVX2/SSE4.1. The `box hash` command should default to BLAKE3 for new use cases. |
| sha2 | 0.11.0 | SHA-224, SHA-256, SHA-384, SHA-512 | Required for interoperability — most file verification workflows (downloads, Docker images) use SHA-256. Part of RustCrypto; uses the `digest` trait, which enables a unified `Hasher` interface. |
| md-5 | (RustCrypto latest) | MD5 — legacy compatibility only | MD5 is cryptographically broken but widely required for legacy file checksums. Note the crate name is `md-5` (hyphenated), not `md5` — the `md5` crate doesn't implement the `digest` trait and is not interoperable with RustCrypto. |

All three implement the `digest::Digest` trait, enabling a single generic streaming function that reads the file in chunks.

### HTTP Client (weather command)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| ureq | 3.3.0 | HTTP GET requests (blocking, sync) | This project has no async requirements. ureq is a blocking HTTP client with native TLS and a minimal dependency tree — far smaller final binary than reqwest+tokio. For a CLI tool making 1-2 API calls at startup, async adds nothing. ureq 3.x uses rustls by default (no OpenSSL, no system DLL dependency). |
| serde + serde_json | 1.0.150 | JSON deserialization of API responses | Standard. Enable `serde/derive` and annotate response structs with `#[derive(Deserialize)]`. |

**Why NOT reqwest?** reqwest pulls in tokio (async runtime) and its many dependencies, adding ~2-4 MB to binary size and significant compile time. The blocking API wrapper still instantiates the async runtime underneath. For this project's single-shot weather fetch, ureq is the correct choice.

**Weather API: Open-Meteo** — Use `https://api.open-meteo.com/v1/forecast` for weather data and `https://geocoding-api.open-meteo.com/v1/search` for city-name-to-coordinates resolution. No API key required, no registration, free for non-commercial use, up to 10,000 calls/day, returns JSON. This is the only free weather API with zero friction (wttr.in is text-only and harder to parse programmatically).

### Image Decoding & ASCII Art

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| image | 0.25.10 | Decode JPEG, PNG, WebP, GIF, BMP, etc. | The de-facto standard Rust image library. Used by artem internally. Needed directly if you want fine-grained control (resize before processing, etc.). |
| artem | 3.0.0 | Convert image to ASCII art | Purpose-built library + CLI. `artem::convert(img, &options)` returns a colored String ready to print. Supports truecolor with 16-color fallback — PowerShell 7 supports truecolor so the output will be full quality. The simplest correct answer for the `box ascii` command. |

### JSON, Base64, UUID, Password Generation

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| serde_json | 1.0.150 | Parse, validate, pretty-print JSON | Standard. `serde_json::from_str()` for validation, `serde_json::to_string_pretty()` for formatting. No alternative is worth considering. |
| base64 | 0.22.1 | Encode/decode base64 | The standard crate. Use `base64::engine::general_purpose::STANDARD.encode(bytes)` and `.decode(str)`. The API changed in 0.21 to use explicit Engine objects — the 0.22 API is stable. |
| uuid | 1.23.3 | Generate and format UUIDs | Standard. Enable `features = ["v4"]` for random UUIDs (most common), `["v7"]` for time-ordered UUIDs. UUID v4 is what developers expect from a `box uuid` command. |
| passwords | 3.1.16 | Random password generation with strength scoring | Provides a fluent builder API and scoring out of the box. For passphrases (word-based), implement a simple word-list picker using `rand` — there is no single authoritative passphrase crate with wide adoption. |
| rand | 0.9.x (latest stable) | Cryptographically secure random for passphrase word picking | Required for secure passphrase generation from a bundled word list. `rand::rngs::OsRng` uses the Windows CryptGenRandom API. |

### Directory Walking (flatten, dupes, tree, du)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| walkdir | 2.5.0 | Recursive directory iteration | The BurntSushi canonical implementation. Simple streaming iterator, configurable symlink following, min/max depth. Use for `tree`, `flatten`, and `du` where you want full control over traversal and output. |
| ignore | 0.4.26 | Directory walking with .gitignore awareness | Built on walkdir by the same author. Use for `dupes` where `.gitignore` awareness is a nice default — developers running duplicate detection in a repo shouldn't have to manually exclude `node_modules` and `target`. Also useful for `bulk-rename` preview to skip VCS noise. |
| rayon | 1.12.0 | Parallel iteration for dupes content hashing | Converting `walkdir` output to a `rayon::ParallelIterator` gives free multi-core parallelism for the content-hash phase of duplicate detection. Essential for performance on large directories. |

### Progress Indication

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| indicatif | 0.18.4 | Progress bars and spinners | Standard for Rust CLIs. Works correctly on Windows terminals. Use for `dupes` (scanning N files), `hash` (large file), and `flatten` (copying many files). Renders in-place without scrolling the terminal. |

### Chrono / Epoch

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| chrono | 0.4.45 | Timestamp parsing and formatting (epoch command) | Standard date/time library. `DateTime<Utc>::from_timestamp(epoch, 0)` and format with `strftime`-style patterns. Use `chrono-tz` if timezone-aware output is needed. |

---

## Cargo.toml Structure

```toml
[package]
name = "box"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "box"
path = "src/main.rs"

[dependencies]
# CLI
clap = { version = "4.6", features = ["derive"] }
anyhow = "1.0"
thiserror = "2.0"

# Terminal
crossterm = "0.29"
owo-colors = "4.3"
enable-ansi-support = "0.3"
indicatif = "0.18"

# QR
qr2term = "0.3"

# Clipboard
arboard = "3.6"

# Notifications (Windows-only)
[target.'cfg(windows)'.dependencies]
winrt-notification = "0.5"

# Hashing
blake3 = "1.8"
sha2 = "0.11"
md-5 = "0.10"       # check crates.io for latest RustCrypto md-5
digest = "0.10"      # unified trait

# HTTP
ureq = { version = "3.3", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1.0"

# Data
base64 = "0.22"
uuid = { version = "1.23", features = ["v4", "v7"] }
chrono = "0.4"

# Passwords
passwords = "3.1"
rand = "0.9"

# Image / ASCII
image = "0.25"
artem = "3.0"

# Directory walking
walkdir = "2.5"
ignore = "0.4"
rayon = "1.12"

[profile.release]
opt-level = 3
lto = "thin"          # "fat" for maximum size reduction at the cost of link time
codegen-units = 1
panic = "abort"
strip = "symbols"
```

**Note on `opt-level`:** Use `opt-level = 3` (speed) not `"z"` (size) — this is a developer tool where speed matters more than binary bytes. `strip = "symbols"` and `lto = "thin"` reduce size meaningfully without hurting runtime performance.

---

## Release Build / Static Linking

**Use `x86_64-pc-windows-msvc`** (the default Rust Windows target).

- MSVC is the native Windows ABI — best compatibility with Windows APIs needed by `arboard`, `winrt-notification`, and `crossterm`.
- Static link the CRT to eliminate the MSVCRT DLL dependency: set `RUSTFLAGS="-C target-feature=+crt-static"` in `install.ps1` before `cargo build --release`. This adds ~100 KB to the binary but makes it truly portable — copy the `.exe` anywhere on Windows 10/11, no redistributable needed.
- The `x86_64-pc-windows-gnu` target (MinGW) was demoted from Tier 1 in Rust 1.88 (May 2025). Avoid it unless cross-compiling from Linux.

```powershell
# install.ps1 build invocation
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc
```

Expected release binary size: 8-15 MB depending on feature usage. This is acceptable for a developer toolbox — do not sacrifice functionality to chase a smaller binary.

---

## Command Architecture

Use the clap derive enum-dispatch pattern. One `Commands` enum, one `match` in `main`:

```rust
// src/main.rs
#[derive(Parser)]
#[command(name = "box", about = "A CLI toolbox")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Flatten(flatten::Args),
    Qr(qr::Args),
    Passgen(passgen::Args),
    Hash(hash::Args),
    // ...23 total
}

fn main() -> anyhow::Result<()> {
    enable_ansi_support::enable_ansi_support().ok();
    let cli = Cli::parse();
    match cli.command {
        Commands::Flatten(args) => flatten::run(args),
        Commands::Qr(args) => qr::run(args),
        // ...
    }
}
```

Each subcommand lives in `src/commands/<name>.rs` and exports `pub struct Args` (clap Args derive) and `pub fn run(args: Args) -> anyhow::Result<()>`.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| clap 4.x derive | argh, pico-args | No derive subcommand dispatch; clap is the community standard with the best help formatting |
| owo-colors + crossterm | termcolor | termcolor uses legacy Windows Console API; owo-colors + crossterm is the correct 2025 ANSI approach |
| ureq (blocking) | reqwest (async) | reqwest forces tokio runtime; adds 2-4 MB and compile time for zero benefit in a sequential CLI |
| arboard | copypasta | copypasta is less actively maintained; arboard is maintained by 1Password and has wider test coverage |
| Open-Meteo API | OpenWeatherMap | OpenWeatherMap requires an API key registration; Open-Meteo is truly zero-friction |
| walkdir / ignore | std::fs::read_dir | `read_dir` is not recursive; walkdir is the correct abstraction |
| blake3 as default hash | SHA-256 as default | BLAKE3 is faster and equally secure; SHA-256 available as `--algo sha256` flag |
| winrt-notification | notify-rust | notify-rust's Windows backend is an afterthought; winrt-notification wraps the native WinRT API |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `termcolor` | Targets deprecated Windows Console API; more complex API, non-ANSI | `owo-colors` + `enable-ansi-support` |
| `colored` | Does not initialize ANSI on Windows; unreliable in PowerShell | `owo-colors` |
| `reqwest` (with tokio) | Pulls in async runtime for synchronous CLI use — significant binary bloat | `ureq 3.x` |
| `md5` (crate) | Does not implement the `digest` trait; not interoperable with RustCrypto | `md-5` (RustCrypto, hyphenated) |
| `notify-rust` | Linux-first; Windows support is unreliable | `winrt-notification` or `winrt-toast` |
| `x86_64-pc-windows-gnu` | Demoted to Tier 2 in Rust 1.88 (May 2025); requires MinGW toolchain | `x86_64-pc-windows-msvc` |
| `tokio` (as direct dep) | No async I/O needs exist in this project; tokio adds compile time and complexity | No runtime needed — all I/O is synchronous |
| `clap` builder API | More verbose than derive; worse DX for 23 subcommands | `clap` derive API |
| `opt-level = "z"` in release | Shrinks binary at the cost of runtime performance; wrong tradeoff for a dev tool | `opt-level = 3` + `strip = "symbols"` |

---

## Windows PowerShell 7 — Specific Considerations

1. **ANSI initialization is mandatory.** Call `enable_ansi_support::enable_ansi_support().ok()` as the very first line of `main()`. The `.ok()` discards the `Result` — if it fails (e.g., output is not a terminal), ANSI codes are simply not emitted, which is correct behavior for piped output.

2. **Unicode rendering.** PowerShell 7 + Windows Terminal renders Unicode well (block elements, box-drawing characters). The `qr2term` half-block QR output and `cowsay` box-drawing will render correctly. Fallback to ASCII art alternatives only if the user pipes to a file.

3. **Exit codes.** PowerShell inspects `$LASTEXITCODE`. Return `std::process::exit(1)` (or use anyhow's automatic behavior) on error — do not `panic!` in production paths.

4. **Console width.** Use `crossterm::terminal::size()` to detect terminal columns before rendering tree/matrix/lolcat output. Avoid hardcoding 80 columns.

5. **Clipboard and notifications require no UAC elevation.** Both the Win32 clipboard API and WinRT toast notifications work from a normal user process — no special permissions needed.

6. **Pomodoro timer blocking.** `std::thread::sleep(Duration::from_secs(25 * 60))` is fine in a CLI tool. The user can Ctrl+C to cancel. No async required.

---

## Sources

- `docs.rs/clap` — version 4.6.1 confirmed; derive API docs reviewed
- `docs.rs/crossterm` — version 0.29.0 confirmed; Windows ANSI documentation reviewed
- `docs.rs/owo-colors` — version 4.3.0 confirmed
- `docs.rs/enable-ansi-support` — version 0.3.1 confirmed; Windows VT mode documentation reviewed
- `docs.rs/arboard` — version 3.6.1 confirmed; Windows backend reviewed
- `docs.rs/qr2term` — version 0.3.3 confirmed
- `docs.rs/blake3` — version 1.8.5 confirmed; performance claims verified
- `docs.rs/sha2` — version 0.11.0 confirmed (RustCrypto)
- `docs.rs/ureq` — version 3.3.0 confirmed; rustls default documented
- `docs.rs/reqwest` — version 0.13.4 confirmed; tokio dependency structure reviewed
- `docs.rs/serde_json` — version 1.0.150 confirmed
- `docs.rs/base64` — version 0.22.1 confirmed; Engine API change documented
- `docs.rs/uuid` — version 1.23.3 confirmed
- `docs.rs/passwords` — version 3.1.16 confirmed
- `docs.rs/walkdir` — version 2.5.0 confirmed
- `docs.rs/ignore` — version 0.4.26 confirmed
- `docs.rs/rayon` — version 1.12.0 confirmed
- `docs.rs/chrono` — version 0.4.45 confirmed
- `docs.rs/indicatif` — version 0.18.4 confirmed
- `docs.rs/anyhow` — version 1.0.102 confirmed
- `docs.rs/thiserror` — version 2.0.18 confirmed
- `docs.rs/image` — version 0.25.10 confirmed
- `docs.rs/artem` — version 3.0.0 confirmed; truecolor support documented
- `docs.rs/winrt-notification` — version 0.5.1; maintenance status uncertain (LOW confidence on long-term viability)
- `docs.rs/winrt-toast` — version 0.1.1 confirmed; windows crate dependency noted
- `docs.rs/windows` — version 0.62.2 confirmed (Microsoft official crate)
- Open-Meteo API — https://open-meteo.com/ — no-key free weather + geocoding API confirmed
- Rust Blog (2025-05-26) — `i686-pc-windows-gnu` Tier 1 demotion in Rust 1.88 confirmed
- rust-cli-recommendations.sunshowers.io — Windows ANSI and color recommendations reviewed

---

*Stack research for: box — Rust CLI Toolbox (Windows PowerShell 7)*
*Researched: 2026-06-22*
