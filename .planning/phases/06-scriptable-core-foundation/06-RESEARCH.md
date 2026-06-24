# Phase 6: Scriptable-Core Foundation - Research

**Researched:** 2026-06-25
**Domain:** Cross-cutting CLI spine integration (global `--json`/`--clip` flags + config-file resolver) into a shipped single-binary multi-subcommand Rust CLI on Windows PowerShell 7
**Confidence:** HIGH — every implementation claim is grounded in the real v1 source (read in full), the locked decisions in CONTEXT.md, and the HIGH-confidence ARCHITECTURE.md blueprint; the two new crates are version-verified against crates.io and slopcheck-clean.

<user_constraints>
## User Constraints (from CONTEXT.md)

> These are LOCKED. Research surfaces *how* to implement them, never *whether* or *which alternative*. The planner MUST honor every line below verbatim.

### Locked Decisions

**JSON Output Contract — the frozen template (SPINE-01)**

- **D-01 — Always-object document root.** Never a bare array at the root.
  - Scalar-only commands → a flat object `{ … }`.
  - Multi-capable commands (any command that *can* return more than one row — `uuid -n`, `hash` [multi-file arrives Phase 8], `du`, `dupes`, `flatten`, `bulk-rename`) → `{ "results": [ … ], "count": N }` **even when N = 1**.
  - `tree` → recursive object with `children: [ … ]`.
  - Rationale: PS7 `ConvertFrom-Json` auto-enumerates a bare top-level array (a footgun — single-element arrays collapse and the root can never grow a sibling key). An always-object root is one mechanical rule, never collapses, stays infinitely extensible. **Supersedes SUMMARY.md line 94's bare-array wording** (keeps its "shape stability" intent).
- **D-02 — Pilot literals (the copy-me template):**
  - `box uuid` → `{"results":[{"uuid":"…","version":"v4"}],"count":1}`
  - `box uuid -n 5` → same element struct `{uuid,version}` × 5, `"count":5`
  - `box hash file` → `{"results":[{"path":"file","algo":"blake3","digest":"…"}],"count":1}` — a **one-element `results` array inside an object**, NOT a bare `[{…}]`, so Phase-8 multi-file `hash a b` is byte-shape-identical (more elements, larger `count`) and no consumer rewrites.
- **D-03 — Field names locked:** `uuid`, `version` (uuid); `path`, `algo`, `digest` (hash). Output is `snake_case`, pretty (`to_writer_pretty`), UTF-8 **no BOM**, single trailing newline (SPINE-01). General rule for later phases: **root is always an object; scalar commands inline their fields; any multi-capable command uses `{results:[…],count:N}`; `tree` is the recursive-object exception with `children`.**

**`hash --verify` Algorithm Resolution — the 64-hex tie (HASH-V2-01)**

- **D-04 — Compute default flips to BLAKE3; verify resolution stays SHA-256.**
  - Compute (`hash/mod.rs:162`): `self.algo.unwrap_or(Algo::Sha256)` → `self.algo.or(config().default_hash_algo).unwrap_or(Algo::Blake3)`.
  - Verify (`algo_from_len`, `hash/mod.rs:78-85`): the length→algo table is **UNCHANGED** — 32→md5, 64→**sha256**, 128→sha512. A bare `box hash --verify <64hex> file` keeps verifying as SHA-256. `box hash --algo blake3 --verify <64hex> file` is the explicit BLAKE3 verify path (existing WR-01 logic supports this — no change needed).
- **D-05 — BLAKE3-fallback diagnostic probe on mismatch.** On the verify-mismatch branch (`hash/mod.rs:154-156` `bail!`), when the supplied digest is 64-hex and the SHA-256 comparison failed: additionally compute the file's BLAKE3 and, if *that* matches, emit a decisive stderr hint ("digest mismatch as sha256, but it MATCHES this file's blake3 — re-run with `--algo blake3`"); otherwise emit the static transitional hint pointing at `--algo blake3`. Hint is **stderr-only**, styling gated on `is_color_on()`, **suppressed under `--json`**. Exit stays **1** on mismatch.

**`--clip` Semantics (SPINE-03)**

- **D-07 — Copy-all, newline-joined.** `--clip` copies *everything the command printed* via the locked `out_line` tee into `CLIP_BUF`, with a single `trim_end()` in `flush_clip()`. `box uuid -n 5 --clip` copies **all 5** UUIDs. "Primary result" resolves uniformly to "the command's full stdout payload."
- **D-08 — Locked sibling rules:** copy-**AND**-print; "Copied to clipboard" confirmation → **stderr**, suppressed when not a TTY; `--clip` forces `COLOR_ON=false`; `--json --clip` copies the **whole** JSON document; `flush_clip()` runs **once on the main thread after successful dispatch**; empty output → no-op, no confirmation.

**Error / Exit Contract under `--json` (SPINE-01)**

- **D-09 — No JSON error envelope.** On any failure under `--json`: stdout stays **EMPTY**, a human-readable `error: …` goes to **stderr**, the process exits **1** (runtime) or **2** (usage). No `{"error":…}` on stdout.
- **D-10 — Malformed config → exit 2.** A corrupt/unparseable `%APPDATA%\box\config.toml` aborts *before* the operation runs → new `BoxError::Config` reusing the existing `main()` `BoxError`→exit-2 downcast. **Missing** config → silent fallback to built-in defaults (never errors a normal `box uuid`) stays locked (SPINE-05).

**Carried Forward (locked at requirements/roadmap/research time)**

- **Config impl (D-1, SPINE-05):** hand-roll `toml` 1.1.2 + `dirs` 6.0.0; `#[serde(default, deny_unknown_fields)] Config` of `Option<T>` fields (no `default_value`); precedence `cli.or(env).or(config).unwrap_or(builtin)`; `%APPDATA%\box\config.toml`; loaded **once** into `OnceLock` via `init_config()` in `main` before dispatch.
- **Spine wiring:** `--json`/`--clip` are `global = true` flags on `Cli`, lifted to `JSON_ON`/`CLIP_ON` atomics via `init_output()` (mirrors the proven `no_color`→`COLOR_ON` idiom). `RunCommand::run(self)` signature **unchanged**; commands consult `is_json_on()`, never a parsed field.
- **No-drift guarantee:** one `#[derive(Serialize)]` output struct per command feeds **both** the human render and `emit_json`.
- **Color/progress hygiene:** `init_output` forces `COLOR_ON=false` whenever `json` **or** `clip` is set.
- **Cross-spine field policy:** large numbers = bare `u64` (D-3); non-UTF-8 paths = `to_string_lossy()` (D-4) — never `to_str().unwrap()`.
- **Phase-6 new deps:** `toml`, `dirs` only (`serde`/`serde_json`/`arboard` already present).

### Claude's Discretion

- **Env-var tier spelling** — SPINE-05 mandates the env tier exists; no scheme is locked. Planner picks a consistent convention; suggested `BOX_<SECTION>_<KEY>` uppercase (e.g. `BOX_HASH_DEFAULT_ALGO`).
- **Config-struct scope in Phase 6** — start tiny: `default_hash_algo` is the only field Phase 6 strictly needs. Whether to stub `weather_units`/`weather_location`/`color` now is the planner's call (lean: include only `default_hash_algo`, with a doc note the struct grows per-command).
- **Exact hint/confirmation wording** — the D-05 mismatch hints and the D-08 "Copied to clipboard" line: keep concise, follow box's existing stderr `error:`/diagnostic style.

### Deferred Ideas (OUT OF SCOPE)

