# Phase 9: Dev-Transform & Visual Depth - Pattern Map

**Mapped:** 2026-06-28
**Files analyzed:** 9 command modules extended in place + 1 spine consumed
**Analogs found:** 9 / 9 (every unit of new work extends its OWN current seam; 2 cross-command copy-me templates supply the new terminal-loop + truecolor surfaces)

> This phase EXTENDS nine existing modules — there are NO net-new command files. For each command the closest analog is its OWN current code at the named seam (a `self-extend` match). Two commands also pull a `copy-me` template from a sibling: `lolcat --animate` copies `matrix`'s loop verbatim, and `ascii` truecolor copies `lolcat`'s gated `.truecolor()`. The spine (`core/output.rs`) is **consumed, never modified** — every new field rides one `#[derive(Serialize)]` struct into both the human render and `emit_json`.

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `src/commands/uuid/mod.rs` | utility / generator | request-response (no input) | own `format_one` + `{results,count}` struct | self-extend |
| `src/commands/epoch/mod.rs` | utility | transform | own `epoch_output` + `format_timestamp` shared math | self-extend |
| `src/commands/color/mod.rs` | utility | transform | own `parse_color` ordering + `rgb_to_hsl` inverse | self-extend |
| `src/commands/json/mod.rs` | utility | transform | own pretty/compact/colorize/emit_json fork | self-extend |
| `src/commands/passgen/mod.rs` | utility / generator | transform | own `build_charset` + `phrase.join(".")` | self-extend |
| `src/commands/matrix/mod.rs` | command / terminal-loop | event-driven (streaming frames) | own consts + `katakana_glyphs` + loop | self-extend |
| `src/commands/qr/mod.rs` | utility | transform + **file-I/O** (new) | own `render_qr` + `EcLevel` | self-extend |
| `src/commands/ascii/mod.rs` | command / renderer | file-I/O (image in) | own `image` pipeline + `luma_to_char` seam | self-extend |
| `src/commands/lolcat/mod.rs` | command / renderer | transform → **terminal-loop** (new) | own `rgb_at` + static render; **matrix loop (copy-me)** | self-extend + copy-me |
| `src/core/output.rs` | spine | — | n/a — **CONSUMED, never modified** | spine |

---

## Pattern Assignments

### `src/commands/uuid/mod.rs` (utility/generator, request-response)

**Analog:** own `format_one` (pure renderer) + `UuidRow`/`UuidOutput` structs. Extend `format_one` to take format options; feed BOTH paths from it (single-source → no-drift).

**The pure renderer to extend** (`uuid/mod.rs:88-95`) — currently takes a single `upper: bool`; widen its signature to a format-options struct:
```rust
fn format_one(u: uuid::Uuid, upper: bool) -> String {
    let s = u.to_string();
    if upper { s.to_uppercase() } else { s }
}
```
**Extension point:** swap `u.to_string()` for a `match form { Plain => u.hyphenated(), Simple => u.simple(), Braces => u.braced(), Urn => u.urn() }.to_string()` (all four are `const fn`, no feature), THEN apply `--upper` as a composable post-pass (mirrors the existing `if upper` line). `--braces`/`--urn` are `conflicts_with` each other at the clap layer (exit 2).

**The frozen struct that takes the formatted value** (`uuid/mod.rs:26-39`):
```rust
#[derive(serde::Serialize)]
struct UuidRow { uuid: String, version: &'static str }
#[derive(serde::Serialize)]
struct UuidOutput { results: Vec<UuidRow>, count: usize }
```
**Extension point:** `version: &'static str` becomes `"v4"`/`"v7"`. The `uuid` field carries the SAME `format_one` output the human line prints (default = formatted-everywhere).

**The build-once → fork pattern to preserve** (`uuid/mod.rs:59-80`) — the `--v7` choice slots into the `.map()`:
```rust
let rows: Vec<UuidRow> = (0..self.count)
    .map(|_| UuidRow {
        uuid: format_one(uuid::Uuid::new_v4(), self.upper),   // ← `if v7 { Uuid::now_v7() } else { new_v4() }`
        version: "v4",                                          // ← "v7" when --v7
    })
    .collect();
if crate::core::output::is_json_on() { /* emit_json */ } else { /* out_line per row */ }
```

