# Phase 6: Scriptable-Core Foundation - Context

**Gathered:** 2026-06-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the **entire shared scriptable spine** — global `--json` + `--clip` flags and the config-file resolver — **once**, and prove it end-to-end on the two cheapest commands (`uuid`, `hash`), co-shipping the BLAKE3-default breaking change with its `hash.default_algo` config escape hatch. Requirements: **SPINE-01, SPINE-03, SPINE-05, HASH-V2-01** (4).

Whatever shapes are chosen here become the **frozen template** that Phase 7 rolls across 16 `--json` commands + 8 `--clip` commands and Phases 8–11 build on — a flaw here costs 23 commands of rework, so the JSON/clip/error/verify shapes below are the highest-leverage decisions in the milestone.

**In scope:** `core::output` spine additions (`JSON_ON`/`CLIP_ON` atomics, `init_output`, `is_json_on`, `emit_json`, `out_line`, `CLIP_BUF`, `flush_clip`); `core::config` (`Config`, `load`, `OnceLock`, `config()`, `init_config`); `BoxError::Config`; global `--json`/`--clip` on `Cli`; `main.rs` wiring; `uuid` + `hash` as first consumers; BLAKE3 compute-default flip; the reusable test template (JSON-purity, `--clip` capture, config-precedence, BLAKE3-default).

**Out of scope (later phases):** rolling the spine across the other 21 commands (Phase 7); per-command depth flags (Phases 8–10); `box config` / `box completions` meta-commands (Phase 11); `clap_complete`, `indicatif`, `chrono-tz`, `windows 0.61`, `uuid v7` deps (later phases).
</domain>

<decisions>
## Implementation Decisions

> Four gray areas were researched (full_maturity advisor tables) and decided. Two were outright contradictions between the project's own docs — resolved below, with the doc-amendment consequences flagged.

### JSON Output Contract — the frozen template (SPINE-01)

- **D-01 — Always-object document root.** Never a bare array at the root.
  - **Scalar-only commands** (color, epoch, passgen, weather, single-value `qr`) → a flat object `{ … }`.
  - **Multi-capable commands** — any command that *can* return more than one row (`uuid -n`, `hash` [multi-file arrives Phase 8], `du`, `dupes`, `flatten`, `bulk-rename`) → `{ "results": [ … ], "count": N }` **even when N = 1**.
  - **`tree`** → recursive object with `children: [ … ]`.
  - Rationale: PS7 `ConvertFrom-Json` auto-enumerates a bare top-level array (a footgun for a frozen schema — single-element arrays collapse and the root can never grow a sibling key). An always-object root is one mechanical rule all 23 authors apply, never collapses, stays infinitely extensible (the `count` key is proof-of-value). Adopts ARCHITECTURE.md Q1d's extensibility principle; **supersedes SUMMARY.md line 94's bare-array wording** (keeps its "shape stability" intent).
- **D-02 — Pilot literals (the copy-me template):**
  - `box uuid` → `{"results":[{"uuid":"…","version":"v4"}],"count":1}`
  - `box uuid -n 5` → same element struct `{uuid,version}` × 5, `"count":5`
  - `box hash file` → `{"results":[{"path":"file","algo":"blake3","digest":"…"}],"count":1}` — a **one-element `results` array inside an object**, NOT a bare `[{…}]`, so Phase-8 multi-file `hash a b` is byte-shape-identical (more elements, larger `count`) and no consumer rewrites.
- **D-03 — Field names locked:** `uuid`, `version` (uuid); `path`, `algo`, `digest` (hash). `algo` (not `algorithm`) matches the existing `--algo` flag + RustCrypto/b3sum convention; `digest` (not `hash`) avoids a key named like the command and is the RustCrypto trait term; `path` generalizes to every filesystem command. Output is `snake_case`, pretty (`to_writer_pretty`), UTF-8 **no BOM**, single trailing newline (SPINE-01). General rule for later phases: **root is always an object; scalar commands inline their fields; any multi-capable command uses `{results:[…],count:N}`; `tree` is the recursive-object exception with `children`.**

### `hash --verify` Algorithm Resolution — the 64-hex tie (HASH-V2-01)

