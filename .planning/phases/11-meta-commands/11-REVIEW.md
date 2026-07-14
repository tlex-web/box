---
phase: 11-meta-commands
reviewed: 2026-07-14T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - Cargo.toml
  - install.ps1
  - src/cli.rs
  - src/commands/completions/mod.rs
  - src/commands/config/mod.rs
  - src/commands/mod.rs
  - src/commands/weather/mod.rs
  - src/core/config.rs
  - src/core/errors.rs
  - src/core/fs.rs
  - src/main.rs
  - tests/completions.rs
  - tests/config_cmd.rs
findings:
  critical: 0
  warning: 4
  info: 6
  total: 10
status: issues_found
---

# Phase 11: Code Review Report

**Reviewed:** 2026-07-14
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the Phase 11 meta-commands: the nested `box config <show|get|set|path>`
command (CFG-01), the `box completions <shell>` generator (CMP-01), and their
supporting `core::config` (validate-before-write `set_value`), `core::fs`
(`atomic_write`), `core::errors` (`ConfigUsage`), `cli.rs` wiring, and
`install.ps1` PATH/`$PROFILE` changes.

The core write path is well-built: `set_value` validates by re-parsing through the
same typed `toml::from_str::<Config>` startup uses before touching disk, and
because values are inserted via `toml::Value::String` into a programmatically-built
table and re-serialized, there is **no TOML-injection vector** — a hostile value is
escaped by the serializer, not spliced into raw text. Path handling is safe (config
path is `%APPDATA%` + hardcoded `box/config.toml`; the cache hashes user keys).
Exit-code contracts (0/1/2) are largely faithful to the documented policy.

No blockers were found. However, four correctness/robustness defects warrant a fix
before ship — most notably that `config show`/`get hash.default_algo` **contradicts
its own documented parity guarantee** by ignoring the live `BOX_HASH_DEFAULT_ALGO`
env tier that `box hash` actually consults, and that a malformed config file bricks
the very meta-commands (and shell-start completions) meant to inspect and repair it.

## Warnings

### WR-01: `config show` / `config get hash.default_algo` ignores the live `BOX_HASH_DEFAULT_ALGO` env tier — violates the module's own parity guarantee

**File:** `src/commands/config/mod.rs:109-120` (`effective()`), `src/commands/config/mod.rs:157-162` (`get` hash arm)
**Issue:** The module doc claims (`src/commands/config/mod.rs:8-12`) that `show`/`get`
"read the EFFECTIVE resolved config through the SAME
`config().<table>.<field>.unwrap_or(builtin)` resolution every command uses, so
`config show` can never lie about what `box hash`/`box weather` will consume."

That is false for `hash`. `run_compute` resolves the algo as
(`src/commands/hash/mod.rs:301-308`):
```rust
let algo = cli_algo
    .or_else(|| std::env::var("BOX_HASH_DEFAULT_ALGO").ok().and_then(|s| parse_algo(&s)))
    .or(crate::core::config::config().hash.default_algo)
    .unwrap_or(Algo::Blake3);
```
but `effective()` / the `get` handler resolve only
`config().hash.default_algo.unwrap_or(Algo::Blake3)` — the `BOX_HASH_DEFAULT_ALGO`
env tier is dropped. With `BOX_HASH_DEFAULT_ALGO=md5` set (and no config key),
`box hash <file>` uses MD5 while `box config show` and the scriptable
`box config get hash.default_algo` both report `blake3`. A user (or script) trusting
`config get` gets the wrong answer. (Weather has no env tier, so its parity holds —
this defect is specific to `hash.default_algo`.)
**Fix:** Mirror hash's chain in the effective resolver so the reported value matches
what the command consumes:
```rust
default_algo: std::env::var("BOX_HASH_DEFAULT_ALGO")
    .ok()
    .and_then(|s| crate::commands::hash::parse_algo(&s)) // make parse_algo pub(crate)
    .or(cfg.hash.default_algo)
    .unwrap_or(Algo::Blake3),
```
(and the same in the `get "hash.default_algo"` arm). Alternatively, if `config show`
is intended to report the *file* view only, correct the module doc and the
`box config` help text so it no longer claims parity with "what `box hash` will
consume."

### WR-02: A malformed `config.toml` bricks the config meta-commands (no repair path) and errors shell-start completions