**Args to extend** (`uuid/mod.rs:42-50`): add `--v7`, `--no-hyphens`, `--braces`, `--urn` (the last two as a `conflicts_with` pair); `--upper` already present.

**Tests live in-module** (`uuid/mod.rs:97-123`): extend `format_one_applies_case` shape — assert each wrapping form's length/shape + v7 version nibble.

---

### `src/commands/epoch/mod.rs` (utility, transform)

**Analog:** the two shared-math helpers `epoch_output` (JSON) and `format_timestamp` (human) — both already share `DateTime::from_timestamp` + `with_timezone(&Local)`, so `--tz`/`relative` extend the SAME math and JSON can't drift from the lines.

**The shared JSON helper** (`epoch/mod.rs:141-150`):
```rust
fn epoch_output(epoch: i64) -> anyhow::Result<EpochOutput> {
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(epoch, 0)
        .ok_or_else(|| anyhow::anyhow!("timestamp {epoch} is out of range"))?;
    let dt_local = dt_utc.with_timezone(&Local);
    Ok(EpochOutput {
        epoch,
        utc: dt_utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        local: dt_local.format("%Y-%m-%d %H:%M:%S %z").to_string(),
    })
}
```
**Extension point:** add `relative: relative_for(epoch, Utc::now().timestamp())` (always-present) and a `tz` field built off the SAME `dt_utc` via `dt_utc.with_timezone(&tz).format("%Y-%m-%d %H:%M:%S %Z %z")`.

**The shared human helper** (`epoch/mod.rs:155-162`) — `--tz` adds a THIRD line here (Local/UTC stay anchors):
```rust
fn format_timestamp(secs: i64) -> anyhow::Result<(String, String)> {
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| anyhow::anyhow!("timestamp {secs} is out of range"))?;
    let dt_local = dt_utc.with_timezone(&Local);
    let local_line = format!("Local: {}", dt_local.format("%Y-%m-%d %H:%M:%S %z"));
    let utc_line = format!("UTC:   {}", dt_utc.format("%Y-%m-%d %H:%M:%S UTC"));
    Ok((local_line, utc_line))
}
```

**The integer-arg human branch — the ONLY place the relative suffix may appear** (`epoch/mod.rs:88-96`). The `now`/date-string branches stay bare integers (scripting-clean):
```rust
if s.parse::<i64>().is_ok() {
    let (local_line, utc_line) = format_timestamp(epoch)?;   // ← append relative_for(...) suffix to each line
    crate::core::output::out_line(&local_line);
    crate::core::output::out_line(&utc_line);
} else {
    crate::core::output::out_line(&epoch.to_string());        // ← STAYS bare (no relative leak)
}
```

**The validation discipline to mirror** for `Tz::from_str` (`epoch/mod.rs:185-188`) — `bail!` clean exit 1 with a hint, never panic:
```rust
bail!(
    "unrecognized date '{s}'; expected one of: RFC3339 (e.g. 2026-06-22T14:30:00Z), \
     'YYYY-MM-DD HH:MM:SS', or 'YYYY-MM-DD'"
);
```

**The frozen struct** (`epoch/mod.rs:31-36`): add `relative: String` (always) + `tz: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`. **Args** (`epoch/mod.rs:43-47`): add `tz: Option<String>`.

**New pure helper:** `relative_for(epoch, now) -> String` (hand-rolled threshold ladder, ~30 lines). ONE helper feeds both the human suffix and the JSON `relative` field. **Dep:** add `chrono-tz` to `Cargo.toml`. **Unit-test the `%Z` output** (Pitfall 2 — `Asia/Tokyo` @ 1700000000 → contains `"JST"` + `"+0900"`).

---

### `src/commands/color/mod.rs` (utility, transform)

**Analog:** `parse_color`'s auto-detect ordering (the `hsl(` branch slots in BEFORE the RGB branch) + `rgb_to_hsl` (the inverse template for the new `hsl_to_rgb`).

