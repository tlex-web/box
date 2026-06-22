# Feature Research

**Domain:** Rust CLI toolbox — 23 subcommands for Windows PowerShell 7
**Researched:** 2026-06-22
**Confidence:** HIGH (core behaviors), MEDIUM (differentiators), HIGH (anti-features)

---

## Cross-Cutting UX Conventions (Apply to All 23 Commands)

These are not optional polish — they are what makes the toolbox feel like one coherent product rather than 23 disconnected scripts.

### Table Stakes: Universal Flags

| Convention | Behavior | Notes |
|-----------|----------|-------|
| `--help` / `-h` | Subcommand help with usage, flags, examples | clap handles this; must be on every subcommand |
| `--version` | Top-level `box --version` shows semver | One binary, one version |
| Exit codes | 0 = success, 1 = error, 2 = bad args | POSIX convention; scripts depend on this |
| Stdout = data | All machine-readable output to stdout | stderr is for human messages, warnings, progress |
| Stderr = messages | Errors, progress, warnings to stderr | Piping works correctly when data/messages are separated |
| `--no-color` / `NO_COLOR` env | Disable ANSI color entirely | Respect the NO_COLOR standard; auto-disable when stdout is not a TTY |
| Non-TTY detection | No color, no spinners, no progress bars when piping | Detect with `atty` crate or `std::io::IsTerminal` |
| `--json` | Machine-readable JSON output to stdout | Not every command needs it, but file-ops and data commands do |
| `--quiet` / `-q` | Suppress informational output; only data or errors | Useful for scripting |

### Table Stakes: Error Handling

| Behavior | Why |
|---------|-----|
| Descriptive error messages to stderr | "No such file" + the path, not just an exit 1 |
| Non-zero exit on any failure | Scripts that check `$LASTEXITCODE` depend on this |
| Don't panic on bad input | Rust panics print ugly backtraces; catch errors and print cleanly |
| Handle missing arguments gracefully | Print short usage hint, not a wall of text |

### Differentiators: Nice Cross-Cutting Touches

| Feature | Value | Notes |
|---------|-------|-------|
| `--dry-run` on all mutating commands | Safety net; users gain confidence | Applies to: flatten, bulk-rename, dupes |
| Shell completion hints in help text | PowerShell users love tab-complete | clap can generate completion scripts |
| Consistent flag naming | `--output` not sometimes `-o` sometimes `--out` | Pick one pattern and apply it everywhere |
| `box` top-level help lists all subcommands with one-line descriptions | Discoverability | `box --help` is the menu |

### Anti-Features: Cross-Cutting

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Interactive prompts on every mutating action | Breaks scripts; annoying in batch use | Default to dry-run; require explicit `--force` to execute |
| Colorized output when piped | Corrupt downstream tools | Auto-detect TTY and strip ANSI codes |
| Panic/unwrap in production paths | Ugly to users | `anyhow` or `thiserror` with friendly messages |
| Logging to stdout mixed with data | Breaks `| clip`, `| jq`, etc. | Logs always to stderr |

---

## Per-Command Feature Landscape

---

### `flatten` — Recursive File Flattener

**Prior art:** `xxcopy` (Windows), `flatten-directory` (Node.js npm), `find + cp` one-liners

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Recursively walk source dir and copy all files to output dir | Core job | LOW | Walk with `walkdir` crate |
| Collision handling: path-prefix rename | When two files share a name, encode source path into filename | MEDIUM | e.g. `docs_sub_report.txt` using `_` as separator for path segments |
| `--dry-run` | Preview what would be copied/renamed before any disk writes | LOW | Print intended output names; no copies made |
| Do not touch originals | Copies, never moves | LOW | Source tree remains intact |
| Do not create subdirectories in output | The whole point; output is flat | LOW | — |
| Report count of files copied and collisions at end | Completion summary | LOW | To stderr |
| Fail if output dir doesn't exist (or optionally create it) | Prevent silent no-ops | LOW | `--create-output` flag or always create |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--separator CHAR` | Let user choose the collision separator (default `_`) | LOW | Some users prefer `-` or `.` |
| `--include-hidden` | Include dotfiles (off by default) | LOW | Hidden files often not wanted in flattens |
| `--extensions .jpg,.png` | Filter which file types to copy | LOW | Useful for "flatten all images" use cases |
| `--json` output | Machine-readable list of {source, destination} mappings | MEDIUM | Useful for post-processing or auditing |
| Collision count in exit code or summary | Tell user how many names were mangled | LOW | Part of the completion summary |
| Progress bar for large trees | Feedback during long copies | MEDIUM | Use `indicatif` crate; disable when not TTY |

#### Edge Cases (Must Handle)

