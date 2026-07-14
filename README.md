<div align="center">

# 📦 box

### One tiny binary. 23 command-line tools. Everything one short command away.

`box` bundles a whole toolbox of genuinely-useful utilities and fun toys into a
single, portable Rust executable — built for **Windows PowerShell 7** and
installed globally so `box <tool>` just works from anywhere.

![Rust](https://img.shields.io/badge/Rust-2021-CE422B?logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white)
![Shell](https://img.shields.io/badge/shell-PowerShell%207-5391FE?logo=powershell&logoColor=white)
![Binary](https://img.shields.io/badge/binary-single%20static%20.exe-2ea44f)
![Tools](https://img.shields.io/badge/tools-23-8957e5)

</div>

---

## ✨ Why box

- **🧩 One binary, 23 tools** — install once; every tool is on your PATH, no per-tool setup.
- **🤖 Scriptable by design** — `--json` on every data command emits one clean, `ConvertFrom-Json`-ready document (no ANSI, no BOM, no progress noise on stdout).
- **📋 `--clip` anywhere** — pipe any command's result straight to the Windows clipboard while it still prints.
- **🎨 True-color output** — ANSI that renders correctly in PowerShell 7, and auto-disables when piped or `NO_COLOR` is set.
- **⚙️ Config-driven defaults** — set per-command preferences in `%APPDATA%\box\config.toml`; precedence is CLI › env › config › built-in.
- **⌨️ Tab-completion** — generate a completion script for PowerShell, bash, zsh, fish, or elvish.
- **🛡️ Safe by default** — every destructive tool (`flatten --move`, `dupes --delete`, `bulk-rename`) previews as a dry-run first and only writes with `--force`.

---

## 🚀 Install

From the repository root in **PowerShell 7**:

```powershell
.\install.ps1
```

This builds a static, MSVC release binary (portable — no redistributable needed),
copies it to `%LOCALAPPDATA%\Programs\box`, adds that folder to your **user** PATH
(idempotently), and refreshes the current session so `box` works immediately. It is
entirely user-scope — never admin, never the machine PATH.

> **Tab-completion:** add `-RegisterCompletions` to also wire completions into your
> `$PROFILE` (opt-in — it never edits your profile otherwise).

<details>
<summary>First-run execution policy</summary>

A fresh machine may block local scripts. Allow them once for your user:

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

…or bypass for a single run: `pwsh -ExecutionPolicy Bypass -File install.ps1`.
</details>

Then confirm:

```powershell
box --help        # every subcommand with a one-line description
box --version     # 0.1.0
```

---

## 🧰 The toolbox

### 📁 Files & directories

| Command | What it does |
|---|---|
| `flatten` | Copy a whole folder tree into one flat directory; collisions renamed, never overwritten |
| `tree` | Print a directory tree with sizes — gitignore-aware, sortable |
| `du` | Disk-usage report, including real NTFS on-disk / compressed sizes |
| `dupes` | Find duplicate files by content (and optionally delete extras) |
| `bulk-rename` | Regex rename with a dry-run preview and an undo manifest |
| `hash` | Compute & verify file hashes — BLAKE3 by default, plus SHA-2 and MD5 |

### 🔧 Encode, convert & generate

| Command | What it does |
|---|---|
| `uuid` | Generate a random UUID |
| `base64` | Encode or decode base64 text |
| `epoch` | Convert between Unix timestamps and human-readable dates |
| `color` | Convert colors between hex, RGB, and HSL |
| `json` | Pretty-print and validate JSON |
| `passgen` | Generate secure passwords and passphrases |
| `qr` | Render a scannable QR code for text or a URL, right in the terminal |
| `ascii` | Render an image as ASCII art |
| `clip` | Read from or write to the system clipboard |

### 🎉 Fun & visual

| Command | What it does |
|---|---|
| `cowsay` | Wrap text in an ASCII-art speech or thought bubble (pick your figure) |
| `fortune` | Print a random fortune or quote |
| `8ball` | Ask the magic 8-ball a question |
| `roast` | Deliver a random programmer roast |
| `lolcat` | Rainbow-colorize piped text |
| `matrix` | The Matrix digital-rain screensaver effect |

### ⏱️ System

| Command | What it does |
|---|---|
| `pomodoro` | Focus timer with a live countdown and a Windows toast on completion |
| `weather` | Current conditions and a 7-day forecast (keyless Open-Meteo — no API key) |

### ⚙️ Meta

| Command | What it does |
|---|---|
| `config` | Read, edit, and locate the config file |
| `completions` | Generate a shell completion script |

Every subcommand has its own `--help` with full flag documentation:

```powershell
box hash --help
```

---

## 💡 Examples

```powershell
box hash setup.exe                    # BLAKE3 digest (default)
box hash setup.exe --algo sha256      # or SHA-256 / SHA-512 / MD5
box uuid --json --clip                # JSON to stdout AND onto the clipboard
box weather London --forecast         # current conditions + 7-day outlook
box qr "https://example.com"          # a scannable QR block in your terminal
box du --on-disk ./project            # true allocated size on NTFS
box dupes ./photos                    # preview duplicates; add --delete --force to act
box flatten ./src ./out --dry-run     # preview a flatten — writes nothing
```

Pipe `--json` straight into PowerShell objects:

```powershell
(box du ./project --json | ConvertFrom-Json).results | Sort-Object size -Descending
```

---

## ⚙️ Configuration

Set per-command defaults so you don't repeat yourself:

```powershell
box config set weather.location "London"    # bare `box weather` now uses it
box config set hash.default_algo "sha256"    # change the default hash algorithm
box config show                              # effective config (human or --json)
box config path                              # where the file lives
```

Config lives at `%APPDATA%\box\config.toml`. A missing or malformed file never
breaks a normal command — it simply falls back to built-in defaults.

---

## ⌨️ Shell completions

```powershell
box completions powershell | Out-String | Invoke-Expression   # this session
```

Add that line to your `$PROFILE` (or run `install.ps1 -RegisterCompletions`) to
load completions on every shell start. `bash`, `zsh`, `fish`, and `elvish` scripts
are generated the same way.

---

## 📐 Conventions

- `--help` on the top level and every subcommand.
- **Data → stdout, status/errors → stderr.** Under `--json`, stdout carries exactly one clean JSON document.
- Global flags on every command: `--json`, `--clip`, `--no-color`.
- **Exit codes:** `0` success · `1` command failure · `2` usage/parse error.
- Color auto-disables when piped or `NO_COLOR` is set; status glyphs (`+` copied, `~` renamed, `-` skipped) keep piped output greppable.

---

<div align="center">

Built in Rust 🦀 for Windows PowerShell 7 · single portable binary, zero runtime dependencies

Third-party license notices: [`LICENSE-THIRD-PARTY.md`](./LICENSE-THIRD-PARTY.md)

</div>