**The auto-detect ordering — the decisive no-collision seam** (`color/mod.rs:131-146`). The new `hsl(` prefix check MUST go BEFORE the RGB branch at `:132` (a bare `hsl(210, 100%, 50%)` contains commas → would mis-hit RGB); the named-color lookup goes AFTER the hex check and BEFORE the final `bail!`:
```rust
    // RGB form: contains a separator (comma or whitespace).
    if trimmed.contains(',') || trimmed.split_whitespace().count() > 1 {   // ← :132 — hsl( check goes ABOVE this
        return parse_rgb(trimmed);
    }
    // Otherwise treat as hex.
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.chars().all(|c| c.is_ascii_hexdigit()) && (hex.len() == 3 || hex.len() == 6) {
        return parse_hex(hex);
    }
    // ← named-color table lookup goes HERE (before bail!) — no CSS name is an all-hex 3/6 string
    bail!("unrecognized color '{s}'; expected hex (...) or RGB (...)");
```

**The inverse template** (`color/mod.rs:199-223`) — `rgb_to_hsl` is pure, crate-free, returns `(u16 h, u8 s, u8 l)`; the new `hsl_to_rgb(h, s, l) -> (u8,u8,u8)` mirrors it as the standard closed form (round-trip test ±1):
```rust
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 { (0.0, 0.0) } else { /* ... */ };
    ((h.round() as u16) % 360, (s * 100.0).round() as u8, (l * 100.0).round() as u8)
}
```

**The JSON doc to extend additively** (`color/mod.rs:50-55` struct; `:79-87` assembly):
```rust
#[derive(serde::Serialize)]
struct ColorOutput { hex: String, rgb: Rgb, hsl: Hsl }     // ← add name: Option<String>, nearest: String (both always-present)
// ...
if crate::core::output::is_json_on() {
    let doc = ColorOutput {
        hex: format!("#{r:02x}{g:02x}{b:02x}"),             // hex stays LOWERCASE-locked (D-19)
        rgb: Rgb { r, g, b },
        hsl: Hsl { h, s, l },
    };
    crate::core::output::emit_json(&doc)?;
    return Ok(());
}
```
**Extension point:** the human block (`:93-97`) gains a name row marking exact `name` vs approximate `~nearest`. New pure helpers: a `const &[(&str,(u8,u8,u8))]` CSS-name table (~148 names incl. `rebeccapurple`) + a redmean weighted-RGB nearest lookup (~10 lines). **No crate.** Anchor test: `black`/`white`/`rebeccapurple`/`cornflowerblue`.

---

### `src/commands/json/mod.rs` (utility, transform)

**Analog:** the pretty/compact/colorize/emit_json fork — the recursive key-sort is applied to the parsed `Value` BEFORE this fork so it feeds all four outputs identically.

**The fork to insert the sort ABOVE** (`json/mod.rs:61-90`):
```rust
Ok(value) => {
    // ← let value = if self.sort_keys { sort_value(value) } else { value };  INSERT HERE (before :69 fork)
    if crate::core::output::is_json_on() {
        return crate::core::output::emit_json(&value);          // :69-71
    }
    if self.compact {
        crate::core::output::out_line(&serde_json::to_string(&value)?);
    } else if is_color_on() {
        print!("{}", colorize(&value, 0));
    } else {
        crate::core::output::out_line(&serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}
```
**Extension point:** a recursive `sort_value(Value) -> Value` that rebuilds each `Value::Object` map in sorted-key order (arrays keep order). `preserve_order` makes the map an ordered `IndexMap`, which is exactly why the sort must be explicit and opt-in. No dep change.

**Args** (`json/mod.rs:37-44`): add `--sort-keys` (opt-in; `preserve_order` STAYS default — never sorted implicitly). **Tests** (`:197-319`): the existing `colorize_preserves_key_order` is the counter-template; add `sort_value` recursive + nested + arrays-keep-order tests.

---

### `src/commands/passgen/mod.rs` (utility/generator, transform)

**Analog:** the pure `build_charset` (the `--no-similar` prune point) + the `phrase.join(".")` site (the `--separator` override). RNG (`OsRng`, `:116`) stays **untouched** (T-V6).

