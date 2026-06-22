# Project Research Summary

**Project:** box - Rust CLI Toolbox
**Domain:** Single-binary multi-subcommand CLI toolbox, Windows PowerShell 7
**Researched:** 2026-06-22
**Confidence:** HIGH

## Executive Summary

`box` is a greenfield Rust CLI toolbox: one binary, 23 subcommands, globally installed on Windows PowerShell 7 via `install.ps1`. The research is unusually clear-cut because the Rust CLI ecosystem has converged on strong defaults for exactly this pattern. The architecture is clap derive with per-command `Args` structs and a `RunCommand` trait -- this keeps `main.rs` under 50 lines forever and makes each command independently testable. The recommended stack covers all 23 commands with high-confidence crate choices verified against docs.rs. There are no meaningful architecture debates to resolve; the patterns are well-established and documented.

The key implementation risk is Windows-specific behavior, not application logic. Four pitfalls must be addressed at the foundation layer before any command is built: `std::fs::canonicalize` produces UNC paths that break downstream path handling (fix: use `dunce::canonicalize` everywhere), the 260-character path limit silently fails on deep directory trees (fix: verbatim prefix or `LongPathsAware` manifest), ANSI color codes must be stripped when stdout is not a TTY (fix: project-wide `output.rs` using `owo-colors` + `enable-ansi-support`), and `install.ps1` must refresh the PATH in the current session or every new user's first experience is "command not found." These are cheap to solve upfront and expensive to fix after 23 commands have shipped.

The recommended phase structure is: Foundation (binary scaffold + install), then a core utilities batch (low-complexity, high-value commands that prove the pattern), then filesystem power tools (commands sharing `walkdir`), then terminal/visual effects, then Windows platform-dependent commands last. The platform-dependent commands (`clip`, `pomodoro`, `weather`) carry the highest integration risk and should be validated in early spikes even if built last. All 23 commands are in-scope for v1 per PROJECT.md; the phase structure is about build order, not scope reduction.

## Key Findings

### Recommended Stack

The stack research is HIGH confidence with all versions verified against docs.rs. The core is: `clap 4.6` (derive API, subcommand dispatch), `anyhow 1.0` (error propagation in binaries), `thiserror 2.0` (typed errors where exit codes differ). Terminal output uses `crossterm 0.29` + `owo-colors 4.3` + `enable-ansi-support 0.3` -- this is the correct 2025 Windows ANSI stack; `termcolor` and `colored` are explicitly wrong for this environment. The binary must be built for `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"` -- the MinGW target was demoted to Tier 2 in Rust 1.88 (May 2025) and should be avoided.

**Core technologies:**
- `clap 4.6` (derive): CLI parsing + subcommand dispatch -- community standard; derive API handles 23 subcommands with zero boilerplate
- `anyhow 1.0`: error propagation -- correct pairing for a binary; `?` operator + `.context()` everywhere
- `thiserror 2.0`: typed errors -- only where distinct exit codes are needed
- `crossterm 0.29`: terminal control for `matrix`, `pomodoro` countdown, terminal size detection
- `owo-colors 4.3` + `enable-ansi-support 0.3`: ANSI color -- Windows-correct approach; respects `NO_COLOR`
- `walkdir 2.5` + `ignore 0.4` + `rayon 1.12`: filesystem traversal -- powers `flatten`, `tree`, `du`, `dupes`, `bulk-rename`
- `ureq 3.3` (not reqwest): HTTP for `weather` -- blocking client, no tokio, far smaller binary
- `arboard 3.6`: clipboard -- maintained by 1Password; correct Win32 API usage
- `winrt-notification 0.5` (with PowerShell AUMID fallback): toast notifications for `pomodoro`
- `blake3 1.8` + `sha2 0.11` + `md-5` (RustCrypto): hashing -- all implement `digest::Digest` for a unified hasher
- `qr2term 0.3`: QR rendering -- handles half-block Unicode automatically; avoids the aspect-ratio pitfall
- `artem 3.0` + `image 0.25`: ASCII art from images
- `uuid 1.23`, `base64 0.22`, `chrono 0.4`, `serde_json 1.0`, `passwords 3.1`, `rand 0.9`: data utilities

**What NOT to use:** `termcolor` (legacy Console API), `colored` (no Windows ANSI init), `reqwest` (pulls in tokio for no benefit), `md5` crate (use `md-5` hyphenated), `notify-rust` (Linux-first), `x86_64-pc-windows-gnu` target (Tier 2 since Rust 1.88).

### Expected Features

