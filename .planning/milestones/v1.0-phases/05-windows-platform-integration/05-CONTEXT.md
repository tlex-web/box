# Phase 5: Windows Platform Integration - Context

**Gathered:** 2026-06-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the **four highest-external-risk commands** on the existing `core` infrastructure — each touches an external API or a Windows-native surface, built last so 18 working commands de-risk the integrations:

- `qr` — render a scannable QR code for arg/stdin text/URL using Unicode half-blocks (QR-01)
- `clip` — pipe stdin → Windows clipboard; `--paste` reads clipboard → stdout; Unicode-correct, no elevation (CLIP-01)
- `pomodoro` — blocking in-place countdown with break/long-break modes, clean Ctrl+C cancel, Windows toast on completion (POMO-01)
- `weather` — city-name or lat/lon → temperature/conditions/wind/humidity via keyless Open-Meteo, metric/imperial, graceful offline error (WTHR-01)

Each command swaps its unit `Commands::*` variant in `src/cli.rs` for a real Args struct implementing `RunCommand`, removes its `not_implemented(...)` arm in `src/main.rs`, and ships with `assert_cmd` / `trycmd` tests. All reuse the Phase-1 core: `core::output` color gating (`is_color_on`/`init_color`, byte-identical-minus-ANSI when piped, D-09/D-10), `terminal_width()`, strict 0/1/2 exit codes, `data→stdout / messages→stderr`. `enable_ansi_support` is already called first in `main()`. `box` is binary-only → `cargo test --bin box`, NOT `--lib`.

**In scope:** QR-01, CLIP-01, POMO-01, WTHR-01 (4 requirements). New Cargo deps: `qrcode` (qr), `arboard` (clip), `tauri-winrt-notification` (pomodoro toast), `ureq` + `serde` derive (weather). `crossterm`/`owo-colors`/`rand`/`serde_json` already present.

**Not in scope (v2 / Out of Scope — see REQUIREMENTS.md):** QR `--save`-to-image / configurable error-correction (VIS-V2-01); QR decode/scan (Out of Scope); clip binary/image clipboard (Out of Scope — text only v1) and `--clip` cross-command integration (DEV-V2-01); pomodoro session counter / auto-break / sound / `--label` and weather `--forecast`/`--json`/cache/stored-location incl. per-user unit preference (SYS-V2-01); weather API-key requirement / radar / maps (Out of Scope).

**Standing todos RESOLVED by this discussion** (STATE.md § Todos): winrt-notification compile-spike → D-09 (verified compiles, chose maintained fork); pomodoro blocking-vs-non-blocking → D-07 (blocking raw-mode); weather default units → D-10 (metric).

</domain>

<decisions>
## Implementation Decisions

> Each area was researched by a dedicated advisor agent (calibration: full_maturity) and chosen by the user from a scored comparison table. All choices converge on the project's established **lean-binary** value, the **locked color-gating contract** (byte-identical-minus-ANSI when piped via `is_color_on()`), and **reuse of in-repo patterns** (the matrix `RawGuard` raw-mode loop).

### Shared / carry-forward (locked by Phases 1–4 — do NOT re-derive)
- **D-00:** Every command follows the established slice: swap the unit variant in `src/cli.rs` for a real `Args` struct + `RunCommand` impl, delete its `not_implemented(...)` arm in `src/main.rs`, all styled output through `core::output::is_color_on()` (no parallel color path / no `owo_colors::set_override`), `data→stdout` / `messages→stderr`, exit `0/1/2`, and ship `assert_cmd` (+ `trycmd` where snapshotable) tests. `box` is binary-only → unit tests run via `cargo test --bin box`.

