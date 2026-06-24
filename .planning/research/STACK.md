# Stack Research — box v2.0 "Toolbox → Toolkit"

**Domain:** Windows-PowerShell-7 Rust CLI toolkit (single static binary, subsequent milestone — deepening an existing 23-command app)
**Researched:** 2026-06-24
**Confidence:** HIGH (versions verified against crates.io API + Context7 + docs.rs on 2026-06-24)

> Scope note: This file covers **only the NEW v2 stack additions/changes**. The settled v1.0 stack (clap 4.6, anyhow/thiserror, owo-colors/crossterm 0.29, serde_json preserve_order, arboard, tauri-winrt-notification, ureq 3.x, walkdir/ignore/rayon, blake3/sha2/md-5, image 0.25, qrcode 0.14, chrono, uuid v4, base64, passwords/rand 0.9) is ground-truth in `Cargo.toml` and is NOT re-researched here.

## TL;DR — What v2 Actually Needs

The v2 feature set is **deliberately crate-light**. Three of the four headline capabilities need **zero or near-zero new dependencies**:

- **`--json`** → no new crate (just `#[derive(Serialize)]` + the already-present `serde` derive + `serde_json`).
- **`--clip`** → no new crate (reuse the already-present `arboard`).
- **per-command depth** → almost entirely std + crates already in the manifest. Only **one genuinely-new functional crate** is required if you want on-disk/compressed size + a completion beep: the Microsoft **`windows`** crate (and even that is optional — see decision table).

Only **two genuinely new crates** are recommended outright:

| New crate | Version | For | Mandatory? |
|-----------|---------|-----|------------|
| `clap_complete` | `4.6.5` | `box completions powershell` | Yes (no other sane way) |
| `windows` | `0.61` | pomodoro completion beep (`MessageBeep`) + du on-disk size (`GetCompressedFileSizeW`) | Only if those two specific features ship |

Plus **one new crate for the chosen config approach** (`config` 0.15) **OR** a 2-crate hand-roll (`toml` + `dirs`) — see the config decision section; this is the one real "pick a lane" call in v2.

Plus **two feature-flag-only edits** to existing deps: `uuid` gains `"v7"`, and `indicatif` is finally added (it was named in CLAUDE.md but never pulled in v1).

---

## Recommended Stack (new in v2)

### Core New Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `clap_complete` | `4.6.5` | Generate a static PowerShell completion script for `box completions powershell` | The official clap companion crate; tracks clap's version line exactly (4.6.x ↔ clap 4.6). Integrates with the existing **derive** `Cli` via `CommandFactory`: `<Cli as CommandFactory>::command()` hands the already-built `clap::Command` to `generate(Shell::PowerShell, &mut cmd, "box", &mut io::stdout())`. No second source-of-truth — the completion script is derived from the exact same arg tree as `--help`. Verified latest 4.6.5 (2026-05-11) on crates.io. |
| `config` | `0.15.24` | Layered config file + env defaults for `box config` | **Recommended over a hand-roll** (rationale below). Maintained by `epage` (the clap maintainer — same ecosystem, same release cadence). Native `flag > env > file > default` precedence via ordered `add_source`, native `File::...required(false)` for "missing file = defaults not an error", native `Environment` source, native `set_default`, native TOML. Verified 0.15.24 (2026-06-16). |
| `windows` | `0.61` | (a) pomodoro completion sound via `MessageBeep`; (b) du true on-disk/compressed size via `GetCompressedFileSizeW` | The Microsoft-official Win32 binding. **Pin to 0.61, NOT 0.62** — `tauri-winrt-notification 0.7.2` already pulls `windows ^0.61` transitively; matching that version lets Cargo unify to a single compiled `windows` build instead of two (smaller binary, faster compile). Only the two needed feature modules are enabled (see install). Latest is 0.62.2 (2025-10-06) but 0.61 is the right pin for dedup. |

### Feature-Flag-Only Changes to EXISTING Crates (NOT new crates)

