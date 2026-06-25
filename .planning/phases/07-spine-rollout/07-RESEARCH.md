# Phase 7: Spine Rollout - Research

**Researched:** 2026-06-25
**Domain:** Mechanical, additive rollout of a frozen `--json`/`--clip` spine across 16 Rust CLI subcommands (Windows PowerShell 7 target)
**Confidence:** HIGH (every claim verified against the real source files this phase touches; the spine primitives and both pilot templates were read in full; no external/network research was needed because this is a closed-codebase mechanical rollout)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

> All four discussed gray areas resolved to the recommended option. Everything else inherits the Phase-6 frozen template verbatim.

**Carried forward from Phase 6 — LOCKED, not re-discussed:**
- **D-01 root rule:** object root always; **scalar commands** → flat object; **multi-capable commands** → `{results:[…],count:N}` even when N=1; **`tree`** → recursive object with `children`. snake_case, `to_writer_pretty`, UTF-8 **no BOM**, single trailing newline.
- **`--clip` (D-07/D-08):** copy-**all** the printed payload, newline-joined, single `trim_end()`; copy-**AND**-print; "Copied to clipboard" confirmation → **stderr**, suppressed when not a TTY; `--clip` forces `COLOR_ON=false`; `--json --clip` copies the whole JSON document; `flush_clip()` runs once on the main thread after successful dispatch; empty output → no-op.
- **Error contract (D-09/D-10):** no JSON error envelope — on failure stdout stays **EMPTY**, human `error:` → stderr, exit **1** (runtime) / **2** (usage). No `{"error":…}` on stdout.
- **No-drift / color-progress hygiene:** one serde struct per command feeds human + JSON; `is_json_on()` fork happens FIRST; progress (indicatif) → stderr, suppressed under `--json`; piped/`--json` output is byte-identical-minus-ANSI.

**Gray Area 1 — filesystem document key:**
- **D-11 — Frozen `{results,count}` everywhere.** `du`, `dupes`, `flatten`, `bulk-rename` all use the same `{results:[…],count:N}` root as the `uuid`/`hash` pilots.
- **D-11a — ⚠ REQUIRED DOC AMENDMENT (pre-planning).** Roadmap **Phase-7 SC1** currently says `box du …--json` yields an object whose **`entries`** array carries size fields. Amend SC1 (and any matching REQUIREMENTS.md/SPINE-02 wording) to say the array key is **`results`** (with `count`). The `tree` "recursive object" half of SC1 is already correct.

**Gray Area 2 — `flatten` / `bulk-rename` under `--json`:**
- **D-12 — `--json` is format-only, orthogonal to `--force`.** dry-run + `--json` → the **plan** as JSON; `--force` + `--json` → the **executed result** as JSON. **`bulk-rename --force --json` now emits the applied renames** (overriding its current silent-on-success behavior — only under `--json`; the human `--force` path stays silent-on-success).
- **D-13 — `dry_run: bool` in the root object.** Per-operation rows: `{src, dst, action, reason}` (`action` mirrors the existing `RowStatus`: copy/rename/skip). The locked summary counts (copied/renamed/skipped + total size for flatten) live as sibling fields in the root alongside `{results,count}`.

**Gray Area 3 — `qr` and `json`:**
- **D-14 — `qr --json` emits encode metadata, NOT glyphs:** `{text, error_correction}`.
- **D-15 — `qr --clip` copies the source text** (SPINE-04 "text payload"), the one deliberate exception to D-07 copy-all.
- **D-16 — `json --json` is identity passthrough.** Emit the **parsed input document verbatim** as pure machine JSON — **NOT** wrapped in `{results,count}`. A documented root-rule exception alongside `tree`.

**Gray Area 4 — field schemas:**
- **D-17 — Nested sub-objects + unified shapes:**
  - **`color`** → `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` (nested).
  - **`epoch`** → unified `{epoch, utc, local}` for **all** input modes (no branching on input direction).
  - **`tree`** → recursive node `{name, type:"dir"|"file", size?, children:[]}` (`size` present for files).
  - **`weather`** → current-only `{location, temperature, unit, conditions, …}` (units read from the API response, never hardcoded).
  - **`dupes`** → `{results:[{size, paths:[…]}], count, wasted_bytes}`.

### Claude's Discretion

- **Exact field names beyond those pinned above** — e.g. base64 (`{input?, output, encoding/mode}`), passgen rows, fortune/8ball/roast (`{text}` or similar single-field object), weather's exact sub-field set. Follow D-01 conventions + the D-17 spirit; keep snake_case.
- **`action` enum string spelling** for D-13 rows (e.g. `"copy"`/`"rename"`/`"skip"`) — match the existing `RowStatus` lowercased.
- **Wrap behavior for soft-wrapped fun commands** (`8ball`/`fortune`/`roast`) under `--json` — emit the unwrapped single string (wrapping is a human-render concern, not data).
- **Whether scalar single-line commands** (`base64`, `epoch`, fun trio, `color`) are "scalar→flat object" vs need a `results` array — per D-01 they are flat scalar objects unless they can return >1 row (`passgen --count`, multi-capable → `{results,count}`).

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within Phase 7 scope. Every per-command **depth** idea (new fields/flags) is roadmapped to Phases 8–10 and deliberately kept out: Phase 7 wraps only the *current* data each command produces in the spine.

**Out-of-scope reminders from the domain section:**
- Per-command **depth flags** (Phases 8–10). `weather` is **current-only** here; `hash` stays single-file; `bulk-rename`/`flatten` get no new filters.
- `box config` / `box completions` (Phase 11).
- **No new deps** — Phase 7 needs none.
- **Display-only commands excluded from both flags:** `matrix`, `pomodoro`, `lolcat`, `ascii`, `clip` (roadmap SC4) — must *omit* `--json`/`--clip`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPINE-02 | `--json` on every applicable value-producing command: `base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay`, `du`, `tree`, `dupes`, `flatten`, `bulk-rename`, `json`, `qr`, `weather` (16) | Per-command JSON-shape map below (one row per command) + the no-drift serde-struct pattern from the `uuid`/`hash` pilots. Wave 7a/7b/7c sequencing already isolates the structurally-novel cases (`tree` recursion, `json` passthrough, `flatten`/`bulk-rename` plan-shape). |
| SPINE-04 | `--clip` on the 6 not-yet-covered single-result commands: `passgen`, `color`, `base64`, `epoch`, `json`, `qr` (`uuid`+`hash` shipped in Phase 6) | The `out_line` tee handles `--clip` automatically for every command whose human path routes through `out_line` — EXCEPT `qr`, which is the D-15 exception (clip copies the source text, not the rendered glyphs). The Validation Architecture defines the per-command clip assertion. |
</phase_requirements>

## Summary

Phase 7 is exactly what it claims to be: a **mechanical, additive rollout** of a spine that is already complete and frozen. The spine primitives (`is_json_on`, `emit_json`, `out_line`, `flush_clip`, `is_color_on`, `init_output`) all exist in `src/core/output.rs` and are live (their forward-compat `#[allow(dead_code)]` was removed in Phase 6 when `uuid`/`hash` became the first consumers). The global `--json`/`--clip` flags are already wired on `Cli` (`src/cli.rs:27-32`) and `main.rs` already calls `init_output` (line 84) and `flush_clip` (line 111) in the load-bearing order. **No `cli.rs` / `main.rs` plumbing changes are needed, and no command's `Args` struct shadows the globals** — verified by reading every target command's `#[derive(Args)]`. **No new dependencies** — serde, serde_json, and arboard are all present in `Cargo.toml`.