- Rolling the spine across the other 21 commands (Phase 7); per-command depth flags (Phases 8–10); `box config` / `box completions` meta-commands (Phase 11); `clap_complete`, `indicatif`, `chrono-tz`, `windows 0.61`, `uuid v7` deps (later phases).
- An opt-in `hash.strict_verify` config key (making the 64-hex tie an error) — surfaced by research as a *possible future* knob, explicitly NOT scoped for v2.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **SPINE-01** | `box <cmd> --json` emits exactly one well-formed JSON document on stdout — snake_case, pretty, UTF-8 no BOM, no ANSI/progress/chrome — established and contract-tested on `uuid`, `hash`. | `emit_json` + `JSON_ON` atomic design (Architecture Pattern 1/2); `init_output` forces `COLOR_ON=false`; JSON-purity test template (Validation Architecture §). D-01/D-02/D-03 lock the shape. |
| **SPINE-03** | `box <cmd> --clip` copies the primary result (copy AND print; "Copied to clipboard" to stderr, suppressed when not a TTY; whole JSON doc under `--json --clip`) — established on pilots. | `out_line` tee + `CLIP_BUF` + `flush_clip` (Architecture Pattern 3); reuse `clip/mod.rs` arboard discipline verbatim. D-07/D-08 lock semantics. |
| **SPINE-05** | Config-file defaults resolve CLI > env > config > builtin; missing/malformed file never errors a normal command (hand-rolled `toml` + `dirs`, `%APPDATA%\box\config.toml`). | `core::config` module + `OnceLock` + `Option<T>` `.or().or().unwrap_or()` merge (Architecture Pattern 4); `BoxError::Config` exit-2 for malformed, silent-default for missing. D-10 locks exit codes. |
| **HASH-V2-01** | BLAKE3 is the default compute algorithm (breaking change to COMPUTE only); bare `--verify <64-hex>` stays SHA-256; `--algo blake3 --verify` is the explicit BLAKE3 path; 64-hex mismatch emits a BLAKE3-fallback diagnostic hint. | One-line flip at `hash/mod.rs:162`; `algo_from_len` table UNCHANGED; D-05 probe attaches at the `bail!` (lines 154-156). D-04/D-05 lock the split. |
</phase_requirements>

## Summary

This phase builds the **entire scriptable spine once** and proves it on the two cheapest commands (`uuid`, `hash`). The shapes chosen here become the frozen template Phase 7 rolls across 23 commands, so the JSON/clip/error/config shapes are the highest-leverage decisions in the milestone — but they are **all already locked** in CONTEXT.md (D-01..D-10). This research therefore surfaces the *implementation-level* knowledge: exact code-edit points (verified against the real v1 source), ordering constraints in `main()`, Windows/PS7 specifics, and the reusable test mechanics — NOT decision re-litigation.

The architecture is a clean graft, not a rewrite. The v1 base already provides the *exact idiom* every spine addition mirrors: the `COLOR_ON: AtomicBool` + `init_color()` + `is_color_on()` triad in `core::output` is cloned for `JSON_ON`/`CLIP_ON` + `init_output()` + `is_json_on()`; the `hash` command is already `Option<Algo>`-shaped so the config-precedence merge is a one-line change to an existing `unwrap_or`; the `clip` command already encodes the correct single-shot main-thread arboard discipline that `flush_clip()` reuses verbatim; and `main()` already has the typed-`BoxError`→exit-2 downcast that `BoxError::Config` slots into. `RunCommand::run(self)` is **left unchanged** — commands consult `is_json_on()`/`out_line`/`emit_json`, never a new `Args` field, so there is zero trait churn and zero impact on the 21 not-yet-adopted commands.

The two genuinely new crates are `toml` (1.1.2) and `dirs` (6.0.0) — both version-verified against crates.io, both slopcheck-clean, both independently cited in CLAUDE.md and the milestone decision D-1. No new crate is needed for `--json` (`serde`/`serde_json` present) or `--clip` (`arboard` present). The single sharpest risk is **`--json` stdout contamination** (any stray ANSI/BOM/chrome byte breaks `ConvertFrom-Json`); the mitigation is structural (`emit_json` writes raw serde, `init_output` forces color off) and is backstopped by a per-command JSON-purity test that becomes Phase 7's copy-me template.

**Primary recommendation:** Mirror the `no_color`/`COLOR_ON` idiom exactly for `JSON_ON`/`CLIP_ON`; keep the `RunCommand` signature unchanged; hand-roll the `Config` as `Option<T>` fields with `#[serde(default, deny_unknown_fields)]`; wire `init_config()` → `init_color()` → `init_output()` in that order in `main()` before dispatch and `flush_clip()` once after a successful dispatch; lock the D-02 pilot JSON literals as known-answer test vectors.

## Architectural Responsibility Map

> Single-binary CLI: there is no browser/server/CDN/DB tier split. The relevant "tiers" are the internal module boundaries of the crate. Mapping each capability to its owning module prevents the recurring mistake of scattering spine logic into command modules.

| Capability | Primary Tier (module) | Secondary Tier | Rationale |
|------------|----------------------|----------------|-----------|
| Global flag → process-global state | `main.rs` (read once) → `core::output` (store) | `cli.rs` (declare) | Mirrors v1 `no_color`→`COLOR_ON`. Flag declared on `Cli`, read once in `main`, lifted to an atomic; never threaded through `run()`. [CITED: ARCHITECTURE.md Q1a] |
| JSON serialization (`emit_json`) | `core::output` | command module (defines `#[derive(Serialize)]` struct) | One shared serializer; each command owns only its output struct. No-drift guarantee: human + JSON read the same struct. [CITED: ARCHITECTURE.md Q1b] |
| `--clip` capture + flush | `core::output` (`out_line`/`CLIP_BUF`/`flush_clip`) | `main.rs` (calls `flush_clip` once) | Centralized capture-then-tee; one arboard call on the main thread on success. Commands just use `out_line` instead of `println!`. [CITED: ARCHITECTURE.md Q3] |
| Config load + precedence merge | `core::config` (load + `config()`) | command module (folds CLI flag on top via `.or()`) | Layers 1–3 (file/env/builtin) resolved centrally; the CLI flag is folded last in each command's `.or()` chain (clap can't distinguish typed-vs-defaulted without `Option`). [CITED: ARCHITECTURE.md Q2] |
| Exit-code mapping | `main.rs` (the 0/1/2 downcast) | `core::errors` (`BoxError::Config`) | `main` is the single owner of exit codes; new typed variant joins the existing exit-2 downcast list. [VERIFIED: src/main.rs:114-120] |
| BLAKE3-default flip + verify probe | `commands::hash` | `core::config` (reads `default_hash_algo`) | The default already lives in `hash/mod.rs`'s `unwrap_or`; flip it there and insert the config tier. Verify table stays in `hash`. [VERIFIED: src/commands/hash/mod.rs:162, 78-85] |

## Standard Stack

### Core (already present — reuse, do NOT add)

| Library | Version (in manifest) | Purpose | Why Standard |
|---------|----------------------|---------|--------------|
| `serde` | `1` (features `["derive"]`) | `#[derive(Serialize)]` on per-command output structs | Already pulled for weather; `--json` reuses it verbatim. [VERIFIED: Cargo.toml:91] |
| `serde_json` | `1.0.150` (features `["preserve_order"]`) | `to_writer_pretty` for `emit_json` | Already present for `json` command. `arbitrary_precision` deliberately OFF (a known landmine). [VERIFIED: Cargo.toml:49] |
| `arboard` | `3.6.1` (`default-features = false`) | clipboard write in `flush_clip` | Already present + proven in `clip/mod.rs`. Do NOT add a second clipboard crate. [VERIFIED: Cargo.toml:77; src/commands/clip/mod.rs] |
| `anyhow` | `1.0` | error propagation; `.context()` at config-load boundary | Already the binary-wide error type. [VERIFIED: Cargo.toml:14] |
| `thiserror` | `2.0` | the typed `BoxError` enum (`+ Config` variant) | Already powers `BoxError`. [VERIFIED: Cargo.toml:15; src/core/errors.rs] |

### Supporting (the two genuinely NEW crates)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `toml` | `1.1.2` | parse `%APPDATA%\box\config.toml` via `toml::from_str` into the `Config` struct | The config-file deserialize path (SPINE-05). [VERIFIED: crates.io — 1.1.2+spec-1.1.0, latest as of 2026-04-01; CITED: docs.rs/toml — `from_str` confirmed, requires `parse`+`serde` features] |
| `dirs` | `6.0.0` | resolve `%APPDATA%` for the config file location (`dirs::config_dir()`) | Locating the config file. [VERIFIED: crates.io — 6.0.0, latest] |

### Alternatives Considered (all rejected by locked decision D-1)

