# Milestones: box — Rust CLI Toolbox

A running record of shipped versions. Newest first.

---

## v2.0 — Toolbox → Toolkit

**Shipped:** 2026-07-14
**Phases:** 6 (Phases 6–11) | **Plans:** 23 | **Tasks:** ~70

**Delivered:** Deepened the shipped 23-command `box` binary into a scriptable PowerShell-7 toolkit — a cross-cutting `--json`/`--clip`/config spine across every applicable command, comprehensive per-command depth flags (including three safety-gated destructive filesystem operations), a BLAKE3-default `hash` with a config escape hatch, and the two frictionless-PS7 meta-commands (`config`, `completions`) generated against the final arg surface. A deepening, not a rewrite — the v1 architecture was preserved throughout.

### Stats

- **Phases:** 6 (Phases 6–11), all verified (human-in-PS7 confirmations deferred as UAT)
- **Plans:** 23 | **Tasks:** ~70
- **Source:** 15,649 Rust LOC across 36 `.rs` files (+10,174 test LOC)
- **Tests:** 507 passing, 0 failed; clippy `--all-targets -D warnings` clean
- **Commits:** 198 total (46 `feat`)
- **Timeline:** 2026-06-25 → 2026-07-14 (~20 days)
- **Git range:** `24c558f` → `v2.0`
- **Requirements:** 34/34 shipped and validated (coverage 34/34, 0 unmapped, 0 dropped)

### Key Accomplishments