| Case | Expected Behavior |
|------|------------------|
| Symlinks in source tree | Skip by default; `--follow-symlinks` to include | 
| Circular symlinks | Detect and skip with warning; never infinite loop |
| Output dir is inside source dir | Detect and exclude the output dir from the walk |
| Two files with identical path-prefixed names (deep nesting collision) | Append numeric suffix: `docs_report.txt`, `docs_report_2.txt` |
| Empty source dir | No-op with message; exit 0 |
| Source file with no extension | Allowed; treat filename as-is |
| Filename with path separator chars in it (rare) | Sanitize separator characters before use |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Move (destructive) mode for v1 | Data loss risk; no undo | Always copy; `--move` can be a v2 feature |
| Overwrite-silently mode | User loses files without knowing | Collision rename is always on; no `--overwrite` flag |
| Nested output structure options | Defeats the purpose of the tool | It flattens; that's what it does |

---

### `qr` — QR Code Renderer

**Prior art:** `qrencode` (C, Linux), `segno` (Python CLI)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Render QR for a string/URL in the terminal | Core job | MEDIUM | Use `qrcode` Rust crate; render as Unicode blocks |
| UTF-8 block characters for compact display | Standard modern terminal QR rendering | LOW | Uses `▀`/`▄`/`█`/` ` for 2-rows-per-char density |
| Works with URLs, plain text, and arbitrary strings | All common use cases | LOW | No content filtering |
| Readable by phone cameras when rendered at normal terminal font size | Must actually work | MEDIUM | Test at typical zoom levels |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--error-correction L/M/Q/H` | Advanced users know the tradeoff | LOW | Default M (15% recovery) |
| `--save FILE.png` | Save QR as image file | HIGH | Requires image crate dependency |
| Accept input from stdin | Pipeable (`echo "https://..." \| box qr`) | LOW | Standard Unix pattern |
| `--size N` | Scale the module size for terminal output | LOW | Default auto-fit to terminal width |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| QR scanner/decode mode | Doubles complexity; different tool | Omit for v1 |
| Generating QR for files (embed file contents) | Capacity limits make this impractical | Omit |

---

### `passgen` — Password / Passphrase Generator

**Prior art:** `pwgen`, `xkcdpass`, `pass`, 1Password generator

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Generate cryptographically random passwords | Security-critical: must use CSPRNG | LOW | `rand` crate with `OsRng` |
| Length control (`--length N`) | Universal requirement | LOW | Default 20 |
| Character set flags (`--upper`, `--lower`, `--digits`, `--symbols`) | Power users need precise control | LOW | All on by default |
| Passphrase mode (`--words N`) | XKCD/diceware approach; memorability | MEDIUM | Bundle EFF wordlist or similar |
| Print to stdout only (no clipboard side effects by default) | Safe composability with pipes | LOW | User decides what to do with it |
| Generate N passwords at once (`--count N`) | Common need in bulk workflows | LOW | Default 1 |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Show entropy estimate alongside password | Educates users; builds trust | LOW | Log2(charset^length) or log2(words^count) |
| `--no-similar` flag | Excludes `il1Lo0O` for readability | LOW | Reduces charset by ~7 chars |
| `--separator CHAR` for passphrases | `correct-horse-battery-staple` vs `correct horse` | LOW | Default `-` |
| Copy directly to clipboard (`--clip`) | Fast workflow | LOW | Call the `clip` subcommand's logic |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Pronounceable password mode | Low entropy for length; misleads users | Stick to random or passphrase |
| Writing to a file | Secrets written to disk = bad default | Pipe to file explicitly if needed |
| Requiring internet for generation | Offline security tool must stay offline | Local CSPRNG only |

---

### `hash` — File Hasher / Verifier

**Prior art:** `sha256sum`, `md5sum`, `certutil` (Windows), `Get-FileHash` (PowerShell)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Hash a file: `box hash file.zip` | Core job | LOW | Default algorithm: SHA-256 |
| Choose algorithm (`--algo sha256/sha512/md5/sha1/blake3`) | Power users need specific algorithms | LOW | Use `sha2`, `md5`, `blake3` crates |
| Verify mode (`--check HASH` or `--check-file checksums.txt`) | Compare against known-good hash | LOW | Exit 0 if match, 1 if mismatch |
| Accept stdin input (`box hash -`) | Pipeable | LOW | Streaming hash from stdin |
| Output: `HASH  filename` format | Matches sha256sum convention; compatible | LOW | Two spaces between hash and name |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Hash multiple files at once | Common need | LOW | Glob expansion or multiple args |
| BLAKE3 support | Faster than SHA on modern CPUs | LOW | `blake3` crate is well-maintained |
| Progress bar for large files | Long-running hashes feel stuck | MEDIUM | Stream with progress; disable non-TTY |
| `--json` output | `{"file": "...", "algo": "sha256", "hash": "..."}` | LOW | Useful for scripting |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Generating checksum files in custom formats | Incompatible with `sha256sum -c` | Use the standard `HASH  filename` format |
| MD5 as default | Broken for security; creates bad habits | Default SHA-256; MD5 available explicitly |

---

### `dupes` — Duplicate File Finder

**Prior art:** `fdupes`, `jdupes`, `fclones`, `rdfind`

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Find files with identical content by hash | Core job | MEDIUM | NOT by name; content identity only |
| Size pre-filter before hashing | Performance: skip hashing unique-sized files | MEDIUM | Standard optimization from fdupes/fclones |
| Display duplicate groups | Each group: shared size + all paths | LOW | Human-readable output: group separator, then paths |
| Recursive directory walk | Any depth | LOW | `walkdir` |
| `--min-size N` | Skip tiny files; often not worth deduplicating | LOW | Default 1 byte (include all) |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Multi-stage hashing (prefix → suffix → full) | Dramatically faster on large trees; fclones pattern | HIGH | First 4KB, last 4KB, then full file |
| `--json` output | List of groups as JSON arrays | LOW | Enables scripting: pipe to deletion tool |
| Summary stats at end | "Found N groups totaling X MB of duplicates" | LOW | Helps user decide what to do |
| `--follow-symlinks` flag | Include symlinked dirs/files | LOW | Off by default |
| Progress indicator during scan | Large trees take time | MEDIUM | Use `indicatif`; suppress non-TTY |

#### Edge Cases

| Case | Expected Behavior |
|------|------------------|
| Hardlinks to same inode | Do NOT report as duplicates by default (same bytes, not wasted space) |
| Zero-byte files | Skip or report as one group; they waste no space |
| Symlinks | Skip unless `--follow-symlinks` |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Auto-delete duplicates | Catastrophic if wrong; irreversible | Print paths; let user decide. `--delete` is v2 |
| Interactive selection UI | Scope creep; complex to build | Pipe `--json` output to a deletion script |
| Byte-by-byte comparison as first step | Slow on large files | Always size-first, then hash |

---

### `bulk-rename` — Regex Bulk Rename

**Prior art:** `rnr` (Rust), Perl `rename`, `mmv`, Windows PowerRename (PowerToys)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Regex pattern + replacement with capture groups | Core job | MEDIUM | `box bulk-rename 'PATTERN' 'REPLACEMENT' [files...]` |
| Dry-run by default | Safety — show what would happen before doing it | LOW | Default mode; require `--force` to execute |
| Preview table: `old name → new name` | Users must see what will change | LOW | Color-coded old (red) → new (green) when TTY |
| Numbered capture groups `$1`, `$2` | Standard regex replacement syntax | LOW | Most users know this from sed/grep |
| Named capture groups `(?P<name>...)` with `$name` | Power user feature | LOW | Rust regex crate supports this natively |
| Recursive option (`-r`) | Apply to subdirectories | LOW | Default: current dir files only |
| Skip files that wouldn't change | Don't rename if result is the same | LOW | Filter out no-op renames before display |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Case transformations (`.upper()`, `.lower()`, `.title()`) in replacement | Common need | MEDIUM | e.g. `$1.lower()` syntax or separate `--lower` flag |
| `--count-from N` | Number files sequentially in replacement | MEDIUM | e.g. `photo_$n` where n starts at 1 |
| `--backup` | Rename originals to `.bak` before renaming | MEDIUM | Safety net for the paranoid |
| Collision detection before executing | Warn if two files would get the same name | MEDIUM | Critical safety check |
| `--json` output of the plan | Machine-readable rename map | LOW | `[{from, to}, ...]` |

#### Edge Cases

| Case | Expected Behavior |
|------|------------------|
| Two different files would get the same name after rename | Abort with error listing the conflicts; do no renames |
| Pattern matches zero files | Warn "no files matched"; exit 0 |
| Replacement produces invalid filename chars on Windows | Error with clear message about the invalid character |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Execute by default (no dry-run) | Too dangerous for bulk file operations | Dry-run default; `--force` to execute |
| Undo/history log | Complex to implement correctly | Dry-run preview is the safety mechanism |
| Moving files across directories in rename | Different tool (mv); scope creep | Rename in place only |

---

### `tree` — Directory Tree Viewer

**Prior art:** `tree` (Unix), `eza --tree`, `fd --tree`

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Print directory tree with box-drawing characters | Core job | LOW | `├──`, `└──`, `│` |
| Show file sizes (`-s`) | The "with file sizes" part of the spec | LOW | Human-readable units (KB/MB) |
| Depth limit (`-L N`) | Prevent infinite output on deep trees | LOW | Default: no limit (but cap at sane depth) |
| Color directories vs files | Standard tree UX | LOW | Dirs: bold/cyan; files: default; disable with `--no-color` |
| File/dir count summary at end | Standard tree behavior | LOW | "X directories, Y files" |
| Respect `.gitignore` by default | Developers expect this | MEDIUM | Use `ignore` crate (same as ripgrep) |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `-a` show hidden files | Power user need | LOW | Hidden by default in spirit of normal use |
| `-d` directories only | Common need: "just show me the structure" | LOW | — |
| `-I PATTERN` ignore pattern | Exclude `node_modules`, `target/`, etc. | LOW | Glob matching |
| `--no-git` skip gitignore | When you want raw truth | LOW | Override the default gitignore respect |
| `--json` output | Machine-readable tree structure | HIGH | Nested JSON; useful for tooling |
| Sort by size | Show biggest items first; like `ncdu` | MEDIUM | — |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Full file permissions display | Not the tool's job; `ls -la` does it | Stick to name + size |
| ASCII-only tree characters | Looks ugly; Unicode is universal on PS7 | Always use box-drawing chars; `--ascii` is an escape hatch |

---

### `clip` — Clipboard Read/Write

**Prior art:** `clip.exe` (Windows, write-only), `Get-Clipboard`/`Set-Clipboard` (PowerShell), `xclip` (Linux)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Write stdin to clipboard: `echo "hello" \| box clip` | Core write path | LOW | Windows: use `arboard` or `clipboard-win` crate |
| Read clipboard to stdout: `box clip --paste` | Core read path | LOW | Enables downstream piping |
| Handle Unicode text correctly | PowerShell uses Unicode strings | LOW | clip.exe has known Unicode issues; bypass it |
| Trim trailing newline option when pasting | Common gotcha: pasted text gets extra blank line | LOW | `--trim` flag or on by default |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `box clip FILE` | Copy a file's contents to clipboard | LOW | Alternative to `cat FILE \| box clip` |
| Show byte count when copying | Confirmation feedback | LOW | "Copied 42 bytes to clipboard" to stderr |
| `--clear` | Wipe the clipboard | LOW | Privacy/security use case |

#### Windows-Specific Notes

- `clip.exe` is write-only and has encoding issues with non-ASCII; use the `clipboard-win` or `arboard` Rust crate instead
- Windows clipboard is process-scoped; the clipboard survives after the process exits
- PowerShell's `Set-Clipboard` works but adds PS dependency; Rust crate is cleaner

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Binary/image clipboard operations | Niche; complex format handling | Text only in v1 |
| Monitoring clipboard for changes | Different tool (clipboard manager) | Omit |

---

### `uuid` — UUID Generator

**Prior art:** `uuidgen` (Unix), `uuid` npm package, online generators

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Generate UUID v4 (random) by default | v4 is the most-used for general purpose | LOW | `uuid` Rust crate |
| Print to stdout with newline | Pipeable | LOW | — |
| `--count N` | Generate multiple at once | LOW | One per line |
| Lowercase hex output by default | Standard format | LOW | `--upper` flag for uppercase |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--version 7` for time-sorted UUIDs | Increasingly preferred for DB primary keys | LOW | RFC 9562; sortable by creation time |
| `--no-hyphens` | Some systems want `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` | LOW | — |
| `--braces` | `{uuid}` format | LOW | Windows COM/registry style |
| Copy to clipboard (`--clip`) | Fast dev workflow | LOW | Reuse clip logic |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| UUID v1 (MAC address) | Leaks machine identity; privacy risk | v4 or v7 |
| UUID validation mode | Out of scope for a generator | Omit for v1 |

