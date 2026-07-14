# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — Full Toolbox

**Shipped:** 2026-06-24
**Phases:** 5 | **Plans:** 22 | **Sessions:** ~5 (2026-06-22 → 2026-06-24)

### What Was Built

- A single Rust binary (`box`) bundling all 23 command-line tools as subcommands, installable globally from PowerShell 7 via `install.ps1`.
- Shared core infrastructure (`src/core/`: errors, output/color, fs/walkdir, input) grown into by each command's first live consumer — no speculative abstraction.
- Five phases ordered by integration risk: Foundation+Flatten → 9 pure transforms → 5 filesystem tools → 4 terminal visuals → 4 Windows-platform commands.
- 7,748 Rust LOC, 150 bin-unit + full integration test coverage, a portable `crt-static` MSVC release binary (5.1 MB).

### What Worked

- **Risk-ordered phasing.** Proving the `RunCommand` pattern on `uuid` before touching `flatten`, and leaving arboard/WinRT/Open-Meteo for last (21 commands working first), meant the riskiest integrations landed against a stable base — Phase 5 compiled its new Windows-API deps first try.
- **Allow-then-remove dead-code discipline.** Each forward-compat `#[allow(dead_code)]` on shared core came off the moment its first live caller landed, keeping `clippy -D warnings` honest and proving the reusable surface had no orphans.
- **Promote-on-second-consumer for shared helpers.** `human_size` lived in flatten until `tree` needed it, then was promoted to `core::output` with the original caller re-pointed and re-tested green — shared code earned its place instead of being designed up front.
- **Snapshot-the-tree safety tests.** Every destructive/abort path (flatten, bulk-rename, dupes-read-only) asserts the directory byte-for-byte unchanged, which is what made the silent-overwrite class of bugs catchable.
- **Post-execution code review per risky phase.** Caught 2 genuine BLOCKERs (bulk-rename `..` path-escape, matrix raw-mode-stuck-on-setup-failure) that verification alone missed.

### What Was Inefficient

- **REQUIREMENTS.md checkboxes drifted from reality.** QR/CLIP/POMO/WTHR stayed `[ ]`/"Pending" in REQUIREMENTS.md after Phase 5 verified complete; the truth lived in STATE.md/ROADMAP/PROJECT and had to be reconciled at milestone close. The phase-transition step that checks off requirements didn't fire for the final phase.
- **gsd-sdk state handlers left STATE.md stale.** `phase.complete` wrote `completed_phases=4/percent=80/Current-Position=EXECUTING` after Phase 5 and had to be hand-corrected to 5/100/COMPLETE — a recurring reconciliation tax across the milestone.
- **Phase summaries lack a machine-readable one-liner field.** Extracting accomplishments at close required reading prose rather than a structured `one_liner:` per summary.

### Patterns Established

- `src/commands/<cmd>/mod.rs` + `RunCommand` trait + ~40-line dispatch-only `main.rs`; new commands are vertical slices with zero same-wave file overlap so they sequence cleanly on the shared registry.
- `is_color_on()`-gated styling with a pure ANSI-emitting walker reached only after the gate — guarantees piped output is byte-identical minus color (proven per command by a piped-no-ANSI test).
- CSPRNG (`OsRng`) for security-relevant randomness vs `rand::rng()` for decorative; non-determinism tested by membership + N-runs-differ properties only, never a seeded exact value.
- Pure I/O-free pre-flight (`-> Vec<Conflict>`) as the safety backstop for any destructive op, unit-tested per rule.

### Key Lessons

1. **Order phases by integration risk, not feature grouping** — the cheapest place to find an architecture problem is the simplest command, and the safest time to attempt the hardest integration is when everything else already works.
2. **Verification ≠ review** — goal-backward verification confirms the feature works; an adversarial code-review pass is what finds the path-escape and the terminal-restore-on-setup-failure. Both were needed; neither was sufficient alone.
3. **Requirement checkboxes must be closed by the same step that completes the phase**, or they silently drift — at milestone close, trust the cross-artifact consensus (STATE/ROADMAP/PROJECT/VERIFICATION) over a single stale table.
4. **Reconcile STATE.md by hand after every gsd-sdk state mutation** — the SDK state handlers are not reliable for completion percentages and position fields.