**The separator seam** (`passgen/mod.rs:137-139`):
```rust
PassgenRow {
    password: phrase.join("."),     // ← phrase.join(&self.separator)  (default stays ".")
}
```

**The charset builder to prune** (`passgen/mod.rs:176-185`):
```rust
fn build_charset(no_symbols: bool) -> Vec<char> {
    let mut set: Vec<char> = Vec::new();
    set.extend_from_slice(LOWER);
    set.extend_from_slice(UPPER);
    set.extend_from_slice(DIGITS);
    if !no_symbols { set.extend_from_slice(SYMBOLS); }
    set                                  // ← when --no-similar, retain(|c| !"il1Lo0O".contains(*c)) and recompute pool
}
```

**The build-once → fork to preserve** (`passgen/mod.rs:159-169`) — entropy must NOT contaminate stdout/JSON:
```rust
if crate::core::output::is_json_on() {
    let doc = PassgenOutput { count: rows.len(), results: rows };   // ← add entropy_bits: f64 (per-config)
    crate::core::output::emit_json(&doc)?;
} else {
    for r in &rows { crate::core::output::out_line(&r.password); }  // ← entropy → eprintln! (STDERR, TTY-gated), NOT out_line
}
```
**Extension point:** entropy = `length * log2(pool_size)` (char) / `words * log2(7776)` (`=12.925`, phrase). Human path → STDERR (keeps secret-only-on-stdout D-14); JSON → top-level `entropy_bits` on `PassgenOutput` (`:46-50`). **Args** (`:88-109`): add `--no-similar`, `--separator <str>`. **Tests** (`:200-268`): extend `default_charset_is_four_class_curated` / `no_symbols_charset_is_alphanumeric_only` shape for the `--no-similar` prune + an `entropy` calc test.

---

### `src/commands/matrix/mod.rs` (terminal-loop, event-driven) — ALSO the copy-me template

**Analog:** own preset consts + `katakana_glyphs` + the existing gated `.truecolor` sites. The `RawGuard`/`is_quit`/loop is **untouched** (this module is the donor for lolcat, see Shared Patterns).

**The consts that become preset-driven** (`matrix/mod.rs:85-94`):
```rust
const SPEED_MIN: i32 = 1;          // ← --speed level → poll(ms) + SPEED_MIN/MAX
const SPEED_MAX: i32 = 2;
const HEAD_RGB: (u8, u8, u8) = (180, 255, 180);   // ← --color preset → HEAD/FADE RGB
const FADE_BRIGHT: u8 = 255;
const FADE_DARK: u8 = 40;
```

**The HEAD/FADE truecolor sites — already gated, keep them gated** (`matrix/mod.rs:178`, `:188`). New `--color` presets feed the SAME `.truecolor` calls:
```rust
queue!(out, MoveTo(x, y as u16), Print(glyph.truecolor(0, g, 0)))?;                       // :178 trail
queue!(out, MoveTo(x, d.head as u16), Print(glyph.truecolor(HEAD_RGB.0, HEAD_RGB.1, HEAD_RGB.2)))?;  // :188 head
```

**The glyph table that `--charset` replaces** (`matrix/mod.rs:306-310`):
```rust
fn katakana_glyphs() -> Vec<char> {
    (KATAKANA_START..=KATAKANA_END).filter_map(char::from_u32).collect()
}
```
**Extension point:** `--charset` = preset name (katakana[default]/ascii/binary/digits) resolving to a table, OR a literal custom string whose chars become the glyph set — replaces the `glyphs` binding at `:156`.

**Args** (`matrix/mod.rs:107-108`, currently `struct MatrixArgs {}`): add three `pub` `ValueEnum`s (`--color`, `--speed`) + `--charset` (preset-or-literal), matching the `hash::Algo` ValueEnum style. All color stays gated on `is_color_on()` (display-only, SC4). **Tests** (`:324+`): add pure preset→RGB / speed-level→poll / charset-resolution helpers (no terminal needed).

---

### `src/commands/qr/mod.rs` (utility, transform + file-I/O)