---

### `json` — JSON Pretty-Printer / Validator

**Prior art:** `jq`, `python -m json.tool`, `fx`

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Pretty-print JSON from stdin or file | Core job | LOW | `serde_json` with indentation |
| Validate and report parse errors with line/column | "Is this valid JSON?" is the main use case | LOW | serde_json gives position on error |
| Exit 0 on valid, 1 on invalid | Scriptable validity check | LOW | — |
| Color-code keys, strings, numbers | Standard in modern JSON tools | MEDIUM | Use `colored` or `yansi` crate |
| Accept stdin: `cat file.json \| box json` | Pipeable | LOW | Standard pattern |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--compact` / `-c` | One-line output | LOW | Minify JSON |
| `--indent N` | Custom indentation (default 2) | LOW | — |
| `--sort-keys` | Alphabetize keys for diffing | LOW | serde_json supports this |
| Basic jq-style path query (`--query '.field'`) | Gateway drug for jq users | HIGH | Complex to implement well; skip v1 |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Full jq expression language | jq already exists and does this | Pretty-print + validate only; jq for queries |
| YAML/TOML conversion | Different tools for different formats | Omit |

---

### `base64` — Base64 Encoder/Decoder

**Prior art:** `base64` (GNU coreutils), `openssl enc -base64`

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Encode: stdin → base64 stdout | Core encode path | LOW | `base64` Rust crate or `data-encoding` |
| Decode: `--decode` / `-d` flag | Core decode path | LOW | — |
| Accept file as argument or stdin | Both patterns are common | LOW | `box base64 FILE` or `cat FILE \| box base64` |
| Output without line wrapping by default | Line wrapping (76-char default) breaks most use cases | LOW | Wrap is a legacy MIME thing; default off |
| URL-safe base64 (`--url-safe`) | Common in JWT and web tokens | LOW | Uses `-_` instead of `+/` |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--wrap N` | MIME-style line wrapping for email use cases | LOW | — |
| Decode with garbage-ignoring (`--ignore-garbage`) | Handles whitespace/newlines in input | LOW | Standard GNU base64 flag |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Base32/Base16/Base58 variants | Scope creep; rare | Omit for v1 |