1. **Scriptable output spine across all 23 commands** — `--json` on the 16 applicable value-producing commands + `--clip` on the 8 single-textual-result commands, all following one house style (one buffered JSON document, no BOM, no ANSI, no chrome; per-command `json_purity` regression test). Proven on the `uuid`+`hash` pilots in Phase 6, then rolled mechanically across the toolkit in Phase 7 (SPINE-01..05).
2. **BLAKE3-default `hash`** (the major-version breaking change) co-shipped with its config escape hatch — `box hash file` now emits BLAKE3; `--algo sha256` and `hash.default_algo` restore old behavior; the `--verify` length table is untouched so stored SHA-256 baselines never silently break, with a D-05 BLAKE3-fallback diagnostic on a 64-hex mismatch (HASH-V2-01).
3. **Filesystem depth (10 flags)** across hash / flatten / dupes / bulk-rename / tree / du — multi-file hash, flatten filters + `--move`, multi-stage hardlink-aware dupes + `--delete`, bulk-rename `--case`/`{n}` + `--backup`, tree gitignore/ignore/dirs-only/sort, du percentage/exclude + `--on-disk`. The three destructive flags (`flatten --move`, `dupes --delete`, `bulk-rename --backup`) each passed a per-plan adversarial code-review gate: dry-run default, `--force`, abort-all-before-any pre-flight, snapshot-the-tree-unchanged test per abort path.
4. **Dev-transform & visual depth (9 flags)** — uuid v7 + format flags, epoch relative/`--tz`, color CSS+HSL, `json --sort-keys`, passgen entropy/`--no-similar`/`--separator`, matrix color/speed/charset, qr `--save` PNG/SVG + EC, ascii truecolor/`--braille`/`--invert`, and the PS7-human-verified animated `lolcat` (bounded alternate-screen rainbow reusing matrix's RAII restore behind an `is_terminal()` gate).
5. **Fun & system depth (7 flags)** — cowsay figures/think-mode, fortune categories, 8ball ASCII art + sentiment color, roast `--language`, pomodoro session-counter/auto-cycle/`--label`/`--sound` (Win32 `MessageBeep`, no audio stack), and weather `--forecast` + response cache + config-stored default location.
6. **Two frictionless-PS7 meta-commands** — `box config show/get/set/path` (+`--json` effective view, validate-before-write atomic config, closed settable-key registry, a self-inflicted exit-2 lockout structurally impossible) and `box completions <shell>` generated from the live final `Cli` via `CommandFactory` so it reflects every subcommand and depth flag, with `install.ps1 -RegisterCompletions` opt-in `$PROFILE` registration (CFG-01, CMP-01).

### Quality Gates

- Every phase shipped verified; the three destructive filesystem flags each cleared a mandatory adversarial code-review gate.
- Code review caught and fixed two real defects verification alone missed: a pre-existing v1 bulk-rename Windows trailing-dot/reserved-name silent-clobber data-loss path (Phase 8 CR-01) and a `lolcat --duration` `Instant` overflow that would bypass the terminal-restore RAII guard under `panic = "abort"` (Phase 9 BL-01).
- Phase 11 verification found 2 gaps (WR-01 env-tier parity, WR-02 malformed-config decoupling); both closed by the 11-03 gap-closure plan before close (507 tests / 0 failures).

### Known Deferred Items at Close: 9 (see STATE.md Deferred Items)

- **9 human-verify PS7 items** persisted as UAT — clipboard round-trips (Phase 6), live stderr progress bar + `du --on-disk` vs Explorer on an NTFS-compressed file (Phase 8), pomodoro auto-cycle UX + weather live network/cache + ASCII/color visual quality (Phase 10), PS7 tab-completion via PSReadLine (Phase 11). All are manual confirmations requiring a live PowerShell 7 terminal; none are code defects (all 34 requirements are code-complete and test-verified).
- `bulk-rename --undo` manifest replay deferred (manifest is written and reconcilable; manual reverse documented).
- Repo-wide rustfmt drift — a `style: cargo fmt` root sweep deferred (newly-authored code is fmt-clean; shipping gates use clippy `-D warnings`, green). Orphaned `core::config::resolve_algo` dead code (advisory).

**Archived:** `milestones/v2.0-ROADMAP.md` · `milestones/v2.0-REQUIREMENTS.md`
**Tag:** `v2.0`

---

## v1.0 — Full Toolbox

**Shipped:** 2026-06-24
**Phases:** 5 (Phases 1–5) | **Plans:** 22 | **Tasks:** ~47

**Delivered:** A single Rust binary bundling all 23 command-line tools as subcommands, installable globally from PowerShell 7 via `install.ps1` — every command real, verified, and reachable on PATH.

### Stats

- **Phases:** 5 (all verified: 5/5 → 24/24 → 4/4 → 16/16 must-haves)
- **Plans:** 22 | **Tasks:** ~47
- **Source:** 7,748 Rust LOC across 32 `.rs` files
- **Tests:** 150 bin-unit + all integration suites green; clippy `--all-targets -D warnings` + `fmt --check` clean
- **Commits:** 162 total (32 `feat`)
- **Timeline:** 2026-06-22 → 2026-06-24 (3 days)
- **Git range:** `960b716` → `8e96e8a`
- **Release artifact:** `x86_64-pc-windows-msvc` + `crt-static` `box.exe` (5.1 MB, portable)

### Key Accomplishments

1. **Single-binary `box` with all 23 subcommands live** — one clap-derive registry, `install.ps1` global PATH install (same-session, human-verified in PS7), strict 0/1/2 exit-code + stdout/stderr discipline, NO_COLOR/TTY-gated ANSI byte-identical-minus-color when piped.
2. **`flatten` anchor** — recursive flatten with source-path collision encoding, dry-run-default safety, symlink/loop guards; silent-overwrite edge cases (Windows trailing dot/space, non-ASCII case-fold, unconditional copy) hardened post-review.
3. **9 pure transform utilities** — uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast; CSPRNG passgen (OsRng, unbiased, EFF wordlist), byte-exact base64 round-trip, gated truecolor color swatch.
4. **5 filesystem power tools** — hash (streaming multi-algo, SHA-256 default), tree, du, dupes (rayon-parallel BLAKE3, read-only), bulk-rename (abort-all-before-any-rename pre-flight, the backstop vs `std::fs::rename`'s silent Windows overwrite).
5. **4 terminal visuals** — json (preserve-order, syntax-colored), lolcat (per-Unicode-scalar sine gradient, unconditional ANSI strip), ascii (image→ASCII), matrix (single-flush-per-frame katakana rain, RAII terminal restore).
6. **4 Windows-platform commands** — qr (Unicode half-block, phone-scan verified), clip (arboard clipboard round-trip), pomodoro (raw-mode countdown + WinRT toast), weather (keyless Open-Meteo).

### Quality Gates

- Every phase shipped verified; human-UAT cleared on four of five phases (Phase 4 matrix + Phase 5 trio human-verified in PS7).
- Post-execution code review on Phases 1, 3, 4, 5 — 2 BLOCKERs (bulk-rename path-escape, matrix raw-mode-stuck) + multiple warnings fixed with covering tests before close.
- Two CLAUDE.md crate recommendations overridden during execution after slop-check/compat validation: `qrcode` over qr2term (D-01), `tauri-winrt-notification` over winrt-notification (D-09).

### Architecture Established

- Single Rust crate (not workspace); `src/commands/<cmd>/mod.rs` per command; `RunCommand` trait (`fn run(self) -> anyhow::Result<()>`).
- `src/core/`: `errors.rs` (BoxError + thiserror), `output.rs` (color init + helpers + `human_size` + `terminal_width`), `fs.rs` (walkdir wrapper, safe_copy, collision rename, `is_hidden`), `input.rs` (read_input / read_input_bytes / read_file_or_stdin).
- `src/main.rs` ~40 lines: parse + dispatch + exit code only.
- Integration tests via `assert_cmd`; snapshot tests via `trycmd`.

### Known Deferred Items

- v2 per-command differentiators (HASH-V2-01, VIS-V2-01, DEV-V2-01, FUN-V2-01, SYS-V2-01, PASS-V2-01, and per-tool FLAT/TREE/DU/DUPE/RENM-V2) — tracked in the v1.0 requirements archive.
- Phase 1 advisory follow-ups (non-blocking, 01-REVIEW.md): install.ps1 PATH empty-segment + smoke-test-by-abspath (WR-03/WR-04); shared flatten render path between dry-run and real run (IN-02/IN-03).

**Archived:** `milestones/v1.0-ROADMAP.md` · `milestones/v1.0-REQUIREMENTS.md`
**Tag:** `v1.0`

---