The per-command cost is the frozen template applied once: add a `#[derive(serde::Serialize)]` output struct, fork on `is_json_on()` **first**, route the human branch through `out_line`, and copy the `tests/uuid.rs::json_purity` test. **8 of the 16 commands already compute their value into an internal struct or a simple scalar** (`uuid`/`hash` are the template; `du`/`tree`/`dupes`/`flatten`/`bulk-rename` have internal `Row`/`Child`/`DupeGroup`/`PlanItem` structs that need `#[derive(Serialize)]` + field rename only — the data model already exists). The genuinely novel cases the wave-ordering is meant to surface are: **`tree`** (its current model is a flat printing recursion, not a node tree — D-17 needs a real recursive `{name,type,size?,children}` struct built alongside the printer), **`json`** (identity passthrough, D-16 — emit the parsed `Value` verbatim, NOT wrapped), **`qr`** (D-14 metadata + D-15 clip-copies-text, the one break from copy-all), and **`weather`** (its `f64` temp/wind/humidity fields are a NaN/precision watch-item, and the unit label must come from `current_units`, never hardcoded).

**Primary recommendation:** Plan three waves exactly as the roadmap sketches (7a pure transforms → 7b filesystem buffered-rows → 7c json/qr/weather). For each command: copy the pilot template, give the *already-computed* value a serde struct, fork `is_json_on()` first, route human output through `out_line`, and add the `json_purity` + (for `--clip` commands) the `clip_roundtrip` test. The only per-command "thinking" is the field schema (mostly locked by D-17, the rest discretion) and, for `flatten`/`bulk-rename`, sharing the existing `Plan` between the JSON document and the human render so the two cannot drift. Amend roadmap SC1 (`entries` → `results`, D-11a) as the first planning step.

## Architectural Responsibility Map

This phase is single-tier (a local CLI binary). The relevant "tiers" are the command module vs the shared core. Mapping each capability to its owner prevents re-implementing spine logic per command.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| JSON serialization (`to_writer_pretty`, no-BOM, trailing `\n`, clip-tee) | `core::output::emit_json` | — | Single serializer = no-drift guarantee. Commands NEVER call serde_json directly for primary output (exception: `json` passthrough, D-16, which already owns its own serde call). |
| `--clip` capture + flush | `core::output::out_line` + `flush_clip` (main) | — | The tee is automatic for any command using `out_line`. Per-command clipboard code is forbidden (it would re-implement arboard 16×). |
| `--json` mode gate | `core::output::is_json_on()` | command `run()` | Command checks `is_json_on()` FIRST; the only stdout write reachable under `--json` is `emit_json`. |
| Color force-off under json/clip | `core::output::init_output` (main) | — | Already forces `COLOR_ON=false`; commands keep gating styling on `is_color_on()`. No per-command change. |
| Per-command output struct + field schema | command `run()` | D-17 / discretion | Each command owns its `#[derive(Serialize)]` struct; the shape rules are D-01/D-17. |
| Value computation (the actual work) | command `run()` / pure helpers | — | UNCHANGED. Phase 7 wraps the existing value; it does not re-compute anything. |
| Progress (indicatif) routing | `core::output` convention (stderr, hidden under `--json`) | command | **Not relevant to Phase 7** — none of the 16 commands have progress bars yet (those land in Phase 8). Noted only so the planner does not introduce one. |

## Standard Stack