| Crate (already present) | Change | For | Notes |
|-------------------------|--------|-----|-------|
| `uuid` `1.23.3` | add feature `"v7"` → `features = ["v4", "v7"]` | `box uuid --v7` (time-ordered UUIDs) | `Uuid::now_v7()` is gated on `std` + `v7`; `v4` already auto-enables `std`. **Formatting flags (`--upper`, hyphenated/simple/braced/urn) need NO feature** — `.hyphenated()`/`.simple()`/`.braced()`/`.urn()` are always-available `const fn`. So uuid format flags = pure code, v7 = one feature flag. |
| `serde` `1` (already `derive`) | none — already has `derive` | `--json` per-command output structs | The v1 manifest already pulls `serde = { version = "1", features = ["derive"] }` (added for weather). `--json` reuses it verbatim. |
| `serde_json` `1.0.150` (already `preserve_order`) | none | `--json` serialization + `json --sort-keys` | `--sort-keys` is `serde_json::to_string_pretty` over a `BTreeMap`-ordered value, or toggling off `preserve_order` ordering for that one path — **no new crate, no feature change**. |
| `arboard` `3.6.1` (already present) | none | `--clip` on every applicable command | The cross-cutting `--clip` spine is exactly v1's `clip` command infra (`Clipboard::new()?.set_text()`), lifted into `core::output`. **Do NOT add a second clipboard crate.** |
| `ignore` `0.4` (already present) | none | `tree` `.gitignore` respect, dirs-only, ignore-pattern | `tree`'s new `--gitignore` walks with `ignore::WalkBuilder` instead of `walkdir`. Already in the v1 manifest (used by `dupes`). **Do NOT add a new gitignore crate.** |
| `crossterm` `0.29` (already present) | none | `lolcat --animate`, `matrix` color/speed/charset | Animation = the existing crossterm raw-mode + cursor/clear primitives already proven by v1's `matrix`. **No new animation/TUI crate.** |
| `rayon` / `blake3` / `walkdir` (already present) | none | dupes multi-stage hashing, hash multi-file, flatten progress | The multi-stage prefix/suffix/full hash is an algorithm change over existing crates, not a dependency change. |

### Supporting / Conditional New Crates

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `indicatif` | `0.18.4` | Progress bars/spinners for flatten / hash (large file) / dupes (N-file scan) | Named in CLAUDE.md's stack but **never actually pulled in v1** — this is its first real introduction. Standard Rust CLI progress crate, renders in place, Windows-terminal-correct. **CRITICAL constraint below.** Add only when the progress UX requirements land. |
| `dirs` | `6.0.0` | Resolve `%APPDATA%` (`config_dir()`) for the config file location | **Needed under BOTH config approaches** — the `config` crate does not resolve the OS config dir for you, so even with `config` you need *something* to find `%APPDATA%`. `dirs::config_dir()` returns `C:\Users\<u>\AppData\Roaming` on Windows — the correct base for `box\config.toml`. 4-fn crate, tiny, the de-facto standard. (`etcetera 0.11.0` is the unopinionated alternative; `dirs` is more idiomatic for "give me %APPDATA%".) |
| `toml` | `1.1.2` | Parse the config file (hand-roll path only) | **Only if you reject the `config` crate** and hand-roll a `Config` struct with `#[derive(Deserialize)]`. Latest 1.1.2 (2026-04-01). Not needed if you adopt the `config` crate (it bundles TOML parsing internally). |

### Development Tools

No new dev-dependencies required. The existing `assert_cmd` / `trycmd` / `insta` / `assert_fs` set covers v2:
- `--json` output → snapshot via **`insta`** (`assert_json_snapshot!`) — already present, ideal for locking the machine-readable contract.
- `completions powershell` script → snapshot the generated script via **`insta`** or assert non-empty via **`assert_cmd`**.
- config precedence → drive with **`assert_fs`** temp config files + env vars.

---

## The One Real Decision: Config Approach

**Recommendation: adopt the `config` crate (0.15.24) + `dirs` (6.0.0) for the `%APPDATA%` lookup.**

The requirement is a 4-layer precedence merge — **flag > env > config-file > built-in default** — with "missing file = use defaults, not an error", on Windows under `%APPDATA%`. Comparison:

| Approach | Precedence merge | Missing-file = defaults | Env source | %APPDATA% | New crates | Verdict |
|----------|------------------|-------------------------|------------|-----------|------------|---------|
| **`config` 0.15** | Native — ordered `add_source`, last wins | Native — `File::from(path).required(false)` | Native — `Environment::with_prefix("BOX")` | needs `dirs` to find it | `config` + `dirs` (2) | **Recommended** |
| `figment` 0.10.19 | Native — `Figment::new().merge().join()` | Native — optional providers | Native — `Env` provider | needs `dirs` | `figment` + `dirs` (2) | Viable, but **stale** (last release 2024-05-17) and Rocket-flavored API; `config` is fresher (2026-06) and same ecosystem as clap |
| Hand-roll `toml` + `Config` struct | **You write the merge** by hand (deserialize → overlay Option fields → fold flags) | You handle `ErrorKind::NotFound → Default::default()` | You read env per field | needs `dirs` | `toml` + `dirs` (2) | More control, but you re-implement precedence + the "missing = default" branch yourself; bug surface for a solved problem |

