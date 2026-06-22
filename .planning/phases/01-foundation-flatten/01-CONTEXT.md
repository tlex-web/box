# Phase 1: Foundation + Flatten - Context

**Gathered:** 2026-06-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship an **installable single-binary `box` scaffold** plus the anchor `flatten` command:

1. **Scaffold** — single Rust crate, `clap` derive, all **23 subcommands registered** and listed in `box --help` with one-line descriptions; only `flatten` is functional this phase (the other 22 are stubs). Shared core infrastructure: error handling (anyhow at boundary, thiserror in modules), output/color (`enable_ansi_support` + owo-colors, auto-off when not a TTY / `NO_COLOR`), fs helpers, exit-code conventions (0/1/2), data→stdout / messages→stderr.
2. **Installer** — `install.ps1` builds release (`x86_64-pc-windows-msvc` + `crt-static`), copies the binary to a dedicated bin dir, updates user PATH idempotently, refreshes the current PS7 session, and smoke-tests `box --help`.
3. **Anchor command** — `flatten <src> <out> [--dry-run]`: recursively copy every file from a source tree into one flat output dir, originals untouched, collision-rename by encoding source path, dry-run preview, completion summary.

**In scope:** FOUND-01..08, FLAT-01..04 (12 requirements).
**Not in scope:** the other 22 commands (Phases 2–5); any flatten v2 flags (`--separator`, `--include-hidden`, `--extensions`, `--json`, `--move`, progress bar).

</domain>

<decisions>
## Implementation Decisions