**Analog:** the pure `render_qr` seam + the already-imported `EcLevel`. The `--error-correction` swaps the level; `--save` branches off the rendered matrix to PNG/SVG (the one genuinely-new filesystem write this phase).

**The pure render seam** (`qr/mod.rs:106-118`) — `EcLevel::M` is hardcoded at `:110`; parameterize it:
```rust
fn render_qr(input: &str) -> anyhow::Result<String> {
    let code = QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?;   // ← take ec: EcLevel param (default M)
    let rendered = code.render::<Dense1x2>().quiet_zone(true).build();
    Ok(rendered)
}
```
The `?` on `with_error_correction_level` already propagates a capacity overflow → clean exit 1 (preserve this — T-05-QR-DoS).

**The fork + metadata doc to extend** (`qr/mod.rs:79-95`):
```rust
if crate::core::output::is_json_on() {
    let doc = QrOutput {
        text: input.clone(),
        error_correction: "M".to_string(),     // ← reflect the CHOSEN level
    };                                          // ← add saved_path: Option<String> when --save
    return crate::core::output::emit_json(&doc);
}
let rendered = render_qr(&input)?;
println!("{rendered}");                         // ← gate behind `if save.is_none()` (suppress glyphs under --save)
crate::core::output::clip_feed(&input);
```

**Struct** (`qr/mod.rs:63-69`): add `saved_path: Option<String>` (`skip_serializing_if`), make `error_correction` reflect the chosen level. **Args** (`:53-57`): add `--error-correction L|M|Q|H` (ValueEnum→`EcLevel`, default M) + `--save: Option<PathBuf>`.

**New file-I/O (the only new I/O surface)** — infer format from extension, `bail!` clean on unknown, wrap writes with `.with_context` (the project's FOUND-05 discipline, like `epoch`'s `bail!` hint):
```rust
"png" => { let img = code.render::<image::Luma<u8>>().quiet_zone(true).build();
           img.save(path).with_context(|| format!("writing {}", path.display()))?; }
"svg" => { let svg: String = code.render::<qrcode::render::svg::Color>().quiet_zone(true).build();
           std::fs::write(path, svg).with_context(|| format!("writing {}", path.display()))?; }
other  => anyhow::bail!("unsupported --save extension '.{other}'; use .png or .svg"),
```
**Dep:** re-enable qrcode `["image","svg"]` (NOT a hand-rolled rasterizer — the `image` feature unifies with the present `image 0.25.10`, `svg` is zero-dep). **Tests** (`:120-150`): extend `render_qr_*` for each `EcLevel`; add `assert_fs` temp-dir file-write + bad-extension exit-1 tests.

---

### `src/commands/ascii/mod.rs` (renderer, file-I/O) — consumes the lolcat copy-me template

**Analog:** own `image` pipeline (`resize_exact` + `to_luma8`) + the single `luma_to_char` luma seam (where `--invert` flips). Truecolor copies `lolcat`'s gated `.truecolor()` (see Shared Patterns). **Sharp edge:** this module currently imports NEITHER `owo_colors` NOR `is_color_on` (`:32-36`) — adding color means importing both for the first time, and the gate is easy to forget (Pitfall 4).

**The pipeline + luma seam** (`ascii/mod.rs:77-93`):
```rust
let gray = img
    .resize_exact(cols, rows, image::imageops::FilterType::Triangle)   // ← ALSO .to_rgb8() for per-cell color; resize 2c×4r for braille
    .to_luma8();
let buf = gray.as_raw();
for y in 0..rows {
    let mut line = String::with_capacity(cols as usize);
    for x in 0..cols {
        let idx = y as usize * cols as usize + x as usize;
        let luma = buf[idx];                       // ← --invert: `let luma = if invert { 255 - luma } else { luma };` HERE
        line.push(luma_to_char(luma, RAMP) as char);   // ← braille engine REPLACES this; truecolor wraps the glyph
    }
    println!("{line}");
}
```

**The single luma seam** (`ascii/mod.rs:104-107`) — `--invert` = `255 - luma` BEFORE this; keep luma for the ramp index even in color mode:
```rust
fn luma_to_char(luma: u8, ramp: &[u8]) -> u8 {
    let idx = (luma as usize * (ramp.len() - 1)) / 255;
    ramp[idx]
}
```