---

### `epoch` — Unix Timestamp Converter

**Prior art:** `date -d @TIMESTAMP` (Linux), `epochconverter.com`, no good Windows equivalent

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Convert timestamp to human date: `box epoch 1718000000` | Core decode path | LOW | `chrono` crate |
| Get current timestamp: `box epoch` (no args) | Common "what time is it in epoch?" use | LOW | — |
| Convert date string to timestamp: `box epoch "2024-06-10"` | Core encode path | LOW | ISO 8601 input |
| Show both local time and UTC | Timezone confusion is the main pain point | LOW | Show both by default |
| Support milliseconds (`--millis`) | JS/frontend ecosystem uses ms | LOW | Detect 13-digit input as ms automatically |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Auto-detect seconds vs milliseconds vs microseconds | Eliminates cognitive overhead | MEDIUM | Heuristic: >1e12 is likely ms |
| `--relative` | "3 days 4 hours ago" | LOW | Useful for log analysis |
| `--tz America/New_York` | Convert in a specific timezone | MEDIUM | `chrono-tz` crate |
| `--json` output | `{"epoch": ..., "iso": "...", "local": "..."}` | LOW | — |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Date arithmetic (`+7 days`) | Different tool; complex parsing | Omit for v1 |

---

### `color` — Color Format Converter

