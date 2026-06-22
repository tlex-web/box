# Pitfalls Research

**Domain:** Rust single-binary CLI toolbox targeting Windows PowerShell 7
**Researched:** 2026-06-22
**Confidence:** HIGH (verified against Rust stdlib issues, official Microsoft docs, and crate documentation)

---

## Critical Pitfalls

### Pitfall 1: `std::fs::canonicalize` Returns Verbatim UNC Paths That Break Everything Downstream

**What goes wrong:**
`std::fs::canonicalize("C:\\foo")` returns `\\?\C:\foo` (a verbatim/extended-length UNC path). Many Windows APIs, external tools, and shell integrations do not accept this prefix. Passing these paths to `Command::new().current_dir(...)`, displaying them to users, or building paths by concatenation with `+` causes silent failures, confusing error messages, or hard panics.

**Why it happens:**
Rust's canonicalize calls `GetFinalPathNameByHandleW` which always returns the `\\?\` prefix. Developers assume `canonicalize` produces a clean `C:\...` path — it does not. This is a tracked, long-standing stdlib issue (rust-lang/rust#42869, rust-lang/rust#80884).

**How to avoid:**
Never use `std::fs::canonicalize` as a general-purpose path normalizer. Use it only when you explicitly want the verbatim-UNC form. For path normalization (collapsing `..`, resolving symlinks without the prefix), use the `dunce` crate's `dunce::canonicalize`, which strips the `\\?\` prefix and returns a plain `C:\...` path. Treat verbatim paths as an internal implementation detail — strip before displaying or passing to any external API.