- **D-04 — Compute default flips to BLAKE3; verify resolution stays SHA-256.** A 64-hex (32-byte) digest is valid for *both* SHA-256 and BLAKE3, so the two are decoupled:
  - **Compute** (`hash/mod.rs:162`): `self.algo.unwrap_or(Algo::Sha256)` → `self.algo.or(config().default_hash_algo).unwrap_or(Algo::Blake3)`. `box hash file` (no `--algo`) now emits BLAKE3.
  - **Verify** (`algo_from_len`, `hash/mod.rs:78-85`): the length→algo table is **UNCHANGED** — 32→md5, 64→**sha256**, 128→sha512. A bare `box hash --verify <64hex> file` keeps verifying as SHA-256, so **no stored SHA-256 baseline silently breaks** (the #1 v2 risk). `box hash --algo blake3 --verify <64hex> file` is the explicit BLAKE3 verify path (the existing WR-01 "explicit `--algo` always wins" logic already supports this — no change needed).
  - Grounding: every real tool (`b3sum --check`, `sha256sum -c`, coreutils, openssl) pins the algorithm by tool/flag/tag and **never guesses by length**; changing the compute default is forward-looking, changing the verify default is backward-destructive.
- **D-05 — BLAKE3-fallback diagnostic probe on mismatch.** On the verify-mismatch branch (`hash/mod.rs:154-156` `bail!`), when the supplied digest is 64-hex and the SHA-256 comparison failed: additionally compute the file's BLAKE3 and, if *that* matches, emit a decisive stderr hint ("digest mismatch as sha256, but it MATCHES this file's blake3 — re-run with `--algo blake3`"); otherwise emit the static transitional hint pointing at `--algo blake3`. Hint is **stderr-only**, styling gated on `is_color_on()`, **suppressed under `--json`**. Exit stays **1** on mismatch (the user requested the default SHA-256 verify and it did not match). 0/1/2 contract preserved.
- **D-06 — ⚠ REQUIRED DOC AMENDMENT (pre-planning).** REQUIREMENTS.md **HASH-V2-01** and SUMMARY.md **Pitfall 6 (line 179)** currently say *"the 64-hex tie now maps to BLAKE3."* That wording **is** the rejected Option B (silently breaks the SHA-256 install base) and contradicts D-04/D-05. Those two lines MUST be amended to: *the breaking change is the **compute** default only; bare `--verify <64-hex>` stays SHA-256 with a BLAKE3 diagnostic probe.* (Not yet edited — flagged here for the planner/executor; see "Specific Ideas".)

### `--clip` Semantics (SPINE-03)

- **D-07 — Copy-all, newline-joined.** `--clip` copies *everything the command printed* via the locked `out_line` tee into `CLIP_BUF`, with a single `trim_end()` in `flush_clip()`. `box uuid -n 5 --clip` copies **all 5** UUIDs. No per-command "primary result" definition — "primary result" (SPINE-03) resolves uniformly to "the command's full stdout payload." This *is* the locked tee mechanism (zero special-casing) and matches every PS7 idiom (`clip.exe`, `Set-Clipboard`, `pbcopy`, `xclip`). `-n 5` is an explicit request for 5; copying 1 would violate least-surprise.
- **D-08 — Locked sibling rules** (SPINE-03 + research): copy-**AND**-print; "Copied to clipboard" confirmation → **stderr**, suppressed when not a TTY; `--clip` forces `COLOR_ON=false` (clipboard never gets ANSI); `--json --clip` copies the **whole** JSON document; `flush_clip()` runs **once on the main thread after successful dispatch** (a failed command never clobbers the clipboard); empty output → no-op, no confirmation.

### Error / Exit Contract under `--json` (SPINE-01)

- **D-09 — No JSON error envelope.** On any failure under `--json`: stdout stays **EMPTY**, a human-readable `error: …` goes to **stderr**, the process exits **1** (runtime — e.g. hash mismatch, file-not-found) or **2** (usage). Preserves SPINE-01 stdout-purity + the FOUND-03 data→stdout/messages→stderr split; matches `gh`/`kubectl -o json`/`cargo --message-format json`/`jq`/`ripgrep`. No `{"error":…}` on stdout (the rejected aws-cli style — pollutes the data channel).
- **D-10 — Malformed config → exit 2.** A corrupt/unparseable `%APPDATA%\box\config.toml` aborts *before* the operation runs (same family as a clap usage error) → new `BoxError::Config` reusing the existing `main()` `BoxError`→exit-2 downcast. Matches BSD `sysexits` `EX_CONFIG` + aws-cli's distinct config code. **Missing** config → silent fallback to built-in defaults (never errors a normal `box uuid`) stays locked (SPINE-05).

### Carried Forward — locked at requirements/roadmap/research time (NOT re-discussed; recorded so the planner has them in one place)

- **Config impl (D-1, SPINE-05):** hand-roll `toml` 1.1.2 + `dirs` 6.0.0; `#[serde(default, deny_unknown_fields)] Config` of `Option<T>` fields (no `default_value`); precedence `cli.or(env).or(config).unwrap_or(builtin)`; `%APPDATA%\box\config.toml`; loaded **once** into `OnceLock` via `init_config()` in `main` before dispatch.
- **Spine wiring:** `--json`/`--clip` are `global = true` flags on `Cli`, lifted to `JSON_ON`/`CLIP_ON` atomics via `init_output()` (mirrors the proven `no_color`→`COLOR_ON` idiom). `RunCommand::run(self)` signature **unchanged**; commands consult `is_json_on()`, never a parsed field.
- **No-drift guarantee:** one `#[derive(Serialize)]` output struct per command feeds **both** the human render and `emit_json` — they cannot diverge.
- **Color/progress hygiene:** `init_output` forces `COLOR_ON=false` whenever `json` **or** `clip` is set; progress (`indicatif`) → stderr and suppressed entirely under `is_json_on()`.
- **Cross-spine field policy:** large numbers = bare `u64` (D-3, document the >2^53 JS caveat); non-UTF-8 paths = `to_string_lossy()` (D-4) — never `to_str().unwrap()`.
- **Phase-6 new deps:** `toml`, `dirs` only (`serde`/`serde_json`/`arboard` already present). `clap_complete`/`indicatif`/`chrono-tz`/`windows 0.61`/`uuid "v7"` belong to later phases.

### Claude's Discretion
- **Env-var tier spelling** — SPINE-05 mandates the env tier exists in precedence (`cli.or(env).or(config)…`) but no scheme is locked. Planner picks a consistent convention; suggested `BOX_<SECTION>_<KEY>` uppercase (e.g. `BOX_HASH_DEFAULT_ALGO`). Open implementation detail, not a user-facing decision.
- **Config-struct scope in Phase 6** — start tiny: `default_hash_algo` is the only field Phase 6 strictly needs. Whether to stub `weather_units`/`weather_location`/`color` now or add them when their commands adopt the spine (Phase 7+) is the planner's call (lean: include only `default_hash_algo`, with a doc note the struct grows per-command).
- **Exact hint/confirmation wording** — the D-05 mismatch hints and the D-08 "Copied to clipboard" line: keep concise, follow box's existing stderr `error:`/diagnostic style.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & locked contracts
- `.planning/ROADMAP.md` § "Phase 6: Scriptable-Core Foundation" — goal, 4 success criteria, plan sketch (06-01 spine / 06-02 pilots).
- `.planning/REQUIREMENTS.md` — SPINE-01, SPINE-03, SPINE-05, HASH-V2-01 + milestone decisions D-1..D-7. **⚠ HASH-V2-01 needs the D-06 amendment** (64-hex verify stays SHA-256, not BLAKE3).
- `.planning/STATE.md` § "Accumulated Context" — locked v2 decisions, critical pitfalls (`--json` contamination, config precedence, arboard main-thread, `--clip`/`--json` force no-color), the v1→v2 architecture graft.
- `.planning/PROJECT.md` — milestone scope, BLAKE3 breaking-change note, v1 Key Decisions.

### Implementation blueprint (HIGH-confidence, read the real v1 source)
- `.planning/research/ARCHITECTURE.md` — **the integration blueprint.** Q1 (`--json` contract: global flag→atomic, one serde struct, interaction matrix, single-document shape), Q2 (config `Option`-merge + `Config`/`load` code), Q3 (`--clip` capture-then-tee `out_line`/`flush_clip` code), Q5 (BLAKE3 flip), and the **New-vs-Modified file ledger (~lines 455-471)** — authoritative for the planner. **⚠ Q1d** here informed D-01 (object-root); **Q5** is consistent with D-04 (verify stays sha256).
- `.planning/research/SUMMARY.md` — research synthesis. **⚠ line 94** (bare-array) superseded by D-01; **⚠ Pitfall 6 / line 179** (verify→BLAKE3) superseded by D-04/D-05 — needs the D-06 amendment.
- `.planning/research/STACK.md` — dependency versions/rationale (`toml`, `dirs`).
- `.planning/research/PITFALLS.md` — full v2 pitfall catalogue (the spine ones are summarized in STATE.md).

### Source files this phase touches
- `src/core/output.rs` — the `COLOR_ON`/`init_color`/`is_color_on` idiom to mirror; home of the new `JSON_ON`/`CLIP_ON`/`emit_json`/`out_line`/`CLIP_BUF`/`flush_clip`.
- `src/cli.rs` — add global `--json`/`--clip` on `Cli`.
- `src/main.rs` — add `init_config()` + `init_output()` before dispatch, `flush_clip()` after.
- `src/core/errors.rs` — add `BoxError::Config` (exit-2). · `src/core/mod.rs` — add `pub mod config;`.
- `src/commands/uuid/mod.rs` — first `--json`/`--clip` consumer. · `src/commands/hash/mod.rs` — second consumer + BLAKE3 flip + verify probe. · `src/commands/clip/mod.rs` — proven arboard discipline to reuse in `flush_clip`.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`src/core/output.rs`** — the existing `COLOR_ON: AtomicBool` + `init_color()` + `is_color_on()` is the *exact* idiom to clone for `JSON_ON`/`CLIP_ON` + `init_output()` + `is_json_on()`. Zero new pattern to invent.
- **`src/commands/hash/mod.rs`** — already `Option<Algo>`-shaped. The compute-default flip is a one-line change at **line 162** (`self.algo.unwrap_or(Algo::Sha256)` → `.or(config().default_hash_algo).unwrap_or(Algo::Blake3)`). The verify table `algo_from_len` (**lines 78-85**) stays as-is (64→sha256). The mismatch `bail!` (**lines 154-156**) is where the D-05 BLAKE3 probe hint attaches. The `Algo` enum (**lines 63-73**) needs `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all="lowercase")]` to round-trip config + JSON.
- **`src/commands/uuid/mod.rs`** — thin `println!`-per-line loop (**line 31**); add a `UuidOutput { uuid, version }` struct, fork on `is_json_on()`, route the human path through `out_line`. `count: u8` (default 1) already covers the multi case → always-wrapped `{results,count}` per D-01.
- **`src/commands/clip/mod.rs`** — proven arboard discipline (main-thread, single-shot, persists past process exit on Windows) to reuse verbatim in `flush_clip()`.

### Established Patterns
- **Thin orchestrator over a pure function** (v1 Pattern 1/5): the pure fn already returns the value → just give it `#[derive(Serialize)]` + field names; human render and `emit_json` read the same struct (no drift). `uuid` and `hash` both already follow this.
- **Global flag → process-global atomic → `is_*_on()`** (the `no_color`/`COLOR_ON` precedent): no per-command field, no `RunCommand` trait churn.
- **0/1/2 exit via `main()` downcast** of typed `BoxError` variants → exit 2; plain `anyhow` error → exit 1 (verify-mismatch already uses this, `hash/mod.rs:156`).
- **Per-command "byte-identical minus ANSI when piped" test** generalizes to a **per-command "stdout parses as exactly one JSON value" purity test** — the regression backstop and the reusable template Phase 7 copies.

### Integration Points
- `src/main.rs`: `init_config()` then `init_output(cli.json, cli.clip)` before the dispatch `match`; `flush_clip()` immediately after a successful dispatch, before the exit-code map. (`Completions`/`Config` dispatch arms are **Phase 11**, not here.)
- `src/cli.rs`: global `--json`/`--clip` land in Phase 6; the `Completions`/`Config` `Commands` variants are **Phase 11**.
- `src/core/errors.rs`: `BoxError::Config { … }` joins `MissingInput`/`UnsupportedHashLength` in the exit-2 downcast list.
</code_context>

<specifics>
## Specific Ideas

- The **D-02 pilot JSON literals are the concrete "I want it like X" target** — every later command copies that exact shape. Lock them in the Phase-6 tests as the template.
- **ARCHITECTURE.md's New-vs-Modified file ledger** (~lines 455-471) is the authoritative file-by-file change list for the planner — Phase 6 implements the `core::output`/`core::config`/`errors`/`cli`/`main` rows + the `uuid`/`hash` rows.
- **D-06 amendment is a pre-planning action item:** before (or as the first step of) planning, amend HASH-V2-01 in `.planning/REQUIREMENTS.md` and Pitfall 6 in `.planning/research/SUMMARY.md` to state the breaking change is the **compute** default only and bare `--verify <64-hex>` stays SHA-256 (with the BLAKE3 diagnostic probe). The roadmap success-criterion #3/#4 wording is already correct (it only asserts compute-default + config override); only HASH-V2-01 + SUMMARY Pitfall 6 carry the stale "verify→BLAKE3" wording.
</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within Phase 6 scope. (The env-var spelling and the Phase-6 config-struct field set are in-phase implementation details under "Claude's Discretion," not deferred capabilities. An opt-in `hash.strict_verify` config key — making the 64-hex tie an error — was surfaced by research as a *possible future* knob but is explicitly NOT scoped for v2.)
</deferred>

---

*Phase: 6-Scriptable-Core Foundation*
*Context gathered: 2026-06-25*