| Instead of | Could Use | Tradeoff | Status |
|------------|-----------|----------|--------|
| hand-roll `toml`+`dirs` | `config` crate 0.15 | Native layered precedence + `required(false)`, but re-litigates D-1's hand-roll choice and adds a heavier dep | **REJECTED by D-1** — hand-roll matches v1 ethos |
| `dirs` | `etcetera` 0.11 / `directories` 6 | Unopinionated base-dir logic | `dirs` is the more idiomatic "give me `%APPDATA%`" and is the locked choice |
| read `%APPDATA%` via `dirs` | `std::env::var_os("APPDATA")` | One fewer dep; ARCHITECTURE.md Q2 sketch uses the env-var form directly | Viable, but D-1 locked `dirs` for portability/clarity. Planner: use `dirs::config_dir()`; the `std::env` form is a documented fallback if `dirs` ever resists. |

**Installation (the ONLY manifest change this phase):**
```toml
# Config-file defaults (SPINE-05, D-1). Hand-roll: toml parse + dirs for %APPDATA%.
toml = "1.1.2"
dirs = "6.0.0"
```
Do NOT add `clap_complete`/`indicatif`/`windows`/`chrono-tz` or the `uuid "v7"` feature — those are Phases 7–11 (out of scope, D-15 in CONTEXT.md).

**Version verification performed:** `cargo search toml` → `toml = "1.1.2+spec-1.1.0"`; `cargo search dirs` → `dirs = "6.0.0"`. Neither appears in `Cargo.lock` yet (confirming genuinely-new deps). `toml::from_str` signature confirmed via docs.rs (returns `Result<T, toml::de::Error>`, which `anyhow::Context::context` wraps cleanly for the D-10 exit-2 path).

## Package Legitimacy Audit

> Run 2026-06-25. slopcheck 0.6.1 available; both new packages scanned clean.

| Package | Registry | Age | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-------------|-----------|-------------|
| `toml` | crates.io | mature (1.1.2, 2026-04-01) | github.com/toml-rs/toml | [OK] | Approved — also cited in CLAUDE.md + STACK.md + D-1 |
| `dirs` | crates.io | mature (6.0.0, 2025-01-12) | github.com/dirs-dev/directories-rs | [OK] | Approved — cited in STACK.md + D-1 |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none new. (slopcheck flags the *already-shipped* `crossterm` as [SUS] via a typosquat heuristic against `crossbeam` — this is a false positive: `crossterm` is the established v1 terminal crate in the manifest since v1, not touched by this phase. No action.)

Both new packages are `[VERIFIED: npm registry equivalent — crates.io]`: confirmed on the correct ecosystem registry (crates.io, not npm/PyPI — no cross-ecosystem confusion), discovered from authoritative sources (CLAUDE.md recommended-stack table, STACK.md, milestone decision D-1), AND slopcheck-clean. No postinstall/build-script risk (neither crate runs a `build.rs` that touches the network).

## Architecture Patterns

### System Architecture Diagram

```
box uuid --json --clip            box hash file.bin            box uuid   (no flags, malformed config present)
        │                                 │                            │
        ▼                                 ▼                            ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│ main.rs  (dispatch only — no business logic)                                    │
│   1. enable_ansi_support()                                                       │
│   2. Cli::try_parse()  ──► cli.json / cli.clip (global bools)                    │
│   3. init_config()  ── load %APPDATA%\box\config.toml ──► OnceLock<Config>       │
│         ├─ file missing  ──────────────► Config::default()  (silent, SPINE-05)   │
│         └─ file malformed ─────────────► Err(BoxError::Config) ──► EXIT 2 (D-10) │  ◄── aborts BEFORE dispatch
│   4. init_color(cli.no_color)                                                    │
│   5. init_output(cli.json, cli.clip)  ──► JSON_ON, CLIP_ON; force COLOR_ON=false │
│            if json||clip                                                         │
│   6. match cli.command { … => args.run() }                                       │
└───────────────────────────────────────────────────────────────────────────────┘
        │                                 │
        ▼                                 ▼
┌─────────────────────────┐   ┌──────────────────────────────────────────────────┐
│ uuid::run(self)         │   │ hash::run(self)                                    │
│  build UuidOutput{..}×N  │   │  algo = self.algo                                 │
│  is_json_on()?           │   │       .or(config().default_hash_algo)             │
│   ├ yes: emit_json(&doc) │   │       .unwrap_or(Algo::Blake3)   ◄── D-04 flip    │
│   └ no:  out_line(line)  │   │  verify? algo_from_len (64→sha256, UNCHANGED)     │
│          per UUID        │   │   mismatch+64hex ─► probe blake3 ─► stderr hint    │
└─────────────────────────┘   │  no-verify: emit_json OR out_line(digest row)      │
        │                      └──────────────────────────────────────────────────┘
        ▼ (every out_line / emit_json tees to CLIP_BUF when CLIP_ON)
┌───────────────────────────────────────────────────────────────────────────────┐
│ core::output                                                                     │
│   JSON_ON / CLIP_ON / COLOR_ON atomics                                           │
│   emit_json(&T)  ──► serde_json::to_writer_pretty(stdout) + "\n"  [+ tee CLIP]   │
│   out_line(&str) ──► println! + (if CLIP_ON) push to CLIP_BUF                    │
│   flush_clip()   ──► (main thread, after Ok dispatch) arboard.set_text(CLIP_BUF) │
└───────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
   main.rs step 7: flush_clip()  (only on Ok)  ──► step 8: 0/1/2 exit map
```

A reader can trace the primary use case `box uuid --json --clip`: parse → load config (default) → set JSON_ON+CLIP_ON+force color off → `uuid::run` builds the `{results:[…],count:1}` doc → `emit_json` writes it to stdout AND tees to CLIP_BUF → `main` calls `flush_clip` (one arboard write on the main thread) → exit 0.

### Recommended Project Structure (the file-by-file change ledger)

> Authoritative source: ARCHITECTURE.md New-vs-Modified ledger (lines 455-471), scoped to Phase 6 rows only. [CITED: ARCHITECTURE.md:455-471]

```
src/
├── main.rs                 # MODIFIED: + init_config(); + init_output(cli.json, cli.clip);
│                           #           + flush_clip() after Ok dispatch; + BoxError::Config in exit-2 downcast
├── cli.rs                  # MODIFIED: + #[arg(long, global=true)] json: bool; + clip: bool on Cli
│                           #           (NO Completions/Config variants — those are Phase 11)
├── core/
│   ├── mod.rs              # MODIFIED: + pub mod config;
│   ├── output.rs           # MODIFIED: + JSON_ON/CLIP_ON atomics + CLIP_BUF (Mutex<String>);
│   │                       #           + init_output, is_json_on, emit_json, out_line, flush_clip;
│   │                       #           init_output forces COLOR_ON=false under json||clip
│   ├── config.rs           # NEW: Config{default_hash_algo: Option<Algo>}, load(), OnceLock, config(), init_config()
│   └── errors.rs           # MODIFIED: + BoxError::Config { path, source/message } (exit-2)
└── commands/
    ├── uuid/mod.rs         # MODIFIED: + UuidOutput{uuid,version}; is_json_on() fork; out_line for human path;
    │                       #           wrap as {results:[…],count:N} per D-01/D-02
    └── hash/mod.rs         # MODIFIED: + HashOutput{path,algo,digest}; flip default to BLAKE3 (line 162);
                            #           + Serialize/Deserialize on Algo; D-05 verify probe at bail! (154-156)

tests/
├── uuid.rs                 # MODIFIED: + JSON-purity test + --clip capture test (the copy-me template)
├── hash.rs                 # MODIFIED: + BLAKE3-default test + JSON test + verify-stays-sha256 + probe-hint test
└── config.rs               # NEW (recommended): config precedence + missing/malformed integration tests
```

### Pattern 1: Global flag → process-global atomic → `is_*_on()` (the spine's load-bearing idiom)