**Prior art:** `convert-color-cli`, `dyetide`, various online tools

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Hex → RGB: `box color "#ff5733"` | Core conversion | LOW | Parse `#RRGGBB` and `RRGGBB` formats |
| RGB → Hex: `box color rgb(255,87,51)` | Core conversion | LOW | Parse `rgb(r,g,b)` format |
| Show a color swatch in terminal | Makes it visual / instantly useful | LOW | Print a block in the actual color using ANSI |
| Output all formats at once | Users want to see all representations | LOW | Show hex, rgb, and hsl together |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| HSL support | Common in CSS workflows | LOW | Conversion math is straightforward |
| Named CSS colors (`box color red`) | Useful for designers | LOW | Map of ~140 named colors |
| Copy to clipboard (`--clip`) | Fast workflow for CSS editing | LOW | Reuse clip logic |
| `--json` output | `{"hex": "#ff5733", "rgb": [255,87,51], "hsl": [...]}` | LOW | — |
| ANSI 256-color or truecolor swatch | Show the color visually in terminal | LOW | Requires ANSI truecolor support (PS7 has it) |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| CMYK support | Print domain; not a dev tool need | Omit |
| Color palette generation | Different tool | Omit |
| oklch/LAB/XYZ conversions | Specialist; rare need | v2 if requested |

---

### `du` — Disk Usage Analyzer

**Prior art:** `du` (Unix), `dust` (Rust), `ncdu` (interactive), `WinDirStat` (Windows GUI)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Show disk usage of current dir as tree | Core job | MEDIUM | Use `walkdir`; aggregate sizes per dir |
| Sort by size descending (biggest first) | Users want to find the space hogs | LOW | — |
| Human-readable sizes (KB/MB/GB) | Raw bytes are unreadable | LOW | `--bytes` flag for raw |
| Depth limit (`-d N`) | Prevent overwhelming output | LOW | Default: 1 level deep |
| Show top N entries (`-n N`) | Focus on the worst offenders | LOW | Default: 20 |
| Percentage bars | Shows relative size visually; dust pattern | MEDIUM | ASCII bar: `[████░░░░░░] 40%` |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Color-code size ranges | Hot (red) = big; cool (green) = small | LOW | Like dust's color scheme |
| `--apparent-size` vs disk usage | Different for sparse files and hardlinks | MEDIUM | Report actual bytes, not block-aligned size |
| `--min-size N` | Filter below threshold | LOW | Reduce noise |
| `--json` output | Full tree as JSON | MEDIUM | Useful for scripting |
| Exclude patterns (`-X node_modules`) | Common noise sources | LOW | — |
| `--no-bar` | Plain text output without ASCII bars | LOW | Better for piping |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Interactive TUI (ncdu-style) | Scope creep; different product | ncdu/WinDirStat for interactive exploration |
| Delete files from within du | Catastrophic risk | Just show; let user delete manually |
| Network filesystem analysis | Unpredictable latency; hangs | Local filesystem only; warn on network paths |

---

### `ascii` — Image to ASCII Art