**Warning signs:**
- Paths shown to users that begin with `\\?\`
- `Command::current_dir(...)` silently spawning in wrong directory
- "Cannot mix UNC and non-UNC paths" errors during path joining operations

**Phase to address:**
Foundation phase — establish a single `util::normalize_path` wrapper around `dunce::canonicalize` used everywhere. Never call `std::fs::canonicalize` directly.

---

### Pitfall 2: The 260-Character Path Limit Silently Truncates or Errors

**What goes wrong:**
`std::fs::copy`, `std::fs::write`, `std::fs::create_dir_all`, and related functions on Windows default to the legacy `MAX_PATH` limit of 260 characters. Operations on deeply nested trees silently fail with `Os { code: 3, kind: NotFound }` or `Os { code: 206 }` — messages that look like "file not found" even though the directory exists. This hits `flatten`, `dupes`, `tree`, and `du` immediately in real-world use cases (node_modules, deeply nested source trees).

**Why it happens:**
The `\\?\` prefix opts in to 32,767-character paths, but Rust does not add it automatically for regular `PathBuf` operations. The registry key `LongPathsEnabled` lifts the limit for apps that opt in via manifest, but unsigned redistributed executables typically lack that manifest. Rust's `std::fs` issues tracker (rust-lang/rust#76586, rust-lang/rust#67403) confirm this is not fixed at the stdlib level.

**How to avoid:**
Use the `verbatim` crate (`verbatim::verbatim_win32()`) or prepend `\\?\` manually before passing deeply nested paths to any `std::fs` call. Alternatively, use the `dunce` + `verbatim` combo: normalize paths with `dunce` for display, prepend `\\?\` for actual I/O operations. Also embed a Windows application manifest with `LongPathsAware` set to `true` using the `embed-resource` crate at build time.

**Warning signs:**
- `NotFound` errors on directories you can see in Explorer
- Operations that work for shallow trees but fail once nesting exceeds ~8-10 levels
- `src/main.rs` paths counting > 220 characters in `node_modules`

**Phase to address:**
`flatten`, `dupes`, `tree`, `du` — all file traversal commands. Test with a synthetic 300-character path in CI.

---

### Pitfall 3: Output Directory Inside Input Directory Causes Infinite File Copy Loop in `flatten`

**What goes wrong:**
`box flatten ./project ./project/flat` — the output directory is nested inside the input. The walker visits `./project/flat` while copying into it, picks up each just-copied file as a new source, copies it again, and either fills the disk, blows past the filename collision counter, or loops until an OS error occurs.

**Why it happens:**
The command naturally wants to accept `<src> <dest>` where both are paths. The failure mode only occurs when dest is a subdirectory of src. Developers testing on small trees miss it; only large trees or fast disks reveal the problem quickly.

**How to avoid:**
At the start of `flatten`, canonicalize both `src` and `dest` with `dunce::canonicalize`. If `dest` starts with `src` (i.e., `dest` is equal to or nested inside `src`), abort with a clear error: "Output directory cannot be inside the source directory." Use `Path::starts_with` on the canonical forms. This check must happen before the walker is started.

**Warning signs:**
- No error but disk fills up unexpectedly
- Output directory grows without bound during run
- Files named with very long path-encoded prefixes accumulate

**Phase to address:**
`flatten` command — add the guard as the first validation step, before any I/O.

---

### Pitfall 4: NTFS Case-Insensitivity Produces Silent Collision Bugs in `flatten` and `dupes`

**What goes wrong:**
`Report.pdf` and `report.pdf` are two different files on Windows (NTFS is case-preserving) but are the same filename if written to a FAT32 drive or treated as a string comparison. In `flatten`, copying both files to the same output directory results in one silently overwriting the other if the collision logic uses case-sensitive `HashMap` keys. In `dupes`, comparing canonical paths with `==` misses cross-case duplicates (e.g., a file referenced via two different cased paths on a case-insensitive mount).

**Why it happens:**
Rust's `String` equality is case-sensitive. Path comparisons via `PathBuf::eq` are also case-sensitive on all platforms, even Windows. Developers port string-comparison logic from Linux where it would be correct, and it misbehaves on Windows.

**How to avoid:**
In `flatten` collision detection: use `path.to_string_lossy().to_ascii_lowercase()` as the HashMap key, not the raw filename. In `dupes`: compare file content (by hash), not paths — content-hash deduplication is immune to case issues. For path existence checks, never rely on string equality; use the filesystem directly (`path.exists()`). Note: `Path::starts_with` is case-sensitive on Windows (rust-lang/rust#66260), so normalize case before using it for containment checks.

**Warning signs:**
- `flatten` silently produces fewer files than expected on trees with mixed-case filenames
- `dupes` misses files that are byte-for-byte identical but referenced with different path casing

**Phase to address:**
`flatten` (collision key normalization), `dupes` (verify content-hash approach is used, not path equality).

---

### Pitfall 5: `walkdir` Follows Symlinks into Loops When `follow_links(true)` Is Set

**What goes wrong:**
`walkdir::WalkDir::new(root).follow_links(true)` will loop infinitely if there is a junction point or symlink that points back to an ancestor directory. On Windows, junction points (`mklink /J`) are extremely common (e.g., `AppData` redirection targets) and are treated as followed symlinks by default. This causes `flatten`, `dupes`, `tree`, and `du` to spin indefinitely.

**Why it happens:**
`walkdir` does detect symlink loops when `follow_links(true)` is set and emits an error entry — but only for true symlink cycles, not necessarily all junction re-entries. More importantly, developers often set `follow_links(true)` to "see everything" without realizing Windows junction points create loops in common system directories.

**How to avoid:**
Default `follow_links` to `false` for all traversal commands. If link-following is desired, make it an explicit opt-in flag (`--follow-links`). When following, use `walkdir`'s built-in loop detection (it does track device/inode pairs when following links). On Windows, always check `DirEntry::path_is_symlink()` and skip junction points unless `--follow-links` is set.

**Warning signs:**
- CPU pegged at 100% with no I/O progress
- File count growing without bound
- Stack depth or open-file-descriptor count climbing

**Phase to address:**
`flatten`, `dupes`, `tree`, `du` — document default behavior (`follow_links(false)`) and add `--follow-links` flag with explicit warning.

---

### Pitfall 6: ANSI/VT Color Codes Emitted When Output Is Piped, Breaking Downstream Tools

**What goes wrong:**
`box tree` or `box du` output is piped into `Out-File`, `Select-String`, or `grep`. Color escape codes (`\x1b[32m`) appear in the output as literal garbage characters, corrupting the downstream data. This is especially bad for `lolcat` (whose purpose is color) and for `tree`/`du` which use color for structure.

**Why it happens:**
ANSI codes are emitted unconditionally based on "is this a terminal?" detection that is wrong, or not done at all. PowerShell 7 enables VT processing automatically for its own host, but a pipe to `Out-File` is not a terminal — `is_terminal()` correctly returns `false` in this case, but only if you check it.

**How to avoid:**
Check `std::io::stdout().is_terminal()` (from the `is-terminal` crate or stable Rust 1.70+) before emitting any ANSI codes. Honor `NO_COLOR` environment variable (`std::env::var("NO_COLOR").is_ok()` → disable color). Honor `CLICOLOR_FORCE=1` → force color even when not a terminal. Provide `--color=always|never|auto` flag on every command that produces colored output. The `owo-colors` crate integrates all of this automatically via `if_supports_color`.

**Warning signs:**
- `box tree | Out-File tree.txt` produces a file full of `\x1b[` sequences
- `box du | Select-String "GB"` returns no matches despite matches existing

**Phase to address:**
Foundation phase — establish a project-wide color utility that wraps `owo-colors` or `colored` with `NO_COLOR`/`is_terminal` checks. Never call raw ANSI macros.

---

### Pitfall 7: QR Codes Render as Stretched Rectangles Because Terminal Cells Are Taller Than Wide

**What goes wrong:**
Rendering one QR module per character cell produces a QR image that is roughly twice as tall as it is wide (because terminal cells are approximately 2:1 height:width). The QR code looks "squished" vertically and phone cameras often fail to scan it reliably because the aspect ratio distorts the finder patterns.

**Why it happens:**
The naive approach maps one QR module to one character. The correct approach packs two vertically-adjacent modules into a single character using Unicode half-block characters (U+2580 `▀` top half, U+2584 `▄` bottom half, U+2588 `█` full, space for empty). This halves the line count and produces a square output.

**How to avoid:**
Use the half-block rendering technique. The `qrcode` crate's `render::unicode` module supports this natively. Alternatively, use the `qr2term` crate which handles the half-block rendering automatically. Always add at least 2 characters of "quiet zone" (empty border) around the QR code — many scanners require it.

**Warning signs:**
- QR output lines are roughly twice as many as expected
- Phone camera fails to scan even simple URLs
- Output looks rectangular rather than square

**Phase to address:**
`qr` command implementation — verify output with a phone scan before shipping.

---

### Pitfall 8: Windows Clipboard Requires Single-Thread Access; Multi-Thread Access Deadlocks

**What goes wrong:**
The Windows clipboard is a global resource that can only be opened by one thread at a time. If `clip` is implemented naively with any async or multi-threaded executor and clipboard operations are not confined to a single thread, the result is an error (`WinError::access_denied`) or a deadlock. Additionally, if the `Clipboard` object is dropped immediately after `set_text()`, the content may vanish before other applications (or clipboard managers) can read it.

**Why it happens:**
`arboard` (the recommended Rust clipboard crate) documents this limitation explicitly. The clipboard on Windows is a global object that may only be opened on one thread at once. Dropping the `Clipboard` object too quickly ends clipboard ownership — content disappears.

**How to avoid:**
Keep clipboard operations on the main thread. Never use `tokio::spawn` or `rayon` for clipboard calls. After `set_text()`, do not drop the `Clipboard` object until you are certain the content has been consumed (for a CLI, this means: write, then verify success, then exit — the OS retains the data after process exit for plain text). Test with Windows Clipboard History (`Win+V`) to verify content appears correctly.

**Warning signs:**
- `clip --set "hello"` succeeds but clipboard is empty when you paste
- Intermittent errors when running `clip` rapidly in scripts

**Phase to address:**
`clip` command — keep synchronous, main-thread-only; document that piped input is consumed before writing to clipboard.

---

### Pitfall 9: Toast Notifications Require a Valid AppUserModelID; CLI Binaries Don't Have One

**What goes wrong:**
`pomodoro` fires a Windows toast notification. Toast notifications on Windows require the sending application to have an `AppUserModelID` (AUMID) registered in the Start Menu or the Windows registry. An unsigned CLI binary without a registered AUMID either silently fails to show the toast, or shows it attributed to "PowerShell" (using `POWERSHELL_APP_ID` as a fallback), which confuses users.

**Why it happens:**
WinRT's `ToastNotificationManager` checks for a valid AUMID. `winrt-notification` and similar crates work around this by using `POWERSHELL_APP_ID` as a fallback, but this means the notification appears to come from PowerShell. The `win-toast-notify` crate documents this limitation and notes the library is "currently in an unstable state."

**How to avoid:**
Accept the PowerShell attribution fallback for v1 — it works and shows the notification. Document the behavior (notification appears from "PowerShell" attribution). If attribution matters, register a custom AUMID during the install script using `New-StartMenuEntry` or the registry approach documented by Microsoft. Use `winrt-notification` with `POWERSHELL_APP_ID` as the fallback for v1. Test on a machine where PowerShell is not pinned to the taskbar (the fallback AUMID may behave differently).

**Warning signs:**
- `pomodoro` exits successfully but no notification appears
- Notification appears but clicking it opens PowerShell instead of nothing

**Phase to address:**
`pomodoro` command — explicitly use AUMID fallback and document in help text that notifications appear from "Windows PowerShell."

---

### Pitfall 10: `install.ps1` PATH Change Does Not Take Effect in the Current Shell Session

**What goes wrong:**
`install.ps1` updates the user PATH via `[System.Environment]::SetEnvironmentVariable("PATH", ..., "User")`. This correctly persists to the registry, but the currently-running PowerShell session does not see the change. The user runs `box --help` immediately and gets "command not found." They assume the install failed when it succeeded.

**Why it happens:**
PowerShell caches environment variables at session startup. Modifying the registry entry does not refresh the live `$env:PATH` in the current process. This is standard Windows environment inheritance behavior — not a Rust or PowerShell bug.

**How to avoid:**
At the end of `install.ps1`, explicitly refresh `$env:PATH` in the current session:
```powershell
$env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","User") + ";" +
             [System.Environment]::GetEnvironmentVariable("PATH","Machine")
```
Then immediately run `box --help` at the end of the install script as a smoke test. Print a clear success message: "Installed successfully. You can now run 'box --help'. (New terminal windows will also work.)"

**Warning signs:**
- User reports "box not found" immediately after install despite install script reporting success
- Script succeeds but `where.exe box` returns nothing in the current session

**Phase to address:**
`install.ps1` — add PATH refresh + smoke test as final two steps.

---

### Pitfall 11: PowerShell Execution Policy Blocks `install.ps1` for New Users

**What goes wrong:**
A developer sends `install.ps1` to a colleague. The colleague runs `.\install.ps1` and gets "cannot be loaded because running scripts is disabled on this system." The default PowerShell execution policy for Windows 11 is `Restricted` (never run scripts) or `RemoteSigned` (downloaded scripts must be signed). An unsigned `install.ps1` downloaded from the internet is blocked.

**Why it happens:**
The execution policy is a UI-level safety feature. Downloaded files have an NTFS "Zone.Identifier" alternate data stream marking them as coming from the internet. PowerShell checks this before running scripts.

**How to avoid:**
Document the one-time command users need to run before the install:
```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```
Alternatively, provide an install one-liner that bypasses policy for that invocation only:
```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```
In the README, include both the bypass one-liner and the permanent fix. Do not suggest `Unrestricted` — it removes all protection permanently.

**Warning signs:**
- First-run failure on machines that have never run custom scripts
- Error message mentions "execution policy" rather than a missing file

**Phase to address:**
`install.ps1` and README — document this prominently as Step 0.

---

### Pitfall 12: Windows Defender / SmartScreen Flags the Unsigned `box.exe` on First Run

**What goes wrong:**
A user downloads `box.exe` built by CI or a release zip. Windows SmartScreen shows "Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting." The user has no obvious path forward — the "More info" → "Run anyway" flow is non-obvious and frightening.

**Why it happens:**
SmartScreen uses reputation-based blocking. Unsigned executables with low download counts have no established reputation and are blocked or warned. This affects `rustup-init.exe` itself (confirmed in rust-lang/rust#56815), so unsigned Rust-compiled executables are reliably affected.

**How to avoid:**
For personal use (single developer, no distribution), this is a non-issue — the binary is built locally. For sharing: document the "More info → Run anyway" path in the README. Long-term: self-signed certificates reduce the friction but don't eliminate it. True fix requires an EV code-signing certificate (~$300-500/year). For v1, consider building and running from source via `cargo build --release` rather than distributing pre-built binaries.

**Warning signs:**
- CI-built artifacts flagged on download
- User reports cannot run `box.exe` on a fresh Windows install

**Phase to address:**
Distribution / README — document before first external share. Not a blocker for personal use.

---

### Pitfall 13: `fs::copy` Does Not Preserve File Timestamps or Metadata on Windows

**What goes wrong:**
`flatten` copies files using `std::fs::copy`. The destination files all have the same "date created" and "date modified" (the time of the flatten operation) rather than the original file's timestamps. Users who rely on file dates for organization find their metadata destroyed. On Windows specifically, `std::fs::copy` does not preserve modification time.

**Why it happens:**
`std::fs::copy` on Windows calls `CopyFileExW`, which preserves some metadata but not all timestamp fields consistently across Rust versions. The stdlib documentation explicitly does not guarantee timestamp preservation. A pull request to preserve times on Unix was accepted (rust-lang/rust#32067) but Windows behavior remains inconsistent.

**How to avoid:**
After each `fs::copy`, explicitly copy timestamps using `filetime::set_file_times`. The `filetime` crate provides a cross-platform way to set `mtime` and `atime`. For `flatten`, the minimum courtesy is to preserve `mtime`. For `dupes` (if it has a delete/move mode), also preserve creation time. Consider the `fs-more` crate which handles metadata copying more completely.

**Warning signs:**
- All files in flatten output have identical timestamps
- `dir` output shows today's date on files that are years old

**Phase to address:**
`flatten` command — add `filetime::set_file_times` call after every `fs::copy`.

---

### Pitfall 14: Reserved Windows Filenames Cause Crashes in `flatten` Collision Encoding

**What goes wrong:**
`flatten` encodes path collisions as `parent_child_file.txt`. If a source path encodes to a name like `CON.txt`, `NUL.txt`, `PRN.txt`, `COM1.txt`, or any of the 22 Windows reserved device names, `fs::write` or `fs::create` silently writes to a device or returns an unexpected error (e.g., `Os { code: 87, message: "The parameter is incorrect." }`). The file is not created.

**Why it happens:**
Windows reserves these names at the kernel level. Even with extensions (`NUL.txt`) or full paths (`C:\out\NUL.txt`), the kernel intercepts access. Developers who test on Linux miss this entirely. Source trees with a file literally named `con` or `nul` trigger the collision encoding to produce a bad destination name.

**How to avoid:**
Add a post-encoding validation step in `flatten` that checks the stem of the generated filename (case-insensitively) against the full list: `CON, PRN, AUX, NUL, COM0–COM9, LPT0–LPT9`. If matched, append an underscore or counter suffix (`NUL_.txt`). Use the `sanitize-filename` crate which handles this validation automatically.

**Warning signs:**
- Files disappear during flatten with no error
- A file in the source tree named `con`, `nul`, `prn`, etc.
- `Os { code: 87 }` errors during writes

**Phase to address:**
`flatten` — filename generation step. Also applies to `bulk-rename` output validation.

---

### Pitfall 15: `crossterm` on Windows Emits Color Codes per Character, Causing ~5 FPS in `matrix`

**What goes wrong:**
`matrix` renders digital rain using colored characters. If each character is written with an individual `crossterm::style::Print(styled_char)` call (which internally calls `SetConsoleTextAttribute` per character on older Windows), performance degrades to 5-10 FPS — a visually choppy result on a 240-column terminal.

**Why it happens:**
On Windows, crossterm can fall back to WinAPI calls (`SetConsoleTextAttribute`) per output unit when VT processing is not enabled. Even when VT is enabled, flushing after each character causes a system call per character rather than per frame. This is documented in crossterm issues and the `sophia.rs` "porting pikachu" post.

**How to avoid:**
Batch all output for an entire frame into a `Vec<u8>` buffer, then write the entire buffer in a single `stdout.write_all(&buf)` + `stdout.flush()` call. Use `crossterm::queue!` (which buffers) rather than `crossterm::execute!` (which flushes immediately). Enable VT processing explicitly at startup with `crossterm::terminal::enable_ansi_support()` or `output_vt100::ensure_ansi_support()`. Target 30 FPS for `matrix` — cap with `std::thread::sleep` to the remainder of the frame budget.

**Warning signs:**
- `matrix` flickers or shows partial frames
- CPU usage very high but FPS is low
- Output looks smooth in Windows Terminal but choppy in conhost.exe

**Phase to address:**
`matrix` and `lolcat` — both require frame-level stdout buffering.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use `std::fs::canonicalize` directly | Simpler code | `\\?\` prefix breaks path concatenation, display, and child processes everywhere it's used | Never — wrap in `dunce::canonicalize` from day one |
| Skip `is_terminal` check, always emit ANSI | Simpler per-command code | Piped output to files/tools is garbage | Never — wrap once in a project-level color utility |
| Hardcode wttr.in URL for `weather` | Easiest weather API, no key | Single-developer project, no SLA, rate-limits behind shared IPs (corporate NAT) | Acceptable for v1 personal use; add fallback for sharing |
| Use `crossterm::execute!` per-line in `matrix` | Works in early testing | Janky FPS on larger terminals | Never once past prototype stage |
| Skip timestamp preservation in `flatten` | Fewer dependencies | Destroys file metadata silently; user discovers months later | Acceptable in prototype; must fix before sharing |
| Ignore reserved filename check in `flatten` | Saves 10 lines of code | Silent data loss when source contains `con`, `nul`, `prn`, etc. | Never — validation is cheap |
| Assume PATH updated in current session after install | Simpler install script | Every new user's first experience is "command not found" | Never — always refresh `$env:PATH` at end of install |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|-----------------|
| `arboard` clipboard | Create Clipboard on a background thread or in async task | Keep all clipboard operations on main thread; `arboard` is not thread-safe on Windows |
| `winrt-notification` toast | Expect custom app branding in notification | Accept PowerShell AUMID fallback for v1; toast will show "Windows PowerShell" as source |
| `reqwest` + weather API | No timeout set, hangs indefinitely on bad network | `Client::builder().timeout(Duration::from_secs(5)).connect_timeout(Duration::from_secs(3))` |
| `open-meteo` API | Assumes always available, no offline handling | Check `Error::is_connect()` / `is_timeout()`, emit clear "No network connection" message and exit 1 |
| `walkdir` on Windows junction points | `follow_links(true)` by default | Default to `follow_links(false)`; add `--follow-links` flag with documented caveats |
| `qrcode` crate unicode renderer | One module per cell → rectangular output | Use `qrcode::render::unicode` with `dense1x2` or use `qr2term` crate for correct half-block rendering |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Hashing every file for `dupes` without pre-filtering by size | `dupes` is extremely slow on large trees | First group files by size (cheap metadata), then hash only groups with >1 member | Trees with >10,000 files |
| Reading full file content to compare in `dupes` instead of hashing | Memory pressure on files > RAM | Hash in streaming chunks (e.g., 64KB reads with `sha2` digest) | Any file > ~500MB |
| Using `walkdir` sequentially and hashing sequentially in `dupes` | 1 CPU core used, slow on multi-core machines with fast SSDs | Use `rayon::par_iter()` over the size-grouped file lists for parallel hashing | Trees > 50,000 files; modern NVMe makes I/O fast enough that CPU becomes the bottleneck |
| `tree` / `du` collecting all entries before printing | `tree` hangs for several seconds on large trees before any output | Stream output: print as you walk rather than collect-then-print | Trees > 100,000 entries |
| `matrix` flushing stdout per character | 5-10 FPS on wide terminals | Buffer entire frame, flush once per frame | Any terminal wider than ~80 columns |
| `lolcat` processing ANSI input naively | Corrupts existing ANSI color codes in input that already has escapes | Strip existing ANSI codes before applying rainbow, or use `strip-ansi-escapes` crate | Any piped input that already contains ANSI colors |
| Collecting all directory entries in memory for `flatten` before copying | OOM on trees with millions of files | Stream: walk and copy one file at a time without collecting | Trees > ~500,000 files |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing `weather` API key in source or binary | Key leaked in public repo or decompiled binary | Read from env var (`OPENWEATHERMAP_API_KEY`) or config file in `%APPDATA%`; document the setup |
| `flatten` silently overwriting destination files if collision logic has bugs | Data loss with no recovery | Always dry-run-preview the collision strategy in tests; never use `fs::copy` with an existing destination without explicit `--overwrite` flag |
| `bulk-rename` applying regex to filenames without bounds checking | A bad regex that matches everything renames all files to the same name | Validate that no two renames produce the same destination filename before executing any rename |
| `clip --set` reading from stdin with no size limit | Malicious pipe fills clipboard (and memory) with gigabytes of data | Cap stdin read at a reasonable limit (e.g., 10MB) and error clearly if exceeded |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `flatten` defaults to overwrite on collision | Silent data loss — the user may not notice until much later | Default to the path-encoding collision strategy; require explicit `--overwrite` to replace |
| `dupes` only reports, never acts | User finds duplicates but has to manually delete | Add `--delete` flag (interactive confirmation) and `--delete-all` for scripted use |
| `bulk-rename` executes immediately without preview | User regrets irreversible renames | Default to dry-run (`--dry-run` is the default); require `--execute` to actually rename |
| `weather` hangs for 30 seconds on bad network | Terminal appears frozen | Set 5-second total timeout; emit "Fetching weather..." immediately so user knows it's working |
| `matrix` and `lolcat` don't restore terminal on Ctrl-C | Terminal left in raw mode or wrong color state after interrupt | Install `ctrlc` crate handler that calls `crossterm::terminal::disable_raw_mode()` and resets color |
| All commands print noisy errors to stdout | Breaks `box tree | grep pattern` pipelines | All error messages go to stderr; only useful output goes to stdout |
| `pomodoro` blocks the terminal for the entire timer | User cannot use their terminal while timer runs | Run timer in background and print "Timer started. Toast notification will fire in 25m." |

---

## "Looks Done But Isn't" Checklist

- [ ] **`flatten`:** Verify it handles output-inside-input guard — test `box flatten ./test ./test/out`
- [ ] **`flatten`:** Verify timestamps are preserved — check `mtime` of copied files matches source
- [ ] **`flatten`:** Verify reserved filename check — create a source file named `con.txt` and flatten it
- [ ] **`dupes`:** Verify content-hash comparison, not path-string comparison — create two files with different names but identical content
- [ ] **`qr`:** Verify QR is square and scannable with a real phone — not just visually correct in terminal
- [ ] **`clip`:** Verify content persists after process exit — paste with Win+V after `box clip` exits
- [ ] **`pomodoro`:** Verify toast notification actually appears — run on a machine that has never run the binary
- [ ] **`matrix`:** Verify terminal is restored after Ctrl-C — check cursor visible and colors reset
- [ ] **`weather`:** Verify graceful failure — disconnect network and confirm error message (not hang)
- [ ] **`install.ps1`:** Verify `box --help` works in the *same* session that ran install — without opening new terminal
- [ ] **`tree`/`du`/`flatten`:** Verify 300+ character path handling — synthetic deep path in CI test
- [ ] **All commands:** Verify piped output has no ANSI codes — `box tree | cat` should be clean text

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| `std::fs::canonicalize` used throughout codebase | MEDIUM | Search-replace with `dunce::canonicalize`; run tests to find `\\?\` leaking into paths |
| ANSI codes emitted unconditionally | LOW | Introduce project-wide color utility wrapper; replace direct ANSI calls in each command |
| `flatten` data loss (overwrite without collision detection) | HIGH | No automated recovery — user must restore from backup. Prevention is the only answer. |
| `matrix` choppy FPS due to per-character flush | LOW | Refactor render loop to buffer frames; two-hour fix once the pattern is understood |
| PATH not updated in current session after install | LOW | User opens new terminal; issue is cosmetic. Fix install script to refresh `$env:PATH`. |
| Toast notifications appear from "PowerShell" | LOW | Acceptable for v1; register custom AUMID in install if attribution becomes important |
| Long paths silently failing | MEDIUM | Add `verbatim` prefix to all I/O paths in file traversal commands; test with synthetic deep paths |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| `canonicalize` returns `\\?\` UNC path | Foundation — wrap in `dunce::canonicalize` utility | No path containing `\\?\` appears in user-visible output |
| 260-char path limit | `flatten`/`dupes`/`tree`/`du` implementation | CI test with 300-char synthetic path succeeds |
| Output dir inside input dir in `flatten` | `flatten` command — first validation | `box flatten ./x ./x/out` prints error and exits 1 |
| NTFS case-insensitive collision in `flatten` | `flatten` command — collision key normalization | Two files `A.txt` and `a.txt` both appear in output with distinct encoded names |
| Symlink/junction loops | All traversal commands | `box tree` on a dir containing a junction loop exits without hanging |
| ANSI codes in piped output | Foundation — color utility | `box tree \| Out-File t.txt` produces clean text in `t.txt` |
| QR code aspect ratio | `qr` command | QR scanned successfully with phone on first attempt |
| Clipboard threading | `clip` command | `clip --set "test"` content visible in clipboard manager after exit |
| Toast AUMID | `pomodoro` command | Toast notification appears within 1 second of timer completion |
| PATH not refreshed after install | `install.ps1` | `box --help` succeeds in the same session that ran `install.ps1` |
| Execution policy blocks install | README / `install.ps1` | README documents bypass command; tested on fresh Windows user |
| SmartScreen blocks binary | README / distribution | README documents "More info → Run anyway" path |
| Timestamps not preserved by `fs::copy` | `flatten` command | `mtime` of flattened file matches source file |
| Reserved filename in collision encoding | `flatten` command | Source file named `con.txt` flatten without error; destination is `con_.txt` |
| `matrix` per-character flush | `matrix` command | 80x24 terminal renders at ≥30 FPS; measured with frame timer |
| Large tree performance | `dupes`/`flatten`/`tree`/`du` | Commands on 100,000-file tree complete in reasonable time (<60s) |

---

## Sources

- [rust-lang/rust#42869 — canonicalize returns UNC paths on Windows](https://github.com/rust-lang/rust/issues/42869)
- [rust-lang/rust#76586 — fs::write fails on long paths](https://github.com/rust-lang/rust/issues/76586)
- [rust-lang/rust#67403 — fs::copy has 260 character limit](https://github.com/rust-lang/rust/issues/67403)
- [gal.hagever.com — Using Long Paths in Windows and Rust](https://gal.hagever.com/posts/windows-long-paths-in-rust)
- [Microsoft Learn — Naming Files, Paths, and Namespaces (reserved names)](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)
- [rust-lang/rust#66260 — Path::starts_with is case-sensitive on Windows](https://github.com/rust-lang/rust/issues/66260)
- [arboard docs — clipboard threading warning](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html)
- [winrt-notification — POWERSHELL_APP_ID fallback](https://docs.rs/winrt-notification)
- [microsoft/terminal Discussion #13971 — ANSI detection in PowerShell](https://github.com/microsoft/terminal/discussions/13971)
- [Rain's Rust CLI recommendations — colors](https://rust-cli-recommendations.sunshowers.io/colors.html)
- [DEV Community — QR half-block rendering in Rust](https://dev.to/sendotltd/a-150-line-rust-cli-that-renders-qr-codes-in-your-terminal-half-block-characters-pack-two-modules-1jei)
- [rust-lang/rust#56815 — SmartScreen triggers on rustup-init.exe](https://github.com/rust-lang/rust/issues/56815)
- [rust-lang/rust #32067 — fs::copy timestamps on Unix](https://github.com/rust-lang/rust/pull/32067)
- [sophia.rs — Porting pikachu, crossterm Windows per-character performance](https://www.sophiajt.com/porting-the-pikachu/)
- [Open-Meteo — Free weather API](https://open-meteo.com/)
- [BurntSushi/walkdir](https://github.com/BurntSushi/walkdir)
- [RFC 1721 — crt-static for Windows MSVC](https://rust-lang.github.io/rfcs/1721-crt-static.html)
- [internals.rust-lang.org — rename without overriding / TOCTOU](https://internals.rust-lang.org/t/rename-file-without-overriding-existing-target/17637)
- [orhun.dev — stdout vs stderr buffering](https://blog.orhun.dev/stdout-vs-stderr/)

---
*Pitfalls research for: Rust Windows PowerShell 7 CLI toolbox (`box`)*
*Researched: 2026-06-22*