**What:** Declare `--json`/`--clip` as `global = true` bools on `Cli`; read them ONCE in `main()`; store in `JSON_ON`/`CLIP_ON` atomics; commands consult `is_json_on()` — never a parsed `Args` field.
**When to use:** Always for spine flags. This is the chosen mechanism (Option C) over per-command fields (Option A) or `from_global` (Option B), both of which churn all 23 `Args` structs. [CITED: ARCHITECTURE.md Q1a, Option table]
**Example (mirror the EXISTING `COLOR_ON` triad verbatim):**
```rust
// core::output — clone the COLOR_ON pattern (src/core/output.rs:28-34 is the template)
// Source: ARCHITECTURE.md Q1a + VERIFIED against src/core/output.rs:28-59
static JSON_ON: AtomicBool = AtomicBool::new(false);
static CLIP_ON: AtomicBool = AtomicBool::new(false);

pub fn is_json_on() -> bool { JSON_ON.load(Ordering::Relaxed) }

pub fn init_output(json: bool, clip: bool) {
    JSON_ON.store(json, Ordering::Relaxed);
    CLIP_ON.store(clip, Ordering::Relaxed);
    // D-08 / Anti-Pattern 4: clipboard & JSON must never carry ANSI.
    if json || clip {
        // Reuse the SAME mechanism init_color uses: store COLOR_ON=false AND
        // owo_colors::set_override(false). (init_color stores both; mirror it.)
        crate::core::output::force_color_off();   // small helper, or inline the two stores
    }
}
```
> **Ordering note (load-bearing):** `init_output` must run AFTER `init_color`, because `init_color` unconditionally stores its own `COLOR_ON` value; if `init_output` ran first its `COLOR_ON=false` would be overwritten. Verified call order in `main()`: `init_config()` → `init_color(cli.no_color)` → `init_output(cli.json, cli.clip)`. [VERIFIED: src/main.rs:72 shows init_color; ARCHITECTURE.md:436-438 confirms the sequence]

### Pattern 2: One `#[derive(Serialize)]` struct feeds both human + JSON (the no-drift guarantee)

**What:** Each command defines one output struct; the human renderer formats *that* struct, `emit_json` serializes *that* struct. They cannot diverge.
**When to use:** Every command gaining `--json`. Pairs with v1's "thin orchestrator over a pure function" (the pure fn already returns the value — just give it `#[derive(Serialize)]` + field names).
**Example:**
```rust
// core::output — Source: ARCHITECTURE.md Q1b (lines 106-114), VERIFIED shape
pub fn emit_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value)
        .context("serializing --json output")?;
    out.write_all(b"\n")?;          // single trailing newline → clean ConvertFrom-Json (D-03)
    // D-08: under --json --clip, the JSON doc must also reach CLIP_BUF.
    if CLIP_ON.load(Ordering::Relaxed) {
        let s = serde_json::to_string_pretty(value)?;   // same value, string form, for the clip tee
        CLIP_BUF.lock().unwrap().push_str(&s);
    }
    Ok(())
}
```
```rust
// commands/uuid — the D-02 pilot literal {results:[…],count:N}
#[derive(serde::Serialize)]
struct UuidRow { uuid: String, version: &'static str }   // version: "v4"
#[derive(serde::Serialize)]
struct UuidOutput { results: Vec<UuidRow>, count: usize }
```
```rust
// commands/hash — the D-02 pilot literal {results:[{path,algo,digest}],count:N}
#[derive(serde::Serialize)]
struct HashRow { path: String, algo: Algo, digest: String }   // Algo serializes lowercase
#[derive(serde::Serialize)]
struct HashOutput { results: Vec<HashRow>, count: usize }
```
> **`Algo` round-trip:** add `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "lowercase")]` to the existing `Algo` enum (src/commands/hash/mod.rs:63-73) so it serializes to `"blake3"`/`"sha256"` in `--json` AND deserializes from the config `default_hash_algo = "sha256"` key. The spellings already match the clap `ValueEnum` variants. [VERIFIED: src/commands/hash/mod.rs:63-73]

### Pattern 3: `--clip` capture-then-tee in core, flushed once in `main` (NOT per-command)

**What:** `out_line` is the ONE print primitive for primary stdout output; it tees the string to `CLIP_BUF` when `CLIP_ON`. `flush_clip()` runs once in `main` after a successful dispatch.
**When to use:** Primary value output of any `--clip`-capable command. Replace the command's `println!` of its primary result with `out_line`.
**Example (reuse the `clip/mod.rs` arboard discipline verbatim):**
```rust
// core::output — Source: ARCHITECTURE.md Q3 (lines 280-302), arboard idiom VERIFIED against src/commands/clip/mod.rs:108-109
static CLIP_BUF: Mutex<String> = Mutex::new(String::new());

pub fn out_line(s: &str) {
    println!("{s}");
    if CLIP_ON.load(Ordering::Relaxed) {
        let mut b = CLIP_BUF.lock().unwrap();
        b.push_str(s);
        b.push('\n');
    }
}

/// Called ONCE in main() after dispatch succeeds. No-op unless --clip was set.
pub fn flush_clip() -> anyhow::Result<()> {
    if !CLIP_ON.load(Ordering::Relaxed) { return Ok(()); }
    let text = CLIP_BUF.lock().unwrap();
    if text.trim_end().is_empty() { return Ok(()); }   // D-08: empty output → no-op, no confirmation
    // SAME single-shot main-thread flow proven in clip/mod.rs: create → one op → return.
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text.trim_end().to_string()).context("write clipboard")?;   // D-07: single trim_end()
    // D-08: "Copied to clipboard" → stderr, suppressed when stderr is not a TTY.
    if std::io::stderr().is_terminal() {
        eprintln!("Copied to clipboard");   // exact wording at planner's discretion
    }
    Ok(())
}
```
> **Why `is_terminal()` on stderr, not stdout, for the confirmation:** D-08 says "suppressed when not a TTY." The confirmation is a stderr diagnostic; gate it on `stderr().is_terminal()` so `box uuid --clip 2>log` doesn't write the confirmation into the log, matching the v1 "messages → stderr, TTY-gated" discipline. [CITED: CONTEXT.md D-08; pattern consistent with src/core/output.rs:42-44 color_enabled which gates on stdout().is_terminal()]

### Pattern 4: Config — `Option<T>` fields + `.or().or().unwrap_or()` merge (precedence by construction)

