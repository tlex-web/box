# Phase 7: Spine Rollout - Context

**Gathered:** 2026-06-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Apply the **frozen Phase-6 `--json`/`--clip` template** to every remaining applicable command — a *mechanical, additive rollout*, not new architecture. Requirements: **SPINE-02, SPINE-04** (2).

**In scope:**
- **`--json` on 16 commands:** `base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay`, `du`, `tree`, `dupes`, `flatten`, `bulk-rename`, `json`, `qr`, `weather` (SPINE-02).
- **`--clip` on the 6 not-yet-covered single-result commands:** `passgen`, `color`, `base64`, `epoch`, `json`, `qr` (`uuid` + `hash` shipped in Phase 6) (SPINE-04).
- Per command: a `#[derive(Serialize)]` output struct feeding **both** the human render (via `out_line`) and `emit_json`; the `is_json_on()` fork; a per-command JSON-purity test (copied from the Phase-6 template).

**Out of scope (later phases):**
- Per-command **depth flags** — new behavior/fields (Phases 8–10). Phase 7 only wraps the *current* data each command produces in the spine; e.g. `weather` is **current-only** here (`--forecast`/cache → Phase 10), `hash` stays single-file (multi-file → Phase 8), `bulk-rename`/`flatten` get no new filters.
- `box config` / `box completions` meta-commands (Phase 11).
- New deps. Phase 7 needs **none** (`serde`/`serde_json`/`arboard` already present).
- **Display-only commands are explicitly excluded** from both flags: `matrix`, `pomodoro`, `lolcat`, `ascii`, `clip` (roadmap SC4) — they must *omit* `--json`/`--clip`, not emit a meaningless document.
</domain>

<decisions>
## Implementation Decisions

> Four per-command gray areas were discussed. Everything else inherits the Phase-6 frozen template verbatim (see `<canonical_refs>` → `06-CONTEXT.md`). All four resolved to the recommended option.

### Carried forward from Phase 6 — LOCKED, not re-discussed (recorded so the planner has them in one place)
- **D-01 root rule:** object root always; **scalar commands** → flat object; **multi-capable commands** → `{results:[…],count:N}` even when N=1; **`tree`** → recursive object with `children`. snake_case, `to_writer_pretty`, UTF-8 **no BOM**, single trailing newline.
- **`--clip` (D-07/D-08):** copy-**all** the printed payload, newline-joined, single `trim_end()`; copy-**AND**-print; "Copied to clipboard" confirmation → **stderr**, suppressed when not a TTY; `--clip` forces `COLOR_ON=false`; `--json --clip` copies the whole JSON document; `flush_clip()` runs once on the main thread after successful dispatch; empty output → no-op.
- **Error contract (D-09/D-10):** no JSON error envelope — on failure stdout stays **EMPTY**, human `error:` → stderr, exit **1** (runtime) / **2** (usage). No `{"error":…}` on stdout.
- **No-drift / color-progress hygiene:** one serde struct per command feeds human + JSON; `is_json_on()` fork happens FIRST; progress (indicatif) → stderr, suppressed under `--json`; piped/`--json` output is byte-identical-minus-ANSI.

### Gray Area 1 — Filesystem multi-row document key (resolves a roadmap↔D-01 conflict)
- **D-11 — Frozen `{results,count}` everywhere.** `du`, `dupes`, `flatten`, `bulk-rename` all use the same `{results:[…],count:N}` root as the `uuid`/`hash` pilots — one mechanical rule, byte-shape-identical to the template, uniform `.results` for PS7 scripting across the whole toolkit.
- **D-11a — ⚠ REQUIRED DOC AMENDMENT (pre-planning).** Roadmap **Phase-7 SC1** currently says `box du …--json` yields an object whose **`entries`** array carries size fields. That wording predates Phase-6 D-01 and loses the conflict. Amend SC1 (and any matching REQUIREMENTS.md/SPINE-02 wording) to say the array key is **`results`** (with `count`). The `tree` "recursive object" half of SC1 is already correct.

### Gray Area 2 — `flatten` / `bulk-rename` under `--json`
- **D-12 — `--json` is format-only, orthogonal to `--force`.** `--json` never changes whether the command mutates; it follows the existing dry-run-default / `--force`-executes gate.
  - dry-run + `--json` → the **plan** as a JSON document; `--force` + `--json` → the **executed result** as a JSON document.
  - **`bulk-rename --force --json` now emits the applied renames** (overriding its current silent-on-success behavior — only under `--json`; the human `--force` path stays silent-on-success).
