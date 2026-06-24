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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~5 | 5 | Baseline: risk-ordered phasing, allow-then-remove core, per-phase post-execution review |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 150 bin-unit + all integration | 34/34 requirements | `human_size` promoted (no `humansize` crate); hand-rolled json colorizer (no colored_json); hand-rolled ascii (artem rejected) |

### Top Lessons (Verified Across Milestones)

1. *(established v1.0)* Order phases by integration risk; attempt the hardest integration last.
2. *(established v1.0)* Verification and adversarial code review are complementary, not redundant.