**Why `config` over the hand-roll:** the precedence-merge + optional-file + env-overlay is *exactly* what `config` exists to do, and it does it with `required(false)` (verified on docs.rs — a missing file is skipped, build still succeeds) and ordered sources (last `add_source` wins). The hand-roll isn't hard, but it re-implements a solved, tested merge and the "missing file → defaults" branch by hand — net more code and more test burden for an identical result. `config` is also maintained by the clap maintainer, so it shares release cadence and quality bar with the CLI core already in the binary.

**Why `config` over `figment`:** `figment` works and has a nice API, but its last release was 2024-05-17 vs `config`'s 2026-06-16 — for a 1-year-stale dep with no functional advantage here, prefer the actively-maintained, same-ecosystem option.

**The precedence wiring (sketch):**
```text
defaults  : ConfigBuilder.set_default("hash.algo", "blake3")?                          // built-in
file      : .add_source(File::from(appdata.join("box/config.toml")).required(false))   // missing = skip
env       : .add_source(Environment::with_prefix("BOX").separator("_"))                // BOX_HASH_ALGO
flags     : applied LAST in code — clap value, if Some, overrides the merged config
```
clap flags stay the top layer in *application* code (a `Some(flag)` overrides the resolved config value) because clap can't cleanly distinguish "user passed the default value" from "user passed nothing" without `Option<T>`/`ArgAction` plumbing — so the clean pattern is: `config` resolves layers 1–3, your command code folds the flag on top.

**If you choose to hand-roll anyway** (legitimate if you want zero merge magic and full control): `toml 1.1.2` + `dirs 6.0.0`, a `#[serde(default)] Config` struct of `Option<T>` fields, `fs::read_to_string` → `io::ErrorKind::NotFound => Config::default()`, and a manual `overlay()` that folds file→env→flag. Reuses the serde derive already present.

---

## Per-Command Depth — Crate Decisions

For each v2 depth feature, the question was "does this need a NEW crate or windows API?" Answers:

| Feature | New crate/API? | Decision |
|---------|----------------|----------|
| **dupes hardlink awareness** | **NO new crate** | `std::os::windows::fs::MetadataExt::number_of_links() -> Option<u32>` exposes the Win32 `BY_HANDLE_FILE_INFORMATION` link count. **Gotcha (verified):** it returns `None` for metadata obtained via `DirEntry::metadata()` — you must use `fs::metadata(path)` / `File::metadata()` to get `Some(n)`. Dedup the hardlinked file set by `(volume_serial_number(), file_index())` (also on `MetadataExt`) so identical-content hardlinks aren't double-counted as wasted space. Pure std. |
| **du apparent vs on-disk (compressed/sparse) size** | **`windows` 0.61** (or skip) | `metadata().len()` is the *apparent* size (free, std). True **on-disk** size needs `GetCompressedFileSizeW` (compression + sparse + cluster rounding). The maintained path is the **`windows` crate** (`Win32::Storage::FileSystem::GetCompressedFileSizeW`). **Do NOT use the `filesize` crate** — it wraps the same API but is **unmaintained (last release 2020-03-19)**. If on-disk size is judged low-value, ship apparent-size only and add **no** crate. |
| **ascii braille rendering** | **NO new crate** | Reuse the existing `image 0.25` (`open → resize → to_luma8`). Braille is a fixed 2×4 dot→bit lookup (`U+2800` + the 8-dot bitmask) over a thresholded luma grid — a ~20-line hand-roll. This matches the manifest's explicit "image-decoding hand-roll exception" precedent (the comment already rejects pulling extra image-pipeline crates). The braille crates found (`make_it_braille`, `braille_pics`, `braille-ascii`) are niche/low-adoption — not worth a dep for a trivial bitmask. |
| **pomodoro completion sound** | **`windows` 0.61** (or none) | The right-sized choice is `MessageBeep(MB_OK)` from `Win32::System::Diagnostics::Debug` — the system notification sound, one FFI call, no audio stack. **Do NOT pull `rodio`/`cpal`/`rusty_audio`** — they drag a full `cpal` audio-device stack (large, slow, overkill for a single completion chime). If a beep isn't worth a Win32 dep, the toast (already shipped via `tauri-winrt-notification`) plays its own notification sound — so "no sound crate at all" is a defensible option. |
| **lolcat `--animate` / matrix color/speed/charset** | **NO new crate** | Reuse `crossterm 0.29` (raw mode, cursor, clear, key-poll) — the exact primitives v1's `matrix` already uses. No new animation/TUI dep. |
| **tree `.gitignore` respect** | **NO new crate** | Reuse `ignore 0.4` (already in v1 for `dupes`). Swap `tree`'s walk to `ignore::WalkBuilder` when `--gitignore` is set. |
| **flatten/hash/dupes progress bars** | **`indicatif` 0.18.4** | See critical constraint below. |