### Cost Observations

- Model mix: predominantly opus (quality model profile) for planning/execution/review; not separately metered this milestone.
- Sessions: ~5 across 3 calendar days.
- Notable: sequential wave execution (one plan per wave on the shared registry) traded parallelism for zero file-overlap conflicts — a deliberate, low-overhead choice for a single-binary crate.

---

## Milestone: v2.0 — Toolbox → Toolkit

**Shipped:** 2026-07-14
**Phases:** 6 (Phases 6–11) | **Plans:** 23 | **Sessions:** ~several (2026-06-25 → 2026-07-14)

### What Was Built

- The shipped 23-command binary deepened into a scriptable PowerShell-7 toolkit: a cross-cutting `--json`/`--clip`/config spine across every applicable command, comprehensive per-command depth flags (34 requirements), a BLAKE3-default `hash` with a config escape hatch, and the `config`/`completions` meta-commands.
- Grafted onto the v1 architecture without rewriting it — `core::output` grew the `--json`/`--clip` primitives, new `core::config` (hand-rolled `toml`+`dirs`) and `core::cache` modules landed, and every command gained one `#[derive(Serialize)]` output struct + an `is_json_on()` fork.
- Three destructive filesystem operations (`flatten --move`, `dupes --delete`, `bulk-rename --backup`), each behind a dry-run default, `--force`, an abort-all-before-any pre-flight, and a mandatory adversarial code-review gate.
- Grew from 7,748 → 15,649 Rust LOC (36 files), 150 → 507 tests; clippy `-D warnings` clean throughout.

### What Worked

- **Build the cross-cutting spine once, on the two cheapest commands.** Proving the `--json`/`--clip`/config spine end-to-end on `uuid`+`hash` in Phase 6 — and freezing the `{results,count}`/`json_purity` template — meant the rollout across 16 commands in Phase 7 was mechanical, with the surprises (base64 binary-safe decode, tree recursive node tree, bulk-rename abort-empty-stdout) surfacing on the pilot/simple commands, not on `flatten`.
- **Co-ship a breaking change with its escape hatch.** Flipping `hash` to BLAKE3-default in the same phase that shipped the config resolver meant `hash.default_algo = "sha256"` existed the moment the default changed — and keeping the `--verify` length table untouched meant stored SHA-256 baselines never broke.
- **One plan + one adversarial review per destructive flag.** Isolating each data-loss operation to its own plan with the v1 bulk-rename gate (dry-run default, `--force`, abort-all-before-any, snapshot-the-tree-unchanged test) kept the blast radius contained and caught real ordering bugs (the two-phase copy→verify-ALL→delete-ALL rethink of `flatten --move`).
- **Generate completions from the live `Cli`.** `CommandFactory` against the final arg surface meant the completion script auto-reflects all 34 depth flags — no hand-maintained list to drift.
- **Code review kept finding what verification missed** (carried from v1): a pre-existing v1 bulk-rename Windows trailing-dot/reserved-name silent-clobber (CR-01) and a `lolcat --duration` `Instant` overflow that would bypass terminal restore (BL-01).

### What Was Inefficient