**Prior art:** `ascii-image-converter` (Go), `jp2a` (C), `caca-utils`

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Convert image to ASCII art in terminal | Core job | HIGH | `image` crate for loading; brightness mapping to chars |
| Fit output to terminal width | Auto-scale to available columns | LOW | Read terminal width with `terminal_size` crate |
| Support JPEG, PNG (minimum) | Most common image formats | LOW | `image` crate handles both |
| Grayscale brightness mapping | Core algorithm | LOW | Map pixel brightness to char density |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Color mode (`--color`) | ANSI truecolor or 256-color per character | HIGH | PS7 supports truecolor; dramatically better output |
| `--width N` | Explicit width override | LOW | — |
| Braille characters mode (`--braille`) | Higher density; sharper output | MEDIUM | Uses Unicode braille block |
| `--invert` | Dark terminals vs light terminals | LOW | Flip the brightness mapping |
| `--complex` charset | Larger character set for more tonal range | LOW | More than just ` .:-=+*#@` |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Video/GIF support | Complex; frame management | v2 if requested |
| Saving output as image | Different tool; complex | Print to terminal only |
| Web URL image input | Network dependency; adds complexity | File input only in v1 |

---

### `cowsay` — ASCII Speech Bubble

**Prior art:** `cowsay` (Perl/Unix), various ports

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| ASCII cow with speech bubble saying input text | Core job | LOW | Hard-code the classic cow |
| Accept text as argument or from stdin | Both patterns are used | LOW | `echo "text" \| box cowsay` |
| Word-wrap long text in bubble | Prevent overflow | LOW | Default width 40 chars; `--width N` |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Multiple figures (`--figure cow/dragon/tux/...`) | Users love the menagerie | MEDIUM | Bundle a few classic figures as embedded strings |
| `--think` mode | Thought bubble instead of speech bubble | LOW | Uses `o` connectors instead of `\` |
| `-d` dead eyes, `-b` borg, etc. | Classic cowsay modes | LOW | Alter the eye characters |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Custom figure files from disk | Security/path complexity; most users use defaults | Bundle a fixed set of figures |
| Network figure downloads | Absurd for a fun toy | Embedded only |

---

### `lolcat` — Rainbow Text Colorizer

**Prior art:** `lolcat` (Ruby), `lolcat` (C re-implementation)

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Pipe stdin through and colorize each character with rainbow gradient | Core job | MEDIUM | ANSI truecolor color cycling; `sin`/`cos` based |
| Works as a pipeline filter: `box tree \| box lolcat` | Must be composable | LOW | Read stdin, write to stdout with colors |
| Disable when stdout is not TTY | When piped further, don't corrupt data | LOW | Strip colors in non-TTY mode |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--freq F` | How fast the colors cycle | LOW | Default ~0.3; lower = slower gradient |
| `--seed N` | Reproducible color pattern | LOW | Same seed = same gradient start |
| `--animate` | Animate the gradient (shift seed over time) | MEDIUM | Ctrl+C to stop |
| `--force` | Force colors even when not TTY | LOW | For the adventurous |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Only ANSI 256-color (not truecolor) | Chunky bands instead of smooth gradients | Use truecolor on PS7; fallback only if needed |
| Speed/performance flags for huge files | Lolcat is for fun, not big data | Keep it simple |

---

### `matrix` — Matrix Digital Rain

**Prior art:** `cmatrix` (C), `matrix-rain` (Node.js), PowerShell matrix scripts

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Full-terminal falling green characters animation | Core job | MEDIUM | Terminal control with `crossterm` crate |
| Ctrl+C to exit cleanly | Must restore terminal state on exit | LOW | Catch Ctrl+C signal; reset terminal |
| Auto-resize on terminal resize | Looks broken without this | MEDIUM | Handle SIGWINCH / resize events |
| Katakana + ASCII characters | The authentic Matrix look | LOW | Mix both character sets |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--color green/red/blue/cyan/white` | Classic is green; let user choose | LOW | — |
| `--speed N` | Control fall speed | LOW | Default medium |
| `--density N` | More or fewer active columns | LOW | — |
| `--chars ascii/katakana/binary/braille` | Different aesthetics | LOW | — |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Saving to video/gif | Extremely complex; different tool | Omit |
| Custom font/image in rain | Scope creep | Omit |

---

### `roast` — Programmer Roast Generator

**Prior art:** None (original); inspired by BOFH excuse server, `fortune` insult packs

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Print a random programmer roast | Core job | LOW | Embedded vec of roast strings |
| Different roast every run | Must not always print the same one | LOW | `rand` crate with `OsRng` or thread_rng |
| Exit 0 always | It's a joke; errors are unexpected | LOW | — |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--language python/javascript/rust/...` | Target-specific roasts | LOW | Filter by language tag on each roast |
| `--count N` | Print multiple roasts | LOW | — |
| Quality over quantity | 50 excellent roasts > 200 mediocre ones | LOW | Curate the list carefully |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| AI-generated roasts (API call) | Network dependency for a fun toy; latency | Curated local list |
| Offensive/personal content | Not funny; just mean | Keep to language/framework/pattern roasts |

---

### `fortune` — Random Fortune/Quote

