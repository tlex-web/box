# box — A Rust CLI Toolbox

`box` is a single Rust binary that bundles ~23 command-line tools — a mix of
genuinely useful utilities and fun toys — exposed as subcommands
(`box flatten`, `box qr`, `box cowsay`, …). It is built for Windows
PowerShell 7 and installed globally so any tool is one short command away.

> **Status:** Phase 1 (Foundation + Flatten). `box flatten` is fully functional;
> the other 22 subcommands are registered and listed in `box --help` but report
> `not yet implemented — coming in a future release` until later phases ship them.

## Install

From the repository root in **PowerShell 7**:

```powershell
.\install.ps1
```

This will:

1. Build the release binary (`x86_64-pc-windows-msvc` with a static CRT, so the
   `.exe` is portable — no redistributable needed).
2. Copy `box.exe` to `%LOCALAPPDATA%\Programs\box`.
3. Add that directory to your **user** PATH, idempotently (re-running never
   duplicates the entry) and without downgrading any `REG_EXPAND_SZ` PATH that
   contains `%VARS%`.
4. Refresh the **current** session's `$env:Path`, so `box` works immediately —
   no need to open a new terminal.
5. Smoke-test `box --help` and report readiness.

The installer is entirely **user-scope** — it never requires admin/UAC and never
writes the Machine PATH.

### One-time: execution policy

A fresh PowerShell 7 machine may block running local scripts under the default
execution policy. Either allow local scripts once for your user:

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

…or bypass the policy for this single run:

```powershell
pwsh -ExecutionPolicy Bypass -File install.ps1
```

### Verify

In the same window after install:

```powershell
box --help        # lists every subcommand with a one-line description
box --version     # 0.1.0
```

## Usage

Every subcommand has its own `--help`:

```powershell
box flatten --help
```

### flatten

Recursively copy every file from a source tree into one flat output folder.
Originals are left untouched, folder structure is dropped, and filename
collisions are renamed by encoding the source path (e.g. `docs_sub_report.txt`)
so nothing is ever silently overwritten.

Preview first with `--dry-run` (writes nothing):

```powershell
box flatten ./src ./out --dry-run
```

Then run for real:

```powershell
box flatten ./src ./out
```

## Conventions

- `--help` is available on the top level and every subcommand.
- Data goes to stdout; status/error messages go to stderr.
- Exit codes: `0` success, `1` a command-level failure (e.g. an unimplemented
  command), `2` a usage/parse error (bad flags, unknown subcommand, or bare
  `box` with no subcommand).
- Color is auto-disabled when output is piped or `NO_COLOR` is set; the status
  glyphs (`+` copied, `~` renamed, `-` skipped) carry the meaning, so piped
  output stays greppable.
