---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_plan: 2
status: executing
stopped_at: Phase 3 Plan 03-01 (hash) complete — HASH-01 shipped; next 03-02 (tree)
last_updated: "2026-06-22T20:14:01.000Z"
progress:
  total_phases: 5
  completed_phases: 2
  total_plans: 14
  completed_plans: 10
  percent: 40
---

# Project State: box — Rust CLI Toolbox

**Last updated:** 2026-06-22
**Updated by:** plan-phase orchestrator (Phase 3 planned — 5 plans)

---

## Project Reference

**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

**Current Focus:** Phase 03 — filesystem-power-tools

**Milestone:** v1 (all 23 commands)

---

## Current Position

Phase: 03 (filesystem-power-tools) — EXECUTING
Plan: 2 of 5 (Plan 03-01 hash complete)
**Phase:** 3 (filesystem-power-tools)
**Current Plan:** 2
**Total Plans in Phase:** 5
**Status:** Executing Phase 03

**Progress:**

```
[████░░░░░░] 40% (2 / 5 phases complete)
Phase 1 [██████████] 4 / 4 plans ✓ complete
Phase 2 [██████████] 5 / 5 plans ✓ complete (verified, human-UAT cleared)
Phase 3 [██░░░░░░░░] 1 / 5 plans — 03-01 hash ✓ (HASH-01); next 03-02 tree
Phase 4 [          ] Not started
Phase 5 [          ] Not started

Overall: 2 / 5 phases complete
```

---

## Phase Map

| Phase | Name | Requirements | Status |
|-------|------|-------------|--------|
| 1 | Foundation + Flatten | FOUND-01..08, FLAT-01..04 (12 reqs) | ✓ Complete (4/4 plans) |
| 2 | Pure Transform Utilities | UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01 (9 reqs) | ✓ Complete (5/5 plans, verified, human-UAT cleared) |
| 3 | Filesystem Power Tools | HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 (5 reqs) | ◆ Executing (1/5 plans — 03-01 hash ✓ HASH-01) |
| 4 | Terminal Visuals | LOL-01, MTRX-01, ASCI-01, JSON-01 (4 reqs) | Not started |
| 5 | Windows Platform Integration | QR-01, CLIP-01, POMO-01, WTHR-01 (4 reqs) | Not started |

---

## Performance Metrics

**Plans executed:** 10
**Plans succeeded:** 10
**Plans failed:** 0
**Phases completed:** 2 / 5 (Phase 3 executing — 1/5 plans)

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 01 | P01 | 5min | 3 | 10 |
| 01 | P02 | 6min | 2 | 7 |
| 01 | P03 | 6min | 2 | 8 |
| 01 | P04 | 2min | 2 (human-verify cleared) | 2 |
| 02 | P01 | 5min | 3 | 6 |
| 02 | P02 | 5min | 2 | 9 |
| 02 | P03 | 6min | 2 (TDD) | 9 |
| 02 | P04 | 11min | 3 (2 TDD) | 11 |
| 02 | P05 | 6min | 2 (2 TDD) | 12 |
| 03 | P01 | 6min | 2 (1 TDD) | 7 |

---

## Accumulated Context

### Key Decisions