**Extension points (three orthogonal additions):**
1. **truecolor (default-on, gated):** sample `.to_rgb8()` per cell; emit `.truecolor(r,g,b)` on the ramp glyph ONLY when `is_color_on()` — degrade to mono `RAMP` when piped/`--json`/`NO_COLOR`. No `--color` flag.
2. **`--braille` (replaces the ramp):** hand-rolled 2×4 engine, `char = U+2800 + bitmask`, little-endian dot→bit mapping, resize to `2*cols × 4*rows`, fixed 50% per-dot luma threshold as a swappable `const`. Color = average the 8 sub-pixels → one gated `.truecolor()` per glyph. Lock bit-order with a unit test. No crate.
3. **`--invert`:** one byte-flip at the luma seam (orthogonal to color + braille).

Stays display-only (SC4 — no `--json` doc; `tests/cli.rs::display_only_omit_json` is the template, plus a `> file` no-`0x1B` scan). **Tests** (`:131-230`): extend the `luma_to_char` extreme/monotonic tests for `--invert`; add a braille bit-order lock.

---

### `src/commands/lolcat/mod.rs` (renderer → terminal-loop) — copies the matrix template; IS the ascii truecolor template

**Analog:** own pure `rgb_at(phase)` sine gradient + `is_color_on()`-gated per-scalar `.truecolor()` (the static one-pass render is the degrade path for `--animate`). For the animation loop itself, copy `matrix`'s `RawGuard`/`is_quit`/`event::poll` VERBATIM (Shared Patterns).

**The gradient to parameterize** (`lolcat/mod.rs:128-134`) — drop the hard-coded `f = 0.1` to a param so `--freq`/`--seed` govern BOTH the static and animated paths (ONE gradient path):
```rust
fn rgb_at(phase: f64) -> (u8, u8, u8) {
    let f = 0.1;                                              // ← rgb_at(phase, freq) — freq becomes a param
    let r = ((f * phase).sin() * 127.0 + 128.0) as u8;
    let g = ((f * phase + 2.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = ((f * phase + 4.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}
```

**The gated per-scalar truecolor — THE copy-me template for ascii** (`lolcat/mod.rs:100-111`):
```rust
if c.is_whitespace() {
    write!(out, "{c}")?;                       // colored space is invisible — emit plain, still advance phase
} else if is_color_on() {                      // ← the SOLE gate; ascii copies this exact structure
    let (r, g, b) = rgb_at(phase);
    write!(out, "{}", c.truecolor(r, g, b))?;
} else {
    write!(out, "{c}")?;                       // plain: byte-identical-minus-ANSI (D-14 / SC4)
}
phase += w;
```

**The static one-pass render = the `--animate` degrade target** (`lolcat/mod.rs:67-118`) — keep the unconditional `strip_ansi_escapes::strip_str` on input (`:74`) on BOTH paths (T-04L-01); single buffered `flush()` (`:83-84`, `:116`).

**Extension point — the new terminal loop (headline PS7 human-verify):** add `--animate`/`--freq`/`--seed`/`--duration` to `LolcatArgs` (`:61-65`; lolcat does NOT currently import crossterm — add it). Enter the loop ONLY when `std::io::stdout().is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()` (the `is_terminal()` AND-gate is MANDATORY — `CLICOLOR_FORCE` can force color true on a pipe); otherwise dispatch to the existing static renderer. `--seed` = initial phase offset; `--duration 0` = run until keypress; persist ONE final static frame after teardown.

> **Human-verify gotcha (document in the 09-03 checkpoint):** `read_input` gates on STDIN-TTY and exits 2 for a no-arg interactive invocation — so `box lolcat --animate` (typed, no text) exits 2 BEFORE the animate branch (the gate is on STDOUT). The PS7 verify MUST pass text as an arg: `box lolcat "Hello World" --animate`.

---

## Shared Patterns

### Terminal-animation loop (RAII + frame-timer-AND-input-gate + single-flush + Press-only quit)
**Source:** `src/commands/matrix/mod.rs:117-322` (the proven D-10 template)
**Apply to:** `lolcat --animate` (verbatim copy — the decisive reason the bounded-alt-screen model beats the line-wrap-fragile `MoveUp` reprint)