### `windows` crate — install footprint

If you ship **both** the pomodoro beep and du on-disk size, one direct `windows` dep covers both with two feature modules:
```toml
windows = { version = "0.61", features = [
    "Win32_System_Diagnostics_Debug",   # MessageBeep (pomodoro)
    "Win32_Storage_FileSystem",         # GetCompressedFileSizeW (du)
] }
```
Version `0.61` (not 0.62.2) so it **unifies with the `windows ^0.61` already pulled by `tauri-winrt-notification 0.7.2`** — Cargo compiles one `windows`, not two. (Verified: tauri-winrt-notification 0.7.2 depends on `windows ^0.61` with `Win32_Foundation`, `UI_Notifications`, etc.)

---

## CRITICAL Constraint — Progress Output MUST NOT Contaminate stdout/`--json`

`indicatif` is the first crate that **writes to the terminal as a side effect**, and the v2 `--json` spine makes stdout a **machine-readable contract**. The hard rules for the roadmap:

1. **Progress bars draw to `stderr`, never stdout.** Construct via `ProgressBar::new(...)` and set `ProgressDrawTarget::stderr()` explicitly (do not rely on defaults). stdout must carry *only* command output / JSON.
2. **`--json` mode must suppress progress entirely** (or keep it strictly on stderr) so `box hash --json | ConvertFrom-Json` is never corrupted by bar-redraw bytes. Safest contract: **`--json` ⇒ `ProgressDrawTarget::hidden()`**.
3. **Non-TTY stderr ⇒ no progress** (`std::io::IsTerminal` on stderr), matching v1's existing piped-vs-TTY gating discipline (the lolcat/color/json color gate already establishes this pattern).

This is a *design* constraint, not a crate choice — flag it loudly for the flatten/hash/dupes phase plans. The same stdout-purity rule applies to **every** command gaining `--json`: warnings, progress, and prompts go to stderr; stdout is the data plane.

---

## Installation (v2 additions only)

```toml
# --- New crates ---
# Shell completions (tracks clap 4.6.x). Static PowerShell script generation.
clap_complete = "4.6.5"

# Config-file defaults: layered flag>env>file>default merge, optional file = no error.
config = { version = "0.15.24", default-features = false, features = ["toml"] }
# %APPDATA% resolution for the config file location (config crate doesn't do this).
dirs = "6.0.0"

# Progress bars — flatten/hash/dupes. MUST draw to stderr; hidden under --json.
indicatif = "0.18.4"

# Win32: MessageBeep (pomodoro sound) + GetCompressedFileSizeW (du on-disk size).
# Pin 0.61 to unify with tauri-winrt-notification's transitive windows ^0.61.
windows = { version = "0.61", features = [
    "Win32_System_Diagnostics_Debug",
    "Win32_Storage_FileSystem",
] }

# --- Feature-flag edits to EXISTING deps (NOT new crates) ---
# uuid: add "v7" for Uuid::now_v7() (was features = ["v4"]).
uuid = { version = "1.23.3", features = ["v4", "v7"] }
```

`default-features = false, features = ["toml"]` on `config` trims its JSON/YAML/INI/RON parsers — you only need TOML, matching the lean-bundle discipline the v1 manifest already follows on `image`/`qrcode`/`arboard`.

