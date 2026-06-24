# Milestones: box — Rust CLI Toolbox

A running record of shipped versions. Newest first.

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