- **D-13 — `dry_run: bool` in the root object.** Each document carries a `dry_run` boolean so a script can distinguish a preview from an applied run. Per-operation rows: `{src, dst, action, reason}` (`action` mirrors the existing `RowStatus`: copy/rename/skip). The locked summary counts (copied/renamed/skipped + total size for flatten) live as sibling fields in the root alongside `{results,count}`.

### Gray Area 3 — `qr` and `json` under the spine (the odd fits)
- **D-14 — `qr --json` emits encode metadata, NOT glyphs:** `{text, error_correction}` (the rendered ▀▄ block is a visual, not a scriptable value).
- **D-15 — `qr --clip` copies the source text** (SPINE-04 "text payload"), the one deliberate exception to D-07 copy-all: pasting the half-block glyphs as clipboard *text* is garbage in most apps, whereas the encoded URL/text is the useful payload.
- **D-16 — `json --json` is identity passthrough.** The `json` command's value *is* a JSON document, so `--json` emits the **parsed input document verbatim** as pure machine JSON (uncolored, validated, single document) — **NOT** wrapped in `{results,count}`. A documented root-rule exception alongside `tree`. (Plain `box json` piped already produces pure uncolored JSON; `--json` forces that purity even on a TTY.)

### Gray Area 4 — Field schemas for structured value commands
- **D-17 — Nested sub-objects + unified shapes:**
  - **`color`** → `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` (nested; scripts via `.rgb.r` in PS7).
  - **`epoch`** → unified `{epoch, utc, local}` for **all** input modes (no branching on input direction — one stable shape whether the user passed a timestamp, a date string, or nothing).
  - **`tree`** → recursive node `{name, type:"dir"|"file", size?, children:[]}` (`size` present for files; per D-01 recursive-object exception).
  - **`weather`** → current-only fields `{location, temperature, unit, conditions, …}` (units read from the API response, never hardcoded — v1 contract; `--forecast` is Phase 10).
  - **`dupes`** → `{results:[{size, paths:[…]}], count, wasted_bytes}` (key per D-11).

### Claude's Discretion (planner/executor latitude)
- **Exact field names beyond those pinned above** — e.g. base64 (`{input?, output, encoding/mode}`), passgen rows, fortune/8ball/roast (`{text}` or similar single-field object), weather's exact sub-field set. Follow D-01 conventions + the D-17 spirit; keep snake_case.
- **`action` enum string spelling** for D-13 rows (e.g. `"copy"`/`"rename"`/`"skip"`) — match the existing `RowStatus` lowercased.
- **Wrap behavior for soft-wrapped fun commands** (`8ball`/`fortune`/`roast`) under `--json` — emit the unwrapped single string (wrapping is a human-render concern, not data).
- **Whether scalar single-line commands** (`base64`, `epoch`, the fun trio, `color`) are "scalar→flat object" vs need a `results` array — per D-01 they are flat scalar objects unless they can return >1 row (`passgen --count`, which is multi-capable → `{results,count}`).
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The frozen template (READ FIRST — Phase 7 is its mechanical application)
- `.planning/phases/06-scriptable-core-foundation/06-CONTEXT.md` — **the authoritative spine contract.** D-01 (JSON root rule), D-02/D-03 (pilot literals + field-naming policy), D-07/D-08 (`--clip` semantics), D-09/D-10 (error/exit contract). Every Phase-7 command copies these shapes.
- `src/commands/uuid/mod.rs` — `UuidOutput { results: Vec<UuidRow>, count }` + `is_json_on()` fork + `out_line` human path. **The copy-me template for multi-capable commands.**
- `src/commands/hash/mod.rs` — `HashOutput { results, count }`, `HashRow { path, algo, digest }`, serde `Algo` enum with `#[serde(rename_all="lowercase")]`. Second reference consumer.
- `src/core/output.rs` — spine primitives to call: `is_json_on()`, `emit_json(&T)`, `out_line(&str)` (tees to clipboard), `flush_clip()`, `is_color_on()`, plus existing helpers (`human_size`, `terminal_width`, `format_row`, `RowStatus`, `dry_run_summary`, `real_run_summary`).