### qr — rendering engine & output (QR-01)
- **D-01:** **Engine = `qrcode` 0.14.1 + its built-in `qrcode::render::unicode::Dense1x2` renderer**, NOT `qr2term` 0.3.3 (the CLAUDE.md pre-pick). ⚠️ This OVERRIDES CLAUDE.md for the same reason `ascii` overrode `artem`: `qr2term::print_qr` **always emits ANSI color** (verified in its `render.rs`: every cell written as `"▄".white().on_black()` via crossterm `Stylize`, no TTY / `is_color_on()` gate), so `box qr "…" > qr.txt` would write raw escape sequences and the polarity lives in the **background color, not the glyph** — both break the locked glyph-is-truth + byte-identical-minus-ANSI-when-piped invariant. `Dense1x2` renders pure Unicode half-blocks (`▀▄█`/space) with **no ANSI**, returns an **owned `String`** via `.build()` (route through the standard data→stdout path; reuse for `--file`), and exposes quiet-zone + polarity control.
- **D-02:** **Render config:** `.quiet_zone(true)` (full ISO-18004 4-module border vs qr2term's sub-spec width-2) and **explicit dark-on-light polarity** — dark modules as the "on" glyph so the printed glyphs carry contrast **independent of the (dark) PowerShell theme**; this is the #1 reason a terminal QR fails to scan. **`EcLevel::M`** (15% recovery, the qrcode default) is FIXED for v1 (an EC-level / `--save` flag is VIS-V2-01). Verify `Dense1x2`'s default polarity is dark-on-light during impl; flip `.dark_color`/`.light_color` only if a quick phone-scan test fails. API: `QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?.render::<Dense1x2>().quiet_zone(true).build()`.
- **D-03:** **Monochrome, no color path** (QR has no color). Input via `core::input` (positional arg / `-` stdin / `--file`). `box qr … > file.txt` yields clean half-block text.

### clip — mode, newline policy, persistence (CLIP-01)
- **D-04:** **Mode = `--paste` flag, copy-by-default.** No flag = read stdin → clipboard (the common path); `box clip --paste` = clipboard → stdout. Matches the CLIP-01 success criterion verbatim and is the lowest-surface option. **`clip` does NOT route through `core::input::read_input`** — that would inherit the no-arg-interactive-TTY → `BoxError::MissingInput`/exit-2 contract, which is wrong for a clipboard copy. Copy reads **raw stdin bytes directly** (`io::stdin().read_to_end`) so it controls UTF-8 validation + newline policy itself. (Rejected: stream auto-detect — collides with the `read_input` TTY contract and Windows `is_terminal()` is fragile under nested pipes/ptys; nested `copy`/`paste` subcommands — heavier than warranted for a 2-state v1 tool, and image clipboard is Out of Scope.)
- **D-05:** **Trailing-newline policy:** on **copy**, strip **at most one** trailing terminator (`\r\n`, then a lone `\n`) — never interior or multiple newlines; on **`--paste`**, emit clipboard text **byte-exact** (no added/stripped newline). Load-bearing fact: PowerShell 7 appends an implicit **CRLF** when piping a string to a native command (`"x" | box clip` sends `x\r\n`), so byte-verbatim copy would land a spurious trailing newline on the clipboard (the exact `clip.exe` friction). Mirrors `pbcopy`/`xclip`. Document the single-trim in `--help`; a future `--raw`/`--no-trim` opt-out is deferred.
- **D-06:** **Crate = `arboard` 3.6.1** (locked). **Windows persistence is SAFE** — `set_text` performs the immediate sequence `OpenClipboard → EmptyClipboard → SetClipboardData(CF_UNICODETEXT, GlobalAlloc'd handle) → CloseClipboard` inside one call; per Microsoft, after `SetClipboardData` succeeds **the system owns the handle**, so copied text survives process exit. The "contents drop when the last `Clipboard` is dropped" problem is **X11/Wayland-specific and does NOT apply on Windows** — no keep-alive / delayed-rendering message pump needed. The STATE.md "arboard main-thread only" pitfall is satisfied by the synchronous single-shot flow: **create the `Clipboard` at the point of use, do one op, return; never spawn the arboard call onto a worker thread.** arboard converts UTF-8 ↔ UTF-16 `CF_UNICODETEXT` correctly (the whole reason for bypassing `clip.exe`) and uses the Win32 API directly (compatible with `+crt-static`, no DLL/async runtime).

### pomodoro — timer model, flags, toast (POMO-01)
- **D-07:** **Timer model = blocking foreground raw-mode loop, reusing the matrix `RawGuard` pattern.** `crossterm` raw mode; **`event::poll(remaining_tick)` is BOTH the ~1s countdown timer AND the keypress reader** (single-threaded, no background thread, no SIGINT race). It deterministically distinguishes **CANCEL** (Ctrl+C delivered as a `KeyEvent` `Char('c')+CONTROL` in raw mode — NOT SIGINT — or `Esc`/`q` → "cancelled" to stderr + exit 1, **NO toast**) from **COMPLETION** (loop falls through → fire toast + exit 0). RAII `Drop` guard restores cursor + disables raw mode on every path incl. panic; filter `KeyEventKind::Press` only (Windows press+release double-fire). In-place countdown redraw via a single clean write per tick; reuse `terminal_width()` and stay pipe-safe. (Rejected: `\r` line-rewrite + default Ctrl+C — SIGINT kills mid-line leaving a dangling partial line + parked cursor, can't listen for `Esc`, and a clean cancel would need a new `ctrlc` dep — undercuts the binding "cancels cleanly" clause.)
- **D-08:** **Flags / durations:** `box pomodoro [MINUTES]` defaults to **25** (work); `--break` → **5**; `--long-break` → **15** (standard Pomodoro Technique, matches the ROADMAP example). A positional minutes override (e.g. `box pomodoro 50`) is in scope; session counter / auto-break / sound / `--label` are SYS-V2-01 (out).
- **D-09:** **Toast crate = `tauri-winrt-notification` 0.7.2** — the **actively maintained drop-in fork** of CLAUDE.md's `winrt-notification`. API is identical: `Toast::new(Toast::POWERSHELL_APP_ID).title("Pomodoro").text1("Break time!").show()` — **`POWERSHELL_APP_ID` means no AUMID registration** (works from a CLI). ⚠️ The standing compile-spike is **RESOLVED empirically**: both `tauri-winrt-notification` 0.7.2 and `winrt-notification` 0.5.1 build clean on rustc 1.90 / `x86_64-pc-windows-msvc` — the real axis is maintenance, so the fork (modern `windows 0.61`, ~1.75M recent downloads) wins over the stale crate (abandoned `windows 0.24` + dead `syn 1.x`). **Fallback ladder** (all hit the same `ToastNotificationManager` API → swap is localized to the pomodoro module): → `winrt-notification` 0.5.1 (same API, verified compiles) → `winrt-toast-reborn` 0.3.8 (modern deps) → PowerShell `New-BurntToastNotification` shell-out (last resort: BurntToast needs `Install-Module`, PS7 Core dropped built-in WinRT). This supersedes CLAUDE.md's documented winrt-notification → winrt-toast fallback note.

### weather — units, location input, API flow (WTHR-01)
- **D-10:** **Default units = metric (°C, km/h)**; `--units metric|imperial` as a clap `ValueEnum` explicit override. Deterministic + testable (no env/locale reads — critical for snapshot-style output), global + SI majority (only US/LR/MM are imperial-first), and **Open-Meteo's own API default** so the no-flag path sends the shortest request with zero unit params. (Rejected: imperial default — wrong for ~95% of the world; OS-locale detect — non-deterministic across machines poisons tests, needs a Windows region dep, and the locale→unit map is just "metric + 3-country exception" anyway. Per-user unit preference belongs in v2 stored-config.)
- **D-11:** **Units applied SERVER-SIDE.** On `--units imperial` append `&temperature_unit=fahrenheit&wind_speed_unit=mph` to the forecast call (default path omits them). No client-side conversion (zero rounding bugs); read the returned `current_units` object for the **authoritative** unit-label suffix rather than hardcoding `°C`/`°F`.
- **D-12:** **Location input = parse-shape disambiguation** (deterministic, no wasted call). Treat the positional arg as `lat,lon` **only if** it matches `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$` with `lat ∈ [-90,90]`, `lon ∈ [-180,180]`; otherwise geocode as a city name. **Geocoding:** `GET https://geocoding-api.open-meteo.com/v1/search?name={city}&count=1&language=en&format=json` (keyless; URL-encode the name) → `results[0].{latitude, longitude, name, admin1, country}`. **Echo the resolved location to stderr** (e.g. `Resolved "London" → London, England, United Kingdom (51.51, -0.13)`) so a wrong match is visible. **Zero results** → `error: no location found for "{name}"` to stderr + exit 1.
- **D-13:** **Forecast call:** `GET https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m` (+ unit params on imperial, D-11). Field map: `temperature_2m`→temperature, `relative_humidity_2m`→humidity (%), `wind_speed_10m`→wind, `weather_code`→conditions via a **WMO `match` returning `&'static str`** (`0 => "Clear sky" … _ => "Unknown"` — no alloc, `_` arm future-proofs). **HTTP = `ureq` 3.3.0** (locked, blocking rustls); deserialize via `serde_json::from_reader(resp.into_body().into_reader())` into a `#[derive(Deserialize)]` struct — **avoid ureq's `json` feature** to stay lean (add `serde`'s `derive` feature; `serde_json` already present). **Errors → stderr + exit 1:** offline/DNS/connect → `error: could not reach weather service (offline?)`; non-2xx → `error: weather service returned {status}`. Exit 2 reserved for clap bad `--units`. Data → stdout, any color gated via `is_color_on()`.

### Claude's Discretion
- Module layout under `src/commands/<cmd>/mod.rs` per command; exact stdout layout/labels for `weather` (temp / conditions / wind / humidity lines) and `qr` (whether to print the input/URL alongside).
- `pomodoro`: exact countdown render format (`MM:SS`), whether to color the countdown digits (gated), exact phase/glyph styling within the single-flush-per-tick rule, and the toast title/body wording.
- `weather`: exact stderr wording within the locked shapes; whether to color conditions/temperature (gated); whether to also surface the geocoded `admin1`/`country` in the stdout header.
- `clip`: exact `--help` wording for the trailing-newline trim note.
- `qr`: the WMO-independent — choice of `with_error_correction_level` vs `new` (M is the same either way) and exact `--help` examples.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements & success criteria (the binding contract)
- `.planning/ROADMAP.md` § "Phase 5: Windows Platform Integration" — goal + the 4 success criteria (exact CLI behaviors: `qr "https://example.com"` scannable half-blocks; `clip` pipe-in + `--paste` read-out, no elevation; `pomodoro` live countdown + toast-on-complete + clean Ctrl+C + `--break`/`--long-break`; `weather "London"` temp/conditions/wind/humidity keyless + `--units imperial` + graceful offline error).
- `.planning/REQUIREMENTS.md` § "Terminal Visuals" (QR-01), § "Dev Utilities" (CLIP-01), § "Whimsy with Utility" (POMO-01, WTHR-01) acceptance criteria; § "v2 Requirements" (VIS-V2-01 QR `--save`/EC; SYS-V2-01 pomodoro session/auto-break/sound/`--label` + weather `--forecast`/`--json`/cache/stored-location; DEV-V2-01 `--clip` integration); § "Out of Scope" (qr decode/scan, clip binary/image, weather API-key/radar/maps).

### Architecture & locked patterns to reuse (do NOT re-derive)
- `.planning/STATE.md` § "Critical Pitfalls to Remember" (⚠️ **arboard clipboard must run on main thread only**; `enable_ansi_support` first in `main()`; matrix buffer-and-flush-once-per-frame ≈ the pomodoro loop discipline), § "Todos" (the 3 standing Phase-5 todos — **all resolved here**: D-07/D-08/D-09/D-10), § "Accumulated Context" `[04-04]` rows (the matrix `RawGuard` / poll-as-timer / `KeyEventKind::Press` decisions pomodoro reuses). `box` is binary-only → `cargo test --bin box`, not `--lib`.
- `.planning/phases/04-terminal-visuals/04-CONTEXT.md` — D-07..D-10 (matrix glyph/loop/teardown: the RAII `RawGuard`, `event::poll` as timer, Ctrl+C-as-KeyEvent, single-flush-per-frame) — **the direct reference for `pomodoro`**; the `is_color_on()`-gated pure-render template.
- `.planning/phases/01-foundation-flatten/01-CONTEXT.md` — D-09/D-10 (glyph-is-truth + `is_color_on()` gating, byte-identical-minus-ANSI when piped), the strict 0/1/2 exit policy.
- `.planning/phases/02-pure-transform-utilities/02-CONTEXT.md` — D-04/D-05 (`core::input` precedence + `read_input` shape, the `-` sentinel + `--file`) — used by `qr`; ⚠️ `clip` deliberately does NOT use `read_input` (D-04).

### Source files to read before implementing
- `src/cli.rs` — the 4 unit variants `Qr` / `Clip` / `Pomodoro` / `Weather` (lines ~85-92) to swap for real Args structs.
- `src/main.rs` — the 4 `not_implemented(...)` arms to remove; the exit-code policy + `BoxError` downcast mapping (exit 2 only for `MissingInput`/`UnsupportedHashLength`; everything else err→exit 1).
- `src/core/output.rs` — `is_color_on()` (`:32`), `init_color()` (`:52`), `terminal_width()` (`:215`), `human_size()`.
- `src/core/input.rs` — `read_input` / `read_file_or_stdin` + `-` sentinel + `--file` (`qr` input; `clip` reads raw stdin instead).
- `src/commands/matrix/mod.rs` — the `RawGuard` RAII teardown + `event::poll`-as-timer loop + `KeyEventKind::Press` filter — **the pattern `pomodoro` reuses**.
- `src/commands/color/mod.rs` — owo-colors `.truecolor(r,g,b)` `is_color_on()`-gated precedent (for any gated color in `pomodoro`/`weather`).

### Tech stack (locked crate versions + the two OVERRIDES)
- `CLAUDE.md` (project root) — `arboard` 3.6.1 (clip, NEW), `ureq` 3.3.0 (weather, NEW — rustls default) + `serde`/`serde_json` (serde_json already present; add `serde` `derive`), Open-Meteo API (keyless, chosen over OpenWeatherMap), the "What NOT to Use" table (reqwest+tokio rejected), `x86_64-pc-windows-msvc` + `+crt-static`. ⚠️ **Two overrides of CLAUDE.md crate picks:** (1) `qrcode` 0.14.1 + `Dense1x2` **replaces `qr2term`** (D-01); (2) `tauri-winrt-notification` 0.7.2 **replaces `winrt-notification` 0.5.1** (D-09) — both documented above with rationale.

### External APIs (no project ADRs/specs exist)
- Open-Meteo Forecast API — `https://open-meteo.com/en/docs` (current-weather params, unit overrides, the WMO `weather_code` → interpretation table). Keyless.
- Open-Meteo Geocoding API — `https://open-meteo.com/en/docs/geocoding-api` (`/v1/search?name=…`). Keyless.

**No external ADRs/specs — all decisions are captured above and in the files listed.**

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/matrix/mod.rs` — the `RawGuard` RAII terminal-restore + `event::poll(Duration)`-as-timer + `KeyEventKind::Press` filter loop; `pomodoro` reuses this family wholesale (the countdown is the only command after matrix to own raw-mode state).
- `core::output::{is_color_on, init_color, terminal_width}` + the startup owo-colors global override — any gated color in `pomodoro`/`weather` routes through this (no parallel color path).
- `core::input::{read_input, read_file_or_stdin}` + `-` sentinel + `--file` — `qr` input; ⚠️ `clip` deliberately reads raw stdin bytes instead (D-04).
- `owo_colors::OwoColorize::truecolor(r,g,b)` already wired + gated; `enable_ansi_support` already first in `main()`.
- `crossterm` 0.29, `owo-colors` 4.3, `rand` 0.9, `serde_json` 1.0.150 are ALREADY dependencies (pomodoro/weather consume them as-is).

### Established Patterns
- `RunCommand` trait + static dispatch: swap the `src/cli.rs` unit variant, remove the `not_implemented(...)` arm.
- Glyph-is-source-of-truth output, color as decoration, byte-identical-minus-ANSI when piped (D-09/D-10); `data→stdout`, `messages→stderr`, exit 0/1/2.
- **Hand-roll / pick the leaner crate when a convenience crate's cost outweighs its value** — D-01 (`qrcode` over `qr2term`) and D-09 (maintained fork over stale crate) continue the artem / colored_json / humansize precedent.
- Tests: integration via `assert_cmd` in `tests/<cmd>.rs`; `trycmd` snapshots where output is stable (qr, weather error paths). Terminal-bound animation/raw-mode loops are hard to snapshot → unit-test the pure pieces (countdown math, WMO mapping, lat/lon parse, newline-trim) + a non-hanging smoke test; **`pomodoro` carries a human-verify checkpoint** (live countdown + real toast + Ctrl+C cancel), like matrix did.

### Integration Points
- New `Cargo.toml` deps: `qrcode = "0.14.1"` (qr), `arboard = "3.6.1"` (clip), `tauri-winrt-notification = "0.7.2"` (pomodoro toast), `ureq = "3.3.0"` (weather) + `serde = { version = "1", features = ["derive"] }`. `serde_json`/`crossterm`/`owo-colors`/`rand` already present.
- Each command removes one `not_implemented(...)` arm in `src/main.rs` and swaps one unit variant in `src/cli.rs`. The four are independent but share `cli.rs`/`main.rs`/`commands/mod.rs` (so they sequence by wave, zero same-wave file overlap — as in Phases 3/4).
- No `core::*` surface changes required (the four consume existing helpers as-is); `pomodoro` joins the matrix raw-mode family without touching matrix.

</code_context>

<specifics>
## Specific Ideas

- **qr:** `QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?.render::<Dense1x2>().quiet_zone(true).build()` → `println!`; pure half-blocks, no ANSI, owned String; verify dark-on-light polarity scans on a dark PS bg.
- **clip:** copy = `io::stdin().read_to_end` → strip one trailing `\r\n`/`\n` → `arboard::Clipboard::new()?.set_text(...)` (create at point of use, main thread, return — persists via OS handle ownership); `--paste` = `get_text()` → stdout byte-exact.
- **pomodoro:** matrix `RawGuard` + `event::poll(remaining)` as timer; cancel (Ctrl+C KeyEvent / Esc / q) → exit 1 no toast; completion → `Toast::new(Toast::POWERSHELL_APP_ID).title(...).text1(...).show()` (tauri-winrt-notification) + exit 0; defaults 25/5/15.
- **weather:** parse-shape lat/lon vs city; geocode `…/v1/search?name=&count=1&language=en` → echo resolved location to stderr; forecast `…/v1/forecast?…&current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m` (+`temperature_unit`/`wind_speed_unit` on imperial); WMO `match → &'static str`; `serde_json::from_reader` over `resp.into_body().into_reader()` (no ureq `json` feature); offline/not-found → stderr + exit 1.

</specifics>

<deferred>
## Deferred Ideas

- **VIS-V2-01** (REQUIREMENTS.md): QR `--save` to an image file + configurable error-correction level. The fixed `EcLevel::M` + owned-`String` render (D-01/D-02) are the clean seam for both.
- **SYS-V2-01:** pomodoro session counter / auto-break / sound / `--label`; weather `--forecast` / `--json` / response cache / stored default location — **including per-user unit preference and OS-locale detection** (D-10 explicitly defers personalization to v2 stored-config).
- **DEV-V2-01:** `--clip` integration across `passgen`/`uuid`/`color` (reuses the `clip` copy path once it exists).
- **Out of Scope** (REQUIREMENTS.md, not v2): `qr` decode/scan mode; `clip` binary/image clipboard (text only v1); `weather` API-key requirement / radar / maps.

### Reviewed Todos (not folded)
None — `gsd-sdk query todo.match-phase 5` returned 0 matches. (Separately, the 3 STATE.md § Todos standing items — winrt compile-spike, pomodoro blocking, weather units — were **resolved as decisions** D-07/D-08/D-09/D-10, not deferred.)

</deferred>

---

*Phase: 5-windows-platform-integration*
*Context gathered: 2026-06-24*