**What:** Every config-overridable flag is `Option<T>` with NO clap `default_value`. `Some` = user typed it; `None` = fall through. The merge is a one-liner.
**When to use:** Every config-overridable setting. `hash` is ALREADY shaped for this (`algo: Option<Algo>`), so the change is a one-line edit to the existing `unwrap_or`.
**Example:**
```rust
// core/config.rs — NEW. Source: ARCHITECTURE.md Q2 (lines 217-243), toml::from_str VERIFIED via docs.rs
use std::sync::OnceLock;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]   // unknown key → loud error (D-10 family), missing key → None
pub struct Config {
    pub default_hash_algo: Option<crate::commands::hash::Algo>,
    // Phase-6 lean scope: ONLY default_hash_algo (Claude's Discretion). Doc-note the struct grows per-command.
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get().expect("init_config() must run before config()")
}

/// Load once in main() BEFORE dispatch. Missing file = silent default (SPINE-05);
/// malformed file = Err(BoxError::Config) → exit 2 (D-10).
pub fn init_config() -> anyhow::Result<()> {
    let cfg = load()?;
    let _ = CONFIG.set(cfg);
    Ok(())
}

fn load() -> anyhow::Result<Config> {
    let Some(path) = config_path() else { return Ok(Config::default()) };
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s)
            .map_err(|e| crate::core::errors::BoxError::Config {
                path: path.display().to_string(),
                message: e.to_string(),
            }.into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))   // %APPDATA%\box\config.toml on Windows
}
```
```rust
// commands/hash — the ONE-LINE precedence flip + config tier (replaces line 162)
// Source: ARCHITECTURE.md Q5 (lines 357-361), VERIFIED against src/commands/hash/mod.rs:162
let algo = self.algo                               // CLI flag (Some = user typed it)  — highest
    .or_else(|| std::env::var("BOX_HASH_DEFAULT_ALGO").ok()   // env tier (Discretion: spelling)
        .and_then(|s| Algo::from_str_lossy(&s)))   // parse env string → Algo (planner picks parse fn)
    .or(crate::core::config::config().default_hash_algo)      // config file tier
    .unwrap_or(Algo::Blake3);                       // NEW builtin (D-04) — the breaking change
```
> **Env-tier note (Claude's Discretion):** SPINE-05 mandates the env tier exists in the precedence chain. The exact spelling is the planner's call; `BOX_HASH_DEFAULT_ALGO` (uppercase `BOX_<SECTION>_<KEY>`) is the suggested convention. The env value is a string and needs the same lowercase→`Algo` parse the config uses; reuse one parse fn. [CITED: CONTEXT.md Claude's Discretion]

### Anti-Patterns to Avoid

> All six are from ARCHITECTURE.md Anti-Patterns (lines 474-505). The four most relevant to Phase 6:

- **Per-command `--json` field duplicated 23×:** declare it once on `Cli` with `global = true`; consult `is_json_on()`. Never add a `json: bool` to any `Args`.
- **Separate code paths for human vs JSON:** one `#[derive(Serialize)]` struct is the single source; both paths read it. A field added is available to both; the JSON can never report a different digest than the human row.
- **`default_value` on a config-overridable flag:** clap fills the default before you can tell "typed" from "defaulted", making config-wins-over-builtin unimplementable. Use `Option<T>` + `.or()`.
- **ANSI leaking into `--json`/`--clip`:** `init_output` forces `COLOR_ON=false` under json||clip; `emit_json` never styles; `is_json_on()` is checked before any styling decision. The per-command JSON-purity test (no `0x1B`, no BOM) is the backstop.
- **(Phase-6-specific) Running `flush_clip()` on failure:** it must run ONLY after `Ok(())` from dispatch, BEFORE the exit-code map — a failed command must never clobber the clipboard with partial output (D-08).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization | A manual `format!`-based JSON writer | `serde_json::to_writer_pretty` (present) | Escaping, Unicode, number formatting, no-BOM all handled correctly; a hand-rolled writer reintroduces the exact contamination bugs SPINE-01 forbids |
| Clipboard write | A second clipboard crate or `clip.exe` shell-out | `arboard` (present, proven in `clip/mod.rs`) | `clip.exe` mangles Unicode (the reason `box clip` exists); arboard does correct UTF-8↔UTF-16 CF_UNICODETEXT and persists past exit on Windows |
| Config precedence | A `config.value.unwrap_or(args.value)` merge | `Option<T>` + `args.or(env).or(config).unwrap_or(builtin)` | The naive merge INVERTS precedence (config beats explicit flag — Pitfall 3); the `.or()` chain wins by construction |
| `%APPDATA%` location | Hard-coded `C:\Users\…\AppData\Roaming` | `dirs::config_dir()` | Roams correctly, no hard-coded user, survives reinstall |
| Missing-file handling | A `Path::exists()` pre-check then read | match on `io::ErrorKind::NotFound => Config::default()` | TOCTOU-free; `exists()`-then-`read` races; one match arm is correct and is the documented pattern |
| Color gate for spine flags | New per-flag color checks | reuse `is_color_on()` + force-off in `init_output` | The single gate already exists and is tested; spine flags just feed it |

**Key insight:** Phase 6 adds almost no new *algorithms* — it wires existing, tested primitives (`serde_json`, `arboard`, the `COLOR_ON` triad, the `BoxError`→exit downcast) into a shared spine. The risk is in the *wiring order and the contamination discipline*, not in any novel code. Hand-rolling any of the above reintroduces solved problems.

## Common Pitfalls

### Pitfall 1: `--json` stdout contamination (the #1 v2 failure mode)
**What goes wrong:** `box <cmd> --json | ConvertFrom-Json` fails because stdout carries a stray byte — an ANSI escape, a UTF-8 BOM, a progress line, a human summary, a `--verify` "OK" line, or a debug `println!`. PS7 consumes the entire stdout stream as one payload, so one extra line is a hard parse error.
**Why it happens:** `--json` is bolted onto a print path built for humans; the trap is treating `--json` as "also print JSON" instead of "print JSON and nothing else."
**How to avoid:** `emit_json` writes raw serde (never `.green()`); `init_output` forces `COLOR_ON=false` under `--json`; the D-05 verify hint is suppressed under `--json`; `is_json_on()` is checked FIRST in each `run()`. For `uuid`/`hash` specifically, audit that the only stdout write reachable when `is_json_on()` is the `emit_json` call. [CITED: PITFALLS.md Pitfall 1; STATE.md "v2 #1 failure mode"]
**Warning signs:** `ConvertFrom-Json: Unexpected character`; color escapes in `$result`; a redirected `> out.json` with progress fragments.

### Pitfall 2: `--json` correctness — BOM, large numbers, non-UTF-8 paths
**What goes wrong:** Syntactically-valid JSON that still breaks consumers — a leading BOM (`EF BB BF`), a `u64` > 2^53 silently rounded by a JS consumer, or a non-UTF-8 NTFS path that panics (`to_str().unwrap()`) or corrupts (`to_string_lossy()` substitutes U+FFFD).
**Why it happens:** JSON "looks done" with ASCII data; failures surface only with real Windows paths/large files.
**How to avoid (locked policies):** no BOM (write via `to_writer`, test first 3 bytes ≠ `EF BB BF`); large numbers = bare `u64` with documented >2^53 JS caveat (D-3 — PS7 handles `Int64`/`BigInteger`); non-UTF-8 paths = `to_string_lossy()`, NEVER `to_str().unwrap()` (D-4). For the `hash` `path` field specifically, the input arrives as `Option<String>` already, so the Phase-6 surface is low-risk — but the policy must be encoded now so Phase 7 inherits it. [CITED: PITFALLS.md Pitfall 2; STATE.md D-3/D-4]
**Warning signs:** a 5 TB size off by a few bytes after a JS round-trip; a path rendering as `�`; `ConvertFrom-Json` erroring only on certain files.

### Pitfall 3: Config precedence inverted (explicit flag silently ignored)
**What goes wrong:** `box hash --algo sha256 file` with `default_hash_algo = "blake3"` in config — and config wins, silently ignoring the explicit flag.
**Why it happens:** clap fills `default_value` before `run()` sees the struct, so a naive merge can't tell "typed" from "defaulted."
**How to avoid:** `algo: Option<Algo>` with NO `default_value` (already the case — VERIFIED hash/mod.rs:48); resolve `self.algo.or(env).or(config).unwrap_or(Blake3)`. Unit-test the precedence matrix as a pure function. [CITED: PITFALLS.md Pitfall 3]
**Warning signs:** a flag that "does nothing"; behavior changing based on whether a config file exists; no test where flag and config disagree.

### Pitfall 4: Malformed config bricking a normal command
**What goes wrong:** A config typo (`default_hash_alg` vs `default_hash_algo`, or invalid TOML) hard-errors `box uuid` — a command that doesn't even read `default_hash_algo`.
**Why it happens:** `deny_unknown_fields` + a too-eager error path turns any config blemish into a global failure. The locked decision (D-10) deliberately accepts this for *malformed* config (exit 2, abort before op) but NOT for *missing* config.
**How to avoid:** The two cases are distinct and BOTH must be tested: missing file → `Config::default()` (silent, `box uuid` still prints a UUID); malformed/unparseable file → `BoxError::Config` → exit 2 (aborts before the op runs). `deny_unknown_fields` means an unknown key IS malformed → exit 2 — this is intentional (matches the project's "nothing silently lost" ethos), but verify the missing-file path stays silent. [CITED: CONTEXT.md D-10; ARCHITECTURE.md Q2:247-250]
**Warning signs:** `box uuid` failing after a config edit; a missing config file producing any stderr output.

### Pitfall 5: BLAKE3-default verify regression (the #1 v2 data risk)
**What goes wrong:** Flipping the *verify* default to BLAKE3 too would silently break every stored SHA-256 baseline (a 64-hex digest is valid for both algorithms). D-04 explicitly forbids this.
**Why it happens:** Conflating the compute default with the verify default — they look symmetric but are not (D-06 amended REQUIREMENTS.md and SUMMARY.md to correct exactly this stale wording).
**How to avoid:** Change ONLY the compute `unwrap_or` (line 162). Leave `algo_from_len` (lines 78-85) byte-for-byte unchanged — 64→sha256 stays. The existing `tests/hash.rs::hash_verify_autodetect` test already pins 64-hex-no-algo → sha256; it must STILL PASS after the flip (it is the regression backstop). Add the D-05 probe at the `bail!` (lines 154-156). [VERIFIED: src/commands/hash/mod.rs:78-85, 154-156, 162; src/../tests/hash.rs:122-139]
**Warning signs:** `tests/hash.rs::hash_verify_autodetect` needing edits (a red flag — it should pass unchanged); a stored `sums.txt` SHA-256 baseline failing after upgrade.

### Pitfall 6: arboard off the main thread / on failure
**What goes wrong:** Calling `arboard::Clipboard::new()` from a worker thread, or flushing the clipboard after a command failed (clobbering it with partial output).
**Why it happens:** Treating clip as a fire-and-forget side effect rather than a single-shot main-thread op.
**How to avoid:** `flush_clip()` runs on the main thread in `main()` ONLY after `Ok(())` from dispatch, before the exit-code map. This reuses the exact single-shot synchronous flow proven in `clip/mod.rs` (create → one op → return). [CITED: STATE.md "arboard main-thread only"; src/commands/clip/mod.rs:24-36 doc comment]
**Warning signs:** clipboard write panicking; a failed command still changing the clipboard.

### Pitfall 7: `init_output` overwriting `init_color`'s decision (ordering bug)
**What goes wrong:** If `init_output` forces `COLOR_ON=false` but runs BEFORE `init_color`, then `init_color` overwrites it back to the TTY-derived value — and a `--json` run on a TTY could emit ANSI.
**Why it happens:** Both functions store `COLOR_ON`; order matters.
**How to avoid:** Call order in `main()` is `init_config()` → `init_color(cli.no_color)` → `init_output(cli.json, cli.clip)`. `init_output` runs LAST so its force-off wins. Equivalently, fold the json/clip force-off INTO `init_color`'s inputs. Test: `box uuid --json` on a (simulated) TTY emits no `0x1B`. [VERIFIED: src/main.rs:72 init_color call site; ARCHITECTURE.md:436-438 sequence]
**Warning signs:** ANSI in `--json` output only when run from an interactive terminal (passes when piped, fails on a TTY).

## Code Examples

### Adding `BoxError::Config` (exit-2 variant)
```rust
// src/core/errors.rs — joins MissingInput / UnsupportedHashLength
// Source: VERIFIED against src/core/errors.rs:7-31 (existing variants) + main.rs:114-120 downcast
#[error("config error in {path}: {message}")]
Config { path: String, message: String },
```
```rust
// src/main.rs — extend the existing exit-2 downcast (lines 114-120)
// Source: VERIFIED against src/main.rs:114-120
match e.downcast_ref::<crate::core::errors::BoxError>() {
    Some(
        crate::core::errors::BoxError::MissingInput
        | crate::core::errors::BoxError::UnsupportedHashLength { .. }
        | crate::core::errors::BoxError::Config { .. },   // NEW
    ) => ExitCode::from(2),
    _ => ExitCode::from(1),
}
```
> **Subtlety:** `init_config()` runs BEFORE the dispatch `match` and returns `anyhow::Result`. To map a malformed-config error to exit 2, `init_config`'s error must reach the SAME downcast path. Two viable wirings: (a) call `init_config()` inside the dispatched flow so its `Err` flows through the existing `match result` block; or (b) handle it inline at the call site with its own `ExitCode::from(2)`. Option (a) keeps one exit-code owner (`main`'s `match result`) — recommended. The planner must ensure the `BoxError::Config` actually reaches the downcast. [VERIFIED: src/main.rs:39-123 structure]

### The D-05 verify-mismatch BLAKE3 probe
```rust
// src/commands/hash/mod.rs — at the mismatch branch (replaces the bail! at lines 154-156)
// Source: CONTEXT.md D-05; structure VERIFIED against src/commands/hash/mod.rs:135-158
} else {
    // D-05: when the supplied digest is 64-hex and sha256 failed, probe blake3.
    if expected.len() == 64 && self.algo.is_none() && !is_json_on() {
        // Re-acquire the input to compute blake3 (the first reader is consumed by streaming).
        // NOTE: input is streamed once; to probe, either buffer-then-rehash or re-open the path.
        // For a file path this is a re-open; for stdin it cannot be re-read — degrade to the
        // static hint when the source is stdin (label == "-").
        let hint = match probe_blake3_matches(&label, expected) {
            Some(true)  => "note: digest mismatch as sha256, but it MATCHES this file's blake3 \
                            — re-run with `--algo blake3`",
            _           => "note: the default compute algorithm is now blake3; pass \
                            `--algo blake3` if this is a blake3 digest",
        };
        eprintln!("{hint}");   // stderr-only, styling gated on is_color_on() (D-05)
    }
    bail!("hash mismatch for {label}: expected {expected}, got {computed}");
}
```
> **Re-read caveat (IMPORTANT for the planner):** the verify path STREAMS the input once (`input.reader` is consumed by `digest_reader`). The D-05 probe needs the bytes a SECOND time to compute BLAKE3. For a file PATH this is a re-open of the path. For piped STDIN there is no second read — the probe must degrade to the static hint when `label == "-"`. The planner should make this explicit in the task (probe only when a real path is available). [VERIFIED: src/commands/hash/mod.rs:130-149 — `read_file_or_stdin` returns a single-pass reader]

### The pilot JSON literals as test vectors (lock these)
```rust
// tests/uuid.rs — JSON-purity + shape (the copy-me template for Phase 7)
let out = /* box uuid --json */;
let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is one JSON value");
assert!(v.get("results").is_some() && v.get("count") == Some(&serde_json::json!(1)));
assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --json stdout");
assert_ne!(&out.stdout[..3.min(out.stdout.len())], b"\xEF\xBB\xBF", "no UTF-8 BOM");
```

## Runtime State Inventory

> This phase is partly a behavior-change (BLAKE3 default flip) but NOT a rename/refactor. It introduces a NEW external state surface (the config file) rather than renaming an existing one. Inventory of state this phase introduces or affects:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | **None** — no datastore, no collection, no user_id. The config file `%APPDATA%\box\config.toml` is NEW state this phase *reads* but does not yet write (no `box config set` until Phase 11). | Document the path + format; no migration. |
| Live service config | **None** — `box` is a standalone CLI; no external service stores box state. Verified: no n8n/Datadog/Tailscale integration in the codebase. | None. |
| OS-registered state | **None** — no Task Scheduler / pm2 / launchd registration. The only OS touchpoint is the clipboard (transient, not registered) and the ANSI console mode (set per-run via `enable_ansi_support`). | None. |
| Secrets/env vars | **NEW (additive):** the env-tier var `BOX_HASH_DEFAULT_ALGO` (spelling = planner's discretion). No existing env var is renamed. | Document the chosen env spelling; no migration. |
| Build artifacts | **None stale** — adding `toml`+`dirs` updates `Cargo.lock` (a normal `cargo build` regenerates it). No egg-info/compiled-binary equivalent carries an old name. | `cargo build` regenerates `Cargo.lock`; commit it. |

**Behavior-change state risk (the real one):** the BLAKE3-default flip changes the *output* of `box hash file` for every existing user's scripts and stored `sums.txt` baselines. This is not "runtime state in a datastore" but it IS a silent-output-change risk. Mitigations are locked: `--algo sha256` escape hatch (unchanged), `hash.default_algo = "sha256"` config escape hatch (NEW this phase), and a loud `--help`/PROJECT changelog note. Bare `--verify <64-hex>` stays SHA-256 so stored baselines do NOT silently break (D-04 — the #1 v2 risk, explicitly defused). [CITED: STATE.md "BLAKE3-default breaking change"; CONTEXT.md D-04]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (cargo) | build/test | ✓ | (project builds on stable MSVC) | — |
| `x86_64-pc-windows-msvc` target | release build | ✓ | (v1 ships on it) | — |
| Windows clipboard (Win32) | `--clip` flush (arboard) | ✓ (interactive) | OS-provided | clip tests `#[ignore]`d on headless CI (shared OS state) — see `tests/clip.rs` |
| `%APPDATA%` env / `dirs::config_dir()` | config path | ✓ | OS-provided | `std::env::var_os("APPDATA")` if `dirs` resolution ever fails; missing → `Config::default()` |
| crates.io (toml, dirs fetch) | first build after manifest edit | ✓ | toml 1.1.2 / dirs 6.0.0 | — |
| PowerShell 7 (`ConvertFrom-Json`) | manual `--json` verification / human-verify checkpoint | ✓ (target platform) | PS7 | automated `serde_json::from_slice` round-trip in tests covers the deterministic half |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** clipboard on headless CI — the deterministic contract (capture/tee/trim, empty-no-op) is unit/integration-tested WITHOUT a live clipboard; the actual `arboard.set_text` is exercised by a locally-run `#[ignore]`d test + a human-verify checkpoint (the proven `tests/clip.rs` pattern). [VERIFIED: tests/clip.rs:1-18 documents the headless-CI clipboard strategy]

## Validation Architecture

> nyquist_validation = true (config.json) — this section is REQUIRED. The validation surfaces below are the reusable templates Phase 7 copies across 23 commands.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + `assert_cmd` 2.2 (black-box binary) + `assert_fs` 1.1 / `tempfile` 3.27 (fixtures) + `predicates` 3.1; `insta` 1.48 available for snapshots |
| Config file | none — `Cargo.toml [dev-dependencies]` only; integration tests live in `tests/*.rs` |
| Quick run command | `cargo test --bin box` (unit tests inside `src/`) |
| Full suite command | `cargo test` (unit + all `tests/*.rs` integration; clip's live-clipboard tests are `#[ignore]`d by default) |

> **Critical invariant:** `box` is binary-only — unit tests run via `cargo test --bin box`, NOT `--lib` (there is no lib target). [VERIFIED: Cargo.toml:7-9 `[[bin]]` only; STATE.md line 99]

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPINE-01 | `box uuid --json` stdout is exactly one JSON value, no ANSI, no BOM, snake_case, `{results,count}` shape | integration | `cargo test --test uuid json_purity` | ❌ Wave 0 (add to tests/uuid.rs) |
| SPINE-01 | `box hash file --json` shape `{results:[{path,algo,digest}],count:1}` (the Phase-8-compatible literal) | integration | `cargo test --test hash json_shape` | ❌ Wave 0 (add to tests/hash.rs) |
| SPINE-01 | under `--json`, a failure leaves stdout EMPTY + `error:` on stderr + exit 1/2 (D-09) | integration | `cargo test --test hash json_error_empty_stdout` | ❌ Wave 0 |
| SPINE-03 | `box uuid -n 5 --clip` captures all 5 lines into CLIP_BUF (capture logic, not live clipboard) | unit (core::output) | `cargo test --bin box out_line_tees` | ❌ Wave 0 (unit in output.rs) |
| SPINE-03 | empty output → no clipboard op, no confirmation (D-08) | unit | `cargo test --bin box flush_clip_empty_noop` | ❌ Wave 0 |
| SPINE-03 | live round-trip (`--clip` then read back) Unicode-exact | integration `#[ignore]` + human-verify | `cargo test --test uuid -- --ignored --test-threads=1` | ❌ Wave 0 (mirror tests/clip.rs) |
| SPINE-05 | precedence: CLI `--algo sha256` beats config `blake3` beats builtin | unit (pure resolver) | `cargo test --bin box precedence_matrix` | ❌ Wave 0 |
| SPINE-05 | missing config → `box uuid` still prints a UUID (silent default) | integration | `cargo test --test config missing_is_silent` | ❌ Wave 0 (new tests/config.rs) |
| SPINE-05 | malformed config → exit 2 BEFORE the op (D-10) | integration | `cargo test --test config malformed_exit2` | ❌ Wave 0 |
| HASH-V2-01 | `box hash file` (no --algo) emits 64-hex BLAKE3 (was sha256) | integration | `cargo test --test hash default_is_blake3` | ❌ Wave 0 (add; flips assertion in existing default test) |
| HASH-V2-01 | `box hash --algo sha256 file` still emits SHA-256 | integration | `cargo test --test hash algo_sha256_still_works` | ✅ (tests/hash.rs:74-89 covers `--algo` arms) |
| HASH-V2-01 | bare `--verify <64-hex sha256>` STILL passes (no regression) | integration | `cargo test --test hash hash_verify_autodetect` | ✅ (tests/hash.rs:122-139 — must pass UNCHANGED) |
| HASH-V2-01 | 64-hex mismatch emits the BLAKE3-fallback hint on stderr (D-05) | integration | `cargo test --test hash verify_blake3_probe_hint` | ❌ Wave 0 |
| HASH-V2-01 | config `default_hash_algo = "sha256"` restores SHA-256 default; CLI `--algo blake3` still wins | integration | `cargo test --test config hash_default_override` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --bin box` (unit: precedence resolver, out_line/flush_clip capture, Algo serde round-trip) — sub-second, the Nyquist quick sample.
- **Per wave merge:** `cargo test` (full unit + integration suite, clipboard tests `#[ignore]`d).
- **Phase gate:** full suite green + a real PS7 `box uuid --json | ConvertFrom-Json` human-verify + a live `--clip` round-trip human-verify before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `tests/config.rs` (NEW) — covers SPINE-05 missing/malformed/precedence-via-binary
- [ ] `tests/uuid.rs` (extend) — JSON-purity + `--clip` capture template (the copy-me template)
- [ ] `tests/hash.rs` (extend) — BLAKE3-default flip + JSON shape + D-05 probe hint; keep `hash_verify_autodetect` PASSING UNCHANGED (regression backstop)
- [ ] Unit tests in `src/core/output.rs` — `out_line` tee, `flush_clip` empty-no-op, `emit_json` no-BOM (mirror the existing `COLOR_LOCK`-serialized atomic-mutation test pattern at output.rs:247-358)
- [ ] Unit tests in `src/core/config.rs` (NEW) — pure precedence resolver matrix + `toml::from_str` malformed→`BoxError::Config`
- [ ] No framework install needed — `assert_cmd`/`assert_fs`/`predicates`/`insta` all present

> **Atomic-test-isolation note:** tests that mutate the process-global `JSON_ON`/`CLIP_ON`/`COLOR_ON` atomics MUST serialize via a `Mutex` lock, exactly like the existing `COLOR_LOCK` at `src/core/output.rs:255` — the default parallel runner will otherwise interleave a `true`/`false` store between another test's store and read. This is the proven v1 pattern; reuse it for the new atomics. [VERIFIED: src/core/output.rs:250-256, 302-303]

## Security Domain

> security_enforcement not explicitly false in config.json → treat as enabled. This is a local single-binary CLI with no network/auth/session surface in Phase 6; the applicable categories are narrow.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface (local CLI) |
| V3 Session Management | no | No sessions |
| V4 Access Control | partial | Config read from user-scoped `%APPDATA%\box` ONLY — never CWD or an arbitrary env-specified path (prevents config-injection altering `hash.default_algo`). [CITED: PITFALLS.md Security table] |
| V5 Input Validation | yes | `toml::from_str` + `deny_unknown_fields` rejects malformed/unknown config keys (→ exit 2, D-10); `Algo` deserialize rejects unknown algorithm strings; non-UTF-8 stdin already rejected before clipboard touch (clip pattern) |
| V6 Cryptography | partial (correctness, not secrecy) | BLAKE3/SHA-2/MD5 via vetted RustCrypto + blake3 crates — never hand-rolled. Checksum compare stays plain `eq_ignore_ascii_case` (a checksum is PUBLIC, not a secret — constant-time compare is wrong over-engineering, T-03-01). [CITED: PITFALLS.md Security table] |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Config injection (world-writable / CWD / arbitrary-path config forces `hash.default_algo`) | Tampering | Read config from user-scoped `%APPDATA%\box` only; an explicit CLI `--algo` always overrides config (precedence) |
| `--clip` pushing a secret to the clipboard silently | Information Disclosure | `--clip` is explicit/opt-in; do NOT auto-clip. (passgen `--clip` is Phase 7; document the clipboard-history caveat there — out of scope here, but the spine must not make clipboard implicit.) [CITED: PITFALLS.md Security table] |
| Non-UTF-8 / malformed config DoS-ing every command | Denial of Service | Malformed → clean exit 2 (D-10), never a panic; missing → silent default. No unbounded read (config files are tiny). |
| ANSI/escape injection via a path echoed into `--json` | Tampering | `serde_json` escapes control chars in string values; `emit_json` never re-emits raw ANSI; non-UTF-8 paths → `to_string_lossy()` (D-4) |

## State of the Art

| Old Approach (v1) | Current Approach (v2 Phase 6) | When Changed | Impact |
|--------------------|-------------------------------|--------------|--------|
| `box hash file` → SHA-256 | `box hash file` → BLAKE3 (compute default) | This phase (HASH-V2-01) | Breaking output change; mitigated by `--algo sha256` + config escape hatch + loud changelog. Verify path UNCHANGED (D-04). |
| No machine-readable output | `--json` global flag, one-object-root contract | This phase (SPINE-01) | Scriptable in PS7; the shape is frozen for 23 commands |
| `box clip` (dedicated command only) | `--clip` global flag on value commands | This phase (SPINE-03) | Any command's output → clipboard via the shared tee |
| Built-in defaults only | `%APPDATA%\box\config.toml` defaults (CLI>env>config>builtin) | This phase (SPINE-05) | Per-user persistent defaults; makes the BLAKE3 break recoverable per-user |

**Deprecated/outdated (do NOT reintroduce):**
- Bare-array JSON root (SUMMARY.md line 94) — superseded by D-01 (always-object root).
- "64-hex verify now maps to BLAKE3" (the stale Pitfall-6 / original HASH-V2-01 wording) — superseded by D-04/D-06; the verify table is UNCHANGED. REQUIREMENTS.md was already amended 2026-06-25 (verified line 169); SUMMARY.md Pitfall 6 amendment is a pre-planning action item flagged below.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Env-tier var spelling `BOX_HASH_DEFAULT_ALGO` (Claude's Discretion — not locked) | Pattern 4 | Low — internal convention; planner may choose another consistent scheme. SPINE-05 only requires the tier EXIST, not its name. |
| A2 | `flush_clip()` confirmation gated on `stderr().is_terminal()` (not stdout) | Pattern 3 | Low — D-08 says "suppressed when not a TTY" without specifying which stream; stderr is the diagnostic channel, so gating on stderr is the consistent reading, but the planner should confirm against the intended UX. |
| A3 | The D-05 probe degrades to the static hint for stdin (`label == "-"`) because the streamed reader can't be re-read | Code Examples | Low-Medium — this is a correct consequence of the single-pass reader (VERIFIED), but the exact degradation behavior is an implementation detail the planner should make an explicit task acceptance criterion. |
| A4 | `init_config()`'s `Err` should flow through `main`'s existing `match result` exit-code path (Option a) rather than a separate inline exit | Code Examples | Low — both wirings produce exit 2; Option (a) keeps one exit-code owner. Planner's structural call. |
| A5 | `dirs::config_dir()` returns `%APPDATA%` (Roaming) on Windows — the correct base for `box\config.toml` | Standard Stack | Low — documented `dirs` behavior (CITED: STACK.md:57); `std::env::var_os("APPDATA")` is the documented fallback. |
| A6 | Config-struct scope = ONLY `default_hash_algo` in Phase 6 (Claude's Discretion: lean) | Pattern 4 | None — explicitly the planner's call per CONTEXT.md; stubbing more fields now is also valid. |

**If this table is empty:** it is not — but every entry is either Claude's Discretion (A1, A6) or a low-risk implementation detail (A2–A5). No locked decision rests on an assumption; all D-01..D-10 claims are CITED to CONTEXT.md and VERIFIED against source.

## Open Questions (RESOLVED)

> Both questions below carry inline recommendations and are implemented in the Phase-6 plans: Q1 (env-tier wiring now) → 06-02 Task 2; Q2 (`init_config()` Err routed through main's exit-2 downcast) → 06-01 Task 3.

1. **Should the env-tier be wired in Phase 6 or stubbed for Phase 7?**
   - What we know: SPINE-05 mandates the env tier in the precedence CHAIN (`cli.or(env).or(config)…`).
   - What's unclear: whether Phase 6 must actually READ `BOX_HASH_DEFAULT_ALGO`, or whether wiring the `.or(env)` slot with a parse fn (even if untested until a value is set) satisfies SPINE-05.
   - Recommendation: wire the full `.or(env).or(config)` chain in `hash` now (it's one extra `.or()`), and add one precedence-matrix unit test that sets the env var — this proves the tier exists and is the template Phase 7 copies. Cheap insurance against re-touching `hash` in Phase 7.

2. **Where exactly does `init_config()` get called so its error reaches the exit-2 downcast?**
   - What we know: `main()`'s exit-code mapping is the single owner of 0/1/2 (verified lines 100-122); `init_config` returns `anyhow::Result`.
   - What's unclear: `init_config` runs BEFORE the dispatch `match` (load-once-before-dispatch), but the `match result` downcast happens AFTER dispatch — the error must be routed to the same downcast.
   - Recommendation: structure so the config-load `Err` is mapped through the same `downcast_ref::<BoxError>()` block (Code Examples Option a). The planner must make this an explicit task step; a naive `init_config()?` in `main` (which returns `ExitCode`, not `Result`) won't compile as-is and could tempt an inline `return ExitCode::from(2)` that bypasses the `error: …` stderr print — avoid that, keep the print + downcast unified.

## Pre-Planning Action Item (from CONTEXT.md D-06 / Specifics)

> Flagged for the planner/executor — a documentation amendment that should be the first task (or a pre-planning edit):

- **REQUIREMENTS.md HASH-V2-01** — ALREADY AMENDED 2026-06-25 (verified line 48 + line 169 footer state "compute default only; bare `--verify <64-hex>` stays SHA-256"). No further action.
- **SUMMARY.md Pitfall 6 / line 179** — STILL carries the stale "the 64-hex tie now maps to BLAKE3" wording (the rejected Option B). This MUST be amended to: *the breaking change is the **compute** default only; bare `--verify <64-hex>` stays SHA-256 with a BLAKE3 diagnostic probe.* This is a pre-planning action item (CONTEXT.md D-06 / Specifics). [CITED: CONTEXT.md D-06; SUMMARY.md line 179 referenced but unverified in this session — planner should confirm and amend]

## Sources

### Primary (HIGH confidence)
- **v1 source, read in full this session:** `src/core/output.rs` (COLOR_ON triad, format_row, atomic-test-isolation pattern), `src/commands/hash/mod.rs` (Option<Algo>, line 162 default, 78-85 verify table, 154-156 bail!, 63-73 Algo enum), `src/commands/uuid/mod.rs` (println loop, format_one), `src/commands/clip/mod.rs` (arboard single-shot main-thread discipline), `src/cli.rs` (no_color global precedent, Commands enum), `src/main.rs` (init_color call site, exit-2 downcast 114-120), `src/core/errors.rs` (BoxError variants), `src/core/mod.rs` (module list), `Cargo.toml` (present deps + dev-deps), `tests/{uuid,hash,clip}.rs` (proven test idioms).
- `.planning/research/ARCHITECTURE.md` — Q1 (--json contract), Q2 (config merge + load code), Q3 (--clip tee), Q5 (BLAKE3 flip), New-vs-Modified ledger (455-471). HIGH (verified against real v1 source per its own header).
- `.planning/phases/06-scriptable-core-foundation/06-CONTEXT.md` — locked decisions D-01..D-10. The authority for all User Constraints.
- `.planning/REQUIREMENTS.md` (SPINE-01/03/05, HASH-V2-01 amended), `.planning/STATE.md` (Accumulated Context, critical pitfalls), `.planning/research/PITFALLS.md` (Pitfalls 1-8), `.planning/research/STACK.md` (toml/dirs rationale, versions).
- docs.rs/toml — `from_str` confirmed (version 1.1.2+spec-1.1.0).

### Tooling verification (this session)
- `cargo search toml` → `toml = "1.1.2+spec-1.1.0"` (current); `cargo search dirs` → `dirs = "6.0.0"` (current). Neither in `Cargo.lock` (genuinely new).
- `slopcheck scan` (v0.6.1) on a stub manifest → `toml [OK]`, `dirs [OK]`. (crossterm [SUS] is a known false-positive against `crossbeam` on the existing manifest — not touched by this phase.)

### Secondary (MEDIUM confidence)
- `.planning/research/SUMMARY.md` — synthesis (line 94 superseded by D-01; Pitfall 6/line 179 superseded by D-04/D-06, amendment pending).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — two new crates version-verified on crates.io, slopcheck-clean, triple-cited (CLAUDE.md/STACK.md/D-1); all other deps already in manifest.
- Architecture: HIGH — every edit point verified against the real v1 source by line number; the spine idiom is a direct clone of the shipped, tested `COLOR_ON` triad.
- Pitfalls: HIGH — drawn from the v2 PITFALLS.md catalogue + STATE.md, cross-checked against the actual `hash`/`clip`/`output` source and the existing `tests/hash.rs` regression backstop.

**Research date:** 2026-06-25
**Valid until:** ~2026-07-25 (30 days — stable domain; the only external moving parts are toml/dirs versions, which are mature). Re-verify crate versions if the manifest edit lands more than a month out.
