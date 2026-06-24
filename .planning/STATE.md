---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_plan: Not started
status: executing
stopped_at: Phase 4 planned (4 plans)
last_updated: "2026-06-24T12:21:21.905Z"
progress:
  total_phases: 5
  completed_phases: 3
  total_plans: 18
  completed_plans: 14
  percent: 60
---

# Project State: box — Rust CLI Toolbox

**Last updated:** 2026-06-24
**Updated by:** plan-phase orchestrator (Phase 4 planned — 4 plans)

---

## Project Reference

**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

**Current Focus:** Phase 4 — terminal visuals

**Milestone:** v1 (all 23 commands)

---

## Current Position

Phase: 03 (filesystem-power-tools) — ✓ COMPLETE & VERIFIED (5/5 plans; 24/24 must-haves; human-UAT cleared; code-review BLOCKER+5 warnings fixed)
**Phase:** 4 (terminal-visuals)
**Current Plan:** Not started
**Total Plans in Phase:** 4
**Status:** Ready to execute

**Progress:**

```
[██████░░░░] 60% (3 / 5 phases complete)
Phase 1 [██████████] 4 / 4 plans ✓ complete
Phase 2 [██████████] 5 / 5 plans ✓ complete (verified, human-UAT cleared)
Phase 3 [██████████] 5 / 5 plans ✓ complete (verified 24/24, human-UAT cleared) — 03-01 hash ✓ (HASH-01), 03-02 tree ✓ (TREE-01), 03-03 du ✓ (DU-01), 03-04 dupes ✓ (DUPE-01), 03-05 bulk-rename ✓ (RENM-01)
Phase 4 [          ] Not started
Phase 5 [          ] Not started

Overall: 3 / 5 phases complete
```

---

## Phase Map

| Phase | Name | Requirements | Status |
|-------|------|-------------|--------|
| 1 | Foundation + Flatten | FOUND-01..08, FLAT-01..04 (12 reqs) | ✓ Complete (4/4 plans) |
| 2 | Pure Transform Utilities | UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01 (9 reqs) | ✓ Complete (5/5 plans, verified, human-UAT cleared) |
| 3 | Filesystem Power Tools | HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 (5 reqs) | ✓ Complete (5/5 plans, verified 24/24, human-UAT cleared) — 03-01 hash ✓ HASH-01, 03-02 tree ✓ TREE-01, 03-03 du ✓ DU-01, 03-04 dupes ✓ DUPE-01, 03-05 bulk-rename ✓ RENM-01 |
| 4 | Terminal Visuals | LOL-01, MTRX-01, ASCI-01, JSON-01 (4 reqs) | Not started |
| 5 | Windows Platform Integration | QR-01, CLIP-01, POMO-01, WTHR-01 (4 reqs) | Not started |

---

## Performance Metrics

