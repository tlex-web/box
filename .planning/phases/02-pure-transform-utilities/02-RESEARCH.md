# Phase 2: Pure Transform Utilities - Research

**Researched:** 2026-06-22
**Domain:** Rust CLI subcommands (pure transforms) + integration/property test strategy
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Lean bundle. Crates only where hand-rolling is pure downside:
  - **Crate** `uuid` 1.23.3 `features=["v4"]` — `Display` gives lowercase-hyphenated; `--upper` = `.to_string().to_uppercase()`.
  - **Crate** `base64` 0.22.1 — `STANDARD` and `URL_SAFE_NO_PAD` engines.
  - **Crate** `chrono` 0.4.45 — Windows local-timezone conversion. Trim default features (drop `oldtime`/`wasmbind`).
  - **Hand-roll** `passgen`, `color`, `cowsay`, `fortune`, `8ball`, `roast`.
- **D-02:** passgen is hand-rolled on `rand` 0.9, NOT the `passwords` crate. ⚠️ MUST sample with `IndexedRandom::choose` / `Rng::random_range`, **never `% len`** (modulo bias).
- **D-03:** color is hand-rolled (closed-form hex↔RGB↔HSL math; unit-test the HSL round-trip). cowsay is hand-rolled (greedy word-wrap + embedded cow art).
- **D-04:** `core::input` auto-detect, layered precedence: (1) positional arg Some & not `"-"` → use it; (2) arg None or `"-"` & stdin NOT a TTY (piped) → read stdin to EOF; (3) arg None & stdin IS interactive TTY → do NOT block; usage error to stderr, exit 2.
- **D-05:** Two helper shapes: `read_input(arg) -> Result<String>` (UTF-8 text) and `read_input_bytes(arg) -> Result<Vec<u8>>` (binary-exact: base64).
- **D-06:** PowerShell 7 piping caveat — PS7 re-encodes native-command output through .NET UTF-16. Read text as UTF-8 `String`; read base64 bytes via `read_to_end` into `Vec<u8>`. `--file PATH` deferred to Phase 3; design extension point now.
- **D-07:** Embedding = `include_str!` text files for large lists (EFF wordlist, fortune, roast under `src/data/`). `const &[&str]` for the 20 8ball answers. No build-script/`phf`.
- **D-08:** RNG split. `OsRng` (Windows ProcessPrng/BCrypt via getrandom) for `passgen`. `rand::rng()` (ThreadRng, ChaCha12, OS-seeded) for fortune/8ball/roast. No manual/fixed seed anywhere. Unbiased selection via `choose`/`random_range` only.
- **D-09:** Content: passphrase = EFF Large (Diceware) 7776-word list (strip dice codes, keep words); ~12.9 bits/word → `--words 4` ≈ 51.6 bits; **CC-BY 3.0 US attribution required**. 8ball = canonical 20 (10 affirmative / 5 non-committal / 5 negative). fortune = ~50–150 CC0/public-domain aphorisms. roast = ~30–80 self-authored/CC0 one-liners.
- **D-10:** `color` output = aligned `label : value` block (Hex, RGB `rgb(r, g, b)`, Tuple `r g b`, HSL) + a **foreground** `██…` truecolor swatch via `.truecolor(r,g,b)`. Foreground swatch only (background-ANSI swatch rejected). Reuse `core::output::{is_color_on, terminal_width}`. New pure helper: `hsl(r,g,b)`.
- **D-11:** `cowsay` output = classic cow + speech bubble, **fixed 40-col** word-wrap with `--width N` override. Single-line `< text >`; multi-line `/ \`, `| |`, `\ /` with shorter lines space-padded to the longest. Hard-break any word longer than the width. Pure ASCII.
- **D-12:** `epoch` date formats = ISO 8601 / RFC 3339 (`2026-06-22T14:30:00Z`) plus `YYYY-MM-DD` and `YYYY-MM-DD HH:MM:SS` (latter two assumed **local**). Anything else → clear error with hint. No `MM/DD/YYYY`. No-arg → current Unix timestamp; integer arg → local + UTC human date.
- **D-13:** `color` input bidirectional, auto-detected — hex (`#3b82f6`, `3b82f6`, `#abc` short) and RGB (`"59,130,246"` or `59 130 246`); detect which, print all reps + swatch.
- **D-14:** `passgen` default charset = lower + upper + digits + curated **paste-safe** symbol subset (no shell/quoting-hostile chars). Flags to restrict (`--no-symbols`). Default length 16; `--words N` passphrase; `--count N` bulk; all output to stdout only.

### Claude's Discretion
- Exact `core::input` error wording; whether bytes/string helpers share an inner reader.
- Module layout under `src/commands/<cmd>/mod.rs` (`8ball` → module `eight_ball`); whether `color` HSL helper lives in color module or `core`.
- The specific curated symbol set for passgen and exact `--charset`/restriction flag surface (default = four-class curated-symbol set, unbiased CSPRNG sampling).
- cowsay's exact cow art bytes; `--width` default/clamp beyond the locked fixed-40 default.
- Exact fortune/roast wording and final counts within ranges; the data-file directory name.
- `uuid` flag spelling beyond locked `-n N` (count) and `--upper`.

### Deferred Ideas (OUT OF SCOPE)
- `--file PATH` input flag for byte-exact commands (design extension point now, implement Phase 3).
- cowsay alternate cow packs + `--think` thought-bubble.
- color `--float` (normalized 0-1) row.
- passgen strength-scoring output.
- epoch timezone-by-name output (`chrono-tz`).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UUID-01 | Generate one+ UUIDs (v4 default), lowercase, one per line | `uuid` 1.23.3 `v4` + `Display`; `--upper`/`-n N`. Test: format regex + count + uniqueness across `-n`. |
| B64-01 | base64 encode + `--decode` from file/stdin, no line-wrap, URL-safe option | `base64` 0.22.1 `STANDARD`/`URL_SAFE_NO_PAD` engines; `core::input::read_input_bytes`. Test: round-trip binary-exact, URL-safe alphabet. |
| EPOC-01 | timestamp→human (local+UTC); date string→timestamp; no-arg→current | `chrono` 0.4.45 `DateTime::from_timestamp` / `parse_from_rfc3339` / `NaiveDateTime`+`Local`. Test: round-trip across 3 modes. |
| COLR-01 | convert hex↔RGB, all reps at once, in-terminal swatch | Hand-rolled hex/RGB/HSL math (D-03/D-10/D-13) + `owo_colors::truecolor`. Test: HSL round-trip unit, hex/RGB parse, swatch ANSI gating. |
| PASS-01 | CSPRNG passwords (len + charset control) + passphrases (`--words N`) + `--count N`, stdout only | Hand-rolled on `rand` 0.9 `OsRng` (unbiased `choose`/`random_range`) + EFF wordlist via `include_str!`. Test: charset membership, length, no-modulo-bias (code review), no symbols flag. |
| COW-01 | ASCII speech bubble for arg or stdin, word-wrapping | Hand-rolled greedy wrap (D-11) + `core::input::read_input`. Test: single vs multi-line bubble, hard-break, exact snapshot. |
| FORT-01 | random fortune, different each run, fits terminal | `include_str!` list + `rand::rng()` + `IndexedRandom::choose`. Test: output ∈ known set + two-runs-differ. |
| 8BAL-01 | classic random Magic 8-Ball response (question optional) | `const &[&str]` 20 answers + `rand::rng()`. Test: output ∈ the 20 + question optional + two-runs-differ. |
| ROST-01 | random programmer roast, different each run | `include_str!` list + `rand::rng()`. Test: output ∈ known set + two-runs-differ. |
</phase_requirements>