**Prior art:** `fortune` (Unix), the classic fortune cookie database

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Print a random quote or aphorism | Core job | LOW | Embedded collection of quotes |
| New quote every run | Random selection | LOW | `rand` crate |
| Reasonable quote length | Avoid quotes that overflow the terminal | LOW | Max ~200 chars per quote |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Categories (`--category programming/philosophy/humor`) | Users have preferences | LOW | Tag quotes with categories |
| `--short` | One-liners only | LOW | Filter by length |
| Pairs naturally with cowsay: `box fortune \| box cowsay` | Classic Unix combo | LOW | Just works via pipes |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Network fortune databases | Latency for a fun toy | Local embedded collection |
| Offensive fortune packs | Off-putting | Curate for developer audience |

---

### `8ball` — Magic 8-Ball Oracle

**Prior art:** Physical Magic 8-Ball toy; `fortune` variants

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Accept a yes/no question and print a classic 8-ball response | Core job | LOW | 20 classic responses embedded |
| Question is optional (just prints a random response) | Works without input | LOW | — |
| Include positive/neutral/negative response types | Authentic 8-ball has all three | LOW | Classic 20 responses: 10 positive, 5 neutral, 5 negative |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| ASCII 8-ball art around the response | Whimsical presentation | LOW | Box/circle decoration |
| Color-code by sentiment | Green=positive, yellow=neutral, red=negative | LOW | ANSI colors; disable non-TTY |

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| AI-generated responses | Ruins the joke; defeats the point | Classic fixed responses only |
| Taking the question seriously | It's a toy | It's a toy |

---

### `pomodoro` — Focus Timer

**Prior art:** `pomo`, `timer`, `pomocli`; PowerShell BurntToast-based timers

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| 25-minute work timer with countdown display | Core Pomodoro behavior | MEDIUM | `crossterm` for countdown; update in-place |
| Windows toast notification on completion | The point of a terminal timer | HIGH | `winrt-notification` or `winrt-toast` crate |
| Ctrl+C to cancel cleanly | Terminal must restore state | LOW | Signal handling |
| In-place countdown (no scrolling) | Scrolling timers are annoying | LOW | `crossterm` cursor overwrite |
| Short break (5 min) and long break (15 min) modes | Standard Pomodoro intervals | LOW | `--break` and `--long-break` flags |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| `--work N` custom work duration | Power user flexibility | LOW | Default 25 |
| `--label "Task name"` | Shows in notification and timer | LOW | Context for the session |
| Sound/beep on completion | Additional notification layer | MEDIUM | Windows Beep API or embedded sound |
| Session counter: "Pomodoro 3 of 4" | Tracks progress in a session | LOW | `--sessions N` flag |
| `--auto-break` | Start break automatically after work | MEDIUM | Chain sessions |

#### Windows-Specific Notes

- Use `winrt-toast` crate for native Windows 10/11 toast notifications
- App ID required for toast — can register under `box` app name
- Terminal countdown: use `crossterm` to write/overwrite current line

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Interactive TUI with stats dashboard | Scope creep; pomodoro is a simple timer | Just a timer with notification |
| Pomodoro history/tracking to file | Can add in v2 if wanted | Keep v1 stateless |
| Cross-session statistics | Same | v2 |

---

### `weather` — Quick Weather Fetch

**Prior art:** `wttr.in` (curl-based), `stormy` (Rust, Open-Meteo), various Go tools

#### Table Stakes

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Fetch current conditions for a location | Core job | HIGH | HTTP fetch from Open-Meteo (no API key) or wttr.in |
| Accept city name or lat/lon | Both user types exist | MEDIUM | Geocoding needed for city → lat/lon |
| Show: temperature, conditions, wind, humidity | The key weather data points | LOW | Open-Meteo JSON parsing |
| Metric and imperial units (`--metric` / `--imperial`) | US vs rest of world | LOW | Default: detect from system locale or ask user |
| Graceful error on no network | Clear error, not a panic | LOW | "Could not reach weather service" |

#### Differentiators

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| Weather emoji/ASCII art per condition | Terminal weather tools are expected to be visual | LOW | ☀️ 🌧️ ⛅ etc. |
| 3-day forecast (`--forecast`) | Often what user actually wants | MEDIUM | Open-Meteo supports this in same API call |
| `--json` output | Scriptable weather data | LOW | Pass through Open-Meteo JSON |
| Cache last result for N minutes | Avoid repeated API calls in quick succession | MEDIUM | Write to temp file with timestamp |
| `--location` stored preference | Remember default location | MEDIUM | Config file in `%APPDATA%\box\config.toml` |

#### Recommended Backend

Use **Open-Meteo** (open-meteo.com) as the weather data source:
- No API key required
- Free for non-commercial use
- Returns structured JSON
- Requires a separate geocoding step for city names (use Open-Meteo geocoding API, also free and keyless)

#### Anti-Features

| Anti-Feature | Why Avoid | Alternative |
|-------------|-----------|-------------|
| Requiring user to register for an API key | Friction kills adoption of a quick tool | Open-Meteo is keyless |
| Full ASCII weather maps | Heavy to render; novelty wears thin | Simple data display |
| Radar/satellite data | Complex data; different domain | Omit |

---

## Feature Dependencies