**Hand-roll config alternative (if `config` is rejected):**
```toml
toml = "1.1.2"
dirs = "6.0.0"
# (uuid/indicatif/windows/clap_complete unchanged)
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `config 0.15` (layered) | `figment 0.10.19` | If you already use Rocket/figment idioms elsewhere — but it's 1yr stale; no reason here |
| `config 0.15` (layered) | hand-roll `toml` + `dirs` | If you want zero merge "magic" and full control over precedence — costs you re-implementing the solved merge + missing-file branch |
| `clap_complete` **static** generation (`generate` + `Shell::PowerShell`) | `clap_complete` **dynamic** (`unstable-dynamic` / `CompleteEnv`) | Only if you need runtime context-aware value completion — but **PowerShell native/dynamic support is still incomplete** (clap issue #3918) and the feature is **unstable**. Static script generation is stable, PS7-proven, and sufficient for completing subcommands + flags. **Use static.** |
| `windows 0.61` (`MessageBeep`) | `rodio` / `cpal` / `rusty_audio` | Only if pomodoro needs to play an actual custom WAV/sound file — not the case for a completion chime |
| `windows 0.61` (`GetCompressedFileSizeW`) | `filesize 0.2.0` crate | Never — `filesize` wraps the same API but is unmaintained since 2020; call the API directly |
| hand-rolled braille (`image` + bitmask) | `make_it_braille` / `braille_pics` | If you later need dithering algorithms (Sierra2Row etc.) for high-fidelity braille art — overkill for `--braille` |
| `dirs 6.0.0` | `etcetera 0.11.0` / `directories 6` | `etcetera` if you want strictly-unopinionated base-dir logic; `dirs` is the more idiomatic "give me %APPDATA%" |

## What NOT to Use / NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| A second clipboard crate for `--clip` | `arboard 3.6.1` is already in the binary and proven (v1 `clip`) | Reuse `arboard` via `core::output` |
| A new JSON/serialize crate for `--json` | `serde` (derive) + `serde_json` (preserve_order) already present | `#[derive(Serialize)]` on output structs → existing `serde_json` |
| `filesize 0.2.0` | Unmaintained since 2020-03-19; wraps the same Win32 call | `windows` crate `GetCompressedFileSizeW` |
| `rodio` / `cpal` / `rusty_audio` | Full audio-device stack for a single completion beep — large + slow build | `windows` crate `MessageBeep`, or rely on the toast's own sound |
| `windows 0.62.2` as a direct dep | Would compile a **second** `windows` build alongside tauri-winrt-notification's `0.61` | Pin direct `windows = "0.61"` to unify |
| `clap_complete` `unstable-dynamic` for PS7 | Unstable feature; PowerShell dynamic completion support is incomplete (clap #3918) | Static `generate(Shell::PowerShell, …)` |
| A new gitignore-walking crate for `tree --gitignore` | `ignore 0.4` already in the manifest (used by `dupes`) | Reuse `ignore::WalkBuilder` |
| A TUI/animation crate (ratatui, etc.) for lolcat/matrix | `crossterm 0.29` already drives v1's `matrix` at ~20 FPS | Reuse `crossterm` primitives |
| `uuid` as a *new* dep for v7 | v7 is a **feature flag** on the existing `uuid 1.23.3`, not a new crate | Add `"v7"` to the existing `features` |
| Letting `indicatif` write to **stdout** | Corrupts `--json` / piped output (the v2 scriptable contract) | `ProgressDrawTarget::stderr()`; `hidden()` under `--json` |
| A new `fmt`/format crate for uuid `--upper`/braced/etc. | `.hyphenated()`/`.simple()`/`.braced()`/`.urn()` are always-available `const fn` | Existing `uuid`, no feature change |

## Stack Patterns by Variant

**If shipping the full du depth (on-disk + apparent size):**
- Add `windows 0.61` with `Win32_Storage_FileSystem`.
- Default `du` to apparent size (`metadata().len()`, std, cross-the-board); `--on-disk` flag switches to `GetCompressedFileSizeW`.

**If shipping the pomodoro completion sound:**
- Add (or extend) `windows 0.61` with `Win32_System_Diagnostics_Debug` → `MessageBeep`.
- If sound is judged low-value, **omit the crate** — the completion toast already plays a notification sound.

**If neither du-on-disk nor pomodoro-sound ships:**
- **No `windows` direct dep at all** — v2 then adds only `clap_complete` + the config crate(s) + `indicatif`, and two feature-flag edits. The leanest possible v2.

**If config is hand-rolled instead of `config`-crate:**
- Drop `config`, add `toml 1.1.2`; keep `dirs 6.0.0`; write the `Option<T>` overlay + `NotFound => default` branch by hand.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `clap_complete 4.6.5` | `clap 4.6` (present) | clap_complete tracks clap's minor line; 4.6.x ↔ clap 4.6 is the matched pair. Generated `Command` comes from the existing derive `Cli` via `CommandFactory` |
| `windows 0.61` (direct) | `tauri-winrt-notification 0.7.2` → `windows ^0.61` (transitive) | **Pin 0.61 to unify** — same semver line → one compiled `windows`. 0.62 would duplicate the build |
| `config 0.15.24` | `serde 1`, `toml 1` (present/internal) | Requires Rust 1.85 / edition 2024 internally — fine (project builds on current stable MSVC). With `default-features=false, features=["toml"]` it bundles its own TOML parser; no conflict with a separately-pinned `toml` |
| `uuid 1.23.3` + `"v7"` | existing `"v4"` | Both features coexist; `now_v7()` needs `std`+`v7`, `v4` already enables `std` |
| `indicatif 0.18.4` | `crossterm 0.29` (present) | indicatif uses its own terminal backend; no conflict. Draw target must be set to stderr explicitly |
| `dirs 6.0.0` | std-only | No transitive surprises; `config_dir()` → `%APPDATA%` (Roaming) on Windows |

## Integration Points (for REQUIREMENTS / roadmap)

- **`completions`**: new meta-command module `src/commands/completions/`. Calls `<Cli as clap::CommandFactory>::command()` → `clap_complete::generate(Shell::PowerShell, &mut cmd, "box", &mut io::stdout())`. Zero changes to the existing arg tree — the script is derived from the same `Cli` that already powers `--help`.
- **`config`**: new meta-command `src/commands/config/` (show/set/path) + a shared `core::config` module that resolves the merged config once at startup and threads defaults into command dispatch. Precedence resolved in `core::config` (layers 1–3 via the `config` crate), flags folded on top in each command.
- **`--json` / `--clip`**: cross-cutting, belong in **`core::output`** (the existing output module). Each applicable command gains a `Serialize` output struct; `core::output` chooses human vs `serde_json::to_string_pretty` and optionally routes the rendered string to `arboard`. Progress (indicatif) lives beside it, hard-gated to stderr / hidden-under-json.
- **`uuid v7`**: pure `box uuid` module change + the one-line `features` edit.
- **`windows` Win32 calls**: isolate `MessageBeep` in `src/commands/pomodoro/` and `GetCompressedFileSizeW` in `src/commands/du/` behind tiny safe wrappers (the v1 pattern for arboard/winrt — Windows FFI localized to the owning command module).

## Sources

- crates.io API — `clap_complete` **4.6.5** (2026-05-11), `indicatif` **0.18.4** (2026-02-14), `config` **0.15.24** (2026-06-16), `figment` **0.10.19** (2024-05-17, stale), `toml` **1.1.2** (2026-04-01), `windows` **0.62.2** (2025-10-06; 0.61 chosen for dedup), `dirs` **6.0.0** (2025-01-12), `etcetera` **0.11.0** (2025-10-28), `filesize` **0.2.0** (2020-03-19, unmaintained) — HIGH
- crates.io API — `tauri-winrt-notification/0.7.2/dependencies` confirms transitive `windows ^0.61` (drives the 0.61 pin) — HIGH
- Context7 `/websites/rs_clap` + `/clap-rs/clap` — `generate` + `Shell` + `ValueHint`; derive→`CommandFactory` integration — HIGH
- docs.rs `config::File` — `.required(false)` skips a missing file, build still succeeds; `ConfigBuilder` ordered `add_source` (last wins) + `set_default` + `Environment` + TOML — HIGH
- docs.rs `uuid::Uuid` — `now_v7()` gated on `std`+`v7`; `.hyphenated()/.simple()/.braced()/.urn()` always-available `const fn` — HIGH
- doc.rust-lang.org `std::os::windows::fs::MetadataExt` — `number_of_links()`/`file_index()`/`volume_serial_number()` return `Option`, `None` for `DirEntry::metadata()` (use `fs::metadata`) — HIGH
- microsoft.github.io windows-docs-rs — `MessageBeep` in `Win32::System::Diagnostics::Debug`; `GetCompressedFileSizeW`/`GetFileSizeEx` in `Win32::Storage::FileSystem`; feature flags `Win32_System_Diagnostics_Debug` / `Win32_Storage_FileSystem` — HIGH
- clap GitHub issues #3918 (PowerShell native-completion gap), #3166 (native-completion tracking), `clap_complete::env` docs — `unstable-dynamic`/`CompleteEnv` is unstable; static generation recommended for PS7 — MEDIUM (issue/discussion threads, dates 2024–2026)

---
*Stack research for: Windows-PS7 Rust CLI toolkit v2 (subsequent milestone — additive deps only)*
*Researched: 2026-06-24*