**Plans executed:** 14
**Plans succeeded:** 14
**Plans failed:** 0
**Phases completed:** 3 / 5 (Phase 3 complete — 5/5 plans, verified 24/24, human-UAT cleared)

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
| 03 | P02 | 4min | 2 (TDD-style) | 9 |
| 03 | P03 | 7min | 2 (TDD-style) | 5 |
| 03 | P04 | 4min | 2 (TDD-style) | 5 |
| 03 | P05 | 6min | 2 (TDD-style) | 5 |

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
| [03-02] `human_size` PROMOTED verbatim to `core::output` (made `pub`, test migrated); flatten re-pointed, local copy + test deleted | D-12. The 1024-based B/KB/MB/GB/TB formatter is now the single shared size helper — tree consumes it now, du (03-03) next. Zero Cargo.toml change (no `humansize` crate). Flatten's 8 integration tests verified still green after the move (behavior-preserving) |
| [03-02] tree renders children via `WalkDir::new(dir).min_depth(1).max_depth(1).follow_links(false).filter_entry(!is_hidden)` per level — NOT `std::fs::read_dir` | `core::fs::is_hidden` takes a `walkdir::DirEntry`, so a WalkDir depth-1 per-directory walk reuses the shared hidden predicate VERBATIM (root exemption walkdir#142 + Windows hidden-attr + symlink no-follow all inherited, D-06/T-03-05) while still giving the per-level is-last control the box-drawing prefixes need. Never re-implement `is_hidden` |
| [03-02] tree's box-drawing glyphs are Unicode STRUCTURE (`├── └── │   ` + gap), distinct from flatten's ASCII `+`/`~`/`-` STATUS glyphs; only dir names are colored (`.blue().bold()`) gated on `is_color_on()` | D-09/D-10. Prefix is accumulated down the recursion (`│   ` for a non-last ancestor, `    ` gap for a last one); branch is `└── ` (last) vs `├── ` (non-last). The single styled token (dir name) is gated so piped output is byte-identical minus ANSI — proven by `tree_piped_no_ansi` |
| [03-02] `tree.trycmd` is backed by a `tree.in/` per-case input fixture (trycmd 1.2 sandbox) | trycmd copies `<name>.in/` into a sandbox and runs the case there, giving `box tree project` a stable, checked-in input tree across machines. Fixture files written with explicit byte content and NO trailing newline so on-disk sizes are CRLF-independent. Root label printed as the passed path (`self.path`), not the dunce-canonical absolute, for a natural render + stable snapshot |
| [03-03] `du --depth N` caps the per-directory recursive ROLLUP (dir's own files = depth 1, via `WalkDir::max_depth(N)`); the summary `{TOTAL}` then sums the CAPPED row totals | D-11. Internally consistent: under `--depth 1`, every immediate-child total is the capped sum, so the grand total is the capped sum too (the summary always equals the sum of the shown-vs-all rows' totals). "Full scan" in D-11 means "all immediate children," NOT "ignore the depth cap" — the cap applies uniformly to rows and total. A Wave-0 test that asserted `4.9 KB` absent from the WHOLE output was corrected to assert the `big/` ROW line (the summary legitimately shows the capped grand total); impl was already correct (Rule-1 test-side fix) |
| [03-03] `du` reuses `core::output::human_size` (D-12, third consumer after flatten/tree) + `core::fs::is_hidden`/`follow_links(false)` VERBATIM for the recursive descendant sum; size column right-aligned to the widest SHOWN value, only the size VALUE colored (`.cyan()`) gated on `is_color_on()` | Logical size via `metadata().len()` (RESEARCH A4 — NOT apparent/on-disk, that's DU-V2); symlinks never followed (0 contribution). Determinism by `collect → sort_by (size desc, name asc)` before printing with distinct-size test fixtures (RESEARCH Pitfall 6 / T-03-12) — the same discipline `dupes` (03-04, rayon) reuses. `--top N` is a POST-SORT truncation of shown rows; the full-scan total is captured BEFORE truncation so the summary always reflects the whole scan |
| [03-04] `dupes` identity = size pre-filter (`HashMap<u64, Vec<PathBuf>>`) THEN content hash; only same-size buckets of `>= 2` are candidates (most files never hashed); candidates content-hashed in PARALLEL with `rayon::par_iter` (first hash error short-circuits the `collect::<anyhow::Result<Vec<_>>>()` → exit 1, no panic, T-03-17) | D-13. BLAKE3 chosen for SPEED — cryptographic-criticality is irrelevant for equality grouping. The few-line `update_reader` native streaming path was LIFTED into `dupes` (`hash_reader_blake3`) rather than widening the `hash` module's surface (the plan's `<interfaces>` note sanctioned this since `hash::hash_blake3` is private) — same algorithm/result, unit-tested against the same `b"box"` known vector |
| [03-04] `dupes` is STRICTLY READ-ONLY (T-03-13, locked Out of Scope): NO `safe_copy`/rename/delete/`File::create` — the only fs handle is a read-only `File::open` for hashing; the `dupes_never_writes` test snapshots the fixture's file set + contents + mtimes and asserts byte-for-byte unchanged after a run | Determinism by `sort_by((hash, path))` BEFORE grouping (consecutive-equal-hash runs ≥2 → groups; RESEARCH Pitfall 6 / T-03-16) with distinct-content test fixtures making the order total. Wasted space = Σ `(group_len - 1) * file_size` via `core::output::human_size` (fourth consumer). Reuses `core::fs::is_hidden` + `follow_links(false)` + `normalize_path` VERBATIM, NO noise list / NO `ignore` crate (D-06/D-07); single `.yellow()` accent gated on `is_color_on()` |
| [03-05] `bulk-rename`'s safety is a PURE I/O-free `preflight(&[Rename], &[existing]) -> Vec<Conflict>` implementing all four D-18 rules; a thin `preflight_plan` wrapper partitions the plan per parent dir (collision scope is per-dir, D-14) and `read_dir`-seeds each occupied set. This is the ENTIRE safety story because `std::fs::rename` SILENTLY OVERWRITES on Windows (no `create_new` analog for moves) — a missed collision is silent data loss. `Conflict` is a 3-variant enum (Collision/Cycle/Separator) so the abort summary explains each clash and the preview stamps the right inline reason. 9 unit tests cover every rule (incl. full-Unicode fold WR-01, renamed-away exclusion, swap-cycle) | Dry-run is the DEFAULT (writes nothing), `--force` executes only after a clean pre-flight (D-19) — INVERTING flatten's plan→preview→execute split while reusing `format_row`/`arrow_col`/`RowStatus::{Rename,Skip}` VERBATIM. Case-only rename (foo→Foo) is correct by construction: byte-exact no-ops are filtered to `(unchanged)` BEFORE preflight, so any rename whose target folds to its own source key is necessarily a real case-only change (Pitfall 5 closed, no special case). Regex `replace` is FIRST-match-only over the FULL base name (D-16/D-17); `${1}abc` vs `$1abc` foot-gun documented in `--help`. Every abort path snapshot-asserts the dir byte-for-byte unchanged. ⚠️ Rule-1 test-side fix: 2 tests asserted via case-EXACT `read_dir` listing not `Path::exists()` (NTFS is case-insensitive/preserving, so exists() falsely matched `IMG_*.jpg` vs `img_*.jpg`) |
| [03-post-review] Phase-3 code-review fixes — CR-01 BLOCKER + WR-01..05 (9f4cf08/518f5b6/a147ab7/5dba60d/42da3db/f4114d8) | CR-01: bulk-rename pre-flight `injects()` now folds `..`/`.`/pure-dots/whitespace-only targets into the SAME rule-4 `Conflict::Separator` refusal as `/`/`\` — closes a path-escape outside the target dir on the ONLY destructive command (std::fs::rename silently overwrites on Windows, no create_new for moves). WR-01: hash `algo` is now `Option<Algo>` so `--verify` length auto-detect fires ONLY when `--algo` is unset (explicit `--algo sha256 --verify <32hex>` no longer mis-verifies as md5). WR-02: tree/du/dupes `bail!` on a file (non-dir) arg. WR-03: friendly "no such directory" pre-check before normalize_path. WR-04: du `--depth`/`--top` + tree `--depth` reject 0 at parse (RangedU64ValueParser, exit 2). WR-05: recursive cross-dir rename test. Each fix has a covering test; 4 INFO findings deferred (03-REVIEW.md). Full suite 98 green + clippy -D warnings + fmt clean |

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

**Last session:** 2026-06-23T15:27:43.044Z
**Stopped At:** Phase 4 context gathered
**Resume File:** .planning/phases/04-terminal-visuals/04-CONTEXT.md

**Next action:** Phase 4 context gathered (04-CONTEXT.md, 14 decisions across lolcat/matrix/ascii/json). Next: `/gsd-plan-phase 4`. Phase-4 new Cargo deps decided: `image` 0.25.10 (ascii, hand-roll — artem rejected), `serde_json` 1.0.150 + `preserve_order` (json), `unicode-width` 0.2 + `strip-ansi-escapes` 0.2 (lolcat — actions the pre-existing strip-ansi todo); crossterm/owo-colors already present (matrix katakana + colors). `box` remains binary-only — unit tests via `cargo test --bin box`, NOT `--lib`.

---
*State initialized: 2026-06-22 by roadmapper*
*Updated: 2026-06-22 by execute-phase orchestrator — Phase 1 COMPLETE (human-verify cleared, verification passed 5/5, flatten review findings CR-01/WR-01/WR-02 fixed)*
*Updated: 2026-06-22 by plan-02-02 executor — uuid + base64 shipped (UUID-01, B64-01); strict dead-code gate restored on the core::input byte path*
*Updated: 2026-06-22 by plan-02-03 executor — epoch + color shipped (EPOC-01, COLR-01); strict dead-code gate restored on the core::input String path (color is the first live read_input caller); first reuse of the core::output is_color_on() gate by a new styled command*
*Updated: 2026-06-22 by plan-02-04 executor — passgen + cowsay shipped (PASS-01, COW-01); OsRng CSPRNG + unbiased choose (no % len, T-V6 grep gate); EFF 7776 list embedded + CC-BY 3.0 US attributed; A1 closed (rand::TryRngCore resolves, no rand_core); cowsay fixed-40 wrap + hard-break + bubble locked by units + 2 trycmd snapshots*
*Updated: 2026-06-22 by plan-02-05 executor — fortune + 8ball + roast shipped (FORT-01, 8BAL-01, ROST-01); whimsy RNG = rand::rng() + unbiased IndexedRandom::choose, non-determinism tested by membership + N=10-runs-differ properties; 70 CC0 fortunes + 42 self-authored roasts embedded (include_str! + eol=lf); 8ball canonical-20 const in the eight_ball module with 8ball CLI name preserved; ALL 9 Phase-2 stubs gone — Phase 2 plans complete (9/9), ready for verification; full cargo test + clippy -D warnings + fmt --check clean*
*Updated: 2026-06-22 by plan-03-01 executor — `box hash` shipped (HASH-01): streaming enum-dispatch Hasher (SHA-256 default; --algo blake3/sha512/md5; RustCrypto digest 0.11 + native blake3 update_reader, no traits-preview/no dyn Digest), const-hex hex (no base16ct alloc change), --verify length-autodetect (32/64/128, sha256 wins the 64-tie) with the 0/1/2 exit contract; core::input grew read_file_or_stdin + ResolvedInput (streaming --file ahead of stdin); BoxError::UnsupportedHashLength exit-2 variant added; 7/7 HASH-01 tests + full suite + clippy -D warnings + fmt --check all green; hash stub gone (4 phase-3 stubs remain: tree/du/dupes/bulk-rename)*
*Updated: 2026-06-22 by plan-03-02 executor — `box tree` shipped (TREE-01): dir-first (D-08) Unicode box-drawing render (├── └── │   + gap), is_color_on-gated blue dir names (D-10), --sizes per-file human_size (blank dirs), --depth N cap, `N directories, M files` summary; reuses core::fs::is_hidden VERBATIM via a WalkDir depth-1 per-level walk (D-06) + follow_links(false) (T-03-05) + normalize_path (T-03-07); flatten's human_size PROMOTED into core::output (pub, test migrated, flatten re-pointed + local copy deleted — flatten 8/8 still green, D-12) ready for du; tree.trycmd backed by a tree.in/ trycmd input fixture; 3/3 TREE-01 tests + tree.trycmd + full suite (77 unit + all integration) + clippy -D warnings + fmt --check all green; tree stub gone (3 phase-3 stubs remain: du/dupes/bulk-rename)*
*Updated: 2026-06-22 by plan-03-03 executor — `box du` shipped (DU-01): one row per immediate child, biggest-first `(size desc, name asc)` sort BEFORE printing (RESEARCH Pitfall 6 / T-03-12), dir rows = recursive non-hidden descendant sum (logical metadata().len(), RESEARCH A4) + trailing `/`, file rows = own size; --depth N caps the per-dir rollup (WalkDir::max_depth, dir's files = depth 1), --top N post-sort truncation of shown rows; right-aligned shared core::output::human_size column (D-12, third consumer), single .cyan() size-value accent gated on is_color_on() (D-11); `{X} of {Y} entries shown. {TOTAL} total.` summary reflects the FULL-scan total (captured before --top); reuses core::fs::is_hidden + follow_links(false) VERBATIM (D-06, T-03-09/10) + normalize_path (T-03-11); 3/3 DU-01 tests + 4 unit tests + full suite (81 unit + all integration) + clippy -D warnings + fmt --check all green; one Rule-1 test-side fix (du_depth_cap row-scoped); du stub gone (2 phase-3 stubs remain: dupes/bulk-rename)*
*Updated: 2026-06-22 by plan-03-04 executor — `box dupes` shipped (DUPE-01): size pre-filter (HashMap<u64,Vec<PathBuf>>, candidates = same-size buckets ≥2, most files never hashed) → rayon par_iter BLAKE3 content hash reusing the 03-01 update_reader native streaming path (lifted as hash_reader_blake3 since hash::hash_blake3 is private; same b"box" known vector, D-13) → sort_by (hash,path) BEFORE grouping (consecutive-equal-hash runs ≥2 → groups; RESEARCH Pitfall 6 / T-03-16, first hash error short-circuits the collect → exit 1, T-03-17) → groups + wasted-space summary (Σ (group_len-1)*size) via core::output::human_size (fourth consumer); STRICTLY READ-ONLY — only fs handle is a read-only File::open, NO write/rename/delete (T-03-13), the dupes_never_writes test snapshots file set + contents + mtimes unchanged; reuses core::fs::is_hidden + follow_links(false) + normalize_path VERBATIM, NO noise list / NO ignore crate (D-06/D-07); single .yellow() accent gated on is_color_on(); 4/4 DUPE-01 tests + 6 unit tests + full suite (87 unit + all integration) + clippy -D warnings + fmt --check all green; dupes stub gone (1 phase-3 stub remains: bulk-rename)*
*Updated: 2026-06-22 by plan-03-05 executor — `box bulk-rename` shipped (RENM-01) → PHASE 3 FEATURE-COMPLETE (5/5 plans): regex first-match `replace` over the FULL base name (D-16/D-17) → in-memory ABORT-ALL-BEFORE-ANY-RENAME pre-flight, a PURE I/O-free preflight()->Vec<Conflict> implementing all four D-18 rules (per-dir case-folded occupied set seeded from on-disk names NOT renamed away, target-vs-target + target-vs-existing collision, cycle/swap = target equals another item's source, path-separator refusal) — the ONLY backstop vs std::fs::rename's silent Windows overwrite, no create_new for moves → dry-run preview is the DEFAULT (writes nothing), --force executes only after a clean pre-flight (D-19); reuses flatten's format_row/arrow_col + case-folded occupied set + encode_no_separator invariant VERBATIM; case-only rename correct by construction (byte-exact no-ops filtered to (unchanged) before preflight, Pitfall 5); ${1}abc foot-gun + full-base-name match documented in --help; every abort path snapshot-asserts the dir byte-for-byte unchanged; one Rule-1 test-side fix (case-exact read_dir listing not Path::exists — NTFS case-insensitive/preserving); 7/7 RENM-01 tests + 9 unit tests + full suite (96 unit + all integration) + clippy -D warnings + fmt --check all green; ALL 5 Phase-3 not_implemented arms gone — phase ready for verification (8 stubs remain: Phase-4 + Phase-5)*
*Updated: 2026-06-23 by execute-phase orchestrator — Phase 3 VERIFIED (24/24 must-haves) + COMPLETE; post-execution code review fixed CR-01 BLOCKER (bulk-rename `..`/`.` path-escape) + WR-01..05 with covering tests (98 tests green, clippy -D warnings + fmt clean, 4 INFO deferred); human-UAT cleared (tree/du/dupes color confirmed in PS7); ROADMAP/STATE/REQUIREMENTS updated — next = Phase 4 (terminal-visuals: LOL-01/MTRX-01/ASCI-01/JSON-01)*