**No new crates.** Phase 7 consumes what is already in `Cargo.toml` (verified). The relevant existing dependencies:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1 (derive) | `#[derive(serde::Serialize)]` on every output struct | `[CITED: Cargo.toml]` Already a direct dep (the `derive` feature was added for weather's `Deserialize` in v1). Every Phase-7 output struct derives `Serialize`. |
| serde_json | 1.0.150 (`preserve_order`) | `emit_json` serializer; `json` command's `Value` | `[CITED: Cargo.toml]` Already present. `preserve_order` keeps `json --json` identity-passthrough key order. `arbitrary_precision` is deliberately OFF (D-04 landmine) — do NOT enable it. |
| arboard | 3.6.1 (no default features) | clipboard write inside `flush_clip` | `[CITED: Cargo.toml]` Already present (text-only; image-data feature off). Phase 7 writes NO per-command clipboard code — `out_line`/`flush_clip` own it. |

### Supporting (already used by the spine, no action)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| owo-colors | 4.3 | `is_color_on()`-gated styling | Unchanged — every command keeps its existing `is_color_on()` gate; `init_output` already force-disables color under json/clip. |
| anyhow | 1.0 | error propagation | Unchanged — the D-09/D-10 error contract (empty stdout, stderr message, exit 1/2) is already enforced by `main.rs`. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `emit_json` (shared) | per-command `serde_json::to_writer` | ✗ Re-implements the no-BOM/trailing-newline/clip-tee policy 16×; defeats the no-drift guarantee. The ONLY sanctioned direct serde_json call is `json`'s identity passthrough (D-16), and it already owns one. |
| `out_line` (shared tee) | per-command arboard | ✗ Re-implements main-thread clipboard discipline 16×. Forbidden. |

**Installation:** None. `cargo build` only.

**Version verification:** Not applicable — no packages are added or changed. All dependencies were verified present in `Cargo.toml` during this research (read in full).

## Package Legitimacy Audit

> **Not applicable.** Phase 7 installs **zero** external packages. The no-new-deps claim from CONTEXT.md (`<domain>` "New deps. Phase 7 needs **none** (`serde`/`serde_json`/`arboard` already present)") is **CONFIRMED** by reading `Cargo.toml`: serde 1 (derive), serde_json 1.0.150 (preserve_order), and arboard 3.6.1 are all present. slopcheck / registry verification is moot because no package is being added. If a future planner deviates and adds a dependency, that deviation is out of Phase-7 scope and must trigger a separate legitimacy gate.

## Architecture Patterns

### System Architecture Diagram

```
box <cmd> [--json] [--clip] [args]
        │
   main.rs (UNCHANGED — already wired in Phase 6)
        ├─ Cli::try_parse()  ── lifts global --json / --clip
        ├─ init_config()     ── (irrelevant to Phase 7; format-only phase)
        ├─ init_color(no_color)
        ├─ init_output(json, clip)  ── sets JSON_ON / CLIP_ON; forces COLOR_ON=false under either
        │
        └─ Commands::<Cmd>(args).run()        ◄── THE ONLY PER-COMMAND EDIT IS HERE
               │
               ├─ compute the value  (UNCHANGED pure helpers)
               │       └─► fill ONE #[derive(Serialize)] struct       ◄── new
               │
               ├─ if is_json_on() {                                    ◄── fork FIRST (Pitfall 1)
               │       emit_json(&doc)   ──► stdout (pure JSON) + tee→CLIP_BUF (if --clip)
               │   } else {
               │       out_line(&human_line)  ──► stdout + tee→CLIP_BUF (if --clip)   ◄── was println!
               │   }
               │
        ◄── flush_clip()  ── main thread, once, on Ok dispatch (UNCHANGED)
        └─ 0/1/2 exit map (UNCHANGED)
```

The data flow that matters: **the value is computed exactly once into a struct; `is_json_on()` forks between `emit_json(&struct)` and a human render *of the same struct*.** That structural single-source is the no-drift guarantee (the JSON can never report a value the human view never showed).

### Recommended Project Structure
No new files. Each of the 16 edits is confined to its existing `src/commands/<cmd>/mod.rs`, plus its existing `tests/<cmd>.rs` (cowsay uses `tests/cmd/*.trycmd` for its end-to-end lock and `src/commands/cowsay/mod.rs` unit tests for the bubble — its `--json` purity test goes in a new or existing harness; see Validation Architecture).

```
src/commands/<cmd>/mod.rs   # + #[derive(Serialize)] struct, is_json_on() fork, out_line routing
tests/<cmd>.rs              # + json_purity test (copied from tests/uuid.rs), + clip_roundtrip for --clip cmds
```

### Pattern 1: The frozen pilot template (copy-me)
**What:** One serde struct feeds both paths; fork on `is_json_on()` first; human path uses `out_line`.
**When to use:** Every one of the 16 commands.
**Example (the literal `uuid` pilot — `src/commands/uuid/mod.rs:52-83`):**
```rust
// Source: src/commands/uuid/mod.rs (the frozen Phase-6 template)
let rows: Vec<UuidRow> = /* …compute once… */;
if crate::core::output::is_json_on() {
    let doc = UuidOutput { count: rows.len(), results: rows };
    crate::core::output::emit_json(&doc)?;
} else {
    for r in &rows {
        crate::core::output::out_line(&r.uuid);   // NOT println! — tees to clip
    }
}
Ok(())
```

### Pattern 2: Scalar → flat object (no `results` wrapper)
**What:** A command that can only ever produce ONE result emits a flat object, not `{results,count}`.
**When to use:** `base64`, `epoch`, `color`, `8ball`, `fortune`, `roast`, `cowsay`, `qr`, `weather` (single-value commands). Per D-01 + the Discretion note, these are flat scalar objects.
**Example (schema, discretion fields):**
```rust
#[derive(serde::Serialize)]
struct Base64Output { output: String, mode: &'static str /* "encode"|"decode" */ }
// emit_json(&Base64Output { … })  — NOT wrapped in results/count
```

### Pattern 3: Multi-capable → `{results,count}`
**What:** A command that *can* return >1 row uses the always-wrapped `{results,count}` shape even when N=1.
**When to use:** `passgen` (`--count`/`--words` produce N lines), and all four filesystem multi-row commands (`du`, `dupes`, `flatten`, `bulk-rename`) per D-11.
**Example:** identical to the `uuid`/`hash` pilots — `{results: Vec<Row>, count: usize}` plus, for `flatten`/`bulk-rename`, the D-13 sibling fields (`dry_run`, summary counts).

### Pattern 4: The two documented root-rule exceptions
- **`tree` (D-17):** recursive node object `{name, type:"dir"|"file", size?, children:[]}` — NOT `{results,count}`. Build a real node tree alongside (or instead of) the flat printer.
- **`json` (D-16):** identity passthrough — `emit_json(&value)` where `value` is the parsed `serde_json::Value`. Emit it VERBATIM, NOT wrapped. (`box json` already produces pure uncolored JSON when piped; `--json` forces that purity even on a TTY.)

### Anti-Patterns to Avoid
- **Two code paths building human text vs JSON independently** — they drift. One struct, two renderings (the no-drift rule, locked).
- **`println!` instead of `out_line` on the human primary-output path** — breaks `--clip` capture for that command (the line never reaches `CLIP_BUF`). The exception is *secondary* chrome that must NOT be copied (rare here).
- **A bare array at the JSON root for a multi-row command** — D-01 says object root always (so a sibling key can be added in Phase 8 without breaking consumers). `{results,count}`, never `[…]`.
- **Hardcoding `weather` units** — the label MUST come from `current_units` (D-17 / Pitfall WTHR-3: imperial wind is labeled `mp/h`, not `mph`).
- **Wrapping `json --json` in `{results,count}`** — D-16: identity passthrough only.
- **Copying the rendered QR glyphs to the clipboard** — D-15: `qr --clip` copies the *source text*.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization (no BOM, trailing `\n`, clip-tee) | per-command `serde_json::to_writer` | `core::output::emit_json(&doc)` | One serializer = the no-drift + no-BOM + clip-tee policy in one place (`src/core/output.rs:124`). |
| Clipboard write | per-command `arboard::Clipboard` | `out_line` (tee) + `flush_clip` (main) | Main-thread discipline + trailing-newline + empty-no-op + stderr-confirmation already correct (`src/core/output.rs:143,160`). |
| JSON-mode detection | a `json: bool` field per `Args` | `is_json_on()` | The global-flag→atomic idiom is already live; a per-command field would be 16× duplication (Anti-Pattern 1, ARCHITECTURE.md Q1). |
| Color suppression under json/clip | per-command checks | `init_output` (already forces `COLOR_ON=false`) | Done once in main; commands keep their existing `is_color_on()` gate untouched. |
| JSON-purity regression test | a bespoke assertion per command | copy `tests/uuid.rs::json_purity` | The exact no-ANSI + no-BOM + single-document + shape assertions are frozen (`tests/uuid.rs:135-179`). |

**Key insight:** Phase 7 invents *nothing*. Every primitive, every test shape, and (for 8 commands) the internal data model already exists. The risk is purely in the field-schema choices and in correctly threading the *existing* internal structs (`Row`, `Child`, `DupeGroup`, `PlanItem`) through serde + `out_line` without re-computing or drifting.

## Per-Command JSON Shape Verification (the SPINE-02 core map)

This is the heart of the research. For each of the 16 commands: its locked/discretion shape, where it currently computes vs renders, and the per-command surprise (if any). Read each command's `mod.rs` line refs to confirm.

### Wave 7a — pure transforms (simplest; surface any surprise here)

| Cmd | Current model | `--json` shape (D-01/D-17 + discretion) | `--clip`? | Struct cost | Notes / surprise |
|-----|---------------|------------------------------------------|-----------|-------------|------------------|
| `base64` | scalar; encodes to a `String` via `println!` (`base64/mod.rs:57`); **decode writes raw bytes** via `stdout().write_all` (`:52`) | flat: `{output, mode}` (+ optional `input`), discretion | **yes** | trivial new struct | ⚠ **Surprise (decode):** decode output can be **arbitrary non-UTF-8 bytes** (binary-safe by design, `:51-54`). A JSON string field cannot hold non-UTF-8. Planner decision: under `--json`, base64-decode output that is not valid UTF-8 must be handled deliberately — emit a base64-of-the-bytes field, or `to_string_lossy` with a documented marker, or refuse non-UTF-8 decode under `--json` (exit 1, empty stdout). Encode is always ASCII → safe. **This is the canonical "surprise surfaces on base64, not flatten" the wave-ordering was designed to catch.** Recommend: encode → flat object; decode under `--json` → emit the decoded bytes as a base64 string (round-trippable) OR document `to_string_lossy`. Flag to user. |
| `epoch` | three modes (now / int→dates / string→ts) via `println!` (`epoch/mod.rs:45-58`) | **D-17 unified** `{epoch, utc, local}` for ALL modes — no branching on input direction | no (not in SPINE-04 list) | small struct; compute all three fields regardless of mode | The human path branches (one-line ts vs two-line dates); the JSON path must produce the **same unified shape** whichever input. Compute `epoch` (i64), `utc` string, `local` string once. `now` mode: epoch = now. int mode: epoch = the int. string mode: epoch = parsed ts. All three then derive utc/local. |
| `color` | scalar; computes `(r,g,b)` + `(h,s,l)`, prints aligned block + swatch (`color/mod.rs:44-58`) | **D-17 nested** `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` | **yes** | nested structs (`Rgb{r,g,b}`, `Hsl{h,s,l}`) | Clean. The swatch is display-only; omit it from JSON. Hex string format is discretion (recommend `#RRGGBB` lowercase to match the existing `:44` uppercase? — note current human uses `#{r:02X}` UPPERCASE; pick one and lock in the struct). |
| `passgen` | **multi-capable** (`--count` N lines, `--words` passphrase) via `println!` (`passgen/mod.rs:116,126`) | **`{results,count}`** (multi-capable, D-01) | **yes** | `{results: Vec<{password}>, count}` (field name discretion) | ⚠ **Security note (Pitfall):** `passgen --clip` copies a secret to the clipboard — already opt-in (the user typed `--clip`), acceptable, but the human path must route through `out_line` for the tee to work. No new clip code. Each generated line becomes one result row. |
| `8ball` | scalar; one answer via `println!` (`eight_ball/mod.rs:77`) | flat `{text}` (or `{answer}`, discretion) | no | trivial | Question arg is display-only/ignored — do not include it. |
| `fortune` | scalar; one aphorism, **soft-wrapped** for the human path (`fortune/mod.rs:48-54`) | flat `{text}` — **emit the UNWRAPPED single string** (Discretion: wrapping is a human concern) | no | trivial | The wrap happens only in the human branch; the JSON branch emits `chosen` verbatim. |
| `roast` | scalar; identical shape to fortune (`roast/mod.rs:42-48`) | flat `{text}` unwrapped | no | trivial | Same as fortune. |
| `cowsay` | scalar; builds bubble + cow art; prints via **`println!` + `print!` + `println!`** (`cowsay/mod.rs:58-61`) | flat `{text}` (the spoken message) — discretion; the ASCII bubble/cow is a *visual*, like qr glyphs | **no** (NOT in SPINE-04 list) | trivial | ⚠ **Surprise (mixed print):** the human render uses three calls including a bare `print!` for the cow (`:59`). Since cowsay is `--json`-only (no `--clip`), the human path does **not** strictly need `out_line` (no clip tee required). But the no-drift rule still applies: the `--json` doc should carry the *input text*, not the rendered bubble. Recommend: `--json` → `{text: <the wrapped-or-raw message>}`; leave the human bubble path as-is (println!/print! is fine because there is no clip to tee). Decide whether JSON `text` is the raw input or the wrapped lines (recommend raw input string). |

### Wave 7b — filesystem buffered-rows (the structural validators)

| Cmd | Current model | `--json` shape | `--clip`? | Struct cost | Notes / surprise |
|-----|---------------|----------------|-----------|-------------|------------------|
| `du` | internal `Row {name, is_dir, size}` (`du/mod.rs:67`), `collect`ed + `sort_by`ed BEFORE printing (`:96-127`) | `{results:[{name, size, is_dir/type}], count, …}` (D-11) | no | add `#[derive(Serialize)]` to `Row` + rename fields; data model already buffered | Clean — the rows are already collected and sorted before render (`:104-113`). Sibling fields: consider `total_bytes` + `total_children` (already computed at `:101-102`) as extra root keys (discretion). `size: u64` — bare u64 is locked (D-3); document the >2^53 caveat. |
| `tree` | **flat printing recursion** — `render_dir` walks + `println!`s as it descends (`tree/mod.rs:119-179`); `Child {name,is_dir,size,path}` is per-level only, never assembled into a node tree | **D-17 recursive** `{name, type:"dir"\|"file", size?, children:[]}` | no | ⚠ **Surprise (real work):** the current model NEVER builds a recursive structure — it prints inline. The `--json` path needs a **new recursive node builder** (`fn build_node(dir) -> Node` that recurses and collects `children: Vec<Node>`), parallel to `render_dir`. `size` present only for files (D-17). | **The single biggest per-command cost in 7b.** The human printer stays; the JSON path is a sibling recursion. Both must use the same `read_children` + `sort_children` so order matches (no-drift). `type` is the string `"dir"`/`"file"` (D-17). Root node = the target dir. |
| `dupes` | internal `DupeGroup {size, paths: Vec<PathBuf>}` (`dupes/mod.rs:53`), grouped + sorted before `render` (`:108`) | **D-17** `{results:[{size, paths:[…]}], count, wasted_bytes}` | no | add `#[derive(Serialize)]` to `DupeGroup` + rename; `wasted_bytes` already computed by `wasted_space()` (`:199`) | Clean — model exists. `paths` are `PathBuf` → `to_string_lossy()` per D-4 (NTFS non-UTF-8 path policy). `count` = number of groups. Empty result: `{results:[], count:0, wasted_bytes:0}` (NOT the "No duplicate files found." human line — that is human chrome). |
| `flatten` | unified `Plan {items: Vec<PlanItem{src,src_label,dst_name,kind,reason}>, to_copy, renamed, skipped}` (`flatten/mod.rs:71-91`); ONE plan feeds both dry-run + real-run already | **D-12/D-13:** `{results:[{src, dst, action, reason}], count, dry_run: bool, copied, renamed, skipped, total_bytes}` (sibling summary counts) | no | add serde to a *projection* of `PlanItem` (rename `kind`→`action` lowercased, `dst_name`→`dst`); `dry_run` from `self.dry_run` | ⚠ **D-12 nuance:** `--json` is orthogonal to dry-run/`--force`. dry-run+`--json` → the plan; real+`--json` → executed result (with actual `copied`/`bytes_written`). The existing code computes `copied`+`bytes_written` only on the real path (`:150-151`); the JSON real-run doc must capture those. `action` enum = `RowStatus` lowercased: `"copy"`/`"rename"`/`"skip"` (discretion spelling). `dst` is `None` for skips. **Per-row JSON should NOT include the aligned/truncated `format_row` output — that is human layout; serialize the raw fields.** |
| `bulk-rename` | `Plan {items: Vec<PlanItem{src,parent,old_name,src_label,new_name,kind,reason}>, to_rename, unchanged, skipped}` (`bulk_rename/mod.rs:480-487`); abort-all-before-any pre-flight | **D-12/D-13:** `{results:[{src, dst, action, reason}], count, dry_run, renamed/to_rename, unchanged, skipped}` | no | serde projection of `PlanItem` | ⚠ **D-12 override (explicit in CONTEXT):** `bulk-rename --force --json` MUST emit the applied renames — overriding the current **silent-on-success** human behavior (`:381` only prints a "Done:" summary). The human `--force` path stays silent; only `--json` emits rows. ⚠ **Conflict/abort under `--json`:** when pre-flight finds conflicts the command `bail!`s (exit 1) and prints the plan to stdout (`:320-322`). Under D-09/D-10 a failure means **stdout stays EMPTY** and the error goes to stderr. So under `--json`, the abort path must NOT print the plan-with-conflicts to stdout — it must keep stdout empty and exit 1 (the conflict detail goes to stderr, or is simply the error message). **This is a real behavioral fork the planner must specify.** |

### Wave 7c — remaining odd-fits

| Cmd | Current model | `--json` shape | `--clip`? | Struct cost | Notes / surprise |
|-----|---------------|----------------|-----------|-------------|------------------|
| `json` | parses input to `serde_json::Value`, pretty/compact/colorized print (`json/mod.rs:51-77`) | **D-16 identity passthrough** — `emit_json(&value)` VERBATIM, NOT wrapped | **yes** (SPINE-04) | none — it already has the `Value` | ⚠ **The only command that calls serde_json directly for primary output, and that is correct here (D-16).** Under `--json`: parse → `emit_json(&value)` (which is pure, no-BOM, trailing newline). `--clip` under `--json` copies that document (via `emit_json`'s tee). **`--clip` without `--json`:** the human path currently uses `print!(colorize)` / `println!(to_string_pretty)` / `println!(to_string)` (`:69,73,63`) — NONE go through `out_line`. To support `--clip` (SPINE-04), the **plain non-color pretty path** must route through `out_line` (or the clip buffer must be fed). Under `--clip`, `COLOR_ON=false` (forced by `init_output`), so the colorize branch is never taken — the plain `to_string_pretty` branch runs; route THAT through `out_line`. `--compact --clip` should copy the compact form. **Invalid JSON → exit 1, empty stdout (D-09)** — unchanged (`:60`). |
| `qr` | renders Unicode half-blocks via `render_qr` → `println!` (`qr/mod.rs:63-65`) | **D-14** `{text, error_correction}` — metadata, NOT glyphs (the `▀▄` block is a visual) | **yes (D-15 exception)** | small struct; `error_correction` is the fixed `"M"` literal today | ⚠ **D-15 — the one break from copy-all:** `qr --clip` copies the **source text** (the encoded URL/text), NOT the rendered glyphs (pasting half-blocks as clipboard text is garbage in most apps). Because the human path `println!`s the glyph block, routing it through `out_line` would copy the glyphs — WRONG. So `qr` must **NOT** rely on the `out_line` tee for `--clip`; instead it must feed the *input text* to the clip buffer explicitly. **Two correct approaches:** (a) keep `println!` for the glyph display and add a dedicated clip-feed of the input text; or (b) under `--clip`, the command appends the source text to `CLIP_BUF` itself. ⚠ This means `qr` needs a **small spine helper or an explicit clip-feed** that the planner must design — `out_line` alone cannot express "print X, copy Y". Confirm whether a `core::output` helper exists for "feed clip buffer without printing" — currently it does NOT (only `out_line` prints+tees and `emit_json` serializes+tees). **The planner likely needs a tiny new primitive (e.g. `clip_only(&str)`) OR `qr` writes to `CLIP_BUF` via a new exposed function.** This is the one place Phase 7 may touch `core::output` — flag it. `error_correction` = `"M"` (the fixed v1 level; QR-V2-01 adds `--error-correction` in Phase 9). |
| `weather` | fetches `ForecastResp`; prints aligned labeled block (`weather/mod.rs:115-122`); echoes resolved location to **stderr** (`:94`) | **D-17 current-only** `{location, temperature, unit, conditions, …}` — units from `current_units`, NEVER hardcoded | no | small struct from the already-parsed `forecast` | ⚠ **Float watch-item (Pitfall 2):** `temperature_2m`, `wind_speed_10m`, `relative_humidity_2m` are all `f64` (`weather/mod.rs:294-298`). Open-Meteo returns finite numbers, so NaN/Inf is unlikely, but a JSON `f64` that is NaN/Inf is invalid JSON — serde_json errors on serialize. Low risk (real API data), but the planner should be aware. ⚠ **Unit label (Pitfall WTHR-3):** the `unit` field MUST be read from `forecast.current_units` (`:106-107`), never the request param — imperial wind is labeled `mp/h`. ⚠ **stderr echo:** the "Resolved …" line is already stderr (`:94`), so it does not contaminate `--json` stdout — GOOD, no change needed. `location` = the resolved label. |

## Common Pitfalls

### Pitfall 1: `--json` stdout contamination (the #1 v2 failure mode)
**What goes wrong:** A stray byte on stdout breaks `ConvertFrom-Json`.
**Why it happens (Phase-7-specific):** Several commands have *multiple* stdout writes on the human path that must ALL be gated behind the `else` of `is_json_on()`: `du` (rows + blank line + summary, `du/mod.rs:127,132-136`), `tree` (root label + tree + blank + summary, `:103,108-109`), `dupes` (groups + summary, `:215-232`), `flatten`/`bulk-rename` (rows + summary). Also `weather`'s stderr echo (`:94`) is fine, but the planner must ensure NO human chrome (the "No duplicate files found." line, the "N directories, M files" summary, the `Resolved →` is stderr OK) reaches stdout under `--json`.
**How to avoid:** Fork on `is_json_on()` **first**, before ANY stdout write. The only stdout write reachable under `--json` is the single `emit_json`. Copy the `json_purity` test (asserts stdout parses as exactly one value + no 0x1B + no BOM).
**Warning signs:** `ConvertFrom-Json` "Unexpected character"; a valid-shaped object with a trailing summary line.

### Pitfall 2: `--json` float / large-number / non-UTF-8-path correctness
**What goes wrong:** NaN/Inf floats are invalid JSON; bare `u64 > 2^53` loses precision in JS consumers; non-UTF-8 NTFS paths corrupt or panic.
**Phase-7 exposure:** floats in `weather` (temp/wind/humidity, `f64`); large `u64` sizes in `du`/`dupes` (bare u64 per D-3 — document the caveat); non-UTF-8 paths in `du`/`tree`/`dupes`/`flatten`/`bulk-rename` (all already use `to_string_lossy()` for display, which is the D-4 policy — confirm the JSON path does the same and NEVER `to_str().unwrap()`).
**How to avoid:** `to_string_lossy()` for all path fields (D-4, already the codebase convention); bare `u64` for sizes (D-3, documented); for `weather` floats, real API data is finite — low risk, but never serialize a hand-computed `f64` that could divide-by-zero (none here).

### Pitfall 3: `--clip` capture gaps (human path not routed through `out_line`)
**What goes wrong:** `--clip` copies nothing (or partial output) because the human primary output still uses `println!`, so the line never reaches `CLIP_BUF`.
**Phase-7 exposure:** the SPINE-04 commands (`base64`, `epoch`, `color`, `passgen`, `json`, `qr`) currently use `println!`/`print!`/`write_all`. Each must route its primary output through `out_line` — EXCEPT `qr` (D-15 special, see below) and `base64 --decode` (raw-bytes path, which cannot use the line-oriented `out_line` for binary output — planner must decide clip semantics for binary decode).
**How to avoid:** Replace the primary `println!` with `out_line` on the human branch. Verify with the `clip_roundtrip` test (copied from `tests/uuid.rs:237`).

### Pitfall 4: `qr --clip` copies the wrong thing (D-15)
**What goes wrong:** Routing the glyph block through `out_line` copies the half-block ▀▄ characters — useless as clipboard text.
**Why it happens:** `out_line` prints AND tees the same string; qr needs to print glyphs but copy the *source text*.
**How to avoid:** Do NOT route the glyph render through `out_line`. The planner must provide a way to feed the source text to the clip buffer without printing it — **this likely requires a tiny new `core::output` primitive** (e.g. `clip_feed(&str)` that pushes to `CLIP_BUF` only when `CLIP_ON`, no stdout). This is the one place Phase 7 may legitimately touch `core::output`. Lock it with a test asserting the clipboard payload equals the input string, NOT the rendered block (CONTEXT § specifics).

### Pitfall 5: `bulk-rename --json` abort/conflict path violating D-09 (empty-stdout-on-error)
**What goes wrong:** The current conflict path prints the plan-with-conflicts to **stdout** then `bail!`s (`bulk_rename/mod.rs:320-322`). Under D-09/D-10, a failed command must leave **stdout EMPTY**.
**How to avoid:** Under `--json`, the abort path must NOT write the plan to stdout. Keep stdout empty, send the conflict explanation to stderr (or just the error), exit 1. The planner must specify this fork explicitly — it is a behavioral change for the `--json` abort case.

### Pitfall 6: `base64 --decode --json` with non-UTF-8 output
**What goes wrong:** Decoded bytes can be arbitrary binary; a JSON string cannot hold non-UTF-8.
**How to avoid:** Planner decision (flag to user): either (a) emit decoded output as a base64 string field under `--json` (round-trippable, lossless), or (b) `to_string_lossy()` with a documented marker, or (c) refuse non-UTF-8 decode under `--json` (exit 1, empty stdout). Encode is always ASCII-safe. **This is the deliberate "surface the surprise on base64" case.**

### Pitfall 7: drift between the human render and the JSON doc
**What goes wrong:** The JSON path re-derives a value differently from the human path.
**How to avoid:** ONE struct feeds both. For `tree` (which needs a parallel recursion) and `flatten`/`bulk-rename` (which project `PlanItem`), ensure both renderings consume the **same** computed data (same `read_children`/`sort_children` for tree; the same `Plan` for flatten/bulk-rename). The `json_purity` test plus a value-equivalence assertion (JSON field == the value the human line shows) is the backstop.

## Runtime State Inventory

> Phase 7 is a pure **additive code change** (new struct + fork + test per command). It is NOT a rename/refactor/migration. There is no stored data, live-service config, OS-registered state, secret, or build artifact that embeds a Phase-7-changed string. The `--json`/`--clip` flags already exist on disk's `Cli` from Phase 6.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 7 reads/wraps existing computed values; writes nothing persistent. | None |
| Live service config | None — `weather` calls Open-Meteo (keyless, read-only) but Phase 7 changes only how the *response* is rendered, not the request. | None |
| OS-registered state | None — `--clip` writes to the Windows clipboard at runtime (transient), not a registration. The clipboard write path (`flush_clip`) is unchanged from Phase 6. | None |
| Secrets/env vars | None new. (`passgen --clip` puts a secret on the clipboard at runtime — opt-in, documented; not a stored secret.) | None |
| Build artifacts | None — no `Cargo.toml` change, no new crate, no codegen. The binary recompiles; `box` is binary-only (`cargo test --bin box`, not `--lib`). | None |

**Nothing found in any category — verified by reading every target `mod.rs`, `Cargo.toml`, and the spine primitives. This phase adds code; it migrates no runtime state.**

## Code Examples

### The JSON-purity test (copy-me, the frozen template)
```rust
// Source: tests/uuid.rs:135-179 (the frozen Phase-7 JSON-purity template)
#[test]
fn json_purity() {
    let out = cmd_output(&["--json"]);            // per-command runner, NO_COLOR=1
    assert!(out.status.success());

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. shape assertions per the command's locked schema (results/count, or flat, or recursive).
    //    … command-specific …

    // 4. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --json stdout");
    // 5. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(&out.stdout[..3.min(out.stdout.len())], b"\xEF\xBB\xBF", "no BOM");
}
```

### The clip round-trip test (copy-me, for SPINE-04 commands)
```rust
// Source: tests/uuid.rs:237-269 — #[ignore]d live-clipboard round-trip
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    let printed = { /* run `box <cmd> --clip`; capture trimmed stdout */ };
    let pasted  = { /* run `box clip --paste`; capture trimmed stdout */ };
    assert_eq!(pasted, printed, "--clip must copy exactly the printed primary result");
}
// For qr (D-15): the assertion is that `pasted` == the INPUT string, NOT the rendered glyph block.
```

### The pilot fork (copy-me, with a path-bearing row — hash variant)
```rust
// Source: src/commands/hash/mod.rs:266-282 — the second reference (path-bearing rows)
if crate::core::output::is_json_on() {
    let doc = HashOutput { count: 1, results: vec![HashRow { path: label.clone(), algo, digest: computed }] };
    crate::core::output::emit_json(&doc)?;
} else {
    crate::core::output::out_line(&format!("{computed}  {label}"));
}
```

## State of the Art

| Old Approach (pre-Phase-6) | Current Approach (frozen, Phase 6) | When Changed | Impact |
|----------------------------|-------------------------------------|--------------|--------|
| Bare `println!` per command, human-only | `is_json_on()` fork → `emit_json` \| `out_line`, one serde struct | Phase 6 (06-02) | Phase 7 applies this to 16 commands mechanically. |
| No clipboard tee | `out_line`/`emit_json` tee to `CLIP_BUF`, `flush_clip` in main | Phase 6 | `--clip` is automatic for `out_line` users (qr is the exception). |
| `entries` array key (roadmap SC1 draft) | `results` array key (D-11) | Phase-7 discuss (D-11a) | **Doc amendment required** before/as the first planning step. |

**Deprecated/outdated:**
- Roadmap SC1's `entries` wording — superseded by D-11 (`results`). Amend per D-11a.

## Assumptions Log

> Claims requiring user/planner confirmation before becoming locked. (Most of Phase 7 is locked by D-01..D-17; the items below are the genuine open schema/behavior decisions surfaced by reading the code.)

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `base64 --decode --json` with non-UTF-8 output needs an explicit policy (base64-string field / lossy+marker / refuse) | Wave 7a + Pitfall 6 | A panic or corrupt JSON on binary decode; or a silently lossy field a consumer feeds back wrong. Needs a user decision. |
| A2 | `qr --clip` (D-15) requires a NEW `core::output` primitive (print glyphs, copy source text) because `out_line` cannot express "print X, copy Y" | Wave 7c + Pitfall 4 | Without it, the glyphs get copied (wrong) OR the planner re-implements clip per-command. This is the one sanctioned `core::output` touch — confirm scope. |
| A3 | `bulk-rename --json` abort/conflict path must keep stdout EMPTY (D-09) and not print the plan-with-conflicts to stdout | Wave 7b + Pitfall 5 | Violates the empty-stdout-on-error contract; breaks `ConvertFrom-Json` on the failure case. Behavioral fork the planner must specify. |
| A4 | `tree --json` needs a NEW recursive node builder (the current model is a flat printing recursion, no node tree exists) | Wave 7b | Underestimating this as "add serde to an existing struct" — it is real (small) new work, the biggest cost in 7b. |
| A5 | Field names beyond D-17 (base64 `mode`/`output`, passgen row key, fun-trio `text`/`answer`, flatten/bulk-rename `action` spelling, color hex case) are discretion — recommend snake_case, `action` = lowercased `RowStatus` | All waves | Low risk (discretion), but the planner should lock exact spellings so the per-command JSON-shape tests are deterministic. |
| A6 | `cowsay --json` emits the input/spoken text (`{text}`), not the rendered bubble; cowsay's human path may keep `println!`/`print!` (no clip tee needed since cowsay is --json-only) | Wave 7a | If a consumer expects the bubble, the schema is wrong — but the bubble is a visual (like qr glyphs), so text is the scriptable value. |

**If this table guides nothing else:** A1, A2, A3, A4 are the four items where "mechanical rollout" has a non-trivial decision. Everything else is genuinely copy-paste-adapt.

## Open Questions (RESOLVED)

> All three resolved during planning via the recommended option and locked into the plans (07-01/07-02/07-03).

1. **`base64 --decode --json` binary output policy (A1). — RESOLVED**
   - What we know: encode → ASCII-safe flat object; decode → arbitrary bytes (binary-safe by design).
   - What's unclear: how to represent non-UTF-8 decoded bytes in a JSON string.
   - Recommendation: under `--json`, emit decode output as a base64 string field (lossless, round-trippable) — or document a `to_string_lossy` marker. Ask the user; default to base64-string-field as the safest.
   - **Resolution:** base64-string-field adopted; never `String::from_utf8(...).unwrap()`. Implemented in 07-01 Task 1 (`json_decode_non_utf8` test).

2. **`qr --clip` mechanism (A2). — RESOLVED**
   - What we know: D-15 says copy the source text, not the glyphs; `out_line` prints+tees the same string.
   - What's unclear: whether to add a `core::output::clip_feed(&str)` primitive or have `qr` push to `CLIP_BUF` directly.
   - Recommendation: add a minimal `clip_feed(&str)` to `core::output` (the cleanest, reusable, testable). This is the only sanctioned `core::output` change in Phase 7 — confirm.
   - **Resolution:** `core::output::clip_feed(&str)` added (the one sanctioned spine change). Implemented in 07-03 Task 1 (`clip_feed_tees_only` unit test), wired into qr.

3. **`bulk-rename --force --json` row emission + abort-path stdout (A3 + D-12). — RESOLVED**
   - What we know: D-12 says `--force --json` emits applied renames (overriding silent-on-success); D-09 says failures keep stdout empty.
   - What's unclear: the exact JSON the abort path emits (nothing on stdout + stderr error is the safe read).
   - Recommendation: success → `{results, count, dry_run:false, …}`; abort → empty stdout, stderr error, exit 1. Lock in the plan.
   - **Resolution:** success → `{results, count, dry_run:false, …}`; abort → empty stdout + stderr error + exit 1. Implemented in 07-02 Task 3 (`json_abort_empty_stdout` test).

## Environment Availability

> Phase 7 is a pure code/config change with no NEW external dependencies. The toolchain and existing deps are confirmed available.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | build + test | ✓ | 1.90.0 | — |
| serde (derive) | every output struct | ✓ | 1 | — (already a dep) |
| serde_json (preserve_order) | `emit_json`, `json` passthrough | ✓ | 1.0.150 | — (already a dep) |
| arboard | `flush_clip` (clip) | ✓ | 3.6.1 | — (already a dep) |
| Windows clipboard | `--clip` live round-trip tests | ✓ (target) | — | `#[ignore]` the live test on headless CI (already the pattern, `tests/uuid.rs:238`) |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** Live clipboard for the round-trip test — already handled by `#[ignore]` + run-locally pattern.

## Validation Architecture

> nyquist_validation is enabled (no `workflow.nyquist_validation: false` in config — treated as enabled). This section feeds the Nyquist VALIDATION.md.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `assert_cmd` 2.2 (black-box binary) + `predicates` 3.1; `tempfile`/`assert_fs` for fs commands; `trycmd` 1.2 for cowsay's transcript lock; `insta` 1.48 available |
| Config file | `Cargo.toml` `[dev-dependencies]` (no separate test config) |
| Quick run command | `cargo test --test <cmd>` (one command's integration tests) |
| Full suite command | `cargo test` (all integration tests) + `cargo test --bin box` (in-crate unit tests — `box` is binary-only, NOT `--lib`) |

### Phase Requirements → Test Map

Every `--json`-enabled command gets a `json_purity` test (copied from `tests/uuid.rs:135`). Every `--clip`-enabled command additionally gets an `#[ignore]`d `clip_roundtrip` test (copied from `tests/uuid.rs:237`). All target commands already have an integration test file (`tests/<cmd>.rs`) EXCEPT cowsay (which uses `tests/cmd/cowsay-*.trycmd` + in-source unit tests — its `json_purity` needs a home; see Wave 0).

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPINE-02 | `base64 --json` one parseable doc, no 0x1B/BOM | integration | `cargo test --test base64 json_purity` | ✅ tests/base64.rs (add test) |
| SPINE-02 | `epoch --json` unified `{epoch,utc,local}` all modes | integration | `cargo test --test epoch json_purity` | ✅ tests/epoch.rs |
| SPINE-02 | `color --json` nested `{hex,rgb,hsl}` | integration | `cargo test --test color json_purity` | ✅ tests/color.rs |
| SPINE-02 | `passgen --json` `{results,count}` multi | integration | `cargo test --test passgen json_purity` + `json_count_multi` | ✅ tests/passgen.rs |
| SPINE-02 | `8ball --json` `{text}` | integration | `cargo test --test eight_ball json_purity` | ✅ tests/eight_ball.rs |
| SPINE-02 | `fortune --json` `{text}` unwrapped | integration | `cargo test --test fortune json_purity` | ✅ tests/fortune.rs |
| SPINE-02 | `roast --json` `{text}` unwrapped | integration | `cargo test --test roast json_purity` | ✅ tests/roast.rs |
| SPINE-02 | `cowsay --json` `{text}` | integration | `cargo test --test cowsay json_purity` | ❌ Wave 0 (no tests/cowsay.rs — only trycmd) |
| SPINE-02 | `du --json` `{results,count,…}` size fields | integration | `cargo test --test du json_purity` | ✅ tests/du.rs |
| SPINE-02 | `tree --json` recursive `{name,type,size?,children}` | integration | `cargo test --test tree json_purity` + `json_recursive_shape` | ✅ tests/tree.rs |
| SPINE-02 | `dupes --json` `{results:[{size,paths}],count,wasted_bytes}` | integration | `cargo test --test dupes json_purity` | ✅ tests/dupes.rs |
| SPINE-02 | `flatten --json` dry-run plan + real-run result; `dry_run` bool | integration | `cargo test --test flatten json_dry_run` + `json_force_run` | ✅ tests/flatten.rs |
| SPINE-02 | `bulk-rename --json` plan + `--force --json` applied rows; abort keeps stdout empty | integration | `cargo test --test bulk_rename json_dry_run` + `json_force_emits_rows` + `json_abort_empty_stdout` | ✅ tests/bulk_rename.rs |
| SPINE-02 | `json --json` identity passthrough, NOT wrapped | integration | `cargo test --test json json_identity_passthrough` | ✅ tests/json.rs |
| SPINE-02 | `qr --json` `{text,error_correction}` metadata not glyphs | integration | `cargo test --test qr json_metadata_not_glyphs` | ✅ tests/qr.rs |
| SPINE-02 | `weather --json` current-only; unit from `current_units` | integration | `cargo test --test weather json_purity` (use the `BOX_WEATHER_BASE_URL` offline seam + a fixture, or assert shape only) | ✅ tests/weather.rs |
| SPINE-04 | `base64 --clip` copies the printed result | integration (#[ignore]) | `cargo test --test base64 -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | `color --clip` copies the printed block | integration (#[ignore]) | `cargo test --test color -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | `epoch --clip` copies the result | integration (#[ignore]) | `cargo test --test epoch -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | `passgen --clip` copies the password | integration (#[ignore]) | `cargo test --test passgen -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | `json --clip` copies the pretty/compact JSON | integration (#[ignore]) | `cargo test --test json -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | **`qr --clip` copies the SOURCE TEXT, not the glyphs (D-15)** | integration (#[ignore]) | `cargo test --test qr -- --ignored clip_copies_source_text` | ✅ (custom assertion: pasted == input) |
| SPINE-02/04 | Display-only `matrix`/`pomodoro`/`lolcat`/`ascii`/`clip` reject/omit `--json`/`--clip` (SC4) | integration | `cargo test --test cli` (or per-cmd: assert `--json` on these is a no-op or unsupported per the chosen omission semantics) | ✅ tests/cli.rs (verify the chosen SC4 mechanism) |

**SC4 note for the planner:** "Omit" needs a concrete mechanism. Because `--json`/`--clip` are **global** flags on `Cli`, `box matrix --json` currently *parses* (the global is accepted) — it just would not be consumed. The planner must decide and test the SC4 contract: either (a) these commands simply ignore the globals (the flag parses but the animation runs as normal — simplest, but `--json` on `matrix` then does nothing, arguably "meaningless"), or (b) the command errors when `--json`/`--clip` is set on a display-only command. The roadmap SC4 wording ("correctly omit … rather than emitting a malformed or meaningless document") leans toward (a) — they must NOT emit a malformed document, which they won't because they never call `emit_json`. **Recommend (a) with a test asserting `box matrix --json` does not emit JSON to stdout (and ideally a doc note).** Confirm with the user — this is a small open decision.

### Sampling Rate
- **Per task commit:** `cargo test --test <cmd>` for the command(s) touched in that task (fast, < 5s each).
- **Per wave merge:** `cargo test` (all integration tests) + `cargo test --bin box` (unit tests) — each wave (7a/7b/7c) runs the full suite green before merge.
- **Phase gate:** Full suite green (`cargo test && cargo test --bin box`) before `/gsd:verify-work`; live `--clip` round-trips run locally once (`cargo test -- --ignored --test-threads=1`) as a human-verify-adjacent check (clipboard is shared OS state).

### Wave 0 Gaps
- [ ] `tests/cowsay.rs` — NEW file (cowsay currently has only `tests/cmd/*.trycmd` + in-source unit tests; it needs an `assert_cmd`-based `json_purity` test). Covers SPINE-02 for cowsay.
- [ ] Per-command `json_purity` test added to each existing `tests/<cmd>.rs` (15 files) — copied from `tests/uuid.rs:135`, adapted to each command's locked shape.
- [ ] Per-command `clip_roundtrip` (`#[ignore]`d) added to the 6 SPINE-04 `tests/<cmd>.rs` — copied from `tests/uuid.rs:237`; qr's variant asserts pasted == input (D-15).
- [ ] `tree` recursive-shape test (`json_recursive_shape`) — asserts `children` nesting + `type`/`size?` per D-17.
- [ ] `flatten`/`bulk-rename` dual tests (`json_dry_run` + `json_force_run`/`json_force_emits_rows` + `json_abort_empty_stdout`) — cover the D-12/D-13 plan-vs-result + the D-09 abort-empty-stdout fork.
- [ ] `weather` `--json` test using the `BOX_WEATHER_BASE_URL` offline seam (`weather/mod.rs:36`) + a forecast fixture (existing fixtures in `tests/fixtures/weather/`) so the test is network-free and deterministic.
- [ ] A small `core::output` unit test if a `clip_feed(&str)` primitive is added for qr (D-15) — assert it tees to `CLIP_BUF` only under `--clip`, no stdout write (mirrors `out_line_tees` at `src/core/output.rs:491`).

*Framework is fully present (assert_cmd/predicates/tempfile/assert_fs/trycmd/insta in `[dev-dependencies]`) — no install needed.*

## Security Domain

> `security_enforcement` not explicitly `false` in config — treated as enabled. Phase 7 is a low-security-surface formatting rollout, but two domain-specific items apply.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | partial | All input parsing is UNCHANGED in Phase 7 (the existing `read_input`/`read_input_bytes`/regex/parse paths). The only new surface is *serialization* of already-validated values. serde_json escapes control chars in string values automatically (no injection via JSON string content). |
| V6 Cryptography | no (no change) | `passgen` CSPRNG + `hash` digests are UNCHANGED. Phase 7 only wraps their *output* in JSON; it does not touch RNG or hashing. |
| V7 Data Protection | yes (clipboard) | `passgen --clip` and (any) `--clip` writes a value to the Windows clipboard. This is opt-in (user typed `--clip`) and documented. The clip path is the unchanged Phase-6 `flush_clip` (main-thread, single-shot). |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret to clipboard (`passgen --clip`) | Information Disclosure | Opt-in only; document that `passgen --clip` puts a secret on the clipboard (clipboard history / cloud sync risk). Do NOT auto-clip — already opt-in. (PITFALLS.md Security row.) |
| Non-UTF-8 NTFS path in `--json` | Tampering / DoS (panic) | `to_string_lossy()` (D-4), NEVER `to_str().unwrap()` — already the codebase convention; confirm the JSON path uses it. |
| ANSI/control-char injection via JSON string values | Tampering (terminal escape) | serde_json escapes control chars in string values on serialize; `emit_json` never styles. The `json_purity` no-0x1B test is the regression backstop. |
| `base64 --decode --json` binary blob | DoS (invalid JSON / panic) | Handle non-UTF-8 decode deliberately (A1/Pitfall 6) — base64-string field or refuse; never `unwrap` a `String::from_utf8`. |

## Sources

### Primary (HIGH confidence — read in full this session)
- `src/core/output.rs` — the complete spine: `is_json_on` (:93), `init_output` (:106), `emit_json` (:124), `out_line` (:143), `flush_clip` (:160), `is_color_on` (:34), `human_size` (:314), `format_row`/`RowStatus`/summaries; plus the spine unit tests (`out_line_tees` :491, `flush_clip_empty_noop` :520, `emit_json_no_bom_no_ansi` :546, `init_output_forces_color_off` :580).
- `src/commands/uuid/mod.rs` — the multi-capable pilot template (`UuidOutput{results,count}` + `is_json_on()` fork + `out_line`).
- `src/commands/hash/mod.rs` — the path-bearing pilot (`HashOutput`, `HashRow{path,algo,digest}`, `Algo` serde `rename_all="lowercase"`).
- The 16 target command files (`base64`, `epoch`, `color`, `passgen`, `eight_ball`, `fortune`, `roast`, `cowsay`, `du`, `tree`, `dupes`, `flatten`, `bulk_rename`, `json`, `qr`, `weather`) — read enough of each to confirm its current compute/render split and serde cost.
- `src/cli.rs` (:21-36 — globals `--json`/`--clip` already wired; no per-command shadowing) and `src/main.rs` (:81-111 — `init_output`/dispatch/`flush_clip` already wired in the load-bearing order).
- `tests/uuid.rs` (:135 `json_purity`, :184 `json_count_multi`, :214 `human_output_unchanged`, :237 `clip_roundtrip`) — the frozen test templates. `tests/hash.rs` (:55 `default_is_blake3`).
- `Cargo.toml` — confirms serde/serde_json/arboard present; no new dep needed; `box` is binary-only.
- `src/core/config.rs` (:42 `Config`, :59 `config()`) — confirms config is irrelevant to Phase 7 (format-only).

### Planning docs (HIGH confidence)
- `.planning/phases/07-spine-rollout/07-CONTEXT.md` — D-01..D-17 locked decisions.
- `.planning/REQUIREMENTS.md` — SPINE-02/SPINE-04 (note the `entries` wording to amend per D-11a, line ~36).
- `.planning/ROADMAP.md` § Phase 7 — SC1..SC4 + the 7a/7b/7c wave sketch (SC1 line ~80 has the `entries` wording to amend).
- `.planning/STATE.md` — Accumulated Context (locked v2 decisions + spine pitfalls).
- `.planning/research/ARCHITECTURE.md` — Q1 (`--json` contract + interaction matrix), Q3 (`--clip` capture-tee), the New-vs-Modified ledger.
- `.planning/research/PITFALLS.md` — `--json` contamination, float/number/path correctness, color/progress leakage, arboard main-thread discipline, the "Looks Done But Isn't" checklist.

### Secondary / Tertiary
- None — this is a closed-codebase mechanical rollout; no web/Context7 research was required. (No `[ASSUMED]` claims about external library behavior exist; the Assumptions Log entries are internal schema/behavior decisions, not unverified external facts.)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified no new deps against `Cargo.toml`; spine primitives read in full.
- Architecture / per-command shapes: HIGH — every target `mod.rs` read; the four real surprises (base64 binary decode, qr clip-text, bulk-rename abort-empty-stdout, tree recursion) identified by reading the actual compute/render code, not assumed.
- Pitfalls: HIGH — drawn from PITFALLS.md cross-referenced against the specific stdout-write sites in each command.
- Validation Architecture: HIGH — the test framework, the frozen `json_purity`/`clip_roundtrip` templates, and the existing per-command test files were all confirmed on disk.

**Research date:** 2026-06-25
**Valid until:** Stable until the codebase changes — this is a closed-codebase analysis, not time-sensitive external research. Re-verify only if a command's `mod.rs` or the spine primitives change before planning. (~30 days nominal.)