The four load-bearing pieces, in order:
```rust
// 1. RAII guard — armed the INSTANT raw mode is on, BEFORE the fallible alt-screen setup (matrix:117-142)
struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();          // errors ignored — drop must not panic
    }
}
// in run():
enable_raw_mode()?;
let _guard = RawGuard;                        // armed BEFORE the next fallible `?`
crossterm::execute!(out, EnterAlternateScreen, cursor::Hide)?;

// 2 + 3. Single flush per frame, THEN the 50ms poll = ~20-FPS timer AND input gate (matrix:205-217)
out.flush()?;                                 // EXACTLY ONCE per frame — never per char (the STATE.md pitfall)
if event::poll(Duration::from_millis(50))? {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press && is_quit(&key) { break; }   // Press-only: Windows fires Press AND Release
    }
}

// 4. Pure quit predicate (matrix:316-322) — q / Esc / Ctrl+C-as-KeyEvent
fn is_quit(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press { return false; }
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}
```
Keep the loop panic-free (no `.unwrap()` on terminal ops) so the Drop guard is the real restore under release `panic = "abort"`.

### Gated per-scalar truecolor (the SOLE color gate)
**Source:** `src/commands/lolcat/mod.rs:104-111` (template) + the gate primitive `src/core/output.rs:34-36`
**Apply to:** `ascii` truecolor (new), `matrix --color` presets (existing `.truecolor` sites `:178`/`:188`), animated `lolcat`
Every new colored token is reached ONLY after `is_color_on()` returns true; the plain branch is byte-identical minus the escape. `ascii` must newly `use owo_colors::OwoColorize;` + `use crate::core::output::is_color_on;`.

### Build-once → `is_json_on()` fork FIRST → single Serialize struct feeds both paths (no-drift)
**Source:** `src/core/output.rs` (`is_json_on` `:93-95`, `emit_json` `:124-134`, `out_line` `:143-150`) — **consumed, never modified**
**Apply to:** all five dev-transform commands (`uuid`, `epoch`, `color`, `json`, `passgen`)
Pattern: compute the value(s) once via a pure helper; fork on `is_json_on()` FIRST (the only stdout write reachable under `--json` is `emit_json`); new fields are ADDITIVE to the frozen Phase-7 struct. `init_output` (`:106-113`) already force-disables color under `--json`/`--clip`, so SC4 byte-identity-minus-ANSI falls out for free.

### Clean-error validation (`bail!` exit 1 with a hint, never panic)
**Source:** `src/commands/epoch/mod.rs:185-188` (`parse_date` hint) + `color/mod.rs:143-146`
**Apply to:** `epoch` `Tz::from_str` (exit 1 + IANA hint), `qr --save` bad-extension (exit 1 + `.png`/`.svg` hint), `color` `hsl(...)` parse. Wrap file writes with `.with_context(...)` (FOUND-05).

---

## No Analog Found

None. Every unit of new work extends an existing pure seam in its own module, and the two genuinely-new surfaces (the `lolcat --animate` terminal loop and the `qr --save` file write) each have a sibling copy-me template (matrix's loop; the project's `.with_context` FOUND-05 I/O discipline). The hand-rolls explicitly chosen over crates per CONTEXT.md — CSS-name table, redmean nearest, `relative_for` ladder, recursive `sort_value`, braille bitmask, entropy math — are ~10-30 line pure functions that attach at the seams above; the planner should follow RESEARCH.md's verified signatures (`09-RESEARCH.md` § Code Examples) for those, NOT a parallel analog.

## Metadata

**Analog search scope:** `src/commands/{uuid,epoch,color,json,passgen,matrix,qr,ascii,lolcat}/mod.rs`, `src/core/output.rs`
**Files scanned:** 10 (9 command modules + the output spine)
**Pattern extraction date:** 2026-06-28
**Cross-checked against:** `09-CONTEXT.md` (locked seams) + `09-RESEARCH.md` § Seam Verification (every line cite confirmed against the real source)