```
clip (logic)
    ├──enhances──> passgen (--clip flag)
    ├──enhances──> uuid (--clip flag)
    └──enhances──> color (--clip flag)

fortune
    └──composes with──> cowsay (via pipe)

cowsay
    └──composes with──> lolcat (via pipe)

flatten
    └──requires dry-run pattern──> (same pattern as bulk-rename and dupes)

pomodoro
    └──requires──> Windows toast notification crate (winrt-toast / winrt-notification)

weather
    └──requires──> HTTP client crate (reqwest) + JSON parsing (serde_json)
    └──requires──> Geocoding API call (Open-Meteo geocoding, same HTTP client)

ascii (image)
    └──requires──> image loading crate (image crate)

matrix, pomodoro (countdown)
    └──require──> terminal control (crossterm)

dupes, du, flatten, bulk-rename
    └──all require──> recursive directory walk (walkdir)
```

---

## MVP Definition

All 23 commands are v1 scope per PROJECT.md. Within each command, this is the build order priority:

### Ship These Behaviors First (Core Loop)

- [ ] `flatten` with dry-run + collision rename — anchor command; validates the whole concept
- [ ] `hash` — table stakes utility; fast to build
- [ ] `passgen` — table stakes; fast to build
- [ ] `uuid` — trivial; immediate value
- [ ] `base64`, `epoch`, `color` — converters; low complexity, high daily utility
- [ ] `json` — developers reach for this constantly
- [ ] `clip` — enables clipboard integration across other commands
- [ ] `tree` — useful daily; moderate complexity
- [ ] `du` — useful daily; moderate complexity
- [ ] `bulk-rename` — needs careful dry-run/collision handling; build on tree/walkdir
- [ ] `dupes` — multi-stage hash; performance matters; build after hash

### Fun Layer (After Core)

- [ ] `cowsay`, `fortune`, `8ball`, `roast` — low complexity; all embedded strings
- [ ] `lolcat` — ANSI truecolor; moderate complexity
- [ ] `matrix` — terminal animation; moderate complexity
- [ ] `qr` — terminal rendering; moderate complexity
- [ ] `ascii` (image) — image loading; highest complexity in fun tier

### Platform-Dependent (Validate Early)

- [ ] `pomodoro` — Windows toast requires early validation of crate/API
- [ ] `weather` — network dependency; test Open-Meteo geocoding + data API together
- [ ] `clip` — validate `arboard`/`clipboard-win` works in PS7 early

---

## Feature Prioritization Matrix

| Command | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| flatten | HIGH | MEDIUM | P1 |
| hash | HIGH | LOW | P1 |
| passgen | HIGH | LOW | P1 |
| clip | HIGH | LOW | P1 |
| uuid | HIGH | LOW | P1 |
| json | HIGH | LOW | P1 |
| base64 | MEDIUM | LOW | P1 |
| epoch | MEDIUM | LOW | P1 |
| bulk-rename | HIGH | MEDIUM | P1 |
| tree | HIGH | MEDIUM | P1 |
| du | HIGH | MEDIUM | P1 |
| dupes | HIGH | HIGH | P2 |
| weather | MEDIUM | HIGH | P2 |
| pomodoro | MEDIUM | HIGH | P2 |
| qr | MEDIUM | MEDIUM | P2 |
| color | MEDIUM | LOW | P2 |
| ascii | MEDIUM | HIGH | P2 |
| cowsay | MEDIUM | LOW | P2 |
| fortune | MEDIUM | LOW | P2 |
| lolcat | MEDIUM | MEDIUM | P2 |
| matrix | LOW | MEDIUM | P3 |
| roast | LOW | LOW | P3 |
| 8ball | LOW | LOW | P3 |

---

## Sources

- [clig.dev CLI UX conventions](https://clig.dev/) — exit codes, TTY detection, stdout/stderr, dry-run patterns
- [dust (bootandy/dust)](https://github.com/bootandy/dust) — disk usage tree behavior and bar visualization
- [rnr (ismaelgv/rnr)](https://github.com/ismaelgv/rnr) — dry-run-first, capture group syntax for bulk rename
- [fclones (pkolaczk/fclones)](https://github.com/pkolaczk/fclones) — multi-stage hashing strategy for duplicate detection
- [qrencode man page](https://linuxcommandlibrary.com/man/qrencode) — error correction levels, output format types
- [Open-Meteo](https://github.com/open-meteo/open-meteo) — free, keyless weather API
- [ascii-image-converter](https://github.com/TheZoraiz/ascii-image-converter) — brightness mapping, color mode, braille mode
- [wttr.in](https://github.com/chubin/wttr.in) — terminal weather UX patterns
- [winrt-notification crate](https://crates.io/crates/winrt-notification) — Windows toast notifications from Rust
- [PowerRename (PowerToys)](https://learn.microsoft.com/en-us/windows/powertoys/powerrename) — preview-first bulk rename UX for Windows

---

*Feature research for: box — Rust CLI toolbox, 23 subcommands, Windows PowerShell 7*
*Researched: 2026-06-22*