### Phase scope & locked contracts
- `.planning/ROADMAP.md` § "Phase 7: Spine Rollout" — goal, 4 success criteria, 3-plan sketch (7a pure transforms / 7b filesystem / 7c json+qr+weather). **⚠ SC1 needs the D-11a amendment** (`entries` → `results`).
- `.planning/REQUIREMENTS.md` — SPINE-02 (16 `--json` commands), SPINE-04 (8 `--clip` commands). **⚠ check for the same `entries` wording per D-11a.**
- `.planning/PROJECT.md` — milestone scope; v1 Key Decisions; the "deepening not rewrite" framing.
- `.planning/STATE.md` § "Accumulated Context" — locked v2 decisions + critical spine pitfalls.

### Research blueprints (HIGH-confidence)
- `.planning/research/ARCHITECTURE.md` — Q1 (`--json` contract + interaction matrix), Q3 (`--clip` capture-tee), and the **New-vs-Modified file ledger** (per-command change list).
- `.planning/research/PITFALLS.md` — `--json` contamination, color/progress leakage, arboard main-thread discipline.

### Source files Phase 7 touches (per command)
- `src/commands/{base64,epoch,color,passgen,8ball,fortune,roast,cowsay,du,tree,dupes,flatten,bulk-rename,json,qr,weather}/mod.rs` — add output struct + `is_json_on()` fork + route human path through `out_line`; add `--clip` consumers per SPINE-04.
- `src/cli.rs` / `src/main.rs` — global `--json`/`--clip` already wired in Phase 6; no spine plumbing changes expected (verify each command's args allow the global flags).
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Phase-6 pilots are the literal template** — `UuidOutput`/`HashOutput` `{results,count}` structs + the `is_json_on()` fork are copy-paste-adapt for all 16 commands. No new pattern to invent.
- **`src/core/output.rs` spine is complete** — `emit_json`, `out_line`, `is_json_on`, `flush_clip`, `is_color_on`, `human_size`, `format_row`, `RowStatus`, `dry_run_summary`/`real_run_summary` all exist. Phase 7 *consumes* them; it adds no spine primitives.
- **`src/commands/clip/mod.rs`** — proven arboard discipline (already reused by `flush_clip` in Phase 6); no clipboard code is written per-command in Phase 7 (the `out_line` tee handles it).

### Established Patterns
- **Thin orchestrator over a pure function** — most Phase-7 commands already compute their value then `println!` it; the rollout is: give the value a `#[derive(Serialize)]` struct + field names, fork on `is_json_on()` first, route the human branch through `out_line`. `du`/`tree`/`dupes`/`flatten`/`bulk-rename` currently have human-only renders (no struct yet) — they need the struct extracted.
- **Per-command JSON-purity test** — the Phase-6 "stdout parses as exactly one JSON value, contains no `0x1B`" test generalizes per command (SC3). Copy it for each of the 16.
- **Scalar vs multi-capable fork** — `passgen --count` and the filesystem commands are multi-capable → `{results,count}`; `base64`/`epoch`/`color`/fun-trio are scalar → flat object (D-01).

### Integration Points
- No `main.rs`/`cli.rs` plumbing changes expected — the global `--json`/`--clip` flags and `init_output`/`flush_clip` wiring landed in Phase 6. Phase 7 is per-command-file work only (plus per-command tests). Confirm each command's subcommand args don't shadow the global flags.
- **Risk ordering (roadmap):** simplest → nested, so any surprise surfaces on `base64`, not `flatten`. Plan waves 7a (pure transforms) → 7b (filesystem buffered-rows) → 7c (json/qr/weather).
</code_context>

<specifics>
## Specific Ideas

- **The `{results,count}` uniformity is the headline decision (D-11):** a PS7 user does `(box <cmd> --json | ConvertFrom-Json).results` for *every* multi-row command — this consistency is the whole point of the spine and overrides the roadmap's per-command `entries` wording.
- **`dry_run` boolean (D-13)** is the concrete "I want scripts to know if it actually ran" target for the two action commands.
- **`qr --clip` copies the encoded text, not the glyphs (D-15)** — the one intentional break from copy-all; lock it with a test that asserts the clipboard payload equals the input string, not the rendered block.
- **D-11a is a pre-planning action item:** amend roadmap SC1 (`entries` → `results`) and any matching REQUIREMENTS.md/SPINE-02 wording before/as the first planning step, mirroring the Phase-6 D-06 doc-amendment pattern.
</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within Phase 7 scope. Every per-command **depth** idea (new fields/flags) is already roadmapped to Phases 8–10 and was deliberately kept out: Phase 7 wraps only the *current* data each command produces.
</deferred>

---

*Phase: 7-Spine Rollout*
*Context gathered: 2026-06-25*