- **Phase status drifted stale in tracking artifacts.** Phase 8 sat at "In Progress"/"Executing" in ROADMAP.md and STATE.md long after all 6 plans were `[x]` complete and verified — the same class of drift v1 flagged (requirement checkboxes / STATE completion fields not closed by the completing step). Reconciled at milestone close.
- **`gsd-sdk` state/archival handlers still not trusted for mutations.** Per the v1 lesson, the milestone close was done with direct file writes rather than `milestone.complete`, to guarantee the MILESTONES.md entry quality and avoid stale STATE.md fields.
- **Human-verify UAT accumulated across phases.** 9 PS7-only confirmations (clipboard, progress bars, toast/pomodoro UX, tab-completion) piled up across Phases 6/8/10/11 and were carried to close as deferred rather than cleared per-phase — reasonable for a solo Windows dev, but they are real un-run checks.

### Patterns Established

- **The frozen spine template.** One `#[derive(Serialize)]` output struct per command + `is_json_on()` fork + `out_line`/`emit_json`; a per-command `json_purity` test (no `0x1B`, no BOM, single value) as the regression backstop. `clip_feed` for the "print X, copy Y" case (qr copies source text, not glyphs).
- **Config precedence as a pure `.or()` chain.** `cli.or(env).or(config).unwrap_or(builtin)`, every overridable flag an `Option<T>` with no `default_value`; missing/malformed config never errors a normal command; one shared `effective_default_algo()` resolver so `config` can never lie about what the consuming command uses.
- **Destructive-op safety recipe.** Dry-run default + `--force` + pure(-ish) abort-all-before-any pre-flight + snapshot-the-tree-unchanged test per abort path + adversarial review — reused verbatim across all three v2 destructive flags.

### Key Lessons

1. **Isolate cross-cutting risk to a pilot before the rollout.** A shared spine flaw found on `uuid` costs 2 commands of rework; found on the 20th adopter it costs 20. Build the cross-cutting mechanism once, freeze it as a copy-me template, then apply mechanically.
2. **A breaking default must ship with its escape hatch in the same phase** — and leave the interop-critical path (here, `--verify` length→algo) untouched so existing data never silently breaks.
3. **One destructive operation per plan, each with its own adversarial review** — the review is where the real ordering bug lives (delete-before-verify), not in the happy-path verification.
4. **Close phase status in the completing step** (still unsolved from v1) — Phase 8's stale "In Progress" is the recurring drift tax; trust cross-artifact consensus at close.
5. **Clear human-verify UAT per phase, not at milestone close** — deferred manual confirmations are genuinely un-run, even when the code is fully test-verified.

### Cost Observations

- Model mix: predominantly opus (quality model profile) for planning/execution/review.
- Sessions: multiple across ~20 calendar days (2026-06-25 → 2026-07-14).
- Notable: the frozen Phase-6 template made Phase 7's 16-command rollout near-mechanical — the highest-leverage single investment of the milestone.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~5 | 5 | Baseline: risk-ordered phasing, allow-then-remove core, per-phase post-execution review |
| v2.0 | ~several | 6 | Cross-cutting spine built once on cheapest pilots then rolled mechanically; breaking change co-shipped with escape hatch; one adversarial review per destructive flag |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 150 bin-unit + all integration | 34/34 requirements | `human_size` promoted (no `humansize` crate); hand-rolled json colorizer (no colored_json); hand-rolled ascii (artem rejected) |
| v2.0 | 507 passing (0 failed) | 34/34 requirements | hand-rolled `core::config` (`toml`+`dirs`, no config framework); hand-rolled `core::cache` (TTL, miss-tolerant); `--json` via serde only; `--clip` via existing arboard |

### Top Lessons (Verified Across Milestones)

1. *(established v1.0)* Order phases by integration risk; attempt the hardest integration last.
2. *(established v1.0)* Verification and adversarial code review are complementary, not redundant.
3. *(established v2.0)* Isolate cross-cutting risk to a cheap pilot, freeze it as a copy-me template, then roll it out mechanically.
4. *(established v2.0)* A breaking default ships with its escape hatch in the same phase; leave interop-critical paths untouched.
5. *(recurring v1.0 → v2.0, still unsolved)* Phase/requirement status drifts stale unless the completing step closes it — trust cross-artifact consensus at milestone close.
