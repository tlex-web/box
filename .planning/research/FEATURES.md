# Feature Research — box v2.0 "Toolbox → Toolkit"

**Domain:** Scriptable single-binary Rust CLI toolbox (23 commands), Windows PowerShell 7 target
**Researched:** 2026-06-24
**Confidence:** HIGH on `--json`/`--clip`/config/completions conventions and PS7 consumption; HIGH on per-command norms (hash, rename, pomodoro, passgen, uuid, qr, braille); MEDIUM where a specific tool's exact schema wasn't published (dust, eza)

> Scope note: every "command" below already ships and works (v1.0). This research is about the *behavior and conventions* of the v2 depth being added, not the commands themselves. The downstream consumer is REQUIREMENTS.md category scoping — so each capability is sorted into **table-stakes / differentiator / anti-feature**, with complexity and dependency on existing infra noted.
> (v1.0 feature research preserved alongside this file as `FEATURES-v1.0.md`.)

---

## PART 1 — The cross-cutting decision: `--json` house style

This is the single most important convention to lock down because it touches every applicable command and is the milestone's "scriptable spine." Get the shape consistent and PS7-native, or the feature fails its one job (being pipeable into `ConvertFrom-Json`).

### How real tools structure their JSON (verified)

| Tool | Shape | Notes |
|------|-------|-------|
| **ripgrep** (`rg --json`) | **NDJSON / JSON Lines** — one JSON object per line, typed by a `"type"` field (`begin`, `match`, `context`, `end`, `summary`). Non-UTF-8 data wrapped as `{"text": ...}` or `{"bytes": <base64>}`. | Streaming-first. Cannot combine with `--count`/`--files` (errors out). |
| **gh** (`gh pr list --json …`) | **Single JSON array of objects** — `[ {…}, {…} ]`, one object per item, only the requested fields. Pretty-printed to a TTY, compact when piped. | The dominant "list command" convention. |
| **hyperfine** (`--export-json`) | **Single root object** `{ "results": [ {…} ] }` — summary stats + per-run timings nested. | Batch/aggregate, written to a file, not streamed. |
| **dust** (`dust -j`) | **Single nested object (tree)** — root node with recursive `children`, for `dust -j \| jq`. | Mirrors the tree it prints. Exact field names not published; confirmed one document, not NDJSON. |
| **fd / eza** | **No JSON output.** fd offers `-0`/`--list-details` instead; eza explicitly **rejected** `--json` (issue #1064, "not planned"). | Useful negative signal: not every tool needs JSON; line-oriented `-0` is an accepted alternative. |

Two camps: **streaming NDJSON** (ripgrep — for unbounded/incremental output) vs **single-document** (gh array, hyperfine object, dust tree — for bounded results built fully before printing).

### CRITICAL: how PowerShell 7 `ConvertFrom-Json` consumes each shape (verified against MS Learn 7.6 docs)

`ConvertFrom-Json` takes **one complete JSON string** (the whole document) and emits objects:

- **JSON array** `[ {…}, {…} ]` → enumerated: **one `PSCustomObject` per element streamed to the pipeline.** `box … | ConvertFrom-Json | Where-Object size -gt 1MB` "just works." (`-NoEnumerate` would keep it as one array object; default enumerates.) **This is the PS7-ergonomic shape.**
- **Single object** `{…}` → one `PSCustomObject`. Fine for scalar/aggregate commands.
- **NDJSON / JSON Lines** (ripgrep's shape) → **NOT natively supported.** A whole NDJSON blob is invalid JSON to `ConvertFrom-Json` (multiple top-level values) and throws. The PS7 user must hand-write `Get-Content file | ForEach-Object { $_ | ConvertFrom-Json }` per line. That is exactly the friction this milestone exists to remove.

**Implication, stated for the record:** for a PS7-first toolkit, **a single JSON document (array for multi-item, object for scalar/tree) is strictly more ergonomic than NDJSON.** NDJSON's only real win is unbounded streaming, which none of box's commands need — every command's result set is bounded and built in memory before printing (flatten plan, hash list, dupes groups, du rows, tree). box should NOT inherit ripgrep's NDJSON model.

### RECOMMENDED house style for box (the deliverable)

1. **Multi-item commands emit a single top-level JSON array of objects**, gh-style:
   - `hash` over N files → `[ {"file":"a.txt","algo":"blake3","digest":"…"}, … ]`
   - `du` rows → `[ {"path":"node_modules","bytes":12345,"percent":42.1}, … ]`
   - `dupes` groups → `[ {"digest":"…","size":1024,"wasted":1024,"paths":["a","b"]}, … ]`
   - `flatten` mapping → `[ {"source":"docs\\sub\\report.txt","dest":"docs_sub_report.txt","action":"copy"}, … ]`
   - `bulk-rename` plan → `[ {"from":"a.txt","to":"a_001.txt"}, … ]`
   - `uuid -n 5` → `["…","…",…]` (array of strings), or array of objects if format metadata is attached.
   - Single-item invocations still emit a 1-element array for shape stability (a script shouldn't branch on count). Document this explicitly.

2. **Scalar / single-result commands emit a single top-level object:**
   - `color #ff8800 --json` → `{ "hex":"#ff8800", "rgb":[255,136,0], "hsl":[33,100,50], "css_name":"darkorange" }`
   - `epoch … --json` → `{ "epoch":1735000000, "utc":"…", "local":"…", "relative":"3 hours ago" }`
   - `weather … --json` → `{ "location":{…}, "current":{…}, "forecast":[…] }`
   - `passgen --json` → `{ "password":"…", "entropy_bits":95.3, "length":16 }` (or array under `--count`).

3. **`tree` is the one genuine nesting case** → single root object with recursive `children` (dust-style); flattening a tree into an array loses the structure that is the command's whole point: `{ "name":"src", "type":"dir", "children":[ {"name":"main.rs","type":"file","bytes":1234}, … ] }`.

4. **Field naming: `snake_case`.** It's the serde/Rust default (`#[derive(Serialize)]` with no rename = the struct field name), matches ripgrep/jq idiom, and PS7 property access is case-insensitive — so snake_case costs PS7 users nothing while staying idiomatic for the JSON ecosystem. Pick it once, apply everywhere. (camelCase would only matter if a JS/browser consumer were primary; it isn't.)

5. **`--json` is exclusive with human/color output**, like ripgrep: when `--json` is set, emit pure JSON to stdout, no ANSI, no progress bar *on stdout* (a progress bar may still render on **stderr** — it doesn't corrupt the JSON on stdout). Errors stay on stderr with the existing 0/1/2 exit contract. Composes cleanly with the existing `is_color_on()` gate.

6. **Pretty vs compact:** gh/jq pretty-print to a TTY and compact when piped. `ConvertFrom-Json` ignores whitespace either way. **Recommendation: pretty by default** (`serde_json::to_string_pretty`) — matches `box json`'s own 2-space house style, helps a human eyeballing `box … --json`, and none of these outputs are large enough for compactness to matter. A future `--json-compact` is a trivial add.

### `--json` table-stakes / differentiator / anti-feature

| Aspect | Category | Complexity | Notes / dependency |
|--------|----------|------------|--------------------|
| Single array (multi) / object (scalar) house style | Table-stakes | LOW per command (derive `Serialize` structs) | New `--json` flag wired through each command's output path; reuses existing stdout/stderr split. serde already a dep. |
| snake_case fields, stable 1-element-array-for-single | Table-stakes | LOW | Pure convention; enforce in review. |
| `tree` recursive object | Differentiator | MEDIUM | Only nesting case; needs a recursive serializable node type. |
| `--json` suppresses ANSI/human output, errors stay stderr | Table-stakes | LOW | Compose with `is_color_on()` gate already present. |
| NDJSON / JSON Lines streaming | **Anti-feature** | — | Breaks `ConvertFrom-Json` (needs per-line parse). box results are bounded; no streaming need. Do NOT adopt ripgrep's model. |
| Per-command bespoke schemas (no shared convention) | **Anti-feature** | — | Inconsistency is the failure mode; the house style above is the antidote. |

---

## PART 2 — The other cross-cutting feature: `--clip`

### Convention (verified)

System clipboard tools (`pbcopy`, `clip.exe`, `wl-copy`, `xclip`) are **silent pipeline sinks** — read stdin, copy, print nothing. That's the *piping* model (`box uuid | clip`). A built-in `--clip` *flag* is a different UX: the established expectation for an in-tool `--copy/--clip` flag (1Password CLI, password generators) is **copy AND still print**, optionally with a one-line stderr confirmation ("Copied to clipboard").

### Recommended `--clip` behavior for box

- **Copy the command's primary result to the clipboard AND still print it to stdout.** Print-and-copy is least-surprising; a script that doesn't want stdout can redirect it, but a human running `box passgen --clip` wants to *see* what was generated and have it ready to paste.
- Any "Copied to clipboard" confirmation goes to **stderr** (never stdout) so it never pollutes a piped result. Optionally suppress when not a TTY.
- `--clip` copies the **raw result text, not the colorized/JSON form** by default. With `--clip --json`, copy the JSON. Never copy ANSI escapes to the clipboard.
- Reuse v1's `arboard` infra (proven, Win32, no elevation, Unicode round-trip human-verified). Zero new dependency.

### Which commands `--clip` makes sense for

| Makes sense (single textual result) | Does NOT make sense |
|-------------------------------------|---------------------|
| `passgen` (THE canonical use case — generate, copy, paste into a form) | `matrix` (animated full-screen; no "result") |
| `uuid` (copy a fresh id) | `pomodoro` (a timer; no result string) |
| `color` (copy hex/rgb) | `lolcat` (decorative; result == input) |
| `hash` (copy a digest) | `cowsay`/`fortune`/`8ball`/`roast` (toys; low value, harmless if cheap) |
| `qr` — **text payload only**, not the rendered blocks | `tree`/`du`/`dupes`/`flatten`/`bulk-rename` (multi-line/structured — clipboard is the wrong channel; `--json` + redirect is the answer) |
| `base64`, `epoch`, `json` (compact form) | `weather` (multi-field display; `--json` is the scriptable path) |

| Aspect | Category | Complexity | Notes |
|--------|----------|------------|-------|
| `--clip` copies + prints, confirmation on stderr | Table-stakes (for textual commands) | LOW | Reuses arboard; gate which commands expose the flag. |
| `--clip` copies raw text, never ANSI; respects `--json` | Table-stakes | LOW | Strip-ANSI already exists for lolcat. |
| `--clip` on visual/timer commands | **Anti-feature** | — | No coherent "result" to copy; don't expose the flag there. |
| Silent copy with no stdout print (pbcopy emulation) | Differentiator (optional) | LOW | Could be `--clip-only`; defer unless asked. |

---

## PART 3 — Per-command depth (the full deferred-V2 set)

For each, v2 additions are sorted into **table-stakes (T)**, **differentiator (D)**, **anti-feature (A)**, with complexity and any dependency on existing commands/infra.

### Filesystem group

**flatten (FLAT-V2)**

| Feature | Cat | Cx | Notes / dependency |
|---------|-----|----|--------------------|
| `--extensions txt,md` filter; `--include-hidden`; `--separator _` (override the `_` path-encoder) | T | LOW | Pure traversal/filter tweaks on the existing walkdir path. |
| `--json` mapping (`[{source,dest,action}]`) | T | LOW | Same plan struct already used for `--dry-run` summary. |
| Progress bar (indicatif) for large copies | D | LOW | Already a dep; render on stderr only, skip under `--json`. |
| `--move` (relocate instead of copy) | D | **MEDIUM** | **Safety-critical.** Must be **rename-when-same-volume, else copy-then-delete-after-verify**; never delete source before the destination write is confirmed. Honour the existing collision-rename + `create_new` loud-fail. Cross-volume `fs::rename` fails on Windows → copy-then-delete fallback is mandatory. v1 held this out for data-loss risk; re-introducing needs the same abort-before-any rigor as bulk-rename. |
| Silent overwrite in `--move` | A | — | Reuse v1's `create_new` + collision-encode; never clobber. |

**hash (HASH-V2)** — *carries the milestone's breaking change.*

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| **BLAKE3 as the default algo** (was SHA-256) | T (breaking) | LOW | The major-version trigger (HASH-V2-01 supersedes HASH-01). `--algo sha256` preserves old behavior. Document loudly — same input now yields a different default digest. |
| Multi-file hashing | T | LOW | Accept N paths; one line each. |
| coreutils output format `<digest>␣␣<filename>` (two spaces) | T | LOW | **The double space is load-bearing** — `sha256sum -c`/verify tooling fails on a single space. Match exactly for interop (box already emits `HASH  filename`). Keep BLAKE3 lines in the same shape so `--verify` round-trips. |
| `--json` `[{file,algo,digest}]` | T | LOW | Array house style. |
| Progress bar for large files | D | LOW | indicatif on stderr; skip under `--json`. |
| BSD-tag format (`BLAKE3 (file) = …`) | D | LOW | Optional interop nicety; defer. |

**dupes (DUPE-V2)**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--json` groups (`[{digest,size,wasted,paths[]}]`) | T | LOW | Array house style; group object holds the path list. |
| Multi-stage hashing (size → head/tail sample → full BLAKE3) | D | **MEDIUM** | Perf win on big trees: cheap prefix/suffix sample eliminates non-dupes before the full hash. Adds correctness surface — sample collisions still need full-hash confirmation; never declare dupes on a partial hash alone. |
| Hardlink awareness (don't count two names for one inode as wasted) | D | MEDIUM | Windows: file-ID/volume lookup; on by default avoids overcounting "wasted" bytes. |
| `--delete` | **A (auto/interactive); D only if non-interactive + explicit keep-policy** | HIGH | v1 held this out as "catastrophic if wrong." If added: **must be non-interactive and scriptable** — no y/N prompts (break piping + the 0/1/2 contract). Safe shape: `--delete --keep first` (keep first path per group, delete rest) behind a mandatory `--force`-style confirm, dry-run by default printing what *would* be deleted, abort-before-any on any error. **Interactive selection menus are the anti-feature.** Strong recommendation: keep dupes read-only OR gate delete extremely conservatively. |

**bulk-rename (RENM-V2)**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Case transforms (upper / lower / title) | T | LOW | Industry-standard (PowerRename, Bulk Rename Utility, Advanced Renamer all have UPPER/lower/Title). Title-case = capitalize first letter of each word. |
| Sequential numbering token + padding | T | LOW | Convention is a token in the replacement string — `{n}`/`<n>` with a padding spec. Recommend `{n}` plus `--start N --step N --pad N` (zero-pad to width: `001,002`). Mirrors PowerRename `${padding=2}` and BRU `<n>`. |
| `--json` plan (`[{from,to}]`) | T | LOW | Same dry-run plan, serialized. |
| `--backup` (write sidecar before rename) | D | MEDIUM | Safety net. Keep the v1 model (dry-run default + `--force`, abort-before-any pre-flight on collisions/cycles/path-escape) as the primary guard; `--backup` is belt-and-suspenders. |
| Undo log / auto-undo | A | — | v1 out-of-scope. Dry-run-first IS the undo; an undo log adds state to a stateless tool. |

**tree (TREE-V2)**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `.gitignore` respect; `--ignore <glob>`; `--dirs-only`; hidden toggle | T | LOW–MED | `.gitignore` via the `ignore` crate (already a project dep for dupes). Standard in fd/eza. |
| `--json` recursive node tree | T | MEDIUM | The one nesting case (see Part 1.3). |
| Sort-by-size | D | LOW | Reuse the size accumulation already in `--sizes`. |

**du (DU-V2)**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Percentage column + in-line bar (`████░░ 42%`) | D | LOW | dust's signature; high visual value, gate bar glyphs behind `is_color_on()`/TTY like other ANSI. |
| Color-coded size ranges (green→yellow→red) | D | LOW | owo-colors already a dep; gate on color flag. |
| `--exclude <glob>` patterns | T | LOW | Table-stakes for a du tool (`node_modules`, `.git`). |
| `--json` rows (`[{path,bytes,percent}]`) | T | LOW | Array house style. |
| Apparent-size vs on-disk (allocated) size | D | MEDIUM | Apparent = logical bytes (most users' mental model); on-disk = block-rounded allocation. Windows allocated size needs `GetCompressedFileSize`/cluster math. coreutils `du` defaults to disk-allocated; a dev tool usually wants apparent. Recommend **apparent as box's default**, add `--disk-usage` for allocated. Document which is default. |

### Dev-transform group (DEV-V2 / PASS-V2 / JSON)

**uuid**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--version 7` (time-ordered) | D | LOW | v7 = 48-bit ms timestamp prefix → sortable, B-tree-friendly (2–10× better DB insert vs v4). Modern default for DB keys; offer it, keep v4 as the box default (least surprise). `uuid` crate `v7` feature. |
| Format flags (`--upper`, `--no-hyphens`, `--braces`, `--urn`) | T | LOW | `--upper` already exists; add brace/URN/no-hyphen forms. |
| `--clip`, `--json` | T | LOW | Cross-cutting. |

**epoch**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Relative time ("3 hours ago" / "in 2 days") | D | LOW | `chrono-humanize` (or `timeago`) crate; chrono already a dep. Add to default human output + `relative` field in `--json`. |
| Timezone support (`--tz America/New_York`) | D | MEDIUM | Needs `chrono-tz` (new dep, bundled IANA db). Without it, only local + UTC (current v1). |
| `--json` `{epoch,utc,local,relative}` | T | LOW | Scalar object. |

**color**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Named CSS color lookup (both directions) | D | LOW | Bundle the 148 CSS named colors as a static table. Forward (`box color rebeccapurple`) trivial; reverse "nearest name" (Euclidean in RGB) is a nice differentiator. |
| HSL output (already supported) + HSL input | T | LOW | Round out conversions. |
| `--clip`, `--json` (`{hex,rgb,hsl,css_name}`) | T | LOW | Scalar object. |

**json**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--sort-keys` | D | LOW | **Conflicts with the existing `preserve_order` default** — must override insertion order *only when opted in*. serde_json with `preserve_order` can be re-sorted before printing. Never sort by default (would break the v1 identity-preserving contract). |

**passgen (PASS-V2)**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Entropy estimate in **bits** | D | LOW | `log2(charset_size) * length` for random; `log2(wordlist_size) * words` for passphrases. Show on stderr or in `--json` (`entropy_bits`). Standard expectation for a serious generator. |
| `--no-similar` (exclude ambiguous glyphs) | T | LOW | Convention excludes `il1Lo0O` (often also `B8/S5/Z2/G6`). Recommend the common `il1Lo0O` set; document exactly which chars. Costs ~0.14 bits/char — recompute entropy from the reduced charset, don't report the full-charset figure. |
| `--separator -` for passphrases | T | LOW | EFF-style `word-word-word`. |
| `--clip`, `--json` | T | LOW | passgen is THE canonical `--clip` command. |
| `--no-similar` leaving an empty charset | A-adjacent | — | Validate the resulting charset is non-empty; loud error, not silent. |

### Visual group (VIS-V2)

**lolcat**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--freq` (gradient frequency) and `--seed` (phase offset) | T | LOW | Standard lolcat knobs; box already computes the sine gradient, these are parameters into it. Deterministic with `--seed` (testable). |
| `--animate` (cycle the gradient in place, looping) | D | **MEDIUM** | Needs crossterm cursor-up + redraw loop and clean Ctrl+C/raw-mode restore — **reuse the matrix RAII terminal-restore pattern** (the v1 review caught a raw-mode-stuck bug; same guard applies). Must no-op to plain output when piped/non-TTY. |

**matrix**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--color <name>`; `--speed`; `--charset katakana\|ascii\|binary` | D | LOW | Parameters into the existing rain loop; owo-colors for color, keep the RAII restore. |

**qr**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--save <file>` PNG/SVG export | D | MEDIUM | qrencode convention: format from extension or `-t`; PNG + SVG are the two that matter. PNG raster via the `image` crate (already a dep); SVG via a tiny path emitter (no new dep). The `qrcode` 0.14 crate can render both. |
| `--error-correction L\|M\|Q\|H` | T | LOW | qrencode uses `-l {L,M,Q,H}`. box currently fixes `EcLevel::M`; expose it (default stays M — good scan/density balance). |
| `--clip` (text payload, NOT the art) | T | LOW | Copy the encoded string, never the block render. |

**ascii**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Color output (truecolor per-cell) | D | LOW | artem already supports truecolor; v1 shipped monochrome. Gate behind `is_color_on()`. |
| `--braille` (2×4 dot density) | D | MEDIUM | Each Braille glyph packs a 2×4 = 8-pixel cell → ~8× the resolution of one ASCII char. Threshold to binary, map 2×4 blocks to the Unicode Braille Patterns block (U+2800+). Needs a threshold parameter. |
| `--invert` (flip dark/light mapping) | T (with braille/mono) | LOW | Essential companion for light-vs-dark terminals; one boolean that flips the ramp/threshold. |

### Fun group (FUN-V2)

**cowsay**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Multiple figures (`-f tux\|dragon\|…`) | D | LOW | Bundle a handful of `.cow`-equivalent ASCII templates; keep the existing 40-col wrap + bubble engine, only the figure swaps. |
| Think-mode (`--think`, thought-bubble `o` connectors) | D | LOW | Classic cowthink; swap `\` tail glyphs for `o`. |

**fortune**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Categories (`--category dev\|wisdom\|…`, `--list-categories`) | D | LOW | Partition the bundled list into tagged buckets. |

**8ball**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| ASCII-art ball | D | LOW | Static art frame around the answer. |
| Sentiment color (affirmative=green / non-committal=yellow / negative=red) | D | LOW | Map the canonical 20 answers to 3 sentiment classes; gate color on `is_color_on()`. |

**roast**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--language <lang>` | D | LOW | Tag the bundled roast list by language; filter. Curated local lists only — **no network/AI** (v1 out-of-scope; keep it). |

### System group (SYS-V2)

**pomodoro**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| Session counter | T | LOW | Display "Pomodoro 3 of 4". |
| Auto-break + auto-long-break cadence | D | MEDIUM | **Canonical cadence: 25 work / 5 short break, long break (15–30 min) after every 4 pomodoros.** Auto-start of the next interval is an app feature (not part of the original technique) — offer it behind `--auto` so it stays scriptable; emit a toast at each transition (reuse v1 tauri-winrt-notification). Keep the v1 clean Ctrl+C/q/Esc cancel + RAII restore. |
| `--label "task"` (shown in countdown + toast) | D | LOW | String into the existing countdown + toast text. |
| Sound on completion | D | MEDIUM | Windows system beep / `.wav` via the OS. Keep optional (`--sound`); headless/CI shouldn't beep. |

**weather**

| Feature | Cat | Cx | Notes |
|---------|-----|----|-------|
| `--forecast [N]` (N-day) | D | MEDIUM | Open-Meteo returns a daily forecast in the same keyless call; parse the `daily` block. Read unit labels from the response (v1 already does — never hardcode). |
| `--json` (`{location,current,forecast[]}`) | T | LOW | Scalar+nested object. |
| Response cache (avoid re-hitting the API for the same place within N min) | D | MEDIUM | Cache file in the config dir keyed by location+units with a TTL; cache miss → fetch. Graceful offline already handled. |
| Stored / default location (no-arg `box weather`) | D | LOW | Depends on the config-file feature below. |

---

## PART 4 — Frictionless PS7 (meta-commands)

### `config` + config-file defaults

**Verified precedence convention (universal across Terraform/AWS/Docker/bat):**

```
command-line flag   (highest — always wins)
  └─ environment variable        (NO_COLOR etc. already honored)
       └─ config file value
            └─ built-in default  (lowest)
```

| Aspect | Cat | Cx | Notes |
|--------|-----|----|-------|
| Flag > env > config > default precedence | T | LOW | The expected, non-surprising order. box already honors `NO_COLOR`/`--no-color` — slot config between env and default. |
| `box config` get/set/list + a TOML file in the OS config dir | T | MEDIUM | `directories`/`%APPDATA%\box\config.toml`. Merge file defaults before clap parses flags. |
| Keys similar tools expose | — | — | `hash.default_algo` (blake3\|sha256\|…), `weather.units` (metric\|imperial), `weather.location` (stored default), `color` (on\|off\|auto), extensible (`uuid.default_version`, `passgen.length`). Mirror bat/AWS sectioned keys. |
| Interactive config wizard / TUI | A | — | Scope creep; `config set k v` is enough (PROJECT.md: one-shot commands only). |

### `completions powershell`

| Aspect | Cat | Cx | Notes |
|--------|-----|----|-------|
| `box completions powershell` emits a PS7 completion script | T | LOW | `clap_complete` PowerShell generator emits a `Register-ArgumentCompleter -Native` script. Near-free since the whole CLI is clap-derive. |
| Completes subcommands + flag names | T | LOW | clap_complete handles both; PS7 only suggests flags after a `-` is typed (PS-native behavior, expected). |
| Tooltips from `--help` short text | D | LOW | clap_complete uses the short help as PS tooltip — free from existing `about` strings. |
| Value hints (e.g. `--algo` → blake3/sha256) | D | LOW–MED | clap `value_parser`/`PossibleValue` enums auto-feed completion candidates (also improves `--help`). Wire algo/units/version as enums. |
| Hand-written/maintained completion script | A | — | Generating from the clap tree is the only sane path; a hand-kept script drifts. |

---

## Feature Dependencies

```
config-file feature
    └──enables──> weather stored/default location (no-arg `box weather`)
    └──enables──> hash default_algo override (interacts with the BLAKE3-default breaking change)

--json house style (Part 1)
    └──required by──> every command's --json variant (must land as a shared convention FIRST,
                       or each command invents its own shape → the anti-feature)

arboard clipboard infra (v1)
    └──reused by──> --clip on passgen/uuid/color/hash/qr-text/base64/epoch/json

matrix RAII terminal-restore pattern (v1)
    └──reused by──> lolcat --animate (same raw-mode-stuck risk the v1 review caught)

`ignore` crate (v1 dupes/bulk-rename)
    └──reused by──> tree .gitignore respect

chrono (v1 epoch) ──enhanced by──> chrono-humanize (relative time), chrono-tz (timezones, NEW dep)

flatten --move ──shares safety model with──> bulk-rename (abort-before-any pre-flight, create_new loud-fail)

dupes --delete ──conflicts with──> "scriptable, no interactive prompts" principle
                                    (resolvable only via non-interactive --keep policy + --force)
```

### Dependency notes

- **`--json` house style must be decided before any per-command `--json` work** — it's the shared contract. Ad-hoc JSON loses the consistency that is the entire point of the scriptable spine.
- **BLAKE3-default `hash` interacts with config** — a user's `hash.default_algo = "sha256"` should restore old behavior; document that the *breaking* default is overridable, easing migration.
- **`--clip`, `tree .gitignore`, `lolcat --animate`, `epoch` relative all reuse existing infra** — low marginal cost, which is why they're differentiators rather than table-stakes.
- **`chrono-tz` is the only meaningful new dependency** (timezone support); everything else reuses crates already in `Cargo.toml`.

---

## MVP / phasing recommendation (for REQUIREMENTS scoping)

### Land first (the spine — everything else leans on it)
- [ ] `--json` house style decided + a shared serialize/print helper — **the contract**.
- [ ] `--clip` shared helper (copy + print + stderr confirm), wired to the textual commands.
- [ ] `completions powershell` (near-free, high frictionless-PS7 payoff).
- [ ] `config` + config file with flag>env>config>default precedence.

### High-value depth (mostly LOW complexity, reuse existing infra)
- [ ] hash: BLAKE3-default (breaking) + multi-file + `--json` + coreutils double-space format.
- [ ] tree: `.gitignore` + dirs-only + sort-by-size + `--json` tree object.
- [ ] du: percentage bars + color ranges + `--exclude` + `--json` + apparent-size default.
- [ ] passgen: entropy bits + `--no-similar (il1Lo0O)` + `--separator` + `--clip`.
- [ ] uuid v7 + format flags; epoch relative-time; color CSS-names; json `--sort-keys`.
- [ ] flatten filters/separator/progress/`--json`; bulk-rename case+numbering+`--backup`.

### Guard carefully (safety / unscriptability risk)
- [ ] flatten `--move` (copy-then-delete-after-verify, never clobber).
- [ ] dupes `--delete` (non-interactive `--keep first` + `--force` + dry-run default, or keep read-only).
- [ ] lolcat `--animate` (RAII restore, no-op when piped).

### Visual/fun polish (defer freely — LOW complexity, LOW urgency)
- [ ] matrix color/speed/charset; qr `--save`/EC; ascii color/braille/invert.
- [ ] cowsay figures/think; fortune categories; 8ball art/sentiment; roast `--language`.
- [ ] pomodoro counter/auto-break/`--label`/sound; weather forecast/cache/stored-location.

---

## Anti-Features summary (the "do NOT build" list — keeps scope honest)

| Anti-feature | Why requested | Why problematic | Instead |
|--------------|---------------|-----------------|---------|
| **NDJSON / JSON Lines** for box's `--json` | ripgrep does it; "streaming" sounds modern | `ConvertFrom-Json` can't consume it — needs per-line `ForEach-Object`; breaks the one-liner. box has no unbounded streams. | Single array (multi) / object (scalar) — one `ConvertFrom-Json` call. |
| **Per-command bespoke JSON schemas** | each command "knows its data best" | inconsistency defeats the scriptable-spine goal | one house style, enforced in review. |
| **Interactive prompts in dupes `--delete` / bulk-rename / any command** | feels safer ("confirm each") | breaks piping, breaks the 0/1/2 exit contract, unscriptable | dry-run-by-default + explicit `--force`/`--keep` policy; abort-before-any. |
| **`--clip` on matrix/pomodoro/lolcat** | "every command should have it" | no coherent textual result to copy | only expose `--clip` where a single result string exists. |
| **bulk-rename undo log / dupes auto-delete-by-default** | convenience | adds state to stateless tools / data-loss risk | dry-run-first IS the undo; never delete without explicit opt-in. |
| **flatten `--move` that deletes before confirming the copy** | speed | data loss on cross-volume/partial write | rename-same-volume else copy-then-delete-after-verify. |
| **Config wizard / interactive TUI; qr decode; jq query language; network-backed fun toys; ascii video** | feature-rich | scope creep; v1 already drew these boundaries | `config set`; print-only; pretty+validate only; curated local lists; still images. |
| **Sorting `json` keys by default** | "tidy output" | breaks v1's identity-preserving `preserve_order` contract | `--sort-keys` opt-in only. |

---

## Sources

JSON / PowerShell consumption (the load-bearing convention):
- [ConvertFrom-Json — Microsoft Learn (PS 7.6)](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertfrom-json?view=powershell-7.6) — array enumerates to one PSCustomObject per element; `-NoEnumerate`; single-string input (no native NDJSON). **HIGH**
- [ripgrep --json (grep-printer JSON)](https://docs.rs/grep-printer/) + [rg issue #930](https://github.com/BurntSushi/ripgrep/issues/930) — NDJSON, typed messages, text/bytes wrapping. **HIGH**
- [gh formatting manual](https://cli.github.com/manual/gh_help_formatting) — `--json` → array of objects, pretty to TTY. **HIGH**
- [hyperfine README / man](https://github.com/sharkdp/hyperfine) — `--export-json` single root object with results[]. **HIGH**
- [dust (bootandy/dust)](https://github.com/bootandy/dust) — `dust -j` single nested tree object (exact fields unpublished). **MEDIUM**
- [eza issue #1064 "add JSON output" — not planned](https://github.com/eza-community/eza/issues/1064) — negative signal. **HIGH**
- [PS NDJSON per-line parsing discussion](https://github.com/MicrosoftDocs/PowerShell-Docs/issues/3543) — confirms ForEach-Object per-line for JSON Lines. **HIGH**

Clipboard / config / completions:
- [Shell copy-to-clipboard conventions — Adam Johnson](https://adamj.eu/tech/2023/10/30/shell-copy-to-clipboard/) — pbcopy/clip/wl-copy silent-sink model. **HIGH**
- [CLI config precedence — cli-guidelines #110](https://github.com/cli-guidelines/cli-guidelines/issues/110), [bat #1152](https://github.com/sharkdp/bat/issues/1152), [Terraform env vars](https://developer.hashicorp.com/terraform/cli/config/environment-variables) — flag>env>config>default. **HIGH**
- [clap_complete PowerShell generator (docs.rs source)](https://docs.rs/clap_complete/latest/src/clap_complete/aot/shells/powershell.rs.html) — Register-ArgumentCompleter, tooltips from short help. **HIGH**

Per-command norms:
- [sha256sum man / coreutils](https://man7.org/linux/man-pages/man1/sha256sum.1.html) + [GNU sha2-utilities](https://www.gnu.org/software/coreutils/manual/html_node/sha2-utilities.html) — two-space `digest␣␣file`, `-c` verify. **HIGH**
- [Bulk Rename Utility manual](https://www.s3.tgrmn.com/bru4/BRU_Manual.pdf) + [PowerRename docs](https://winsides.com/how-to-batch-rename-files-using-powerrename-windows/) — numbering tokens, padding, case transforms. **HIGH**
- [Password entropy + exclude-ambiguous (il1Lo0O ≈ 0.14 bits/char)](https://dev.to/snappy_tools/password-security-explained-why-length-beats-complexity-and-how-entropy-works-16j7), [1Password community: exclude similar](https://www.1password.community/discussions/developers/password-generator-to-exclude-similar-characters/81490). **HIGH**
- [UUID v7 time-ordered vs v4](https://helppdev.com/en/blog/uuidv7-vs-uuidv4-why-time-ordered-identifiers-are-taking-over) — 48-bit ms prefix, sortable, 2–10× DB insert. **HIGH**
- [Pomodoro Technique — Wikipedia](https://en.wikipedia.org/wiki/Pomodoro_Technique) — 25/5, long break after 4. **HIGH**
- [qrencode man](https://manpages.ubuntu.com/manpages/noble/man1/qrencode.1.html) — `-t PNG/SVG/...`, `-l L|M|Q|H`, `-o FILE`. **HIGH**
- [Image→Braille (TheZoraiz/ascii-image-converter)](https://github.com/TheZoraiz/ascii-image-converter) — 2×4 dot cell, threshold, invert, U+2800 block. **HIGH**
- [chrono-humanize / timeago crates](https://docs.rs/chrono-humanize) — relative-time formatting. **HIGH**

---
*Feature research for: scriptable single-binary Rust CLI toolbox (box v2.0), Windows PowerShell 7*
*Researched: 2026-06-24*