**File:** `src/main.rs:81-84`, `src/core/config.rs:131-154`
**Issue:** `main` runs `init_config()` for **every** subcommand before dispatch; a
malformed/unknown-key file returns `BoxError::Config` → exit 2 before any command
runs (`src/main.rs:81`). Consequences the meta-commands do not anticipate:
- `box config set hash.default_algo blake3` — the intended way to overwrite a bad
  value — never runs, because startup load fails first. `set_value` builds its base
  from `config()` (`src/core/config.rs:218`), which is unreachable. The
  "self-inflicted exit-2 lockout is structurally impossible" guarantee (T-11-02)
  covers `set` never *writing* a bad file, but an **externally** corrupted file
  (bad hand-edit, or a partial write from another tool) locks the user out of the
  `config set` repair path entirely.
- `box config path` / `box config show` also exit 2, so box cannot even locate/print
  the file for the user to fix (the path does appear in the error text, but the
  dedicated "locate, never read" command is dead in exactly the state it is needed).
- `install.ps1 -RegisterCompletions` writes `box completions powershell | ... |
  Invoke-Expression` into `$PROFILE`. If the config later becomes malformed, that
  line runs `box completions` — which also exits 2 via `init_config` — at **every**
  new PowerShell session, printing `error: config error ...` to stderr on each shell
  start even though completion generation needs no config at all.
**Fix:** Decouple config-independent commands from the global config gate. Either
route `Config`/`Completions` dispatch ahead of `init_config()`, or make
`init_config()` for the `config` command tolerant (load-or-default and surface the
parse error only on `show`/`get`, while keeping `path`/`set` usable for repair). At
minimum, `box config path` and `box completions <shell>` should not fail on a
malformed config file.

### WR-03: `atomic_write` uses a fixed `<path>.tmp` sibling — concurrent writers can corrupt it and a failed run leaves a stray temp

**File:** `src/core/fs.rs:141-150`
**Issue:** The temp sibling name is deterministic (`config.toml.tmp`):
```rust
let mut tmp = path.as_os_str().to_os_string();
tmp.push(".tmp");
```
Two concurrent `box config set` processes write the same temp path via
`std::fs::write` (truncate + write); their writes can interleave, and the later
`rename` then promotes possibly-mixed content into `config.toml` — defeating the
concurrency half of an "atomic write." A run that writes the temp but fails the
`rename` (e.g. Windows sharing violation while the target is open) also leaves a
stale `config.toml.tmp` behind. The whole purpose of this primitive is safe
replacement, so the fixed name is a real robustness gap.
**Fix:** Use a unique temp name in the same directory and rename it over the target,
e.g. suffix with the PID and/or a random token, or use `tempfile::NamedTempFile::
new_in(parent)?` then `.persist(path)` (tempfile is already a dev-dep; promote or
hand-roll):
```rust
let mut tmp = path.as_os_str().to_os_string();
tmp.push(format!(".{}.tmp", std::process::id()));
```

### WR-04: `install.ps1` smoke test throws instead of reaching its graceful "open a new terminal" branch

**File:** `install.ps1:76-81`
**Issue:** With `$ErrorActionPreference = 'Stop'` (line 32), if `box` still does not
resolve on PATH in the current session, `& box --help` raises a terminating
`CommandNotFoundException` and aborts the script **before** the `else { Write-Warning
... }` branch:
```powershell
& box --help | Out-Null
if ($LASTEXITCODE -eq 0) { ... } else { Write-Warning "Installed, but 'box' did not run ..." }
```
The else-branch is written for precisely the "box did not run in this session" case
(its message even says so), but that case throws rather than falling through, so the
intended graceful hint is unreachable and the install ends in a raw exception.
**Fix:** Guard the smoke test so a missing command degrades to the warning:
```powershell
if (Get-Command box -ErrorAction SilentlyContinue) {
    & box --help | Out-Null
    if ($LASTEXITCODE -eq 0) { Write-Host "box is ready. Try: box --help" }
    else { Write-Warning "Installed, but 'box' exited non-zero in this session..." }
} else {
    Write-Warning "Installed, but 'box' is not yet on PATH in this session. Open a new terminal and try 'box --help'."
}
```

## Info

### IN-01: `config set` silently discards comments and formatting in an existing `config.toml`