## Summary

Phase 2 is a low-risk, high-repetition phase: nine pure-transform subcommands sharing one new helper (`core::input`) and reusing the Phase 1 core (`output` color gating, `RunCommand` trait, strict 0/1/2 exit codes, data→stdout/messages→stderr). The design contract is fully locked in CONTEXT.md (D-01..D-14); the crate versions are locked and verified. This research does **not** revisit those decisions — it supplies the implementation-and-validation layer on top of them.

The two areas that carry real, non-obvious implementation risk are (1) the **rand 0.9 API surface** — there were breaking renames between 0.8 and 0.9, and `OsRng` no longer implements `Rng` directly — and (2) the **test strategy for non-determinism and byte-exactness**, which is the highest-value output of this research because Nyquist validation is enabled (`workflow.nyquist_validation: true`). The non-deterministic commands (fortune/8ball/roast/uuid/passgen) reseed from the OS every process, so the test discipline is "assert membership in a known set" + "assert two runs differ" + "assert structural properties (length, charset, format)" — never "assert exact value." Round-trip/property tests cover the deterministic ones (base64 encode→decode, color HSL↔RGB, epoch date↔timestamp).

**Primary recommendation:** Slice per-command (CLI variant → command logic → tests) on top of one shared `core::input` foundation slice that ships first. For each command: pure logic functions live in the module and are `#[cfg(test)]` unit-tested for properties/round-trips; the CLI surface (flags, stdin, exit codes, ANSI gating) is integration-tested via `assert_cmd` in `tests/<cmd>.rs`; locked-layout output (cowsay bubble, color block) is snapshot-tested via `trycmd` in `tests/cmd/*.trycmd`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Argument/flag parsing | CLI (clap derive in `cli.rs`) | — | Each unit variant becomes an Args struct; consistent with Phase 1 `FlattenArgs`. |
| Input acquisition (arg vs stdin vs TTY) | `core::input` (NEW shared) | command module | One reusable helper (D-04/D-05); first consumer is base64/cowsay/epoch/color. |
| Pure transform logic | command module (`src/commands/<cmd>/`) | `core` (only `hsl` may live in core per discretion) | Business logic per RunCommand; unit-testable without a terminal. |
| Random selection | command module via `rand` | `core::input` (none) | RNG split D-08: `OsRng` for passgen, `rand::rng()` for whimsy. No shared RNG helper needed. |
| Color/ANSI gating | `core::output` (REUSE) | command module | `is_color_on()`/`truecolor` already gate once at startup; color swatch must follow this, no parallel path. |
| Terminal width | `core::output::terminal_width` (REUSE) | — | cowsay deliberately does NOT use it (fixed 40, D-11); fortune MAY use it to fit. |
| Exit-code mapping | `main.rs` (REUSE) | — | `Ok(())`→0, `Err`→1, clap parse error→2. `core::input` TTY-no-arg case must surface as exit 2 (usage error). |
| Embedded data | `src/data/*.txt` via `include_str!` (NEW) | command module | D-07; compiled into binary, no runtime file I/O, auditable. |

## Standard Stack

### Core (locked — verified on crates.io via `cargo add` resolution + slopcheck `[OK]`)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `uuid` | 1.23.3 | UUID v4 generation/formatting | `[VERIFIED: crates.io registry]` Canonical uuid-rs crate; `Display` gives RFC-4122 lowercase-hyphenated; avoids hand-rolling variant/version bit-masking (D-01). |
| `base64` | 0.22.1 | base64 encode/decode | `[VERIFIED: crates.io registry]` marshallpierce canonical crate; zero transitive deps; `STANDARD` + `URL_SAFE_NO_PAD` engines (D-01). |
| `chrono` | 0.4.45 | timestamp↔date, local TZ | `[VERIFIED: crates.io registry]` chronotope canonical crate; Windows local-timezone conversion is the trap D-01 names hand-rolling avoids. |
| `rand` | 0.9.x | CSPRNG for passgen; ThreadRng for whimsy | `[VERIFIED: crates.io registry]` rust-random canonical crate; `OsRng` (passgen) + `rng()` (whimsy) per D-08. |

### Supporting (already in Cargo.toml from Phase 1 — reuse, do not re-add)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `owo-colors` | 4.3 | truecolor swatch for `color` | `.truecolor(r,g,b)` on the `██` glyphs (D-10), gated by `is_color_on()`. |
| `crossterm` | 0.29 | `terminal_width()` | fortune fit-to-terminal (optional); NOT cowsay (D-11 fixed 40). |
| `anyhow` | 1.0 | error propagation | every `run()` returns `anyhow::Result<()>`; `.context()` at I/O boundaries. |
| `clap` | 4.6 | derive Args structs | each command's flags. |

### Alternatives Considered (all rejected by locked decisions — do NOT revisit)
| Instead of | Could Use | Tradeoff (why rejected) |
|------------|-----------|-------------------------|
| hand-rolled passgen on `rand` | `passwords` 3.1.16 | Rejected D-02: pins `rand` 0.8 (duplicate dep), drags `random-pick`/`random-integer`/bcrypt/md5, no passphrase mode. |
| hand-rolled color math | `csscolorparser` | Rejected D-03: only needed if full CSS-color input syntax becomes a feature (deferred). |
| `phf`/build-script data | `include_str!` | Rejected D-07: these need indexed random pick, not keyed lookup. |
| background-ANSI color swatch | foreground `██` swatch | Rejected D-10: background swatch becomes a blank line when ANSI-stripped, needs a banned parallel color path. |

**Installation (add the four locked crates; trim chrono defaults per D-01):**
```bash
cargo add uuid@1.23.3 --features v4
cargo add base64@0.22.1
cargo add chrono@0.4.45 --no-default-features --features clock,std   # drops oldtime/wasmbind; clock pulls iana-time-zone for Local
cargo add rand@0.9
```
> **chrono feature note** `[VERIFIED: cargo add dry-run]` default features include `oldtime` (deprecated Duration alias) and `wasmbind`. `Local` timezone conversion (needed by D-12) requires the `clock` feature, which transitively enables `iana-time-zone`. Use `--no-default-features --features clock,std` to satisfy D-01's "trim defaults" while keeping `Local`. Verify `Local` still resolves after trimming (a `cargo build` smoke is the gate).
>
> **uuid feature note** `[VERIFIED: cargo add dry-run]` `--features v4` auto-enables `rng` + `std` (and `getrandom` transitively). No extra features needed for the locked scope.

## Package Legitimacy Audit

> Ecosystem = crates.io. slopcheck 0.6.1 ran in `scan` mode against a probe `Cargo.toml`; all four returned `[OK]`.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `uuid` | crates.io | mature (years) | very high (top-50 crate) | github.com/uuid-rs/uuid | [OK] | Approved |
| `base64` | crates.io | mature (years) | very high | github.com/marshallpierce/rust-base64 | [OK] | Approved |
| `chrono` | crates.io | mature (years) | very high | github.com/chronotope/chrono | [OK] | Approved |
| `rand` | crates.io | mature (years) | very high | github.com/rust-random/rand | [OK] | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

All four are canonical ecosystem crates referenced from official docs (docs.rs) and resolve cleanly via `cargo add`; combined with slopcheck `[OK]` they qualify as `[VERIFIED: crates.io registry]`. No `postinstall`-equivalent risk exists for Rust crates in this set (no build-script network calls; `getrandom`/`uuid` build scripts only probe target cfg).

## Architecture Patterns

### System Architecture Diagram

