# Phase 1: Foundation + Flatten - Research

**Researched:** 2026-06-22
**Domain:** Rust single-binary clap-derive CLI scaffold + PowerShell 7 installer + recursive file-flattening command (Windows-first)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**Install location & PATH (FOUND-07, FOUND-08)**
- **D-01:** `install.ps1` copies `box.exe` to **`%LOCALAPPDATA%\Programs\box`** (Win11 per-user installed-app convention; no admin, no roam, the install dir *is* the PATH entry).
- **D-02:** PATH update is **idempotent, registry user-scope, dedup-guarded** — read `[Environment]::GetEnvironmentVariable('Path','User')`, split on `;`, append the bin dir only if `-notcontains` it, write back. ⚠️ If the existing user PATH contains `%VAR%` references, write with `Set-ItemProperty -Path 'HKCU:\Environment' -Name Path -Value $newPath -Type ExpandString` to avoid the `REG_EXPAND_SZ → REG_SZ` regression (dotnet/runtime#1442). For literal absolute paths the `[Environment]` form is sufficient.
- **D-03:** **Current-session refresh** must rebuild `$env:Path` by re-reading **both** Machine and User scopes from the registry and rejoining.
- **D-04:** Re-install behavior = **plain overwrite** (`Copy-Item -Force`). No version-checking. PATH dedup guard makes repeated installs idempotent.

**CLI scaffold & stub UX (FOUND-01..05)**
- **D-05:** Stubs are **real `clap`-derive enum variants** with doc-comment descriptions (doc comment → `about`), each dispatched to a handler returning a structured `NotImplemented` error (thiserror `BoxError::NotImplemented`). (Rejected: `external_subcommand`, `hide=true`, feature-gating, `todo!()`.)
- **D-06:** Invoking an **unbuilt command exits code 1**. Message to **stderr**: `error: 'qr' is not yet implemented — coming in a future release`.
- **D-07:** **Exit code 2 is reserved for clap parse errors** — `box badcmd`, missing/invalid args. `main()` must NOT collapse all errors to 1.
- **D-08:** **Bare `box`** (no subcommand) → `#[command(arg_required_else_help = true)]` on the top-level `Cli`: prints help and **exits 2**.
  > ⚠️ **Research flag — see Open Question OQ-1 and Pitfall 2.** Verified clap 4.x behavior is that `arg_required_else_help` prints help to **stdout and exits 0**, NOT exit 2. Honoring D-08's "exit 2" requires an explicit override in `main()`; it is not clap's default. The exact pattern is documented below.

**flatten output format (FLAT-03)**
- **D-09:** Output style **B — leading status glyph + arrow + color**: `+` plain copy, `~` collision rename, `-` skipped. **Glyph is the source of truth; color is decoration only.** ASCII glyphs (`+ ~ -`), `->` arrow for copy mapping. This becomes the **UX template for the other 22 commands.**
- **D-10:** Color gating: `std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()` (plus `--no-color`). Plain/piped layout byte-identical minus ANSI. Reasons inline: `[collision]`, `[collision x2]`, `(skipped: symlink)`, `(skipped: reserved name)`. Arrows aligned into a column, capped at terminal width (crossterm), over-long paths truncated middle with `…`.
- **D-11:** Summary wording (locked):
  - Dry-run: `Dry run: nothing was copied.` then `Plan: {n} to copy, {n} renamed for collisions, {n} skipped.`
  - Real run: `Done: copied {n} files ({n} renamed for collisions), skipped {n}. {size} written.`

**flatten default scope (FLAT-01, FLAT-02, FLAT-04)**
- **D-12:** **Skip hidden files/dirs by default.** "Hidden" = base name starts with `.` **OR** Windows `FILE_ATTRIBUTE_HIDDEN` bit (`file_attributes() & 0x2`). Apply in `walkdir`'s `filter_entry` so hidden *directories* prune their whole subtree cheaply.
- **D-13:** **Auto-create the output dir** (`fs::create_dir_all`, including missing parents).
- **D-14:** **Merge into an existing non-empty output dir** (do not refuse) — but **collision-check incoming names against pre-existing files in the output dir**, seeded by `read_dir` of the output dir before the copy loop.
- **D-15:** **Collision-rename prefix** built from source path **relative to the canonicalized source root** (via `dunce::canonicalize`): replace each separator (`\` and `/`) with `_`, drop drive letter / leading separator, **sanitize Windows-reserved stems** (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`, case-insensitive, with/without extension) and trailing dots/spaces. If still collides → **numeric-suffix fallback** before the extension (`name_1.ext`, `name_2.ext`).

### Claude's Discretion
- `box --version` source — read from `Cargo.toml` via `clap`'s `#[command(version)]`; start at `0.1.0` (FOUND-02).
- Exact phrasing of one-line `about` text per stub command (use verbs from REQUIREMENTS.md command list).
- Internal module layout for the dry-run planner vs executor in `flatten` (share one plan; dry-run writes nothing).
- Whether the `{size} written` byte count is accumulated during copy (cheap) — include unless it complicates the executor.

### Deferred Ideas (OUT OF SCOPE)
None for this phase. The other 22 commands (Phases 2–5) and all flatten v2 flags (`--separator`, `--include-hidden`, `--extensions`, `--json`, `--move`, progress bar) are explicitly out of scope and tracked in REQUIREMENTS.md / ROADMAP.md.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FOUND-01 | `box --help` lists all 23 commands; `box <cmd> --help` shows per-command usage | Pattern 1 (clap-derive enum, doc-comment → about). All 23 variants registered in one `Commands` enum. Verified: doc comment on enum variant = short+long help. |
| FOUND-02 | `box --version` prints semver | `#[command(version)]` on `Cli` reads `Cargo.toml` `version` at compile time. Start `0.1.0`. |
| FOUND-03 | Shared UX: data→stdout, messages/errors→stderr, exit codes 0/1/2 | Pattern 3 (exit-code mapping in `main()`), Pattern 4 (output helpers). Code Example "main.rs exit-code dispatch". |
| FOUND-04 | Color auto-disables when not TTY / `NO_COLOR` / `--no-color`; ANSI enabled in PS7 | Pattern 5 (color gating). `enable_ansi_support()` first line of `main()`; `std::io::IsTerminal` + `NO_COLOR` + `--no-color`. |
| FOUND-05 | Clear stderr message instead of panic on bad input / missing files | anyhow `?` + `.context()`; never `unwrap`/`panic!` in command paths; `main()` catches `Err` → stderr + exit 1. |
| FOUND-06 | Paths normalized safely (no `\\?\` leak); long/edge paths no silent failure | `dunce::canonicalize` everywhere (Pitfall 1). Long-path handling: see Pitfall 5 + Open Question OQ-2. |
| FOUND-07 | `install.ps1` builds release, copies to dedicated bin dir, idempotent user-PATH add | install.ps1 design section; D-01, D-02. Code Example "install.ps1". |
| FOUND-08 | `install.ps1` refreshes current-session PATH + smoke-tests `box --help` | D-03 session refresh (merge User+Machine); smoke test as final step. |
| FLAT-01 | Recursively copy every file into one flat output dir; originals untouched; no subdirs in output | walkdir traversal; `fs::copy` (not move); flat output. Architecture Responsibility Map. |
| FLAT-02 | Collision → encode source path; deep double-collision → numeric suffix | D-15 collision encoding + reserved-name sanitization + numeric fallback. Code Example "collision rename". |
| FLAT-03 | `--dry-run` previews without writing; completion summary | D-09/D-11 output; planner/executor split (Pattern 6). |
| FLAT-04 | Refuse output-inside-source loop; skip symlinks safely; preserve timestamps | Pitfall 3 (containment guard), Pitfall 4 (symlinks/junctions), Pitfall 6 (timestamp preservation via `std::fs::FileTimes`). |
</phase_requirements>

## Summary

This phase is a **walking skeleton**: a single Rust crate where all 23 subcommands are registered with clap-derive (only `flatten` functional), plus an `install.ps1` that makes `box` globally runnable in the *same* PowerShell 7 session, plus the `flatten` anchor command. The crate stack is **locked** (CLAUDE.md) — clap 4.6, anyhow 1.0, thiserror 2.0, owo-colors 4.3, enable-ansi-support 0.3, crossterm 0.29, walkdir 2.5 — and was not re-researched. This research instead goes deep on the four areas the orchestrator flagged as highest-value: (1) the `install.ps1` PATH manipulation and the `REG_EXPAND_SZ → REG_SZ` regression, (2) the clap-derive pattern for 23 stubs with a `NotImplemented` handler and **correct exit-code mapping**, (3) ANSI bootstrap ordering and TTY/`NO_COLOR` color gating, and (4) `flatten` internals (canonicalize-before-walk, hidden-dir pruning, source-relative collision encoding with reserved-name sanitization).

The architecture is fully settled by the project-level research (`.planning/research/ARCHITECTURE.md`): single crate, `src/commands/<cmd>/mod.rs` per command, a `RunCommand` trait, `src/core/{errors,output,fs}.rs`, ~40-line `main.rs`. There are no open architecture debates. The verified-but-surprising finding that drives planning is in **Pitfall 2 / OQ-1**: clap's `arg_required_else_help` does **not** exit 2 — it prints help to stdout and exits 0. D-08 requires exit 2, so `main()` must override this explicitly. The other load-bearing Windows details (UNC paths, timestamp loss, reserved filenames, junction loops, session-PATH refresh) are all addressed at the foundation layer so the remaining 22 commands inherit correct behavior.

**Primary recommendation:** Build the thin vertical slice in this order — (1) Cargo scaffold + `RunCommand` trait + `Commands` enum with all 23 variants → (2) `src/core/{errors,output,fs}` shared infra → (3) `flatten` planner/executor → (4) `install.ps1` + smoke test. Use `std::fs::FileTimes` (stable since Rust 1.75; toolchain is 1.90) for timestamp preservation instead of the `filetime` crate. Implement `main()` exit-code mapping that preserves clap's exit 2 for parse errors, returns 1 for `BoxError::NotImplemented` and other runtime errors, and **explicitly** turns the bare-`box` help case into exit 2 to satisfy D-08.

## Architectural Responsibility Map

This is a single-tier native CLI binary. "Tier" here means the layer that owns each capability inside the process, plus the OS/installer boundary.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Argument parsing, `--help`, `--version`, subcommand dispatch | `src/cli.rs` (clap-derive `Cli` + `Commands`) | `src/main.rs` (match → `.run()`) | clap owns parsing; `main` owns only dispatch + exit-code mapping (no business logic). |
| Exit-code policy (0/1/2) | `src/main.rs` | `src/core/errors.rs` (`BoxError` → code) | Exit codes are a process-boundary concern; only `main` returns `ExitCode`. |
| ANSI bootstrap + color gating (TTY/`NO_COLOR`/`--no-color`) | `src/core/output.rs` | `src/main.rs` (calls `enable_ansi_support()` first) | One place owns color so all 23 commands inherit identical behavior. |
| Path normalization (`dunce::canonicalize`, UNC-strip) | `src/core/fs.rs` | per-command modules call it | Single wrapper prevents `std::fs::canonicalize` UNC leakage anywhere. |
| Directory traversal + hidden pruning | `src/core/fs.rs` (walkdir wrapper) | `flatten` (and later tree/du/dupes) | Shared so Phase 3 reuses the exact same walk + filter semantics. |
| Collision rename + reserved-name sanitization | `src/core/fs.rs` (or `flatten/rename.rs`, pure + unit-testable) | `flatten` | Pure function; unit-tested in isolation; reused by `bulk-rename` later. |
| File copy + timestamp preservation | `src/core/fs.rs` (`safe_copy`) | `flatten` executor | Centralizes the `fs::copy` + `FileTimes` pairing so no command forgets timestamps. |
| `flatten` plan (dry-run) vs execute | `src/commands/flatten/mod.rs` | `src/core/fs.rs` helpers | One plan structure feeds both dry-run print and real copy (Pattern 6). |
| Build → install → PATH → session refresh → smoke test | `install.ps1` (OS/installer boundary) | — | Distribution is a PowerShell concern outside the binary; the binary just needs to exist on PATH. |

## Standard Stack

> The runtime crate stack is **LOCKED** in CLAUDE.md and verified in `.planning/research/STACK.md`. It was **not** re-researched. The tables below cover (a) the locked runtime crates this phase actually uses and (b) the **new dev/util crates** this phase introduces (path/timestamp helpers + the test harness), which were verified this session.

### Core (locked — used this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.6 (derive) | Parsing, subcommand dispatch, `--help`/`--version` | [CITED: CLAUDE.md] Community standard; one enum = all 23 commands. |
| anyhow | 1.0 | Error propagation in command `run()` | [CITED: CLAUDE.md] `?` + `.context()`; print at `main()`. |
| thiserror | 2.0 | Typed `BoxError` (e.g., `NotImplemented`) | [CITED: CLAUDE.md] Lets `main()` map specific variants to exit codes. |
| owo-colors | 4.3 | Inline colorization for flatten glyphs/summary | [CITED: CLAUDE.md] Zero-alloc; builds ANSI strings. |
| enable-ansi-support | 0.3 | One-time Windows ANSI bootstrap | [CITED: CLAUDE.md] First line of `main()`. |
| crossterm | 0.29 | `terminal::size()` for width-capping flatten output | [CITED: CLAUDE.md] Used here only for terminal width (D-10). |
| walkdir | 2.5 | Recursive traversal with `filter_entry` pruning | [CITED: CLAUDE.md] Anchor for flatten; reused Phase 3. |

### Supporting (new this phase — verified this session)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dunce | 1.0.5 | `canonicalize` without `\\?\` UNC prefix | [VERIFIED: crates.io via cargo search] Mandatory per STATE.md; wrap in `core::fs::normalize_path`. |
| (stdlib) `std::fs::FileTimes` | Rust 1.75+ (toolchain 1.90) | Preserve mtime/atime after copy (FLAT-04) | [VERIFIED: doc.rust-lang.org] **Prefer over the `filetime` crate** — no new dependency. See note below. |

### Test / Dev Tooling (new this phase — verified this session)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| assert_cmd | 2.2.2 | Black-box integration tests of the compiled `box` binary | [VERIFIED: crates.io via cargo search] `tests/<cmd>.rs`; the backbone of the Validation Architecture. |
| predicates | 3.1.4 | Output/exit-code assertions for assert_cmd | [VERIFIED: crates.io via cargo search] `predicate::str::contains`, regex matchers. |
| assert_fs | 1.1.4 | Temp-dir fixtures + filesystem assertions | [VERIFIED: crates.io via cargo search] Build nested source trees for flatten tests. |
| tempfile | 3.27.0 | Lower-level temp dirs (alternative/complement to assert_fs) | [VERIFIED: crates.io via cargo search] Use if assert_fs fixtures are insufficient. |
| trycmd | 1.2.0 | File-based CLI transcript snapshots (help text, flatten plan) | [VERIFIED: crates.io via cargo search] Best for the locked dry-run sample (D-09) and `--help` listing. |
| insta | 1.48.0 | Inline/`.snap` snapshots for structured output | [VERIFIED: crates.io via cargo search] Optional; use where trycmd transcripts are awkward. |

> **`filetime` crate (0.2.29) — verified but NOT recommended for this phase.** `std::fs::FileTimes` + `File::set_times` is stable since Rust 1.75 and preserves both modified and accessed time on Windows. With toolchain 1.90, prefer the stdlib to avoid a dependency. The `filetime` crate remains a valid fallback if you need to set times without an open `File` handle. [VERIFIED: doc.rust-lang.org/std/fs/struct.FileTimes.html]

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `std::fs::FileTimes` | `filetime` crate 0.2.29 | filetime works on older toolchains and takes a path (no open handle), but adds a dependency for behavior the stdlib now provides. |
| `std::io::IsTerminal` (stdlib trait) | `is-terminal` crate 0.4.17 | The crate is redundant on Rust ≥1.70; the stdlib trait is the right call. Do NOT add the crate. |
| `trycmd` for help/plan snapshots | `insta` | trycmd's transcript format matches the locked dry-run sample (D-09) more directly; insta is better for non-CLI structured values. |
| Pruning hidden dirs in `filter_entry` | Post-filtering after full walk | `filter_entry` avoids descending into `.git`/`.venv` entirely (cheaper). Locked by D-12. |

**Installation (Cargo.toml additions for this phase):**
```toml
[dependencies]
clap = { version = "4.6", features = ["derive"] }
anyhow = "1.0"
thiserror = "2.0"
owo-colors = "4.3"
enable-ansi-support = "0.3"
crossterm = "0.29"
walkdir = "2.5"
dunce = "1.0"
# NOTE: no `filetime` — use std::fs::FileTimes (Rust 1.75+).
# NOTE: no `is-terminal` — use std::io::IsTerminal (Rust 1.70+).

[dev-dependencies]
assert_cmd = "2.2"
predicates = "3.1"
assert_fs = "1.1"
tempfile = "3.27"
trycmd = "1.2"
insta = "1.48"
```

**Version verification performed this session:** `cargo search` (crates.io sparse index) confirmed: `dunce 1.0.5`, `filetime 0.2.29`, `assert_cmd 2.2.2`, `predicates 3.1.4`, `assert_fs 1.1.4`, `tempfile 3.27.0`, `insta 1.48.0`, `trycmd 1.2.0`, `is-terminal 0.4.17`. The locked runtime crates were taken as final from CLAUDE.md per scope instructions.

## Package Legitimacy Audit

> slopcheck 0.6.1 was installed and run against crates.io this session. All new crates clean.

| Package | Registry | slopcheck | Source Repo | Disposition |
|---------|----------|-----------|-------------|-------------|
| dunce | crates.io | OK | github.com/kornelski/dunce | Approved |
| filetime | crates.io | OK | github.com/alexcrichton/filetime | Approved (fallback only — prefer stdlib) |
| assert_cmd | crates.io | OK | github.com/assert-rs/assert_cmd | Approved (dev) |
| predicates | crates.io | OK | github.com/assert-rs/predicates-rs | Approved (dev) |
| assert_fs | crates.io | OK | github.com/assert-rs/assert_fs | Approved (dev) |
| tempfile | crates.io | OK | github.com/Stebalien/tempfile | Approved (dev) |
| insta | crates.io | OK | github.com/mitsuhiko/insta | Approved (dev) |
| trycmd | crates.io | OK | github.com/assert-rs/trycmd | Approved (dev) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
**Locked runtime crates** (clap, anyhow, thiserror, owo-colors, enable-ansi-support, crossterm, walkdir) were verified in CLAUDE.md/STACK.md and are not re-audited here.

## Architecture Patterns

### System Architecture Diagram

```
PowerShell 7 session
        │
        │  .\install.ps1
        ▼
┌───────────────────────────────────────────────────────────────┐
│ install.ps1                                                    │
│  $env:RUSTFLAGS="-C target-feature=+crt-static"                │
│  cargo build --release --target x86_64-pc-windows-msvc         │
│      │ build ok? ($LASTEXITCODE)                               │
│      ▼                                                         │
│  Copy-Item -Force box.exe → %LOCALAPPDATA%\Programs\box\       │
│      ▼                                                         │
│  user PATH (HKCU\Environment): split ';' → -notcontains? →     │
│      append → write (ExpandString if %VARS% present)           │
│      ▼                                                         │
│  $env:Path = User-scope  ⊕  Machine-scope   (current session)  │
│      ▼                                                         │
│  smoke test: & box --help   →  prints 23 commands              │
└───────────────────────────────────────────────────────────────┘
        │  (box now on PATH, same session)
        ▼
 box flatten ./src ./out [--dry-run]
        │
        ▼
┌───────────────────────────────────────────────────────────────┐
│ main()                                                         │
│  1. enable_ansi_support()        ← FIRST line                  │
│  2. Cli::try_parse()                                           │
│        ├─ parse error  ───────────────► clap exit 2 (stderr)   │
│        ├─ --help/--version ───────────► clap exit 0 (stdout)   │
│        └─ bare box (no subcmd) ───────► OVERRIDE → exit 2 (D-08)│
│  3. core::output::init_color(args.no_color)                    │
│  4. match cli.command → args.run()  (RunCommand trait)         │
│        ├─ Flatten(args) → real logic                           │
│        └─ Qr/Hash/... (22) → Err(BoxError::NotImplemented)     │
│  5. Ok(())            → ExitCode 0                              │
│     Err(NotImplemented)→ stderr msg, ExitCode 1  (D-06)        │
│     Err(other)        → stderr "error: {e:#}", ExitCode 1      │
└───────────────────────────────────────────────────────────────┘
        │  (Flatten path)
        ▼
┌───────────────────────────────────────────────────────────────┐
│ flatten                                                        │
│  src_root  = dunce::canonicalize(src)                          │
│  out_root  = dunce::canonicalize(create_dir_all(out) then …)   │
│  GUARD: out_root.starts_with(src_root)? → abort (Pitfall 3)    │
│      ▼                                                         │
│  occupied = read_dir(out_root) lowercased  (D-14 seed)         │
│      ▼                                                         │
│  WalkDir(src_root).filter_entry(!hidden)  (D-12 prune)         │
│      │  for each file entry:                                   │
│      │    symlink? → skip (-)                  (Pitfall 4)      │
│      │    name = base; collide(occupied)? →                    │
│      │        encode source-relative path (D-15) → sanitize    │
│      │        reserved stems → numeric-suffix fallback (~)     │
│      │    else plain (+)                                       │
│      ▼  Plan { entries: Vec<PlanItem{src, dst, kind, reason}> }│
│      ├─ --dry-run → print plan, write NOTHING, summary (D-11)  │
│      └─ execute  → safe_copy(src→dst) + FileTimes preserve     │
│                    accumulate bytes → summary (D-11)           │
└───────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure
```
cli-tools/
├── Cargo.toml              # deps above + [profile.release] (lto thin, codegen-units 1, strip, panic=abort)
├── install.ps1             # build → copy → PATH → session refresh → smoke test
├── src/
│   ├── main.rs             # ~40 lines: ansi bootstrap, parse, dispatch, exit-code mapping
│   ├── cli.rs              # Cli + Commands enum — ALL 23 variants, doc comments = about
│   ├── core/
│   │   ├── mod.rs
│   │   ├── errors.rs       # BoxError (thiserror): NotImplemented { cmd }, plus future variants
│   │   ├── output.rs       # init_color(no_color), color-gating, glyph/summary helpers
│   │   └── fs.rs           # normalize_path (dunce), walk wrapper, safe_copy(+FileTimes), is_hidden
│   └── commands/
│       ├── mod.rs          # RunCommand trait + pub use
│       ├── flatten/
│       │   ├── mod.rs      # FlattenArgs + impl RunCommand (plan/execute orchestration)
│       │   └── rename.rs   # pure collision-encoding + reserved-name sanitization (unit-tested)
│       └── stub.rs         # shared NotImplemented handler for the other 22 (or inline per module)
└── tests/
    ├── cli.rs              # --help lists 23, --version, badcmd exit 2, bare box, NotImplemented exit 1
    └── flatten.rs          # dry-run plan, collision rename, timestamps, containment guard, piped=no-ANSI
```

> The 22 stub modules need not each be a folder this phase. A single `commands/stub.rs` with one `NotImplemented` handler, dispatched from `main`'s match arms, satisfies D-05/D-06 with minimal code. The doc-comment `about` text lives on the `Commands` enum variants in `cli.rs` regardless. Folders for the 22 are created in their own phases.

### Pattern 1: Clap-derive enum with 23 variants, doc-comment → about (D-05)
**What:** One `Commands` enum; each variant's `///` doc comment becomes its `box --help` one-liner AND its `box <cmd> --help` long help.
**When to use:** Always — this is the single registry of all commands.
**Verification:** clap derive turns the doc comment on an enum variant into the subcommand's about/help text. [CITED: docs.rs/clap `_derive` tutorial; clap-rs/clap discussion #1619]
```rust
// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "box", version, about = "A Rust CLI toolbox", arg_required_else_help = true)]
pub struct Cli {
    /// Disable colored output (also honors NO_COLOR and non-TTY stdout)
    #[arg(long, global = true)]
    pub no_color: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Flatten a folder tree into a single output directory
    Flatten(crate::commands::flatten::FlattenArgs),
    /// Render a QR code for text or a URL in the terminal
    Qr,
    /// Generate secure passwords and passphrases
    Passgen,
    /// Compute and verify file hashes
    Hash,
    // ... all 23. Stub variants need no Args struct (or an empty one).
}
```
> Note: `version` on `#[command(...)]` reads `Cargo.toml`'s `version` at compile time (FOUND-02). `arg_required_else_help = true` is set per D-08 — but see Pitfall 2 for the exit-code override it requires.

### Pattern 2: RunCommand trait + static dispatch
**What:** `trait RunCommand { fn run(self) -> anyhow::Result<()>; }`; `main` matches and calls `.run()`. No `Box<dyn>`.
**When to use:** Always.
```rust
// src/commands/mod.rs
pub trait RunCommand { fn run(self) -> anyhow::Result<()>; }
```

### Pattern 3: NotImplemented stub handler (D-06)
**What:** The 22 unbuilt commands return a typed error that `main` maps to a stderr message and exit 1.
```rust
// src/core/errors.rs
use thiserror::Error;
#[derive(Debug, Error)]
pub enum BoxError {
    #[error("'{cmd}' is not yet implemented — coming in a future release")]
    NotImplemented { cmd: &'static str },
}
```
The stub arm in `main` produces `Err(BoxError::NotImplemented { cmd: "qr" }.into())`. `main` prints `error: {e}` to stderr (note the lowercase `error:` prefix to match clap's style, per the locked sample) and returns exit 1.

### Pattern 4: main() exit-code mapping that preserves clap's 2 (D-07) and honors D-08
**What:** Use `try_parse()` (not `parse()`) so `main` controls every exit. Preserve clap's exit 2 for genuine parse errors; force exit 2 for the bare-`box` help case (D-08); exit 0 for `--help`/`--version`; exit 1 for runtime errors.
**Why this matters:** `Cli::parse()` calls `Error::exit()` internally, which exits **0** for `DisplayHelpOnMissingArgumentOrSubcommand` (bare `box`) — contradicting D-08. `try_parse()` returns the error so `main` can inspect `error.kind()` and override. [VERIFIED: clap_builder source — `DisplayHelpOnMissingArgumentOrSubcommand` maps to `Stream::Stdout`, `use_stderr()==false`, `exit_code()==0`]
```rust
// src/main.rs  (~40 lines)
use std::process::ExitCode;
use clap::error::ErrorKind;
use clap::Parser;

fn main() -> ExitCode {
    let _ = enable_ansi_support::enable_ansi_support(); // FIRST — before any output

    let cli = match crate::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // --help / --version: clap prints to stdout, exit 0
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            // Bare `box` (no subcommand): clap would exit 0 to stdout.
            // D-08 requires exit 2. Print help to STDERR and force 2.
            if matches!(e.kind(), ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand) {
                let _ = e.print(); // prints help text
                return ExitCode::from(2);
            }
            // Genuine parse errors (badcmd, bad/missing args): clap's own exit 2.
            e.exit(); // never returns; preserves exit 2 to stderr (D-07)
        }
    };

    crate::core::output::init_color(cli.no_color);

    let result = match cli.command {
        crate::cli::Commands::Flatten(args) => args.run(),
        crate::cli::Commands::Qr      => Err(box_err("qr")),
        crate::cli::Commands::Passgen => Err(box_err("passgen")),
        // ... 20 more stub arms
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");   // lowercase prefix, stderr only (FOUND-03)
            ExitCode::from(1)            // runtime error / NotImplemented (D-06)
        }
    }
}

fn box_err(cmd: &'static str) -> anyhow::Error {
    crate::core::errors::BoxError::NotImplemented { cmd }.into()
}
```
> ⚠️ Whether `e.print()` for the bare-`box` case should write to stderr or stdout: `e.print()` writes to the stream clap chose (stdout for this kind). If a strict reading of D-03/FOUND-03 ("messages→stderr") is required, use `eprintln!("{e}")` instead of `e.print()` for that one branch. **Flag for the planner / discuss-phase.** (See OQ-1.)

### Pattern 5: Color gating (D-10, FOUND-04)
**What:** Decide color once, at startup, from `--no-color` ∧ `NO_COLOR` ∧ TTY. owo-colors honors a global override.
```rust
// src/core/output.rs
use std::io::IsTerminal; // stdlib, Rust 1.70+

pub fn color_enabled(no_color_flag: bool) -> bool {
    !no_color_flag
        && std::env::var_os("NO_COLOR").is_none()
        && std::io::stdout().is_terminal()
}

pub fn init_color(no_color_flag: bool) {
    owo_colors::set_override(color_enabled(no_color_flag));
}
```
> `set_override(false)` forces all owo-colors `.green()` etc. to no-op, so the **plain layout is byte-identical minus ANSI** (D-10). The flatten glyph (`+ ~ -`) is printed unconditionally; only the color wrapper is gated.

### Pattern 6: flatten planner/executor split (FLAT-03, Claude's discretion)
**What:** Build a `Vec<PlanItem>` first; dry-run prints it and stops; execute consumes the same plan. One source of truth → dry-run and real run can never diverge.
```rust
enum ItemKind { Copy, Rename, Skip }
struct PlanItem { src: PathBuf, dst: Option<PathBuf>, kind: ItemKind, reason: Option<&'static str> }
struct Plan { items: Vec<PlanItem>, to_copy: usize, renamed: usize, skipped: usize }
```
Dry-run: build `Plan` (resolving collisions against the seeded `occupied` set so the *preview names match what a real run would produce*), print, emit D-11 dry-run summary. Execute: same `Plan`, then `safe_copy` each + `FileTimes`, accumulate bytes, emit D-11 real-run summary.

### Anti-Patterns to Avoid
- **`Cli::parse()` in `main`** — it auto-exits 0 on bare `box`, defeating D-08. Use `try_parse()`.
- **Business logic in `main` match arms** — each arm is one `.run()` call; logic lives in modules.
- **`std::fs::canonicalize`** — leaks `\\?\` UNC. Always `dunce::canonicalize` (Pitfall 1).
- **`std::process::exit()` inside commands** — bypasses Drop and breaks assert_cmd. Return `Err`; only `main` returns `ExitCode`.
- **Per-`println!` `is_terminal()` checks** — decide color once in `init_color`.
- **Collision-checking source names only** — must seed from `read_dir(out)` (D-14) or prior output is silently clobbered.
- **Building plan from un-canonicalized paths** — collision prefix must be relative to the `dunce`-canonicalized source root (D-15).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Path canonicalization without UNC | Manual `\\?\` stripping | `dunce::canonicalize` | Handles verbatim/UNC edge cases; battle-tested. |
| Recursive walk + subtree pruning | Manual `read_dir` recursion | `walkdir` + `filter_entry` | `filter_entry` avoids descending hidden dirs entirely (D-12). |
| Timestamp preservation after copy | Win32 `SetFileTime` FFI | `std::fs::FileTimes` + `File::set_times` | Stable stdlib (1.75+); cross-platform; no dependency. |
| TTY detection | `isatty` FFI / `atty` crate | `std::io::IsTerminal` | Stable stdlib (1.70+); `atty` is unmaintained. |
| Color override plumbing | Per-command `if color` branches | `owo_colors::set_override` | One global switch; piped output stays clean. |
| Exit-code → stderr/stdout policy | Custom error printing | clap `Error::exit()` for parse errors + explicit `ExitCode` for the rest | clap already encodes the POSIX-2 convention; only override the D-08 case. |
| Idempotent PATH append | String `.Contains()` substring test | split `;` + `-notcontains` | Substring test false-matches `C:\foo` inside `C:\foobar`. |
| CLI black-box tests | Spawning `box.exe` by hand | `assert_cmd::Command::cargo_bin("box")` | Builds + locates the binary, captures stdout/stderr/exit. |
| Temp source trees in tests | Manual mkdir/cleanup | `assert_fs` / `tempfile` | Auto-cleanup; ergonomic nested-tree builders. |

**Key insight:** Every hand-rolled Windows path/timestamp/TTY primitive is a documented foot-gun (UNC leakage, lost mtime, unmaintained `atty`). The stdlib (1.90 toolchain) plus `dunce` and `walkdir` cover all of them — this phase introduces zero novel low-level code outside the collision-encoding function, which is pure and unit-testable.

## Runtime State Inventory

> **Greenfield phase — but `install.ps1` writes OS-registered state.** A pure-code phase would omit this section; here the installer touches the Windows registry and live process environment, so it is in scope.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore is created or read this phase. | None — verified: flatten only copies files; no DB/collection/key names involved. |
| Live service config | None — no external service. | None. |
| OS-registered state | **User PATH** in `HKCU\Environment` (`Path` value) gains `%LOCALAPPDATA%\Programs\box`. This persists across reboots and is read by every new shell. | install.ps1 writes it idempotently (D-02). **Uninstall is out of scope** but the entry remains until manually removed — note for future. ⚠️ Must preserve `REG_EXPAND_SZ` type if existing PATH has `%VARS%` (Pitfall 3). |
| Secrets/env vars | None created. `$env:RUSTFLAGS` is set transiently inside install.ps1 for the build only. | None — `$env:RUSTFLAGS` is process-local to the script; does not persist. |
| Build artifacts | `target/release/box.exe` (build output) and the installed copy at `%LOCALAPPDATA%\Programs\box\box.exe`. Re-install overwrites the latter (`Copy-Item -Force`, D-04). | None beyond the overwrite; no stale egg-info/package-cache equivalent in a single-binary Rust crate. |

**The canonical question (after every file is updated, what runtime state persists?):** the user PATH registry entry and the live-session `$env:Path`. Both are handled by install.ps1 (D-02 write, D-03 refresh). Nothing else survives a re-run.

## Common Pitfalls

### Pitfall 1: `std::fs::canonicalize` leaks `\\?\` UNC paths (FOUND-06)
**What goes wrong:** `std::fs::canonicalize("C:\\src")` → `\\?\C:\src`. This prefix corrupts the collision-encoding prefix (drive-letter logic), breaks `Path::starts_with` containment checks, and shows up in user-facing flatten output.
**Why it happens:** Rust's canonicalize calls `GetFinalPathNameByHandleW`, which always prefixes `\\?\` (rust-lang/rust#42869).
**How to avoid:** Wrap `dunce::canonicalize` in `core::fs::normalize_path` and use it for BOTH src and out roots before anything else. Never call `std::fs::canonicalize`.
**Warning signs:** any path beginning `\\?\` in flatten output; containment guard misfiring.
[CITED: gal.hagever.com/posts/windows-long-paths-in-rust; rust-lang/rust#42869] [VERIFIED: dunce 1.0.5 via cargo search]

### Pitfall 2: `arg_required_else_help` exits 0 to stdout, not 2 to stderr — contradicts D-08
**What goes wrong:** With `#[command(arg_required_else_help = true)]` and `Cli::parse()`, running bare `box` prints the help to **stdout** and exits **0**. D-08 mandates **exit 2**.
**Why it happens:** clap classifies this as `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand`, which `clap_builder` maps to `Stream::Stdout`, `use_stderr()==false`, `exit_code()==0` — identical to a normal `--help`. This changed in clap 3-beta-4 (clap-rs/clap#2767) and remains the 4.x behavior.
**How to avoid:** Use `Cli::try_parse()` and, in `main`, match `ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand` and `return ExitCode::from(2)` explicitly (Pattern 4). Decide deliberately whether to print that help to stdout (`e.print()`) or stderr (`eprintln!`) — see OQ-1.
**Warning signs:** `box; echo $LASTEXITCODE` prints `0` instead of `2`; success-criterion 2's "`box badcmd` exits 2" passes but bare-`box` silently doesn't.
[VERIFIED: clap_builder source `error/mod.rs` `stream()`/`exit_code()`; clap-rs/clap#2767, #2021] — **This is the single most planning-relevant finding; ensure a test asserts bare-`box` exit code.**

### Pitfall 3: install.ps1 PATH write downgrades `REG_EXPAND_SZ` → `REG_SZ`, corrupting `%VAR%` entries (D-02)
**What goes wrong:** If the user's existing PATH contains unexpanded refs like `%USERPROFILE%\bin`, reading it (which auto-expands), appending, and writing back with the wrong API stores literal expanded text as a `REG_SZ`. Future `%VAR%` edits stop expanding; other tools that expect `REG_EXPAND_SZ` misbehave.
**Why it happens:** `Get-ItemPropertyValue`/`[Environment]::GetEnvironmentVariable` expand `%VARS%` on read; a naive write loses the original type. (Mirrors microsoft/WindowsAppSDK#1599, PowerShell#16812.)
**How to avoid:** Per D-02 — if the existing user PATH contains `%`, write via `Set-ItemProperty -Path 'HKCU:\Environment' -Name Path -Value $newPath -Type ExpandString`. For purely literal paths, `[Environment]::SetEnvironmentVariable(...,'User')` is sufficient. Read the raw (un-expanded) value when deciding, ideally via `Get-Item -Path HKCU:\Environment | %{ $_.GetValue('Path', '', 'DoNotExpandEnvironmentNames') }`.
**Warning signs:** after install, `reg query HKCU\Environment /v Path` shows `REG_SZ` where it was `REG_EXPAND_SZ`; `%VAR%` entries appear expanded.
[VERIFIED: WebSearch — microsoft/WindowsAppSDK#1599, PowerShell/PowerShell#16812, microsoft/terminal#9741]

### Pitfall 4: flatten output dir inside source dir → infinite copy loop (FLAT-04)
**What goes wrong:** `box flatten ./project ./project/flat` — the walker re-visits files it just copied, copying forever, filling the disk.
**How to avoid:** After `dunce::canonicalize` of both roots, abort if `out_root.starts_with(&src_root)`. ⚠️ `Path::starts_with` is **case-sensitive on Windows** (rust-lang/rust#66260) and NTFS is case-insensitive — lowercase both canonical paths before the check, or also guard by excluding `out_root` from the walk. Do this **before** any I/O.
**Warning signs:** disk fills; output grows without bound; very long path-encoded names accumulate.
[CITED: .planning/research/PITFALLS.md Pitfall 3; rust-lang/rust#66260]

### Pitfall 5: 260-char `MAX_PATH` silently fails on deep trees (FOUND-06)
**What goes wrong:** `fs::copy`/`create_dir_all` on a deeply nested source return `Os { code: 3, NotFound }`-style errors that look like "file not found" even though the path exists.
**How to avoid:** For this phase, the minimum is to **surface the error clearly** (anyhow `.context("copying {src}")`) rather than silently dropping the file (FOUND-06 forbids silent failure). A fuller fix (verbatim `\\?\` prefix for I/O, or a `LongPathsAware` manifest) is a larger change — see OQ-2. At minimum, add a CI test with a ~300-char synthetic path asserting a clear error (or success if long-paths are enabled).
**Warning signs:** flatten reports fewer files than the tree contains; `NotFound` on dirs visible in Explorer.
[CITED: .planning/research/PITFALLS.md Pitfall 2; rust-lang/rust#67403, #76586]

### Pitfall 6: `fs::copy` does not preserve timestamps on Windows (FLAT-04)
**What goes wrong:** Every flattened file gets today's mtime; original dates are destroyed.
**How to avoid:** After each copy, set times from source metadata using `std::fs::FileTimes`:
```rust
let meta = std::fs::metadata(&src)?;
let times = std::fs::FileTimes::new()
    .set_modified(meta.modified()?)
    .set_accessed(meta.accessed()?);
std::fs::File::options().write(true).open(&dst)?.set_times(times)?;
```
Centralize in `core::fs::safe_copy` so no command forgets it.
**Warning signs:** `dir` shows today's date on years-old files; the timestamp test fails.
[VERIFIED: doc.rust-lang.org/std/fs/struct.FileTimes.html — stable 1.75, preserves modified+accessed on Windows]

### Pitfall 7: Reserved Windows filenames crash the collision encoder (FLAT-02)
**What goes wrong:** A source path encoding to `CON.txt`, `NUL.txt`, `COM1.txt`, etc. makes `fs::write` fail with `Os { code: 87 }` or silently target a device — the file vanishes.
**How to avoid:** After encoding (D-15), check the stem case-insensitively against `CON, PRN, AUX, NUL, COM0–9, LPT0–9` (with or without extension); if matched, append `_` before the extension. Also strip trailing dots/spaces (Windows trims them, creating hidden collisions). Implement as a pure function in `flatten/rename.rs`; unit-test each reserved name.
**Warning signs:** a source file literally named `con`/`nul` disappears; `Os { code: 87 }`.
[CITED: Microsoft Learn "Naming Files, Paths, and Namespaces"; .planning/research/PITFALLS.md Pitfall 14]

### Pitfall 8: `walkdir` follows junctions into loops; `filter_entry` has a first-entry gotcha (FLAT-04, D-12)
**What goes wrong:** (a) With `follow_links(true)`, Windows junctions (very common) cause infinite traversal. (b) `filter_entry`'s predicate is also applied to the **root** entry — if your `is_hidden` predicate returns true for the root (e.g., the user passes a dotted dir as src), the walk yields nothing (walkdir#142).
**How to avoid:** Keep `follow_links(false)` (default); skip entries where `entry.path_is_symlink()` is true, emitting a `(skipped: symlink)` line (D-10). For `filter_entry`, only apply the hidden test to non-root entries (`entry.depth() > 0`), so the user can flatten a dir that itself starts with `.`.
**Warning signs:** CPU pegged, file count unbounded; or flatten of a normal tree yields zero files because the root matched the hidden predicate.
[VERIFIED: WebSearch — walkdir filter_entry docs, BurntSushi/walkdir#142; .planning/research/PITFALLS.md Pitfall 5]

### Pitfall 9: PATH not visible in current session / execution policy blocks the script (FOUND-08)
**What goes wrong:** User runs install, then `box --help` in the same window → "command not found", because the registry change doesn't refresh the live `$env:Path`. Separately, a fresh machine's default execution policy may block `.\install.ps1`.
**How to avoid:** D-03 — rebuild `$env:Path` from **both** User and Machine scopes at the end of the script, then run `box --help` as a smoke test. Document the one-time `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` (or `powershell -ExecutionPolicy Bypass -File install.ps1`) in the README.
**Warning signs:** success-criterion 1 fails (`box --help` not found in same session).
[CITED: .planning/research/PITFALLS.md Pitfalls 10, 11; abdus.dev PATH update pattern]

## Code Examples

### install.ps1 — build, copy, idempotent PATH (REG_EXPAND_SZ-safe), session refresh, smoke test
```powershell
#Requires -Version 7
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$BinDir = Join-Path $env:LOCALAPPDATA 'Programs\box'          # D-01
$ExeSrc = '.\target\x86_64-pc-windows-msvc\release\box.exe'
$ExeDst = Join-Path $BinDir 'box.exe'

# 1. Build (crt-static, MSVC) — CLAUDE.md Release Build guidance
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 2. Copy (plain overwrite — D-04)
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
Copy-Item -Force $ExeSrc $ExeDst
Write-Host "Installed to $ExeDst"

# 3. Idempotent user-PATH update (D-02), preserving REG_EXPAND_SZ if %VARS% present
$key   = 'HKCU:\Environment'
$rawPath = (Get-Item -Path $key).GetValue('Path', '', 'DoNotExpandEnvironmentNames')
$entries = $rawPath -split ';' | Where-Object { $_ -ne '' }
if ($entries -inotcontains $BinDir) {
    $newPath = (@($entries) + $BinDir) -join ';'
    if ($rawPath -match '%') {
        Set-ItemProperty -Path $key -Name 'Path' -Value $newPath -Type ExpandString  # avoids REG_SZ downgrade
    } else {
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    }
    Write-Host "Added $BinDir to user PATH"
} else {
    Write-Host "$BinDir already in user PATH — skipped"
}

# 4. Refresh current session — merge User + Machine (D-03)
$env:Path = [Environment]::GetEnvironmentVariable('Path','User') + ';' +
            [Environment]::GetEnvironmentVariable('Path','Machine')

# 5. Smoke test (FOUND-08)
& box --help | Out-Null
if ($LASTEXITCODE -eq 0) { Write-Host "box is ready. Try: box --help" }
else { Write-Warning "Installed, but 'box' did not run in this session. Open a new terminal." }
```
> `'DoNotExpandEnvironmentNames'` reads the raw `%VAR%` form so the dedup check and the type decision are correct. [VERIFIED: WebSearch — PowerShell registry expansion behavior]

### Collision rename — source-relative encoding + reserved-name sanitization (D-15, FLAT-02)
```rust
// src/commands/flatten/rename.rs  (pure; unit-testable)
use std::path::Path;

const RESERVED: &[&str] = &[
    "CON","PRN","AUX","NUL",
    "COM1","COM2","COM3","COM4","COM5","COM6","COM7","COM8","COM9",
    "LPT1","LPT2","LPT3","LPT4","LPT5","LPT6","LPT7","LPT8","LPT9",
];

/// Encode a path relative to the canonical source root into a flat filename.
/// `rel` must already be relative to the dunce-canonicalized source root (D-15).
pub fn encode_relative(rel: &Path) -> String {
    let mut s = rel.to_string_lossy().replace(['\\', '/'], "_");
    // drop any leading separator artifact
    s = s.trim_start_matches('_').to_string();
    sanitize_reserved(&s)
}

/// If the stem (case-insensitive, with or without extension) is a reserved
/// device name, or the name has trailing dots/spaces, make it safe.
pub fn sanitize_reserved(name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (name, None),
    };
    let is_reserved = RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem));
    let mut stem = stem.trim_end_matches(['.', ' ']).to_string();
    if is_reserved { stem.push('_'); }
    match ext {
        Some(e) => format!("{stem}.{e}"),
        None => stem,
    }
}

/// Append `_1`, `_2`, … before the extension until `occupied` (lowercased) is free.
pub fn dedupe<'a>(name: &str, occupied: &std::collections::HashSet<String>) -> String {
    let key = name.to_ascii_lowercase();
    if !occupied.contains(&key) { return name.to_string(); }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    for n in 1.. {
        let cand = format!("{stem}_{n}{ext}");
        if !occupied.contains(&cand.to_ascii_lowercase()) { return cand; }
    }
    unreachable!()
}
```
> Seed `occupied` from `read_dir(out_root)` (lowercased) before the loop (D-14), and insert each chosen name as you go so within-run collisions also dedupe. NTFS case-insensitivity → always key on `to_ascii_lowercase()` (PITFALLS Pitfall 4).

### Hidden-entry predicate for `filter_entry` (D-12)
```rust
// src/core/fs.rs
use walkdir::DirEntry;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

pub fn is_hidden(entry: &DirEntry) -> bool {
    if entry.depth() == 0 { return false; } // never prune the root (walkdir#142)
    let dot = entry.file_name().to_str().map_or(false, |s| s.starts_with('.'));
    #[cfg(windows)]
    let attr = entry.metadata().map_or(false, |m| m.file_attributes() & 0x2 != 0); // FILE_ATTRIBUTE_HIDDEN
    #[cfg(not(windows))]
    let attr = false;
    dot || attr
}
// usage: WalkDir::new(&src_root).into_iter().filter_entry(|e| !is_hidden(e))
```
[VERIFIED: WebSearch — std::os::windows::fs::MetadataExt::file_attributes, FILE_ATTRIBUTE_HIDDEN = 0x2; walkdir filter_entry semantics]

### assert_cmd integration test skeleton
```rust
// tests/cli.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_flatten_and_a_stub() {
    Command::cargo_bin("box").unwrap().arg("--help").assert()
        .success()
        .stdout(predicate::str::contains("flatten").and(predicate::str::contains("qr")));
}

#[test]
fn badcmd_exits_2() {
    Command::cargo_bin("box").unwrap().arg("definitely-not-a-cmd").assert()
        .failure().code(2);   // clap parse error (D-07)
}

#[test]
fn bare_box_exits_2() {
    Command::cargo_bin("box").unwrap().assert()
        .failure().code(2);   // D-08 — REQUIRES the main() override (Pitfall 2)
}

#[test]
fn stub_exits_1_to_stderr() {
    Command::cargo_bin("box").unwrap().arg("qr").assert()
        .failure().code(1)
        .stderr(predicate::str::contains("not yet implemented"))
        .stdout(predicate::str::is_empty());   // FOUND-03: errors never on stdout
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `atty` crate for TTY detection | `std::io::IsTerminal` | Rust 1.70 (2023) | Drop the dependency; `atty` is unmaintained. |
| `filetime` crate for timestamps | `std::fs::FileTimes` + `File::set_times` | Rust 1.75 (2023) | No dependency; preserves mtime+atime on Windows. Use stdlib. |
| `Cli::parse()` everywhere | `try_parse()` when you need exit-code control | clap 4.x | Required to satisfy D-08 (override bare-`box` exit code). |
| MinGW `x86_64-pc-windows-gnu` | `x86_64-pc-windows-msvc` | Rust 1.88 demoted GNU to Tier 2 (May 2025) | Build target is MSVC + crt-static (CLAUDE.md). |

**Deprecated/outdated:**
- `atty` — unmaintained; replaced by stdlib `IsTerminal`.
- `is-terminal` crate — redundant on Rust ≥1.70; do not add.
- `termcolor` / `colored` — wrong Windows ANSI story (CLAUDE.md "What NOT to Use").

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 22 stubs can share one `commands/stub.rs` `NotImplemented` handler this phase (folders deferred to their own phases). | Project Structure | Low — purely organizational; doc-comment `about` still lives on the enum variants regardless. |
| A2 | The locked-stack crate versions (clap 4.6, etc.) from CLAUDE.md are still current/compatible with toolchain 1.90. | Standard Stack | Low — taken as final per scope; `cargo build` will surface any incompatibility immediately. |
| A3 | Preserving `accessed()` time (not just `modified()`) is acceptable/desired in flatten. D-15/FLAT-04 only mandate mtime. | Pitfall 6 | Low — preserving both is harmless; if `accessed()` is unavailable on some FS, set only `modified()`. |

**Note:** Most factual claims this session are `[VERIFIED]` or `[CITED]`. The above are the only genuine assumptions. No `[ASSUMED]` package names were introduced — all crates were verified via `cargo search` + slopcheck.

## Open Questions

1. **OQ-1 — Where does the bare-`box` help text go, and what exit code? (affects D-03 vs D-08)**
   - What we know: clap's default for `arg_required_else_help` is **stdout, exit 0**. D-08 wants exit 2. FOUND-03/D-03 say "messages → stderr."
   - What's unclear: whether the bare-`box` help should print to **stdout** (matching `--help`) or **stderr** (matching "messages") while still exiting 2.
   - Recommendation: Print to **stderr** and exit 2 (`eprintln!("{e}")` in the override branch) — this is internally consistent with "help-as-error → stderr" and the strict 0/1/2 convention. Surface to discuss-phase if the user prefers stdout. Either way, **a test must assert exit 2** for bare `box`.

2. **OQ-2 — How far to take long-path (>260 char) handling this phase? (FOUND-06)**
   - What we know: FOUND-06 forbids "silent failure" on long paths. Full mitigation (verbatim `\\?\` I/O prefix or `LongPathsAware` manifest via `embed-resource`) is a non-trivial addition.
   - What's unclear: whether Phase 1 must *succeed* on >260-char paths, or only *fail loudly* (clear error, non-silent).
   - Recommendation: For the walking skeleton, satisfy FOUND-06's "no silent failure" by wrapping copy I/O in `.context(...)` so deep-path errors are reported per-file, and add a CI test asserting a clear error (not a silent drop). Defer full verbatim-prefix/manifest support unless the user requires success on >260-char trees. Flag for discuss-phase.

3. **OQ-3 — `winrt-notification` Phase 1 compile spike (STATE.md todo).**
   - What we know: STATE.md lists "Spike `winrt-notification 0.5` compilation against project MSRV during Phase 1" to de-risk Phase 5.
   - What's unclear: whether this spike belongs in *this* phase's plan or is tracked separately. It is out of Phase 1's stated scope (FOUND/FLAT only) but is an explicit STATE.md todo.
   - Recommendation: Treat as an optional, isolated build-only task (add the dep behind `[target.'cfg(windows)'.dependencies]`, `cargo check`, then remove or keep gated). Do not let it block the flatten slice. Flag to the planner as a discretionary de-risking task.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (cargo/rustc) | building the crate | ✓ | rustc 1.90.0 / cargo 1.90.0 | — (toolchain ≥1.75 needed for `FileTimes`; 1.90 satisfies) |
| `x86_64-pc-windows-msvc` target | release build (CLAUDE.md) | ✓ (default on Windows) | bundled with 1.90 | — |
| PowerShell 7 | running install.ps1 | ✓ (per project target / env) | PS7 | — |
| crates.io access | resolving deps at build time | ✓ (cargo search succeeded) | — | vendored deps if offline |
| MSVC build tools (linker) | crt-static MSVC link | Assumed ✓ (rustc 1.90 MSVC host installed) | — | If `link.exe` missing, install VS Build Tools — **verify during scaffold task** |

**Missing dependencies with no fallback:** none confirmed. ⚠️ Verify the MSVC linker is present on first `cargo build --release` (it is implied by a working MSVC-host rustc, but a clean machine may lack VS Build Tools).
**Missing dependencies with fallback:** none.

## Validation Architecture

> Nyquist validation is enabled (`workflow.nyquist_validation: true`). This section maps each of the 5 phase success criteria to an automatable test.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + `assert_cmd 2.2` / `predicates 3.1` / `assert_fs 1.1` / `trycmd 1.2` (+ optional `insta 1.48`) |
| Config file | none — `[dev-dependencies]` in `Cargo.toml` (created in Wave 0) |
| Quick run command | `cargo test --test cli` (scaffold/exit-code suite, fast) |
| Full suite command | `cargo test` (all unit + integration) |

### Phase Requirements → Test Map
| Req / Success Criterion | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SC1 / FOUND-01 | `box --help` lists all 23 subcommands w/ descriptions | integration (trycmd snapshot of help) | `cargo test --test cli` | ❌ Wave 0 |
| SC1 / FOUND-07,08 | install.ps1 makes `box` runnable in same session | manual + scripted smoke | `pwsh -File install.ps1` then `box --help` (CI: run install.ps1, assert exit 0) | ❌ Wave 0 (PS Pester optional) |
| SC2 / FOUND-02 | `box --version` prints semver | integration | `cargo test --test cli` (`version_is_semver`) | ❌ Wave 0 |
| SC2 / FOUND-03 | `box badcmd` exit 2; bare `box` exit 2; errors→stderr only | integration | `cargo test --test cli` (`badcmd_exits_2`, `bare_box_exits_2`, `stub_exits_1_to_stderr`) | ❌ Wave 0 |
| SC3 / FOUND-04 | `box flatten --help` piped = no ANSI; terminal = colored | integration (assert no `\x1b[` when piped) | `cargo test --test cli` (`piped_help_has_no_ansi`) | ❌ Wave 0 |
| SC4 / FLAT-02,03 | dry-run on nested dup-name tree shows collision plan, copies nothing | integration (assert_fs fixture + trycmd) | `cargo test --test flatten` (`dry_run_plans_collisions_writes_nothing`) | ❌ Wave 0 |
| SC5 / FLAT-01 | real run flattens all files, no subdirs in out | integration | `cargo test --test flatten` (`flatten_copies_all_files_flat`) | ❌ Wave 0 |
| SC5 / FLAT-04 | timestamps preserved; originals untouched; nothing lost/overwritten | integration (compare mtime; count files) | `cargo test --test flatten` (`preserves_mtime`, `originals_untouched`, `no_silent_overwrite`) | ❌ Wave 0 |
| FLAT-04 | output-inside-source aborts | integration | `cargo test --test flatten` (`out_inside_src_aborts`) | ❌ Wave 0 |
| FLAT-04 | symlink/junction skipped, no loop | integration (create symlink fixture) | `cargo test --test flatten` (`skips_symlinks`) | ❌ Wave 0 |
| FLAT-02 | reserved-name source (`con.txt`) sanitized, not lost | unit | `cargo test` (`rename::sanitize_reserved` cases) | ❌ Wave 0 |
| FOUND-06 | deep (>260) path fails loudly, not silently | integration | `cargo test --test flatten` (`deep_path_no_silent_loss`) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --test cli` (fast scaffold/exit-code assertions) + `cargo build`.
- **Per wave merge:** `cargo test` (full suite) + `cargo clippy -- -D warnings`.
- **Phase gate:** Full suite green AND a manual `install.ps1` → same-session `box --help` confirmation before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `tests/cli.rs` — covers SC1, SC2, SC3, FOUND-01..05 (help listing, version, exit codes, piped-no-ANSI, stub error).
- [ ] `tests/flatten.rs` — covers SC4, SC5, FLAT-01..04, FOUND-06 (dry-run plan, real copy, timestamps, containment guard, symlink skip, deep path).
- [ ] Unit tests in `src/commands/flatten/rename.rs` — `encode_relative`, `sanitize_reserved` (all reserved names), `dedupe` numeric fallback, NTFS-case keying.
- [ ] `[dev-dependencies]` block added to `Cargo.toml` (assert_cmd, predicates, assert_fs, tempfile, trycmd, insta).
- [ ] Optional: `tests/cmd/*.trycmd` transcript for the locked dry-run sample (D-09) and the 23-command `--help` listing.
- [ ] Optional: Pester test (or a CI step) that runs `install.ps1` in a sacrificial PATH and asserts the bin dir is added once (idempotency).

## Security Domain

> `security_enforcement` not set in config → treated as enabled. This is a local file-utility + installer with no network, auth, or untrusted input, so most ASVS categories are N/A; the live ones are filesystem-safety and installer integrity.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | partial | Installer writes user-scope registry/PATH only — **no admin/UAC** (D-01/D-02). Never write Machine scope. |
| V5 Input Validation | yes | Path args validated: canonicalize, containment guard, reserved-name sanitization, hidden filtering. Reject paths that escape the source root in the encoded name. |
| V6 Cryptography | no | No crypto this phase (hashing is Phase 3). |
| V12 Files & Resources | yes | Core domain. No silent overwrite (D-14 collision check); copy-only never move (FLAT-01); skip symlinks (no traversal escape); no path-injection via encoded names. |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Output-inside-source copy loop (DoS: disk fill) | Denial of Service | Canonical containment guard before I/O (Pitfall 4). |
| Symlink/junction traversal escaping source root | Tampering / Elevation | `follow_links(false)`; skip symlinks; never follow into ancestors (Pitfall 8). |
| Encoded collision name escaping output dir (`..`, separators) | Tampering | Encoding replaces all separators with `_`; result is a single flat filename — no path components, no `..` survives. Unit-test that no encoded name contains a separator. |
| Reserved device name write (`CON`, `NUL`) → data loss | Tampering / DoS | Reserved-stem sanitization (Pitfall 7). |
| PATH registry type downgrade corrupting other apps' `%VARS%` | Tampering | `ExpandString` write when `%` present (Pitfall 3). |
| install.ps1 execution-policy / SmartScreen friction | (operational, not a vuln) | Document `RemoteSigned`/`Bypass`; user-scope only, no elevation. |
| Silent data loss on deep paths | Information disclosure / integrity | Surface per-file errors loudly; no silent drop (FOUND-06, Pitfall 5). |

## Sources

### Primary (HIGH confidence)
- doc.rust-lang.org/std/fs/struct.FileTimes.html — `FileTimes`/`set_times` stable 1.75, preserves modified+accessed on Windows.
- doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html — `file_attributes()`, `FILE_ATTRIBUTE_HIDDEN = 0x2`.
- docs.rs/clap `_derive` tutorial + `error::ErrorKind` + clap_builder `error/mod.rs` source — doc-comment→about; `DisplayHelpOnMissingArgumentOrSubcommand` → stdout/exit 0; `Error::exit()` → exit 2 for parse errors.
- docs.rs/walkdir `FilterEntry` — `filter_entry` prunes subtrees; root-entry gotcha (BurntSushi/walkdir#142).
- `cargo search` (crates.io sparse index) — verified versions: dunce 1.0.5, filetime 0.2.29, assert_cmd 2.2.2, predicates 3.1.4, assert_fs 1.1.4, tempfile 3.27.0, insta 1.48.0, trycmd 1.2.0, is-terminal 0.4.17.
- slopcheck 0.6.1 scan against crates.io — all 8 new crates `OK`.
- CLAUDE.md — locked runtime stack, versions, "What NOT to Use", crt-static/MSVC build.
- .planning/research/{ARCHITECTURE,PITFALLS,STACK,FEATURES,SUMMARY}.md — project-level research (architecture, 15 pitfalls, stack).

### Secondary (MEDIUM confidence)
- clap-rs/clap #2767, #2021 — `arg_required_else_help` exit-code history (3-beta-4 change to exit 0).
- microsoft/WindowsAppSDK #1599, PowerShell/PowerShell #16812, microsoft/terminal #9741 — REG_EXPAND_SZ vs REG_SZ corruption patterns.
- clap-rs/clap discussion #1619 — subcommand about text from doc comments.
- abdus.dev / PowerShell install-powershell.ps1 — idempotent PATH update pattern (via project research).

### Tertiary (LOW confidence)
- None required this phase — all load-bearing claims verified or cited above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — locked crates cited from CLAUDE.md; new util/test crates verified via cargo search + slopcheck.
- Architecture: HIGH — fully settled by project research; no open debates.
- Exit-code / clap behavior: HIGH — verified against clap_builder source; surfaced the D-08 contradiction.
- install.ps1 / PATH: HIGH — REG_EXPAND_SZ regression and session-refresh confirmed via Microsoft/PowerShell issues.
- flatten internals: HIGH — canonicalize, hidden pruning, timestamps, reserved names, symlink handling all verified against stdlib/walkdir docs and project pitfalls.

**Research date:** 2026-06-22
**Valid until:** 2026-07-22 (stable domain; clap/stdlib behaviors are slow-moving). Re-verify clap exit-code behavior only if the clap major version changes.