All 23 commands are v1 scope. Key cross-cutting conventions: stdout for data only, stderr for messages/progress, `--dry-run` default on mutating commands (`flatten`, `bulk-rename`), `NO_COLOR`/non-TTY strips ANSI, consistent flag naming.

**Must have (table stakes):**
- `--help` / `-h` on every subcommand (clap handles this automatically)
- Exit codes: 0 success, 1 error, 2 bad args (clap exits 2 on parse errors natively)
- Dry-run default on `flatten` and `bulk-rename` -- require `--force` / `--execute` to write changes
- Collision-rename strategy in `flatten` (path-encoding, not silent overwrite)
- Content-hash deduplication in `dupes` (not filename comparison)
- SHA-256 as default hash algorithm in `hash` (BLAKE3 available via `--algo blake3`)
- Non-TTY / `NO_COLOR` detection disabling ANSI on all colorized commands
- Open-Meteo API for `weather` (no API key required; keyless geocoding included)
- CSPRNG (`OsRng`) for all password and passphrase generation in `passgen`

**Should have (differentiators):**
- `--json` output on file-ops commands (`flatten`, `dupes`, `hash`, `tree`, `du`) for scripting
- Progress indicators via `indicatif` on long-running commands (`dupes`, large-file `hash`, large-tree `flatten`)
- Multi-stage hashing in `dupes` (size pre-filter then prefix hash then full hash) for performance on large trees
- Entropy estimate display in `passgen`; color swatch rendering in `color` (ANSI truecolor block)
- UUID v7 support in `uuid` (time-ordered; increasingly preferred for DB keys)
- 3-day forecast in `weather` (Open-Meteo supports it in the same API call)

**Defer (v2+):**
- `--move` destructive mode in `flatten`; auto-delete in `dupes`
- Full jq expression language in `json` (pretty-print + validate is the v1 job)
- Undo/history in `bulk-rename`; pomodoro session statistics
- Image `ascii` from URL (file input only in v1); Base32/Base58 in `base64`

### Architecture Approach

The architecture is a single Rust crate (not a workspace) with a `src/commands/<cmd>/mod.rs` module per command. Each module exports `pub struct Args` (clap derive) and implements `pub trait RunCommand { fn run(self) -> anyhow::Result<()>; }`. `main.rs` is approximately 40 lines: parse `Cli`, match on `Commands` enum, call `.run()`, map `Err` to `ExitCode::FAILURE`. Shared infrastructure lives in `src/core/`. The `8ball` command maps to a Rust module named `eight_ball` (Rust identifiers cannot start with a digit).

**Major components:**
1. `src/main.rs` -- parse + dispatch only; ~40 lines; no business logic ever lives here
2. `src/cli.rs` -- `Cli` struct + `Commands` enum; the single authoritative list of all 23 commands
3. `src/core/` -- `errors.rs` (BoxError with thiserror), `output.rs` (color init + print helpers), `fs.rs` (walkdir wrapper, safe copy, collision rename)
4. `src/commands/<cmd>/mod.rs` -- 23 self-contained modules; each owns its `Args` + `RunCommand` impl
5. `install.ps1` -- build release binary (crt-static), copy to dedicated bin dir, idempotent PATH update, session PATH refresh, smoke test
6. `tests/<cmd>.rs` -- black-box integration tests via `assert_cmd`; one file per command

### Critical Pitfalls