**File:** `src/core/config.rs:232-263`
**Issue:** `build_config_toml` reconstructs the document from the typed startup
snapshot (`toml::to_string(base)` at line 252), not from the on-disk file text.
Any user-authored comments, blank lines, or key ordering in `config.toml` are lost
on the next `config set`. This is an acceptable-by-design tradeoff (the module is
explicit about round-tripping through the typed schema), but is worth documenting in
the `config set` help so it is not a surprise.
**Fix:** Document the behavior, or (if preservation is desired) splice into the
parsed on-disk `toml::Table` read from the file text rather than the typed snapshot.

### IN-02: `config set` enum values are case-sensitive while `--algo` / `BOX_HASH_DEFAULT_ALGO` are case-insensitive

**File:** `src/core/config.rs:261, 270`
**Issue:** The value is inserted as a raw string and validated via serde
`toml::from_str::<Config>`, which for `Algo`/`Units` uses
`#[serde(rename_all = "lowercase")]` — a **case-sensitive** match. So
`box config set weather.units Imperial` (capital I) fails with exit 2, even though
`box weather --units Imperial` (ValueEnum, case-insensitive) and
`BOX_HASH_DEFAULT_ALGO=SHA256` (via `parse_algo`, case-insensitive) both succeed.
The rejection is loud (no silent data issue), but the inconsistency across tiers is
a confusing UX papercut.
**Fix:** Normalize enum-valued keys before validation using the shared
`ValueEnum::from_str(_, true)` parser (the same path `--algo`/env use), or document
that config values must be lowercase.

### IN-03: `config get`'s key handling can drift from `SETTABLE_KEYS`

**File:** `src/commands/config/mod.rs:157-182`
**Issue:** `get` uses a hardcoded three-arm `match` on the key literals with a
catch-all `_ => Err(unknown_key(key))`, whereas `set` derives its acceptance from
the `SETTABLE_KEYS` constant (`src/core/config.rs:235`). If a fourth settable key is
added to `SETTABLE_KEYS` (and to `set`) but the reviewer forgets the `get` arm,
`box config get <newkey>` will report it as an unknown key even though `set` accepts
it — a silent get/set divergence.
**Fix:** Add a compile-time or unit-test guard that every `SETTABLE_KEYS` entry is
handled by `get`, or drive `get` from a single key→resolver table shared with the
registry.

### IN-04: `serde_str` silently renders an empty string on non-string serialization

**File:** `src/commands/config/mod.rs:125-130`
**Issue:** `serde_str` does `.and_then(|v| v.as_str()...).unwrap_or_default()`, so if
a value ever serializes to a non-string (a future numeric/bool/struct config value),
`config show` would print `key = ` (empty) with no error rather than failing loudly.
Safe for today's string-only enums, but a latent silent-failure landmine.
**Fix:** For the current string enums this is fine; consider `debug_assert!` on the
`None` branch, or fall back to `serde_json::to_string(v)` so a non-string still shows
*something* diagnostic.

### IN-05: `install.ps1` PATH dedup can append a duplicate when the existing entry is an unexpanded literal

**File:** `install.ps1:54-56`
**Issue:** `$rawPath` is read un-expanded (`DoNotExpandEnvironmentNames`) but
`$BinDir` is the fully-expanded absolute path. The dedup check
`$entries -inotcontains $BinDir` therefore fails to match an existing entry stored as
`%LOCALAPPDATA%\Programs\box`, and a second (expanded) copy is appended. Re-running
box's own installer is idempotent (it always writes the expanded form), but the guard
is not robust to a literal `%VAR%` form written by another tool.
**Fix:** Compare against an expanded projection of `$entries` (e.g.
`[Environment]::ExpandEnvironmentVariables($_)`) before deciding to append.

### IN-06: `install.ps1` `$PROFILE` idempotency sentinel is a naive substring match

**File:** `install.ps1:100`
**Issue:** `Select-String -Quiet -Pattern '# box completions' -Path $PROFILE` skips
the append whenever any line *contains* that substring — including an unrelated user
comment like `# box completions were flaky` — which would silently leave completions
unregistered. It also does not verify the actual one-liner is present/intact, only
the sentinel.
**Fix:** Use a more specific, tool-owned sentinel (e.g.
`# box completions (managed by install.ps1)`) and/or verify the recipe line itself is
present.

---

_Reviewed: 2026-07-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
