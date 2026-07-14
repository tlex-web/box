# Walking Skeleton — box (Rust CLI Toolbox)

**Phase:** 1
**Generated:** 2026-06-22

## Capability Proven End-to-End

A user runs `.\install.ps1` in PowerShell 7 and, in the *same session*, runs `box flatten ./src ./out` to safely flatten a nested folder tree — proving the full stack: crate build → MSVC/crt-static release binary → global PATH install → clap dispatch → shared core infra → a real command doing real, safe filesystem work.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Language / binary | Single Rust crate (not a workspace), one `box.exe` binary | PROJECT.md core value: one binary, all tools one short command away. A workspace adds ceremony with no payoff for a single distributable. |
| CLI framework | clap 4.6 derive, one `Commands` enum, all 23 variants registered | D-05; doc comment → `about` keeps every command in `box --help` with per-command `--help`. Locked in CLAUDE.md. |
| Command pattern | `RunCommand` trait (`fn run(self) -> anyhow::Result<()>`); `main()` is dispatch + exit-code only | STATE.md Architecture; keeps `main.rs` ~40 lines, business logic in modules. The pattern every Phase 2-5 command reuses. |
| Error handling | anyhow at the binary boundary + thiserror `BoxError` in modules; never panic in command paths | CLAUDE.md stack; `main()` maps `Err` → stderr + exit 1, preserves clap exit 2 for parse errors (D-07). |
| Exit-code policy | Strict 0 (ok) / 1 (runtime error incl. NotImplemented) / 2 (clap parse error + bare box). `try_parse()` in main with explicit D-08 override | D-06/D-07/D-08; bare `box` exit 2 requires overriding clap's exit-0 default (RESEARCH Pitfall 2) — the single most load-bearing finding. |
| Color / output | `enable-ansi-support` bootstrap first line of main; owo-colors gated once via `owo_colors::set_override` from TTY ∧ `NO_COLOR` ∧ `--no-color` (stdlib `IsTerminal`) | FOUND-04, D-10; one global switch → piped output is byte-identical minus ANSI. The output glyph/summary helpers (D-09/D-11) are the UX template for the other 22 commands. |
| Path / FS layer | `dunce::canonicalize` everywhere (never `std::fs::canonicalize`); `walkdir` + `filter_entry` hidden pruning; `std::fs::FileTimes` for timestamps (no `filetime` crate) | FOUND-06, D-12; shared `core::fs` so Phase 3 (hash/tree/du/dupes/bulk-rename) inherits UNC-safe, hidden-aware, timestamp-correct traversal. |
| Distribution | `install.ps1`: build (MSVC + crt-static) → copy to `%LOCALAPPDATA%\Programs\box` → idempotent user-scope PATH (REG_EXPAND_SZ-safe) → live-session refresh → smoke test | FOUND-07/08, D-01/D-02/D-03/D-04; user-scope only, no admin, same-session usability. |
| Build target | `x86_64-pc-windows-msvc` + `RUSTFLAGS="-C target-feature=+crt-static"`; release profile `lto=thin, codegen-units=1, strip, panic=abort, opt-level=3` | CLAUDE.md; MSVC required by Windows-API crates in later phases; static CRT makes the exe portable. |
| Directory layout | `src/{main.rs, cli.rs}`, `src/core/{errors,output,fs}.rs`, `src/commands/<cmd>/mod.rs` (+ `flatten/rename.rs`), `tests/{cli,flatten}.rs` | STATE.md Architecture / RESEARCH Recommended Project Structure; the 22 stubs share one `commands/stub.rs` handler this phase — per-command folders are added in their own phases. |
| Testing | Rust harness + `assert_cmd`/`predicates`/`assert_fs`/`tempfile`/`trycmd` (+`insta`); unit tests for pure `rename.rs`, integration for CLI + flatten | VALIDATION.md; black-box tests of the compiled binary back the Nyquist sampling contract. |

## Stack Touched in Phase 1

- [x] Project scaffold (Cargo crate, clap dispatch, release profile, lint via clippy, test runner) — Plan 01
- [x] Routing — clap subcommand dispatch with all 23 commands registered (only `flatten` functional) — Plan 01
- [x] Real read AND write — `flatten` reads a source tree and writes flattened copies (with timestamp preservation) to the output dir — Plan 03
- [x] Real interactive command wired end-to-end — `box flatten <src> <out> [--dry-run]` through `RunCommand` → core::fs/output — Plan 03
- [x] Deployment — `install.ps1` builds + installs + makes `box` runnable in the same PS7 session, smoke-tested — Plan 04

## Out of Scope (Deferred to Later Slices)

> Explicit so future phases do not re-litigate Phase 1's minimalism.

- The other 22 commands' real logic — they are registered clap stubs returning `BoxError::NotImplemented` (Phases 2-5).
- All flatten v2 flags: `--separator`, `--include-hidden`, `--extensions` filter, `--json` mapping output, progress bar, `--move` mode (REQUIREMENTS.md FLAT-V2-01).
- Full long-path (>260 char) *success* — Phase 1 only guarantees loud failure, never silent loss (FOUND-06); verbatim `\\?\` I/O or a `LongPathsAware` manifest is deferred (RESEARCH OQ-2).
- `winrt-notification` compile spike (STATE.md todo to de-risk Phase 5) — tracked separately; not allowed to block the flatten slice (RESEARCH OQ-3).
- Uninstall / PATH-entry removal — out of scope; the entry persists until manually removed.

## Subsequent Slice Plan

Each later phase adds vertical slices on top of this skeleton without altering its architectural decisions (RunCommand, core infra, exit-code policy, install path):

- Phase 2: Pure transform utilities (uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast) — proves the RunCommand pattern on zero-integration commands.
- Phase 3: Filesystem power tools (hash, tree, du, dupes, bulk-rename) — reuse `core::fs` walk + collision-rename from flatten.
- Phase 4: Terminal visuals (lolcat, matrix, ascii, json) — reuse `core::output` color gating; add crossterm rendering.
- Phase 5: Windows platform integration (qr, clip, pomodoro, weather) — highest external/API risk, built last on a proven base.