1. **`std::fs::canonicalize` returns UNC paths** -- use `dunce::canonicalize` everywhere from day one; establish a single `core::fs::normalize_path` wrapper (rust-lang/rust#42869)

2. **260-character path limit silently fails** -- affects `flatten`, `dupes`, `tree`, `du`; use `verbatim` crate or verbatim prefix for I/O on deep paths; embed `LongPathsAware` manifest via `embed-resource`; add CI test with 300-character synthetic path

3. **ANSI codes corrupt piped output** -- establish `core::output::init_color()` called once in `main()`; use `owo-colors` `if_supports_color()` which checks `is_terminal()` + `NO_COLOR` automatically

4. **`install.ps1` PATH not visible in current session** -- always refresh the session PATH at end of install by merging user + machine PATH from registry; run `box --help` as smoke test

5. **Output dir inside input dir causes infinite copy loop in `flatten`** -- canonicalize both paths with `dunce::canonicalize` before the walker starts; abort if dest starts with src using `Path::starts_with` on canonical forms

Additional pitfalls: `walkdir` junction point loops (default `follow_links(false)`), NTFS case-insensitive collision bugs in `flatten` (lowercase HashMap keys), `matrix` per-character stdout flush causing ~5 FPS (buffer entire frame, flush once per frame), clipboard threading in `arboard` (main thread only), reserved Windows filenames in `flatten` output (`CON.txt`, `NUL.txt` -- use `sanitize-filename` crate).

## Implications for Roadmap

Based on research, the natural phase structure follows the architecture dependency groups: foundation first, then zero-external-dep commands to prove the pattern, then filesystem commands (shared `walkdir` dep), then terminal effects, then Windows platform commands last (highest integration risk).

### Phase 1: Foundation

**Rationale:** Every pitfall mitigation and shared infrastructure must be in place before any command is built. The `install.ps1` is the product delivery mechanism -- if it does not work, nothing else matters.
**Delivers:** Compilable `box` binary with `--version` and `--help`; `install.ps1` that correctly builds, installs, and makes `box` available in the current PS7 session; `src/core/` with `dunce`-based path normalization, color utility, and shared error types.
**Addresses:** `--version`, `--help`, exit codes, ANSI initialization, install experience
**Avoids:** UNC path pitfall, ANSI corruption pitfall, PATH session refresh pitfall, execution policy pitfall

### Phase 2: Core Utilities (Pure Transforms)

**Rationale:** These 9 commands have zero external API/filesystem/Windows dependencies. They prove the `RunCommand` pattern, the `Args` struct convention, and the testing approach. Fast to build, immediately useful, zero integration risk.
**Delivers:** `uuid`, `base64`, `epoch`, `color`, `passgen`, `cowsay`, `roast`, `fortune`, `8ball` -- all functional with unit and integration tests
**Uses:** `uuid 1.23`, `base64 0.22`, `chrono 0.4`, `rand 0.9`, `passwords 3.1`, `owo-colors` (color swatch in `color`)
**Implements:** `RunCommand` trait pattern; per-command `Args` structs; `assert_cmd` integration test pattern established for all subsequent phases

### Phase 3: Filesystem Power Tools

**Rationale:** These 6 commands share `walkdir`, the collision-rename logic, and the dry-run pattern. Build `flatten` first (anchor command per PROJECT.md). The `dupes` multi-stage hashing and `bulk-rename` collision detection are the hardest problems here and benefit from `flatten` being debugged first.
**Delivers:** `flatten` (dry-run, collision rename, path guard, timestamp preservation), `tree`, `du`, `hash`, `bulk-rename`, `dupes`
**Uses:** `walkdir 2.5`, `ignore 0.4`, `rayon 1.12` (parallel hashing in `dupes`), `blake3 1.8`, `sha2 0.11`, `md-5`, `indicatif 0.18`
**Avoids:** Output-inside-input loop, NTFS case collision, 260-char path limit, symlink/junction loops, reserved filename generation, `fs::copy` timestamp destruction

### Phase 4: Terminal Visual Effects

**Rationale:** These commands share `crossterm` and require frame-buffered output. Independent of filesystem logic. Build `lolcat` before `matrix` -- simpler scope teaches the stdout-buffering pattern.
**Delivers:** `lolcat`, `matrix`, `json`, `ascii` (image to ASCII)
**Uses:** `crossterm 0.29`, `serde_json 1.0`, `image 0.25`, `artem 3.0`
**Avoids:** Per-character stdout flush in `matrix` (buffer full frame, flush once per frame), existing ANSI codes corrupted by `lolcat` (strip before rainbow-colorizing)

### Phase 5: Windows Platform Integration

**Rationale:** These 4 commands depend on Windows-specific APIs or external services with the highest integration risk. Build `qr` first (no external service), then `clip`, `pomodoro`, `weather`. Validate `winrt-notification` compiles as a Phase 1 spike.
**Delivers:** `qr`, `clip`, `pomodoro`, `weather`
**Uses:** `qr2term 0.3`, `arboard 3.6`, `winrt-notification 0.5` (PowerShell AUMID fallback), `ureq 3.3`, `serde_json 1.0`
**Avoids:** QR aspect-ratio distortion (use `qr2term` half-block rendering), clipboard threading deadlock (main thread only), toast AUMID failure (use PowerShell AUMID fallback), weather hanging on bad network (5-second timeout)

### Phase Ordering Rationale

- Foundation before everything because three critical shared utilities (path normalization, color init, PATH refresh) must exist before any command can be correctly built
- Pure transforms before filesystem commands because they prove the `RunCommand` pattern at zero risk -- if the architecture is wrong, discover it on `uuid`, not on `flatten`
- Filesystem commands grouped together because `walkdir` setup, dry-run pattern, and collision-rename logic are shared; `flatten` is the anchor that defines the safety patterns `dupes` and `bulk-rename` build on
- Terminal effects after filesystem because `crossterm` is an independent dependency group; building visual effects before utilities risks scope drift
- Platform-dependent commands last because `arboard`, `winrt-notification`, and `ureq`+Open-Meteo carry the highest external/Windows-API risk -- validate with spikes during Phase 1

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 5 (`pomodoro`):** `winrt-notification` maintenance status is uncertain; may need to switch to `winrt-toast` -- validate crate compiles against MSRV before Phase 5 planning; run as a Phase 1 spike
- **Phase 5 (`weather`):** Open-Meteo geocoding behavior for edge-case city names (disambiguation, non-ASCII) needs validation; caching strategy for rapid repeated calls needs a design decision
- **Phase 3 (`dupes`):** Multi-stage hashing implementation complexity; rayon parallelism over file I/O requires tuning to avoid disk thrashing on HDDs vs SSDs

Phases with standard patterns (skip research-phase):
- **Phase 2 (pure transforms):** `uuid`, `base64`, `epoch`, `color`, `passgen`, `cowsay`, `fortune`, `roast`, `8ball` all follow trivially documented patterns
- **Phase 1 (foundation):** Clap derive binary scaffold and PowerShell install scripts are fully documented; architecture pattern is established
- **Phase 4 (`json`):** `serde_json` pretty-print + validation is a one-page implementation

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crate versions verified against docs.rs; Windows-specific choices confirmed against current platform guidance including Rust 1.88 MinGW demotion |
| Features | HIGH (core), MEDIUM (differentiators) | Table stakes grounded in prior art (fdupes, rnr, dust, clig.dev); differentiator priorities are reasoned but not empirically validated against actual user needs |
| Architecture | HIGH | Clap derive + `RunCommand` trait + single-crate module tree is the documented community standard; all patterns have reference implementations |
| Pitfalls | HIGH | All 15 pitfalls grounded in specific GitHub issues, official Microsoft docs, or crate documentation; not inferred |

**Overall confidence:** HIGH

### Gaps to Address

- **`winrt-notification` vs `winrt-toast` selection:** Validate the recommended crate compiles against the project MSRV during Phase 1 as a spike; if not, switch to `winrt-toast 0.1` with `windows 0.62` directly
- **`pomodoro` background vs foreground:** Decide whether to block the terminal for the full 25-minute timer or print timer-started and exit immediately, relying solely on the toast notification
- **`weather` default unit system:** Decide whether to default to metric, detect from Windows locale, or prompt once and cache in `%APPDATA%ox\config.toml`
- **`lolcat` ANSI stripping:** The `strip-ansi-escapes` crate was not included in the STACK.md Cargo.toml template; add it during Phase 4 planning

## Sources

### Primary (HIGH confidence)
- docs.rs (all crate versions) -- clap, crossterm, owo-colors, enable-ansi-support, arboard, blake3, sha2, ureq, walkdir, ignore, rayon, chrono, indicatif, anyhow, thiserror, artem, winrt-notification, qr2term
- rust-lang/rust GitHub issues #42869, #80884, #76586, #67403, #66260 -- Windows path and filesystem behavior
- Microsoft Learn -- Naming Files, Paths, and Namespaces (reserved filenames, MAX_PATH)
- Rust CLI Recommendations (rust-cli-recommendations.sunshowers.io) -- colors, argument handling
- Command Line Applications in Rust (rust-cli.github.io) -- testing, exit codes, subcommands
- Rust Blog 2025-05-26 -- x86_64-pc-windows-gnu Tier 2 demotion in Rust 1.88
- Open-Meteo (open-meteo.com) -- keyless free weather + geocoding API confirmed

### Secondary (MEDIUM confidence)
- clig.dev -- CLI UX conventions (exit codes, stdout/stderr separation, dry-run patterns)
- bootandy/dust -- disk usage tree behavior and bar visualization patterns
- ismaelgv/rnr -- dry-run-first, capture group syntax for bulk rename
- pkolaczk/fclones -- multi-stage hashing strategy for duplicate detection
- TheZoraiz/ascii-image-converter -- brightness mapping, color mode, braille mode

### Tertiary (LOW confidence)
- winrt-notification 0.5 long-term maintenance status -- described as uncertain; validate before Phase 5
- win-toast-notify as alternative crate -- maintenance status unverified; fallback option only

---
*Research completed: 2026-06-22*
*Ready for roadmap: yes*