| Decision | Rationale |
|----------|-----------|
| Phase 1 bundles Foundation + Flatten | Foundation is not deliverable without a real command; flatten is the anchor command per PROJECT.md; shipping both together means Phase 1 ends with an installable, usable tool |
| Pure transforms in Phase 2 before filesystem tools | These 9 commands have zero external integration risk and prove the RunCommand pattern; finding architecture problems on `uuid` is cheaper than finding them on `flatten` |
| Filesystem tools in Phase 3 (after Phase 1 anchor) | All 5 share walkdir infrastructure already established by flatten; collision-rename and dry-run patterns are proven before being reused |
| Terminal visuals in Phase 4 (parallel-eligible with Phase 3) | crossterm dependency group is independent of walkdir group; lolcat teaches frame-buffered output before matrix |
| Platform commands in Phase 5 (last) | arboard, winrt-notification, and Open-Meteo carry the highest Windows API / external service risk; building them last means 21 other commands are working before the riskiest integrations are attempted |
| BLAKE3 as default hash in `hash` command ⚠️ SUPERSEDED by Phase 3 D-01 | Default is now **SHA-256** (the REQUIREMENTS.md HASH-01 / ROADMAP success-criterion #1 binding contract); BLAKE3 is available via `--algo blake3`. The faster-on-modern-CPUs rationale still justifies the opt-in, but BLAKE3-as-default is deferred to HASH-V2-01. Read this row as "BLAKE3 via `--algo blake3`" |
| `x86_64-pc-windows-msvc` target with crt-static | MinGW demoted to Tier 2 in Rust 1.88; MSVC required for arboard and winrt-notification; static CRT makes exe portable |
| winrt-notification needs Phase 1 compile spike | Maintenance status uncertain; validate it compiles before Phase 5 planning to avoid late-phase blocker |
| [01-01] Bare `box` prints help to stderr and exits 2 | Resolves OQ-1 toward strict "messages -> stderr" while satisfying D-08 (clap's `arg_required_else_help` default is exit 0) |
| [01-01] Stubs are real clap-derive enum variants dispatched to a NotImplemented handler | D-05 — keeps all 23 commands visible in `box --help` while only `flatten` will become functional |
| [01-01] main() owns the strict 0/1/2 exit-code policy via `Cli::try_parse` | `parse()` auto-exits 0 on bare box; `try_parse` lets main() preserve clap's exit 2 for parse errors (D-07) and force exit 2 for bare box (D-08) |
| [01-01] Committed `Cargo.lock` with the manifest | Binary crate — the lockfile is part of the reproducible-build contract |
| [01-02] Gate row coloring on our own `COLOR_ON` flag, not `owo_colors::set_override` | The plain `.green()` trait method is unconditional; `set_override` only affects the `if_supports_color` API. Consulting our own AtomicBool is what makes piped output byte-identical minus ANSI (D-10) |
| [01-02] Enabled the `owo-colors` `supports-colors` feature | Required for `set_override`/`with_override` to compile; the locked default feature set excluded it |
| [01-02] `safe_copy` preserves atime best-effort, mtime always | Some filesystems don't report `accessed()`; only mtime is mandated by FLAT-04, so a missing atime must not fail the copy (Assumption A3) |
| [01-03] flatten `encode_relative` strips `..`/`.` traversal segments (not just leading separators) | The RESEARCH sample left `.._escape.txt`; the threat register (T-03-pathinject) requires no literal `..` survives the encoded name |
| [01-03] flatten `sanitize_reserved` trims trailing dots/spaces BEFORE matching reserved stems | The RESEARCH order matched the untrimmed stem, so `con .txt` was not recognised as `CON`; reordering closes a hidden-collision gap |
| [01-03] flatten is first-claim-wins, deterministic by walkdir order | The first file to take a base name keeps it (Copy); later same-named files are collision-renamed — both always land with distinct names, never lost |
| [01-03] Removed both forward-compat `#[allow(dead_code)]` (core::output, core::fs) | flatten is now a live caller of every helper; clippy `-D warnings` stays clean, proving the reusable surface has no orphans |
| [01-04] install.ps1 authored to match the 01-RESEARCH annotated example exactly | Only additions are two defensive guards (post-build Test-Path on the produced exe, Copy-Item -LiteralPath) that harden the documented flow with no happy-path behavior change |
| [01-04] Release MSVC + crt-static link verified read-only (build only, no install) | `cargo build --release --target x86_64-pc-windows-msvc` with crt-static compiles clean and box.exe runs (box 0.1.0) — resolves the carried-over "MSVC+crt-static unverified" todo from 01-01/01-03; the actual install + user-PATH mutation is reserved for the human-verify gate |
| [01-post-review] flatten silent-overwrite hardening (CR-01/WR-01/WR-02, fixed e1a8f38) | `sanitize_reserved` now trims trailing dots/spaces from the WHOLE name (was stem-only) so Windows-truncated names like `report.`/`report` can't collapse onto one file; collision keys use full-Unicode `to_lowercase` (was ASCII-only, missed `RÉSUMÉ` vs `résumé`); `safe_copy` opens dst with `create_new` so a missed collision errors loudly instead of clobbering. 4 regression tests added; supersedes the original stem-only trim note above |
| [02-01] chrono added with `default-features = false, features = ["clock","std"]` | Trims `oldtime`/`wasmbind` per D-01 while keeping `Local` (needed by epoch D-12); verified `cargo build` still resolves `Local` |
| [02-01] core::input branch-3 returns `BoxError::MissingInput` via `.into()` (never `bail!`) | A typed variant downcasts in main.rs to `ExitCode::from(2)` (D-04 usage error); a type-erased anyhow error would wrongly map to exit 1 (RESEARCH Pitfall 2) |
| [02-01] Forward-compat `#[allow(dead_code)]` on core::input readers + `BoxError::MissingInput` | The foundation slice lands ahead of its Wave-2 callers; allows are documented to come off once the first command (base64/cowsay/epoch/color) becomes a live caller — mirrors the [01-03] allow-then-remove pattern |
| [02-01] Cargo.toml completed for all of Phase 2 in this plan | The four crates (uuid v4, base64, chrono, rand 0.9) added once so Wave-2 command plans never touch the manifest, keeping their file-ownership footprints parallel-clean |
| [02-02] base64 is the first live `core::input::read_input_bytes` consumer; removed the forward-compat `#[allow(dead_code)]` from the byte path | base64 calls `read_input_bytes` and constructs `BoxError::MissingInput` (no-arg interactive TTY → exit 2), so the byte-path allow (read_input_bytes/resolve_bytes/MissingInput) came off, restoring the strict dead-code gate. The String readers (`read_input`/`resolve`) keep their scoped allow until cowsay/epoch/color go live — allow-then-remove is per-item by call-graph reachability, not per-module (STATE.md [01-03] pattern) |
| [02-02] uuid/base64 anchored v4-regex assertions match the single trimmed line, not raw stdout | Captured stdout carries a trailing newline, so `predicate::str::is_match("^…$")` against the whole buffer never matches a correct UUID; the tests split to lines and match the trimmed line (caught during GREEN, test-side fix only) |
| [02-02] base64 decode uses `from_utf8_lossy + .trim()` then `engine.decode`, and writes raw bytes via `stdout().write_all` | Trimming tolerates the piped trailing newline (Pitfall 3); writing bytes (not a String) keeps decoded output byte-exact incl. non-UTF-8 (T-02-04); a malformed alphabet maps to an `anyhow` Err → exit 1 with no panic (T-02-03) |
| [02-03] epoch self-resolves input (no-arg = "print now", NOT exit-2 missing-input) so it does NOT call `core::input::read_input`; color requires input so it delegates to `read_input` | For epoch a no-arg interactive TTY means "print the current timestamp" (a feature), not the missing-input/exit-2 case — so epoch has its own `resolve_value`. color requires input and inherits the exit-2-on-no-arg-TTY contract via `read_input`. This makes color, NOT epoch, the first live String-path caller |
| [02-03] color is the first live `core::input::read_input` (String) consumer; removed the forward-compat `#[allow(dead_code)]` from `read_input` + `resolve` | Mirrors the 02-02 byte-path removal and the [01-03] allow-then-remove precedent: the byte path went live with base64, the String path now lives with color, so `core::input` carries no forward-compat allows. The color swatch is the ONLY color path — gated solely on `is_color_on()` (no `set_override`, no background-SGR fill) so piped output is byte-identical minus ANSI (D-10) |
| [02-04] A1 closed to fact: `rand::TryRngCore` resolves under rand 0.9 (compiled `cargo --example` probe) → Cargo.toml UNCHANGED, no `rand_core` dep; passgen RNG is `OsRng.unwrap_err()` (D-08 literal) | The plan flagged A1 as a LOW-risk assumption; a one-line probe with the full import chain (`OsRng`/`TryRngCore`/`IndexedRandom::choose`/`random_range`) compiled clean, so the re-export path works and `rand_core` was never added. The bias-freedom + CSPRNG-source guarantee is a grep code-review gate (OsRng present, no `% len`), NOT a statistical test (T-V6) |
| [02-04] passgen passphrase separator is a DOT, not a hyphen | Some EFF words are hyphenated (`t-shirt`, `yo-yo`, `drop-down`, `felt-tip`), so a hyphen separator makes word boundaries ambiguous; a dot is paste-safe in PS7 and never appears inside an EFF word (Rule-1 fix of a latent ambiguity). EFF list stored words-only (dice codes stripped) + `.gitattributes eol=lf` so no `\r` leaks via `include_str!` on a CRLF (autocrlf=true) checkout |
| [02-04] cowsay fixed-40 wrap (NOT terminal width); trycmd normalizes `\`→`/` in snapshots (RESEARCH A4) | Fixed width keeps pipe-vs-TTY output reproducible (D-11). trycmd's Windows path normalization converts the cow's backslashes to forward slashes in the stored snapshot, so the byte-exact bubble (with real `\`) is locked by the `bubble` unit tests and the trycmd files are the end-to-end render lock — do not fight the harness. EFF CC-BY 3.0 US attribution attached via clap `after_help` (the variant doc-comment is locked byte-identical by help.trycmd) |
| [02-05] Whimsy RNG is `rand::rng()` (OS-seeded ThreadRng), NOT `OsRng` | fortune/roast/8ball are decorative, not security — no CSPRNG requirement (D-08). A fresh process reseeds from the OS so repeated calls differ. Unbiased `IndexedRandom::choose` is still used (over `% len`) as a distribution-quality choice, not a security gate. Non-determinism is tested by PROPERTY only (membership + N=10-runs-≥2-distinct), never a seeded/exact value (RESEARCH Pattern F) |
| [02-05] fortune fits-the-terminal soft-wrap breaks only between words and only when the line exceeds `terminal_width()` (FORT-01, Open Question 3) | Wrapping at word boundaries keeps the wrapped render whitespace-equal to its source entry, so the membership test stays valid (an over-long single word is left whole). roast reuses the same helper for a consistent UX at near-zero cost |
| [02-05] 8ball question accepted but discarded for the draw (`let _ = self.question;`) | Classic 8-ball; the answer is drawn uniformly regardless. Makes the no-injection-surface contract (T-02-10) self-documenting. The Rust module is `eight_ball` (digit-leading-ident pitfall) while the CLI name stays `8ball` via the preserved `#[command(name = "8ball")]` attribute |
| [02-05] List commands share one source of truth: integration tests `include_str!` the SAME asset the binary embeds and parse it identically | Membership assertions cannot drift from shipped data (no hardcoded duplicate of the 70/42 lists). 8ball's 20 are duplicated in the test — a `const` doesn't re-export cheaply — but the in-module count + tone-split + non-empty unit tests guard the const's shape. `.gitattributes eol=lf` reused for both new text assets (CRLF-leak root-cause fix from 02-04) |
| [03-01] hash hex encoding uses `const-hex::encode` (already a dep), NOT `base16ct` `alloc` | Resolves the RESEARCH open item with ZERO Cargo.toml change: `const_hex::encode` takes the digest-0.11 hybrid-array `finalize()` output directly (`AsRef<[u8]>`, no `.as_slice()`) and was verified to match `sha256sum`. blake3 self-hexes via `to_hex()`. The `base16ct` `alloc` feature stays off |
| [03-01] hash enum-dispatch Hasher: one generic `hash_rustcrypto<D: Digest>` (sha256/sha512/md5) + a SEPARATE native `blake3::Hasher::update_reader` arm — NO `dyn Digest`, NO `traits-preview` | D-03. blake3's `digest::Digest` impl is behind the unstable `traits-preview` feature, so it gets its own arm on the stable native `Hasher`; every algorithm streams (64 KiB loop for RustCrypto, SIMD-internal for blake3) — no whole-file buffering (T-03-03). This Hasher infra is what `dupes` (03-04) reuses |
| [03-01] `core::input::read_file_or_stdin` returns a streaming `ResolvedInput { reader: Box<dyn Read>, label }`, NOT bytes; `--file` branch sits AHEAD of stdin | hash must stream a multi-GB payload, so the new layer carries an open handle + a coreutils label (path, or `-` for stdin) rather than a `Vec` — distinct from the byte/String resolvers. `-` sentinel + `MissingInput`→exit-2 inherited. `ResolvedInput` needed a manual `Debug` impl (Box<dyn Read> isn't Debug) for test `.unwrap_err()` |
| [03-01] hash `--verify`: only an UNSUPPORTED length is the typed exit-2 `UnsupportedHashLength`; a well-formed-but-mismatched hash is a plain `bail!` (exit 1) | D-04 / Pitfall 1. Length auto-detect maps 32→md5, 64→sha256 (wins the sha256/blake3 64-tie — `--algo blake3` is the only way to verify a 64-hex blake3), 128→sha512; `--algo` is the explicit override. Compare is plain `eq_ignore_ascii_case` (a checksum is PUBLIC, NOT constant-time — T-03-01) |
| [03-01] `box` is a binary-only crate, so `cargo test --lib` does NOT work | The plan's `cargo test --lib core::input` verify command errors (`no library targets`); the in-module unit tests run via `cargo test --bin box <filter>` (or the default `cargo test`). Note for all future Phase-3 plans that verify with `--lib` |

### Critical Pitfalls to Remember

- Use `dunce::canonicalize` everywhere — never `std::fs::canonicalize` (produces UNC paths)
- Call `enable_ansi_support::enable_ansi_support()` as first line of `main()` before any output
- `install.ps1` must refresh `$env:Path` in the current session (merge user + machine PATH from registry)
- `flatten` must canonicalize both src and dest before walker starts; abort if dest is inside src
- `matrix` must buffer full frame and flush once per frame (not per character — causes ~5 FPS)
- `arboard` clipboard must run on main thread only
- Windows reserved filenames (`CON.txt`, `NUL.txt`) must be sanitized in `flatten` output
- `8ball` maps to Rust module `eight_ball` (identifiers cannot start with a digit)
- Build target: `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"`

### Architecture Established

- Single Rust crate (not workspace); `src/commands/<cmd>/mod.rs` per command
- `RunCommand` trait: `fn run(self) -> anyhow::Result<()>` implemented by each Args struct
- `src/core/`: `errors.rs` (BoxError + thiserror), `output.rs` (color init + print helpers), `fs.rs` (walkdir wrapper, safe_copy, collision rename)
- `src/main.rs`: ~40 lines, parse + dispatch + exit code only, no business logic
- Integration tests via `assert_cmd` in `tests/<cmd>.rs`; snapshot tests via `insta`/`trycmd`

### Todos

- [ ] Spike `winrt-notification 0.5` compilation against project MSRV before Phase 5 planning (NOT done in Phase 1; the release MSVC + crt-static build IS verified, but the winrt crate itself was not exercised)
- [ ] Decide `pomodoro` blocking vs non-blocking timer before Phase 5 planning
- [ ] Decide `weather` default unit system (metric / imperial / locale-detect) before Phase 5 planning
- [ ] Add `strip-ansi-escapes` crate to Cargo.toml for `lolcat` during Phase 4 planning
- [ ] Code-review advisory follow-ups (01-REVIEW.md, non-blocking): WR-03/WR-04 `install.ps1` PATH empty-segment + smoke-test-by-abspath; IN-02/IN-03 share one flatten render path between dry-run and real run

### Blockers

None.

---

## Session Continuity

**To resume:** Read `.planning/ROADMAP.md` for phase goals, then read `.planning/STATE.md` (this file) for current position and context.

**Last session:** 2026-06-22T20:14:01.000Z
**Stopped At:** Phase 3 Plan 03-01 (hash) complete — HASH-01 shipped, full suite green
**Resume File:** .planning/phases/03-filesystem-power-tools/03-02-PLAN.md

**Next action:** Execute Plan 03-02 (`tree`) — Wave 2. It shares cli.rs/main.rs and promotes flatten's `human_size` into `core::output` (D-12). Reuse the 03-01 `core::input::read_file_or_stdin` pattern if a streaming input is needed.

---
*State initialized: 2026-06-22 by roadmapper*
*Updated: 2026-06-22 by execute-phase orchestrator — Phase 1 COMPLETE (human-verify cleared, verification passed 5/5, flatten review findings CR-01/WR-01/WR-02 fixed)*
*Updated: 2026-06-22 by plan-02-02 executor — uuid + base64 shipped (UUID-01, B64-01); strict dead-code gate restored on the core::input byte path*
*Updated: 2026-06-22 by plan-02-03 executor — epoch + color shipped (EPOC-01, COLR-01); strict dead-code gate restored on the core::input String path (color is the first live read_input caller); first reuse of the core::output is_color_on() gate by a new styled command*
*Updated: 2026-06-22 by plan-02-04 executor — passgen + cowsay shipped (PASS-01, COW-01); OsRng CSPRNG + unbiased choose (no % len, T-V6 grep gate); EFF 7776 list embedded + CC-BY 3.0 US attributed; A1 closed (rand::TryRngCore resolves, no rand_core); cowsay fixed-40 wrap + hard-break + bubble locked by units + 2 trycmd snapshots*
*Updated: 2026-06-22 by plan-02-05 executor — fortune + 8ball + roast shipped (FORT-01, 8BAL-01, ROST-01); whimsy RNG = rand::rng() + unbiased IndexedRandom::choose, non-determinism tested by membership + N=10-runs-differ properties; 70 CC0 fortunes + 42 self-authored roasts embedded (include_str! + eol=lf); 8ball canonical-20 const in the eight_ball module with 8ball CLI name preserved; ALL 9 Phase-2 stubs gone — Phase 2 plans complete (9/9), ready for verification; full cargo test + clippy -D warnings + fmt --check clean*
*Updated: 2026-06-22 by plan-03-01 executor — `box hash` shipped (HASH-01): streaming enum-dispatch Hasher (SHA-256 default; --algo blake3/sha512/md5; RustCrypto digest 0.11 + native blake3 update_reader, no traits-preview/no dyn Digest), const-hex hex (no base16ct alloc change), --verify length-autodetect (32/64/128, sha256 wins the 64-tie) with the 0/1/2 exit contract; core::input grew read_file_or_stdin + ResolvedInput (streaming --file ahead of stdin); BoxError::UnsupportedHashLength exit-2 variant added; 7/7 HASH-01 tests + full suite + clippy -D warnings + fmt --check all green; hash stub gone (4 phase-3 stubs remain: tree/du/dupes/bulk-rename)*