```
                          box <cmd> [args] [--flags]   [stdin pipe?]
                                      │
                                      ▼
                        ┌──────────────────────────┐
                        │  main.rs                  │
                        │  enable_ansi_support()    │  (first line, Phase 1)
                        │  Cli::try_parse()         │──parse err──► exit 2 (stderr)
                        │  init_color(no_color)     │
                        │  match Commands::* .run() │
                        └────────────┬─────────────┘
                                     │ Ok(())→0 / Err→1
                     ┌───────────────┼────────────────────────────┐
                     ▼               ▼                             ▼
         ┌────────────────┐  ┌────────────────┐         ┌──────────────────┐
         │ arg-only cmds  │  │ input cmds      │         │ random cmds       │
         │ uuid, passgen  │  │ base64,cowsay,  │         │ fortune,8ball,    │
         │ epoch(no-arg)  │  │ epoch,color     │         │ roast             │
         └───────┬────────┘  └───────┬─────────┘         └────────┬─────────┘
                 │                    │                            │
                 │            ┌───────▼─────────┐                  │
                 │            │ core::input     │                  │
                 │            │ read_input /    │                  │
                 │            │ read_input_bytes│                  │
                 │            │  (1) arg Some   │                  │
                 │            │  (2) piped→stdin│                  │
                 │            │  (3) TTY→exit 2 │                  │
                 │            └───────┬─────────┘                  │
                 ▼                    ▼                            ▼
         ┌────────────────┐  ┌────────────────┐         ┌──────────────────┐
         │ rand::OsRng    │  │ pure transform │         │ rand::rng()       │
         │ (unbiased pick)│  │ (encode/decode,│         │ IndexedRandom::   │
         │ → password     │  │  parse, wrap)  │         │   choose          │
         └───────┬────────┘  └───────┬────────┘         └────────┬─────────┘
                 │                    │                           │
                 ▼                    ▼                           ▼
            stdout (data)        stdout (data)              stdout (data)
            errors → stderr      errors → stderr            errors → stderr
                                 color swatch gated by core::output::is_color_on()
```

### Recommended Project Structure
```
src/
├── core/
│   ├── input.rs           # NEW: read_input / read_input_bytes (D-04/D-05); registered in core/mod.rs
│   └── output.rs          # REUSE: is_color_on, terminal_width, truncate_middle, init_color
├── data/                  # NEW (name = discretion): include_str! assets
│   ├── eff_large_wordlist.txt   # 7776 words, dice codes stripped (or strip at load)
│   ├── fortunes.txt             # ~50-150 CC0 aphorisms, one per line (or blank-line delimited)
│   └── roasts.txt               # ~30-80 CC0 one-liners, one per line
└── commands/
    ├── uuid/mod.rs        # UuidArgs { count: u8, upper: bool }
    ├── base64/mod.rs      # Base64Args { decode: bool, url_safe: bool, input: Option<String> }
    ├── epoch/mod.rs       # EpochArgs { value: Option<String> }
    ├── color/mod.rs       # ColorArgs { input: Option<String> }  (+ hsl/parse pure fns)
    ├── passgen/mod.rs     # PassgenArgs { length, count, words, no_symbols, ... }
    ├── cowsay/mod.rs      # CowsayArgs { text: Option<String>, width: usize }
    ├── fortune/mod.rs
    ├── eight_ball/mod.rs  # NOTE: module is eight_ball; CLI name stays "8ball" via #[command(name)]
    └── roast/mod.rs
```

