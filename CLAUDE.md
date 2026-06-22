<!-- GSD:project-start source:PROJECT.md -->
## Project

**box — A Rust CLI Toolbox**

`box` is a single Rust binary that bundles ~23 command-line tools — a mix of genuinely useful utilities and fun toys — exposed as subcommands (`box flatten`, `box qr`, `box cowsay`, …). It's built for a developer running Windows PowerShell 7, installed globally so any tool is one short command away.

**Core Value:** The toolbox must be **globally available and instantly usable from PowerShell 7** — type `box <command>` from anywhere and the tool just works. If the breadth of commands is impressive but they aren't reachable on PATH, the project has failed at its one job.

### Constraints

- **Tech stack**: Rust, single binary with a subcommand CLI (e.g. `clap`) — keep the surface consistent across all 23 tools.
- **Platform**: Must run in Windows PowerShell 7. Windows-specific integrations (clipboard, toast, ANSI color) must work in that terminal.
- **Distribution**: Global availability achieved via `install.ps1` (build → copy to bin dir → add to user PATH). No assumption that `~/.cargo/bin` is already on PATH.
- **Consistency**: All subcommands share a coherent UX — `--help` everywhere, predictable flag conventions, sensible exit codes.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

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
### File Hashing (hash command)
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| blake3 | 1.8.5 | BLAKE3 hashing — the primary algorithm | Faster than SHA-256 on modern CPUs, cryptographically secure, auto-detects AVX2/SSE4.1. The `box hash` command should default to BLAKE3 for new use cases. |
| sha2 | 0.11.0 | SHA-224, SHA-256, SHA-384, SHA-512 | Required for interoperability — most file verification workflows (downloads, Docker images) use SHA-256. Part of RustCrypto; uses the `digest` trait, which enables a unified `Hasher` interface. |
| md-5 | (RustCrypto latest) | MD5 — legacy compatibility only | MD5 is cryptographically broken but widely required for legacy file checksums. Note the crate name is `md-5` (hyphenated), not `md5` — the `md5` crate doesn't implement the `digest` trait and is not interoperable with RustCrypto. |
### HTTP Client (weather command)
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| ureq | 3.3.0 | HTTP GET requests (blocking, sync) | This project has no async requirements. ureq is a blocking HTTP client with native TLS and a minimal dependency tree — far smaller final binary than reqwest+tokio. For a CLI tool making 1-2 API calls at startup, async adds nothing. ureq 3.x uses rustls by default (no OpenSSL, no system DLL dependency). |
| serde + serde_json | 1.0.150 | JSON deserialization of API responses | Standard. Enable `serde/derive` and annotate response structs with `#[derive(Deserialize)]`. |
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
## Cargo.toml Structure
# CLI
# Terminal
# QR
# Clipboard
# Notifications (Windows-only)
# Hashing
# HTTP
# Data
# Passwords
# Image / ASCII
# Directory walking
## Release Build / Static Linking
- MSVC is the native Windows ABI — best compatibility with Windows APIs needed by `arboard`, `winrt-notification`, and `crossterm`.
- Static link the CRT to eliminate the MSVCRT DLL dependency: set `RUSTFLAGS="-C target-feature=+crt-static"` in `install.ps1` before `cargo build --release`. This adds ~100 KB to the binary but makes it truly portable — copy the `.exe` anywhere on Windows 10/11, no redistributable needed.
- The `x86_64-pc-windows-gnu` target (MinGW) was demoted from Tier 1 in Rust 1.88 (May 2025). Avoid it unless cross-compiling from Linux.
# install.ps1 build invocation
## Command Architecture
#[derive(Parser)]
#[command(name = "box", about = "A CLI toolbox")]
#[derive(Subcommand)]
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
## Windows PowerShell 7 — Specific Considerations
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
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