### Install location & PATH (FOUND-07, FOUND-08)
- **D-01:** `install.ps1` copies `box.exe` to **`%LOCALAPPDATA%\Programs\box`** (the Win11 per-user installed-app convention — VS Code / GH CLI use this root; no admin, does not roam, single binary so the install dir *is* the PATH entry).
- **D-02:** PATH update is **idempotent, registry user-scope, dedup-guarded** — read `[Environment]::GetEnvironmentVariable('Path','User')`, split on `;`, append the bin dir only if `-notcontains` it, write back. ⚠️ If the existing user PATH contains `%VAR%` references, write with `Set-ItemProperty -Path 'HKCU:\Environment' -Name Path -Value $newPath -Type ExpandString` to avoid the `REG_EXPAND_SZ → REG_SZ` regression (dotnet/runtime#1442). For literal absolute paths the `[Environment]` form is sufficient.
- **D-03:** **Current-session refresh** must rebuild `$env:Path` by re-reading **both** Machine and User scopes from the registry and rejoining — using the persisted write alone won't propagate to the live process, and using only the User scope would drop `System32` etc. from the session.
- **D-04:** Re-install behavior = **plain overwrite** (`Copy-Item -Force`). box is one self-contained static binary with no version-pinned side files; version-checking adds friction for no safety benefit. The PATH dedup guard makes repeated installs idempotent.

### CLI scaffold & stub UX (FOUND-01..05)
- **D-05:** Stubs are **real `clap`-derive enum variants** with doc-comment descriptions (doc comment → `about`), each dispatched to a handler returning a structured `NotImplemented` error (thiserror `BoxError::NotImplemented`). This is the only approach that keeps all 23 commands in `box --help` *and* gives per-command `--help`. (Rejected: `external_subcommand`, `hide=true`, feature-gating, `todo!()` — each either hides stubs or panics.)
- **D-06:** Invoking an **unbuilt command exits code 1** (a runtime "feature absent" condition, not a usage mistake). Message to **stderr**: `error: 'qr' is not yet implemented — coming in a future release`.
- **D-07:** **Exit code 2 is reserved for clap parse errors** — `box badcmd`, missing/invalid args. clap emits these automatically; `main()` must NOT collapse all errors to 1.
- **D-08:** **Bare `box`** (no subcommand) → `#[command(arg_required_else_help = true)]` on the top-level `Cli`: prints help and **exits 2** (consistent with the strict 0/1/2 convention).

### flatten output format (FLAT-03)
- **D-09:** Output style **B — leading status glyph + arrow + color**: `+` plain copy, `~` collision rename, `-` skipped. **Glyph is the source of truth; color is decoration only**, so output is honestly pipe-safe (`grep '^  ~'` still finds every collision). Use ASCII glyphs (`+ ~ -`), not Unicode, so they render reliably in PowerShell 7 regardless of font. The `->` arrow is used for the copy mapping (cosmetic if misrendered). This becomes the **UX template for the other 22 commands.**
- **D-10:** Color gating: `std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()` (plus `--no-color`). Plain/piped layout is byte-identical minus ANSI. Skipped/rename reasons shown inline: `[collision]`, `[collision x2]`, `(skipped: symlink)`, `(skipped: reserved name)`. Arrows aligned into a column, capped at terminal width (crossterm), over-long paths truncated in the middle with `…` so the filename stays visible.
- **D-11:** Summary wording (locked):
  - Dry-run: `Dry run: nothing was copied.` then `Plan: {n} to copy, {n} renamed for collisions, {n} skipped.`
  - Real run: `Done: copied {n} files ({n} renamed for collisions), skipped {n}. {size} written.`

### flatten default scope (FLAT-01, FLAT-02, FLAT-04)
- **D-12:** **Skip hidden files/dirs by default.** "Hidden" = base name starts with `.` **OR** carries the Windows `FILE_ATTRIBUTE_HIDDEN` bit (`std::os::windows::fs::MetadataExt::file_attributes() & 0x2`). Apply in `walkdir`'s `filter_entry` so hidden *directories* (`.git`, `.venv`) prune their whole subtree cheaply. This is the only default under which the deferred v2 `--include-hidden` flag is coherent.
- **D-13:** **Auto-create the output dir** (`fs::create_dir_all`, including missing parents) — creating a dir is not data loss.
- **D-14:** **Merge into an existing non-empty output dir** (do not refuse) — but **collision-check incoming names against pre-existing files in the output dir, not just against this run's files.** Before the copy loop, `read_dir` the output dir to seed the occupied-name set. Source-only checking would silently clobber prior output, violating the core no-silent-data-loss promise.
- **D-15:** **Collision-rename prefix** is built from the source path **relative to the canonicalized source root** (via `dunce::canonicalize`, never absolute drive-letter paths): replace each separator (`\` and `/`) with `_`, drop drive letter / leading separator, then **sanitize Windows-reserved stems** (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`, case-insensitive, with/without extension) and trailing dots/spaces. If the encoded name still collides, **numeric-suffix fallback** before the extension (`name_1.ext`, `name_2.ext`).

### Claude's Discretion
- `box --version` source — read from `Cargo.toml` via `clap`'s `#[command(version)]`; start at `0.1.0` (FOUND-02).
- Exact phrasing of one-line `about` text per stub command (use the verbs from REQUIREMENTS.md command list).
- Internal module layout for the dry-run planner vs executor in `flatten` (so dry-run and real run share one plan), as long as dry-run writes nothing.
- Whether the `{size} written` byte count is accumulated during copy (cheap) — include unless it complicates the executor.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements & success criteria
- `.planning/ROADMAP.md` § "Phase 1: Foundation + Flatten" — goal + the 5 success criteria (what must be TRUE).
- `.planning/REQUIREMENTS.md` § "Foundation & Distribution" (FOUND-01..08) and § "flatten (anchor)" (FLAT-01..04) — full acceptance criteria. Also § "v2 Requirements / FLAT-V2-01, HASH-V2-01…" for what is explicitly deferred.

### Project intent & locked decisions
- `.planning/PROJECT.md` — Core Value, Key Decisions table, and Out of Scope (cross-platform, Scoop/winget, move/overwrite modes).
- `.planning/STATE.md` § "Architecture Established", § "Critical Pitfalls to Remember", § "Key Decisions" — **the binding architecture and pitfall list** (RunCommand trait, `src/core/` layout, dunce, ANSI bootstrap order, install.ps1 PATH refresh, flatten canonicalize-before-walk, reserved-name sanitization, `8ball`→`eight_ball` module, MSVC + crt-static build).

### Tech stack (locked crate versions)
- `CLAUDE.md` (project root) — the full recommended stack with confirmed versions: `clap 4.6.1`, `anyhow 1.0.102`, `thiserror 2.0.18`, `owo-colors 4.3.0`, `enable-ansi-support 0.3.1`, `crossterm 0.29.0`, `walkdir 2.5.0`, plus the "What NOT to Use" table and Release Build / static-linking guidance. Use these versions; do not re-research the stack.

**No external ADRs/specs exist** — all decisions are captured above and in the four files listed.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Greenfield** — no source code yet (`src/` does not exist). This phase establishes the assets the other phases reuse.

### Established Patterns (from STATE.md — to be created this phase, then reused)
- `RunCommand` trait: `fn run(self) -> anyhow::Result<()>` implemented by each command's Args struct.
- `src/main.rs` ≈ 40 lines: parse + dispatch + exit-code mapping only, **no business logic**; `enable_ansi_support::enable_ansi_support()` is the first line of `main()`.
- `src/commands/<cmd>/mod.rs` per command; `src/core/{errors.rs, output.rs, fs.rs}` (errors = BoxError + thiserror; output = color init + print helpers + TTY/NO_COLOR gating; fs = walkdir wrapper, `safe_copy`, collision rename).
- Tests: integration via `assert_cmd` in `tests/<cmd>.rs`; snapshot via `insta`/`trycmd`.

### Integration Points
- `flatten`'s output helpers (glyph/color/summary, TTY gating) and `src/core/fs.rs` (walkdir wrapper, collision rename, reserved-name sanitization) are the shared surfaces Phases 3 (hash/tree/du/dupes/bulk-rename) and 4 (visuals) will build on. Design them as reusable now.

</code_context>

<specifics>
## Specific Ideas

- flatten dry-run sample the user approved (exact target output):
  ```
    + src\readme.md            -> readme.md
    ~ src\docs\sub\report.txt  -> docs_sub_report.txt   [collision]
    - src\bin\link.txt            (skipped: symlink)

  Dry run: nothing was copied.
  Plan: 4 to copy, 3 renamed for collisions, 2 skipped.
  ```
- Stub error message form the user approved: `error: 'qr' is not yet implemented — coming in a future release` (lowercase `error:` prefix, matching clap's own style).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. (flatten v2 flags and the other 22 commands are already tracked in REQUIREMENTS.md / ROADMAP.md.)

</deferred>

---

*Phase: 1-foundation-flatten*
*Context gathered: 2026-06-22*