### Pattern 1: Command module = Args struct + pure functions + cfg(test) unit tests
**What:** Each command's `mod.rs` holds the clap `Args` struct (implements `RunCommand`), the pure transform/format functions, and a `#[cfg(test)] mod tests`. `run()` is a thin orchestrator: acquire input → call pure fn → print to stdout.
**When to use:** Every command. Keeps logic terminal-free and unit-testable (mirrors `flatten`'s `human_size`/`collision_reason` unit tests).
**Example:**
```rust
// Source: pattern derived from src/commands/flatten/mod.rs (existing repo convention)
use clap::Args;
use crate::commands::RunCommand;

#[derive(Debug, Args)]
pub struct UuidArgs {
    /// Number of UUIDs to generate.
    #[arg(short = 'n', long = "count", default_value_t = 1)]
    pub count: u8,
    /// Print UUIDs in uppercase.
    #[arg(long)]
    pub upper: bool,
}

impl RunCommand for UuidArgs {
    fn run(self) -> anyhow::Result<()> {
        for _ in 0..self.count {
            let u = uuid::Uuid::new_v4().to_string();   // lowercase hyphenated (RFC-4122)
            println!("{}", if self.upper { u.to_uppercase() } else { u });
        }
        Ok(())
    }
}
```

### Pattern 2: `core::input` shared reader with TTY no-block guard (D-04/D-05)
**What:** One helper consulted by base64/cowsay/epoch/color. Mirrors the `std::io::IsTerminal` gate already in `core::output`.
**When to use:** Any command that accepts "arg or piped stdin." Designed so a future `--file PATH` (Phase 3) slots in without reshaping.
**Example:**
```rust
// Source: pattern mirrors std::io::IsTerminal use in src/core/output.rs:14,43
use std::io::{IsTerminal, Read};
use anyhow::bail;

/// UTF-8 text input (cowsay, epoch, color).
pub fn read_input(arg: Option<String>) -> anyhow::Result<String> {
    match arg.as_deref() {
        Some(s) if s != "-" => Ok(s.to_string()),
        _ => {
            if std::io::stdin().is_terminal() {
                // D-04 branch 3: never block on an interactive TTY.
                bail!("no input: pass an argument or pipe data");  // → exit 2 (see note)
            }
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Binary-exact input (base64). read_to_end, no UTF-8 validation (D-05/D-06).
pub fn read_input_bytes(arg: Option<String>) -> anyhow::Result<Vec<u8>> {
    match arg.as_deref() {
        Some(s) if s != "-" => Ok(s.as_bytes().to_vec()),
        _ => {
            if std::io::stdin().is_terminal() {
                bail!("no input: pass an argument or pipe data");
            }
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            Ok(buf)
        }
    }
}
```
> ⚠️ **Exit-code landmine (D-04 branch 3).** The current `main.rs` maps every `Err` to exit **1**. A "no input on TTY" condition is a **usage** error and D-04 says it must be exit **2**. `anyhow::bail!` → exit 1 under the current mapping. To get exit 2, the planner must choose one: (a) a typed `BoxError::Usage`/`MissingInput` variant that `main.rs` maps to 2 (mirrors how `NotImplemented`→1 is handled), or (b) the command detects the TTY case and returns a sentinel main.rs recognizes. **Option (a) is recommended** — it is the existing pattern (`core::errors::BoxError`) and keeps `main.rs` the single owner of exit codes. Whichever is chosen, an integration test must assert exit 2 (see Validation Architecture).

### Pattern 3: Unbiased random selection (D-02/D-08) — the rand 0.9 API
**What:** `IndexedRandom::choose` returns `Option<&T>` and is uniform (no modulo bias). `random_range` (NOT `gen_range` — renamed in 0.9) for indices/lengths.
**When to use:** Every random pick. NEVER `slice[rng.next_u32() as usize % slice.len()]`.
**Example:**
```rust
// Source: docs.rs/rand/0.9.0 — IndexedRandom::choose, Rng::random_range
use rand::seq::IndexedRandom;   // brings .choose() onto slices
use rand::Rng;                  // brings .random_range()

// whimsy commands (fortune/8ball/roast): OS-seeded ThreadRng
let mut rng = rand::rng();                       // 0.9: replaces thread_rng()
let answer: &str = ANSWERS.choose(&mut rng).expect("non-empty");

// random length within a range (if a feature needs it)
let n: usize = rng.random_range(1..=10);         // 0.9: replaces gen_range()
```

### Pattern 4: OsRng for passgen (D-08) — the non-obvious wrapper
**What:** In rand 0.9, `OsRng` implements `TryRngCore`/`TryCryptoRng`, **NOT** `Rng`. You cannot call `.choose()`/`.random_range()` on it directly. Wrap it.
**When to use:** passgen only (CSPRNG-grade requirement).
**Example:**
```rust
// Source: docs.rs/rand_core/0.9 TryRngCore::unwrap_err; docs.rs/rand/0.9 rngs::OsRng
use rand::rngs::OsRng;
use rand_core::TryRngCore;        // brings .unwrap_err()
use rand::seq::IndexedRandom;
use rand::Rng;

// Wrap the fallible OS RNG into an infallible Rng-implementing adapter.
let mut rng = OsRng.unwrap_err();           // UnwrapErr<OsRng>: impls RngCore → Rng
let ch: &char = CHARSET.choose(&mut rng).expect("non-empty charset");
let idx: usize = rng.random_range(0..WORDLIST.len());
```
> **Two valid implementations of D-08's "OsRng for passgen":**
> 1. `OsRng.unwrap_err()` — wraps the raw OS CSPRNG; every draw hits the OS. Simplest, matches D-08 literally.
> 2. `rand::rngs::StdRng::from_os_rng()` — a ChaCha12 CSPRNG seeded once from the OS; faster for many draws, still CSPRNG-grade.
> Both satisfy "CSPRNG-grade." (1) is the most literal reading of D-08 ("`OsRng` ... for passgen"). The planner should pick one and note it; `rand_core` may need adding to `Cargo.toml` if option (1)'s `unwrap_err()` import is used (it re-exports through `rand` in 0.9 — verify `use rand::TryRngCore` resolves before adding `rand_core` separately).

### Pattern 5: base64 0.22 engine API (D-01/D-05)
**What:** Engine-object API; `STANDARD` and `URL_SAFE_NO_PAD` are real constants. `URL_SAFE_NO_PAD` avoids the `=` padding ambiguity D-01 implies for URL-safe.
**Example:**
```rust
// Source: docs.rs/base64/0.22.1 — Engine trait + general_purpose constants (both verified to exist)
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

let engine = if url_safe { &URL_SAFE_NO_PAD } else { &STANDARD };
let encoded: String = engine.encode(&bytes);          // no line-wrapping by default (B64-01)
let decoded: Vec<u8> = engine.decode(input.trim())?;  // trim trailing newline from piped input
```
> ⚠️ **Decode whitespace:** piped input (especially under PS7) may carry a trailing newline. `engine.decode` rejects non-alphabet bytes, so `.trim()` the input string (or strip trailing `\r\n`) before decoding, or the round-trip test will fail on the newline. This is the classic "echo adds EOL" base64 footgun.

### Pattern 6: chrono epoch parsing across the 3 modes (D-12)
**Example:**
```rust
// Source: docs.rs/chrono/0.4.45 — verified method signatures
use chrono::{DateTime, Utc, Local, NaiveDateTime, NaiveDate, TimeZone};

// no-arg → current Unix timestamp
let now = Utc::now().timestamp();

// integer arg → local + UTC human date
let dt_utc = DateTime::from_timestamp(secs, 0).ok_or_else(|| anyhow::anyhow!("timestamp out of range"))?;
let dt_local = dt_utc.with_timezone(&Local);

// date string → timestamp (try formats in order; first match wins)
// 1) RFC3339 / ISO8601 with offset/Z
if let Ok(dt) = DateTime::parse_from_rfc3339(s) { return Ok(dt.timestamp()); }
// 2) "YYYY-MM-DD HH:MM:SS" as LOCAL
if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
    return Ok(Local.from_local_datetime(&naive).single()
        .ok_or_else(|| anyhow::anyhow!("ambiguous local time"))?.timestamp());
}
// 3) "YYYY-MM-DD" as LOCAL midnight
if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
    let naive = d.and_hms_opt(0,0,0).unwrap();
    return Ok(Local.from_local_datetime(&naive).single()
        .ok_or_else(|| anyhow::anyhow!("ambiguous local time"))?.timestamp());
}
anyhow::bail!("unrecognized date '{s}'; expected RFC3339, YYYY-MM-DD, or YYYY-MM-DD HH:MM:SS");
```
> **DST ambiguity:** `from_local_datetime` returns `LocalResult` which can be `None` (nonexistent during spring-forward) or `Ambiguous` (during fall-back). `.single()` collapses to an error in those cases — handle it rather than `.unwrap()` so a spring-forward time doesn't panic.

### Anti-Patterns to Avoid
- **`slice[n % slice.len()]` for random pick** — modulo bias (D-02 explicitly bans). Use `IndexedRandom::choose`.
- **`gen_range`/`thread_rng()`** — deprecated/renamed in rand 0.9. Use `random_range`/`rand::rng()`.
- **Calling `.choose()`/`.random_range()` on bare `OsRng`** — won't compile in 0.9 (`OsRng` is `TryRngCore`, not `Rng`). Wrap with `.unwrap_err()` or use `StdRng::from_os_rng()`.
- **A fixed/manual seed anywhere** (D-08) — kills the "different each run" property and any seeded test would mask real bias. Tests assert *properties*, not seeded values.
- **`read_to_string` for base64** — destroys binary bytes / errors on non-UTF-8. Use `read_to_end`→`Vec<u8>` (D-05/D-06).
- **A background-ANSI color swatch** — D-10 rejects it (blank line when stripped).
- **`terminal_width()` for cowsay** — D-11 fixes 40 cols so pipe-vs-TTY output is reproducible.
- **Mapping the TTY-no-input case to exit 1** — D-04 requires exit 2 (usage error). See Pattern 2 landmine.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID v4 bit-masking | manual variant/version bits | `uuid` 1.23.3 | RFC-4122 variant/version masking is easy to botch (D-01). |
| base64 alphabet/padding | custom encode tables | `base64` 0.22.1 engines | URL-safe alphabet + NO_PAD edge cases (D-01). |
| local-timezone conversion | manual TZ offset math | `chrono` 0.4.45 `Local` | Windows local TZ + DST is the trap D-01 names. |
| uniform random pick | `% len` indexing | `IndexedRandom::choose` | modulo bias (D-02). |
| OS CSPRNG | reading `/dev/urandom`/Win API yourself | `rand` `OsRng` (getrandom→ProcessPrng) | platform CSPRNG plumbing is solved (D-08). |

**Hand-roll (per locked D-01/D-02/D-03) — these ARE the right call:** passgen sampling loop, color hex/RGB/HSL math, cowsay greedy wrap + cow art, fortune/8ball/roast lists. The value there is formatted output / bundled content, not algorithmic risk.

**Key insight:** This phase deliberately hand-rolls the *presentation/content* layer and crate-delegates the *correctness-critical* layer (crypto RNG, RFC formats, TZ math). The test strategy mirrors this split: crate-backed pieces get round-trip/format tests; hand-rolled pieces get property + snapshot tests.

## Common Pitfalls

### Pitfall 1: rand 0.8→0.9 API drift (the single biggest implementation risk)
**What goes wrong:** Code written from training memory uses `thread_rng()`, `gen_range`, or calls `.choose()` on `OsRng` — none compile against rand 0.9.
**Why it happens:** rand 0.9 renamed `thread_rng()`→`rng()`, `gen_range`→`random_range`, and split RNG traits (`OsRng` is now `TryRngCore`/`TryCryptoRng`, not `Rng`).
**How to avoid:** Use `rand::rng()`, `random_range`, `IndexedRandom::choose`, and `OsRng.unwrap_err()` (or `StdRng::from_os_rng()`). All verified against docs.rs/rand/0.9.0 in this research.
**Warning signs:** `method not found on OsRng`, `gen_range deprecated`, `cannot find function thread_rng`.

### Pitfall 2: TTY-no-input maps to exit 1 instead of 2
**What goes wrong:** `core::input` bails when no arg + interactive TTY, but `main.rs` maps `Err`→1; D-04 requires exit 2.
**Why it happens:** `anyhow::Error` carries no exit-code intent; `main.rs`'s single mapping flattens to 1.
**How to avoid:** Add a typed `BoxError::MissingInput`/`Usage` variant mapped to 2 in `main.rs` (Pattern 2). Test it (Validation Architecture row).
**Warning signs:** integration test expecting `.code(2)` for the no-input case fails with code 1.

### Pitfall 3: base64 round-trip fails on a trailing newline
**What goes wrong:** Piped input carries `\n` (or `\r\n` under PS7); `decode` rejects it; the encode→decode test fails.
**Why it happens:** stdin readers capture the terminal newline; base64 alphabet excludes whitespace.
**How to avoid:** `.trim()`/strip trailing CR/LF before decode. For the encode path, decide whether to encode the trailing newline (echo includes it) — tests must pass bytes explicitly (`write_stdin(b"abc")`, no newline) to be deterministic.
**Warning signs:** `InvalidByte` decode error on otherwise-valid base64.

### Pitfall 4: PS7 UTF-16 pipe re-encoding is NOT reproduced by assert_cmd (D-06)
**What goes wrong:** `assert_cmd`'s `write_stdin` writes raw bytes straight to the child's stdin pipe — it does NOT route through PowerShell, so the test passes even though a real PS7 `… | box base64` may have re-encoded the bytes through UTF-16. The test gives false confidence that the *PS7 pipe* is byte-exact.
**Why it happens:** `assert_cmd` is a direct process spawn, bypassing any shell. The D-06 caveat is about the *shell's* pipeline, not the program's stdin reader.
**How to avoid:** Test the *Rust-side* `read_input_bytes` path byte-exactly via `write_stdin` (this is real and valuable — it proves `read_to_end` doesn't mangle bytes). Document explicitly that the PS7-pipeline byte-exactness is a **manual/human-verify** check (or deferred to the `--file PATH` Phase 3 path which sidesteps the shell). Do NOT claim the automated test covers the PS7 pipe.
**Warning signs:** a reviewer assumes "stdin test passes" ⇒ "PS7 pipe is safe" — it does not.

### Pitfall 5: cowsay hard-break math off-by-one
**What goes wrong:** A word longer than `width` must hard-break (D-11); naive greedy wrap either loops forever or produces a line wider than the bubble, breaking the border alignment.
**Why it happens:** greedy word-wrap assumes every word fits; the hard-break branch is the edge case.
**How to avoid:** When `word.len() > width`, split the word into `width`-sized chunks before wrapping. Unit-test with a word longer than the width and assert no output line exceeds `width`.
**Warning signs:** bubble top/bottom border length ≠ longest content line; a panic on `width=0`.

### Pitfall 6: include_str! path is relative to the SOURCE FILE, not the crate root
**What goes wrong:** `include_str!("../../data/fortunes.txt")` resolves relative to the `.rs` file doing the include, not `CARGO_MANIFEST_DIR`. Wrong relative depth → compile error.
**Why it happens:** `include_str!` is a compiler macro keyed on the current file's path.
**How to avoid:** Count directory levels from the including module to `src/data/`. From `src/commands/fortune/mod.rs` to `src/data/fortunes.txt` is `include_str!("../../data/fortunes.txt")`. Confirm with a trivial compile. Alternative: include from a single `src/data/mod.rs` and re-export, so the relative path lives in one place.
**Warning signs:** `couldn't read ... No such file or directory` at compile time.

### Pitfall 7: EFF wordlist has dice-code prefixes
**What goes wrong:** `eff_large_wordlist.txt` lines are `11111\tabacus` (5-digit dice code, TAB, word). Selecting a raw line yields the code too.
**Why it happens:** the canonical EFF file is Diceware-formatted.
**How to avoid:** strip the prefix at load: `line.rsplit('\t').next()` (or `split_whitespace().last()`). Unit-test that the parsed list has 7776 entries and no entry contains a digit/tab.
**Warning signs:** passphrase words like `11111abacus`.

### Pitfall 8: color short-hex `#abc` expansion (D-13)
**What goes wrong:** `#abc` must expand to `#aabbcc` (each nibble doubled), not be parsed as a 3-byte value.
**How to avoid:** detect length-3 (or length-4 with `#`) hex and expand each char: `r = c0c0, g = c1c1, b = c2c2`. Unit-test `#abc` → `(170,187,204)`.
**Warning signs:** `#abc` produces a wrong/garbage color.

## Code Examples

### Color HSL round-trip (D-03/D-10 requires unit-testing this)
```rust
// Source: standard closed-form RGB↔HSL (no crate); verified against CSS Color Module Level 3 formulas
/// RGB (0-255) → HSL (h:0-360, s:0-100, l:0-100), integer-rounded for display.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let (rf, gf, bf) = (r as f64/255.0, g as f64/255.0, b as f64/255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = d / (1.0 - (2.0*l - 1.0).abs());
        let h = if max == rf { ((gf - bf)/d).rem_euclid(6.0) }
                else if max == gf { (bf - rf)/d + 2.0 }
                else { (rf - gf)/d + 4.0 };
        (h * 60.0, s)
    };
    (h.round() as u16 % 360, (s*100.0).round() as u8, (l*100.0).round() as u8)
}
// Unit test (D-10): assert rgb_to_hsl(59,130,246) ≈ (217, 91, 60) within ±1 rounding.
```

### Color swatch gated by Phase-1 color decision (D-10, reuse — no parallel path)
```rust
// Source: src/core/output.rs is_color_on() pattern (existing repo gate)
use owo_colors::OwoColorize;
use crate::core::output::is_color_on;

let swatch = "██████████";
if is_color_on() {
    println!("{}", swatch.truecolor(r, g, b));   // owo-colors global override agrees with COLOR_ON
} else {
    println!("{swatch}");                          // byte-identical minus ANSI (D-10)
}
```

## Runtime State Inventory

> Not a rename/refactor phase. The only "swap" is replacing 9 unit `Commands::*` variants with Args structs and removing 9 `not_implemented(...)` arms — pure source edits, no runtime/stored/OS state involved. No data migration. (Verified: the `box` binary stores nothing, registers nothing, and reads no external config; all command behavior is process-local.)

## Validation Architecture

> Nyquist validation is ENABLED (`workflow.nyquist_validation: true`). This section drives VALIDATION.md and the per-plan test strategy. It is the highest-value output of this research.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + `assert_cmd` 2.2 (integration) + `trycmd` 1.2 (CLI snapshots) + `predicates` 3.1; `insta` 1.48 available but trycmd is the established snapshot path |
| Config file | none — Cargo conventions; integration tests in `tests/`, unit tests `#[cfg(test)]` in each module |
| Quick run command | `cargo test --test <cmd>` (one command's integration tests) or `cargo test <module>` (unit) |
| Full suite command | `cargo test` (all unit + integration; then `cargo clippy -- -D warnings` and `cargo fmt --check` as the Phase-1 quality gate) |

### The three test shapes for this phase (decision rules)
1. **Unit test (`#[cfg(test)]` in module)** — pure functions: HSL round-trip, hex/RGB parse incl. `#abc` expansion, cowsay wrap/hard-break, EFF list parse (7776, no digits), passgen charset membership, epoch format parsing. Deterministic, terminal-free, fast.
2. **Integration test (`assert_cmd` in `tests/<cmd>.rs`)** — CLI surface: flags, stdin path, exit codes, ANSI gating, stdout-only/stderr-only discipline, non-determinism properties (membership + two-runs-differ). Black-box against the compiled binary.
3. **trycmd snapshot (`tests/cmd/*.trycmd`)** — locked exact layouts: cowsay single/multi-line bubble, color block. Only for **deterministic** output (NEVER for random commands — they'd flake). Force `NO_COLOR=1` (or rely on piped non-TTY) so snapshots have no ANSI.

### Testing non-determinism (fortune/8ball/roast/uuid/passgen) — anti-flake patterns
Each `box` run is a fresh OS-seeded process (D-08, no fixed seed), so the discipline is **properties, not values**:

- **Membership** — assert output ∈ a known set. For 8ball, the 20 answers are a `const &[&str]`; a unit test can `assert!(EIGHT_BALL_ANSWERS.contains(&output_line))`. For fortune/roast, expose the parsed list (e.g. `pub(crate) fn entries() -> &'static [&str]`) and assert membership in unit tests; integration tests assert output is non-empty + single record.
- **Two-runs-differ (probabilistic, made robust)** — running once and comparing two outputs can collide by chance (1/20 for 8ball). To keep it deterministic-enough: run the command **N times and assert ≥2 distinct values** appear (with N=10, P(all identical) = (1/20)^9 ≈ 2e-12 for 8ball; for 50+ fortunes it's astronomically small). This converts "differ" into a near-certain property without a seed.
  ```rust
  // Source: assert_cmd pattern (docs.rs/assert_cmd) + statistical robustness
  use std::collections::HashSet;
  use assert_cmd::Command;
  #[test]
  fn eight_ball_varies_across_runs() {
      let mut seen = HashSet::new();
      for _ in 0..10 {
          let out = Command::cargo_bin("box").unwrap()
              .args(["8ball", "will it work?"]).output().unwrap();
          seen.insert(String::from_utf8_lossy(&out.stdout).trim().to_string());
      }
      assert!(seen.len() >= 2, "8ball must vary across runs (got {seen:?})");
  }
  ```
- **uuid uniqueness** — `box uuid -n 100` must yield 100 distinct lines: parse stdout, collect into a `HashSet`, assert `len()==100` AND each matches the v4 regex `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` (note the version nibble `4` and variant nibble `[89ab]`). `--upper` → assert uppercase variant.
- **passgen variation** — `box passgen --count 10` must yield 10 distinct passwords; assert `HashSet::len()==10`. Collision probability for 16-char passwords is negligible.

### Crypto-sampling correctness (passgen, D-02/D-14) — what is testable vs review-only
| Property | Testable? | How |
|----------|-----------|-----|
| Length = 16 default; `--length N` honored | YES (unit + integration) | assert each password `.chars().count()==expected`. |
| Charset = curated four-class by default | YES (unit) | assert every char ∈ the curated allowed set; assert ≥1 of each class is *possible* (not guaranteed per-password). |
| `--no-symbols` excludes symbols | YES (integration) | generate many, assert NO char ∈ symbol set across a large sample. |
| Curated symbols are paste-safe (no shell-hostile chars) | YES (unit) | assert the symbol constant excludes `` ` ' " $ \ ; | & < > ( ) `` and space. |
| `--words N` passphrase has N words from EFF list | YES (unit + integration) | split on the separator, assert `len()==N`, assert each word ∈ parsed EFF list. |
| Entropy = ~12.9 bits/word → `--words 4` ≈ 51.6 bits | PARTIAL | assert wordlist length == 7776 (⇒ log2(7776)=12.925 bits/word is a property of the list, so N words = N·12.925); the *math* is a unit test on the list size, the per-password draw is review-only. |
| **No modulo bias** (D-02 core requirement) | **REVIEW-ONLY (primary) + statistical (secondary)** | The selection uses `IndexedRandom::choose`/`random_range` (uniform by construction) — verify by **code review** that no `% len` appears. A statistical chi-square test over a huge sample *could* detect gross bias but is flaky and slow; treat it as optional, not a gate. The real guarantee is "we used the unbiased API," provable by inspection. |
| Uses `OsRng` (CSPRNG), not ThreadRng | REVIEW-ONLY | runtime output is indistinguishable; assert by code review that passgen constructs `OsRng`-backed RNG (D-08). |

> **Key validation insight:** "unbiased CSPRNG selection" is fundamentally a **construction guarantee** (we called the unbiased, OS-backed API), not an output assertion. The automated tests prove *structural* properties (length, charset, count, word membership, wordlist size); the *bias-freedom* and *CSPRNG-source* are locked by code review (grep for `% ` near selection; grep for `OsRng`). Document this split in VALIDATION.md so a reviewer doesn't expect a flaky statistical test to be the gate.

### Round-trip / property tests (deterministic commands)
| Command | Round-trip | Test |
|---------|-----------|------|
| base64 | encode(bytes) → decode → bytes (binary-exact) | unit: random `Vec<u8>` incl. non-UTF-8/zero bytes → `decode(encode(x))==x` for both STANDARD and URL_SAFE_NO_PAD. integration: `write_stdin(b"\x00\xff..")` → encode → pipe back → `--decode` → assert original bytes. |
| color | RGB → HSL → (display only; HSL→RGB optional) | unit (D-10): `rgb_to_hsl(59,130,246) ≈ (217,91,60)` ±1; `#abc`→(170,187,204); `"59,130,246"` and `"59 130 246"` both parse to (59,130,246). |
| epoch | timestamp → date-string → timestamp (each of 3 modes) | unit: `parse(format(ts)) == ts` for RFC3339, `YYYY-MM-DD HH:MM:SS` (local), `YYYY-MM-DD` (local midnight). integration: no-arg prints an integer ≈ now; integer arg prints both local+UTC lines. |

### Windows PS7 stdin byte-exactness (D-06) — explicit scope boundary
- **Automated (assert_cmd `write_stdin`)** proves the **Rust-side** reader (`read_input_bytes`→`read_to_end`) is byte-exact. `write_stdin` accepts `impl Into<Vec<u8>>` and writes raw bytes directly to the child stdin pipe — it does **NOT** go through PowerShell, so it tests the program, not the shell.
- **PS7 pipeline UTF-16 re-encoding** (the actual D-06 caveat) is **out of automated scope** — it's a property of the *shell*, not the binary, and `assert_cmd` bypasses the shell. Mark this as a **manual/human-verify** smoke (`Get-Content -AsByteStream file | box base64` round-trip in a real PS7) OR note it's sidestepped by the deferred `--file PATH` path (Phase 3). VALIDATION.md must NOT claim the automated test covers the PS7 pipe.

### `core::input` TTY no-block branch (D-04 branch 3) — testing without a real TTY
- The hang-prevention guard fires when `stdin().is_terminal()` is true AND no arg is given. In `assert_cmd`/CI, stdin is a pipe (not a TTY), so the guard does NOT naturally fire — you can't easily simulate an interactive TTY.
- **Test the reachable, important contract instead:** run the command with **no arg and stdin closed/empty-but-piped** is the *piped* branch (reads empty → command-specific behavior). To exercise branch 3's exit-2 path deterministically, the cleanest approach is a **unit test of the input function's decision** by injecting the `is_terminal` result (refactor `read_input` to take an `is_tty: bool` + a reader, with the public wrapper supplying the real ones). Then unit-assert: `read_input_inner(None, is_tty=true, ...)` returns the MissingInput error. The integration test asserts the *piped-empty* and *arg-given* branches; the TTY branch is a unit test on the decision logic.
- This sidesteps the "can't fake a TTY in CI" problem while still proving the no-hang contract and the exit-2 mapping (combined with a `main.rs` test that the MissingInput error → code 2).

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UUID-01 | v4 format, lowercase, one/line, `-n N` count, uniqueness, `--upper` | integration | `cargo test --test uuid` | ❌ Wave 0 |
| B64-01 | encode/decode round-trip (bytes), `--url-safe`, no wrap, stdin path | unit + integration | `cargo test --test base64` ; `cargo test base64::` | ❌ Wave 0 |
| EPOC-01 | 3 modes round-trip, no-arg=now, integer=local+UTC | unit + integration | `cargo test --test epoch` ; `cargo test epoch::` | ❌ Wave 0 |
| COLR-01 | hex/RGB parse + `#abc`, HSL round-trip, swatch ANSI-gated | unit + trycmd | `cargo test color::` ; `cargo test --test cli` (trycmd) | ❌ Wave 0 |
| PASS-01 | length, curated charset, `--no-symbols`, `--words N`, `--count N` distinct, stdout-only | unit + integration | `cargo test --test passgen` ; `cargo test passgen::` | ❌ Wave 0 |
| COW-01 | single/multi-line bubble, hard-break, arg+stdin | unit + trycmd | `cargo test cowsay::` ; trycmd snapshot | ❌ Wave 0 |
| FORT-01 | output ∈ list, varies across N runs, non-empty | integration | `cargo test --test fortune` | ❌ Wave 0 |
| 8BAL-01 | output ∈ the 20, question optional, varies across N runs | unit + integration | `cargo test eight_ball::` ; `cargo test --test eight_ball` | ❌ Wave 0 |
| ROST-01 | output ∈ list, varies across N runs | integration | `cargo test --test roast` | ❌ Wave 0 |
| (shared) | `core::input` 3 branches; TTY-no-arg → exit 2 | unit + integration | `cargo test input::` ; per-command no-input integration | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test <module>::` for the command touched + `cargo clippy -- -D warnings` (Phase-1 gate stays).
- **Per wave merge (slice):** `cargo test --test <cmd>` (that command's integration suite) + `cargo fmt --check`.
- **Phase gate:** full `cargo test` green + `cargo clippy -- -D warnings` clean + `cargo fmt --check` clean before `/gsd:verify-work`. The existing `tests/cli.rs` trycmd `help.trycmd` snapshot will need updating (the `--help` listing text is unchanged, but if any `about` doc-comment changes when swapping a variant, the snapshot drifts — re-run `TRYCMD=overwrite cargo test` and review the diff).

### Wave 0 Gaps
- [ ] `tests/uuid.rs` — UUID-01 (format regex, count, uniqueness, `--upper`)
- [ ] `tests/base64.rs` — B64-01 (round-trip bytes, url-safe, stdin)
- [ ] `tests/epoch.rs` — EPOC-01 (3 modes, no-arg, integer)
- [ ] `tests/passgen.rs` — PASS-01 (length, charset, no-symbols, words, count distinct, stdout-only)
- [ ] `tests/fortune.rs`, `tests/eight_ball.rs`, `tests/roast.rs` — membership + varies-across-runs
- [ ] `tests/cowsay.rs` + `tests/cmd/cowsay*.trycmd` — bubble layouts, hard-break (unit) + snapshot
- [ ] `tests/cmd/color*.trycmd` — locked color block layout (NO_COLOR) snapshot; color parse/HSL is unit-tested in-module
- [ ] `core::input` unit tests (decision logic, injected `is_tty`) — likely in `src/core/input.rs` `#[cfg(test)]`
- [ ] Framework install: none — `assert_cmd`/`predicates`/`trycmd`/`insta`/`assert_fs`/`tempfile` already in `[dev-dependencies]`.
- [ ] Add `rand_core` to deps **only if** `OsRng.unwrap_err()` import (`rand::TryRngCore`) does not resolve through `rand` 0.9's re-exports (verify first).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `rand::thread_rng()` | `rand::rng()` | rand 0.9.0 | whimsy RNG construction. |
| `Rng::gen_range(a..b)` | `Rng::random_range(a..b)` | rand 0.9.0 | range draws (deprecated alias still compiles with warning). |
| `OsRng: Rng` (callable directly) | `OsRng: TryRngCore` → wrap with `.unwrap_err()` | rand 0.9.0 / rand_core 0.9 | passgen MUST wrap OsRng. |
| `SliceRandom::choose` | `IndexedRandom::choose` | rand 0.9.0 | trait renamed; import `rand::seq::IndexedRandom`. |
| getrandom Windows = RtlGenRandom | getrandom 0.3 Windows = `ProcessPrng` | getrandom 0.3 (pulled by rand 0.9) | OS CSPRNG backend; confirms D-08's "Windows CSPRNG" claim. |
| base64 free fns (`base64::encode`) | Engine objects (`STANDARD.encode`) | base64 0.21 | use `Engine` trait + `general_purpose` constants. |

**Deprecated/outdated:**
- `thread_rng()`, `gen_range`, `SliceRandom` (rand 0.8 names) — do not use.
- `base64::encode`/`decode` free functions (pre-0.21) — removed; use engines.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `rand::TryRngCore::unwrap_err()` is reachable through the `rand` crate's re-exports in 0.9 (so `rand_core` need not be a direct dep) | Pattern 4 / Wave 0 | LOW — if not, add `rand_core = "0.9"`; a one-line build check resolves it. The wrapper method name `unwrap_err` is `[CITED: docs.rs/rand_core/0.9]`. |
| A2 | `StdRng::from_os_rng()` exists in rand 0.9 as an alternative CSPRNG construction | Pattern 4 | LOW — option (1) `OsRng.unwrap_err()` is the verified primary path; A2 is a documented-but-secondary alternative. `[ASSUMED]` (not separately verified this session). |
| A3 | EFF `eff_large_wordlist.txt` lines are `5-digit-dice` + TAB + word | Pitfall 7 | LOW — strip-last-whitespace-token is robust regardless of exact delimiter; unit-test asserts 7776 clean entries. `[CITED: eff.org/dice]` for count/entropy; `[ASSUMED]` for exact delimiter. |
| A4 | trycmd snapshots are stable for cowsay/color under `NO_COLOR=1` on Windows (no CRLF surprise) | Validation Architecture | MEDIUM — trycmd normalizes line endings, but verify the first snapshot run; if CRLF leaks, set the snapshot's expected output accordingly or normalize in the command. |
| A5 | The `help.trycmd` snapshot text is unchanged by swapping unit variants → Args structs (doc-comment `about` stays) | Phase gate | LOW — keep the existing `///` descriptions verbatim when adding fields; if changed, regenerate the snapshot. |

**If a planner/discuss step wants zero ambiguity:** confirm A1 (one build check) and the passgen RNG choice (option 1 vs 2 in Pattern 4) before the passgen slice.

## Open Questions (RESOLVED)

> All three resolved during Phase 2 planning (2026-06-22). Resolutions are reflected in the committed plans and recorded inline below.

1. **Exit-2 mechanism for the TTY-no-input case (D-04 branch 3).**
   - What we know: D-04 mandates exit 2; `main.rs` currently maps all `Err`→1.
   - What's unclear: typed `BoxError::MissingInput` variant (recommended) vs another mechanism.
   - Recommendation: add a typed variant mapped to 2 in `main.rs` (mirrors `NotImplemented`→1). Decide in the `core::input` foundation slice.
   - **RESOLVED:** typed `BoxError::MissingInput` → exit 2, wired in the `core::input` foundation slice (Plan 02-01); proven by base64's no-input path (Plan 02-02).
2. **passgen RNG construction: `OsRng.unwrap_err()` vs `StdRng::from_os_rng()`.**
   - What we know: both are CSPRNG-grade; D-08 names `OsRng`.
   - Recommendation: option 1 (`OsRng.unwrap_err()`) — most literal D-08 reading; note it in the plan.
   - **RESOLVED:** `OsRng.unwrap_err()` (the literal D-08 reading), in the passgen slice (Plan 02-04). Assumption A1 (whether `TryRngCore` re-exports via `rand` 0.9 vs needing direct `rand_core`) is verified in that plan before use.
3. **fortune "fits the terminal" (FORT-01).**
   - What we know: D-11 fixes cowsay at 40, but fortune's "sized to fit" is unspecified.
   - Recommendation: if a fortune line exceeds `terminal_width()`, wrap it (reuse a word-wrap helper, possibly shared with cowsay); otherwise print as-is. Low risk; planner's call within discretion.
   - **RESOLVED:** soft-wrap at `terminal_width()` when a line exceeds it, else print as-is (Plan 02-05).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (cargo/rustc) | all | ✓ | rustc 1.90.0 / cargo 1.90.0 | — |
| crates.io network (cargo add/fetch) | first build | ✓ | — (registry reachable via cargo; direct HTTP API blocked but cargo index works) | vendored Cargo.lock already committed |
| `x86_64-pc-windows-msvc` target + crt-static | release build (Phase-1 contract) | ✓ (verified in Phase 1) | — | — |
| Interactive TTY (for manual D-06 PS7 pipe smoke) | manual byte-exact verify | n/a in CI | — | manual human-verify step |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** crates.io direct API is rate-limited (used cargo index + slopcheck instead — fully sufficient).

## Security Domain

> `security_enforcement` not present in config.json → treat as enabled. This phase has a genuine crypto surface (passgen) but no auth/session/network/storage.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | no auth surface. |
| V3 Session Management | no | stateless one-shot CLI. |
| V4 Access Control | no | no resources to gate. |
| V5 Input Validation | yes | epoch date parsing (reject ambiguous formats, D-12); base64 decode (reject non-alphabet bytes — the crate does this); color input parse (reject malformed hex/RGB). All errors → stderr, exit 1/2, never panic (FOUND-05). |
| V6 Cryptography | yes | passgen MUST use OS CSPRNG (`OsRng`/getrandom→ProcessPrng) and unbiased selection (`IndexedRandom::choose`/`random_range`) — never `% len`, never a seeded/predictable RNG (D-02/D-08). Do NOT hand-roll the RNG. |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Predictable passwords (weak/seeded RNG) | Information Disclosure | `OsRng` CSPRNG; no fixed seed (D-08); code-review gate. |
| Modulo bias skewing charset distribution | Information Disclosure | `IndexedRandom::choose`/`random_range` (D-02); review for `% len`. |
| Password leaking via stdout to a log/pipe unintentionally | Information Disclosure | passgen output is stdout-only by design (D-14); no `--clip` in v1; do not log passwords to stderr. |
| Shell-injection-hostile passwords breaking when pasted | (usability/safety) | curated paste-safe symbol set excludes `` ` ' " $ \ ; | & `` etc. (D-14). |
| Panic on malformed input (epoch/color/base64) | Denial of Service (self) | `Result`-everywhere, `.context()`, no `.unwrap()` on user input (FOUND-05). |

## Sources

### Primary (HIGH confidence)
- docs.rs/rand/0.9.0 — `IndexedRandom::choose` (returns `Option<&T>`, import `rand::seq::IndexedRandom`); `Rng::random_range` (renamed from `gen_range`); `rand::rng()` (replaces `thread_rng()`); `rngs::OsRng` (zero-sized, implements `TryRngCore`/`TryCryptoRng`, NOT `Rng`).
- docs.rs/rand_core/0.9.0 — `TryRngCore::unwrap_err()` → `UnwrapErr<Self>` (the infallible wrapper for OsRng).
- docs.rs/base64/0.22.1 — `Engine` trait; `engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD}` all confirmed; `engine.encode/decode` dot-call.
- docs.rs/chrono/0.4.45 — `DateTime::parse_from_rfc3339`, `DateTime::from_timestamp`, `NaiveDateTime::parse_from_str`, `NaiveDate::parse_from_str`, `Local`/`Utc`, `Utc::now().timestamp()`.
- `cargo add --dry-run` (this session) — uuid 1.23.3 (`v4`→`rng,std`), base64 0.22.1, chrono 0.4.45 (defaults incl. `oldtime`), rand 0.9 (`os_rng,thread_rng,std_rng`) all resolve on crates.io.
- slopcheck 0.6.1 `scan` (crates.io ecosystem) — uuid/base64/chrono/rand all `[OK]`.
- Existing repo: `src/core/output.rs`, `src/main.rs`, `src/cli.rs`, `src/commands/{mod,stub,flatten}.rs`, `tests/cli.rs`, `tests/flatten.rs`, `tests/cmd/help.trycmd`, `Cargo.toml` — verified all symbols referenced by CONTEXT.md still exist (`is_color_on` output.rs:32, `terminal_width` output.rs:195, `truncate_middle` output.rs:174, `init_color` output.rs:52, `RunCommand` commands/mod.rs:13, `#[command(name="8ball")]` cli.rs:52, `not_implemented` arms main.rs:62-83).
- eff.org/dice + eff.org/deeplinks/2016/07 — EFF Large Wordlist: 7776 words (6^5), ~12.9 bits/word, CC-BY 3.0 US.

### Secondary (MEDIUM confidence)
- docs.rs/assert_cmd — `write_stdin` (accepts bytes; writes raw to child pipe, bypasses shell); `output()`/`assert()`.
- GitHub rust-random/getrandom#414 + Wikipedia CryptGenRandom — Windows getrandom 0.3 backend = `ProcessPrng` (modern CNG CSPRNG).

### Tertiary (LOW confidence)
- `StdRng::from_os_rng()` as an alternative passgen RNG construction — `[ASSUMED]`, not separately verified this session (A2).
- Exact EFF wordlist line delimiter (dice-code + TAB) — `[ASSUMED]`; mitigated by strip-last-token + a 7776-count unit test (A3).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions locked + verified via cargo resolution + slopcheck; APIs verified on docs.rs.
- Architecture: HIGH — reuses Phase-1 patterns confirmed present in source; `core::input` design is a direct mirror of the existing `IsTerminal` gate.
- Pitfalls: HIGH for the rand 0.9 drift and base64/chrono APIs (doc-verified); MEDIUM for the trycmd-on-Windows CRLF and PS7-pipe boundary (testable assumptions flagged).
- Validation: HIGH — strategy follows the existing repo test conventions and the established non-determinism anti-flake patterns.

**Research date:** 2026-06-22
**Valid until:** 2026-07-22 (stable stack; rand/base64/chrono/uuid are mature and slow-moving)
