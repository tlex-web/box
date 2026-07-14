# Phase 7: Spine Rollout - Pattern Map

**Mapped:** 2026-06-25
**Files analyzed:** 16 command `mod.rs` + 16 test files (15 existing `tests/<cmd>.rs` + 1 NEW `tests/cowsay.rs`)
**Analogs found:** 16 / 16 (every command maps to the `uuid`/`hash` pilot or the spine primitives; only the field schema + 4 surprises are per-command work)

> **This is a mechanical, additive rollout.** Phase 7 invents no new pattern. Every target file copies ONE of:
> - **`src/commands/uuid/mod.rs`** — the multi-capable pilot (`{results,count}` + `is_json_on()` fork + `out_line`).
> - **`src/commands/hash/mod.rs`** — the path-bearing pilot (`HashRow{path,algo,digest}` + serde enum `rename_all="lowercase"`).
> - **`src/core/output.rs`** — the spine it CONSUMES (`is_json_on`, `emit_json`, `out_line`, `flush_clip`, `is_color_on`, `human_size`, `format_row`, `RowStatus`, `dry_run_summary`/`real_run_summary`).
>
> Planner: each plan's action = "copy the fork shape from the analog, give the *already-computed* value a `#[derive(Serialize)]` struct, fork `is_json_on()` FIRST, route the human branch through `out_line`, copy the `json_purity` test." The only thinking is the field schema (mostly locked by D-17) and the 4 surprises flagged below.

---

## The Canonical Analogs (READ THESE FIRST — every assignment refers back here)

### Multi-capable pilot — `src/commands/uuid/mod.rs`

The literal copy-me template for `passgen` + all 4 filesystem commands.

**Struct pair** (`uuid/mod.rs:26-39`):
```rust
#[derive(serde::Serialize)]
struct UuidRow { uuid: String, version: &'static str }
#[derive(serde::Serialize)]
struct UuidOutput { results: Vec<UuidRow>, count: usize }
```

**The fork — compute once, fork FIRST, human path via `out_line`** (`uuid/mod.rs:59-81`):
```rust
let rows: Vec<UuidRow> = /* compute once */;
if crate::core::output::is_json_on() {                 // fork FIRST (Pitfall 1)
    let doc = UuidOutput { count: rows.len(), results: rows };
    crate::core::output::emit_json(&doc)?;             // ONLY stdout write under --json
} else {
    for r in &rows {
        crate::core::output::out_line(&r.uuid);        // NOT println! — tees to clip
    }
}
Ok(())
```

### Path-bearing pilot — `src/commands/hash/mod.rs`

Reference for the single-row-but-wrapped shape and the serde enum.

**Serde enum** (`hash/mod.rs:88-99`): `#[derive(... serde::Serialize, serde::Deserialize)]` + `#[serde(rename_all = "lowercase")]` → `Algo::Blake3` serializes to `"blake3"`. Copy this for `flatten`/`bulk-rename`'s `action` enum.

**Row + fork** (`hash/mod.rs:115-130, 266-282`):
```rust
#[derive(serde::Serialize)]
struct HashRow { path: String, algo: Algo, digest: String }
// ...
if crate::core::output::is_json_on() {
    let doc = HashOutput { count: 1, results: vec![HashRow { path: label.clone(), algo, digest: computed }] };
    crate::core::output::emit_json(&doc)?;
} else {
    crate::core::output::out_line(&format!("{computed}  {label}"));
}
```
Note `path: label.clone()` — `label` is already a lossy-safe `String`; **never `to_str().unwrap()`** on a path (D-4).

### Spine primitives — `src/core/output.rs` (CONSUME, do not modify — except the one qr exception)

| Primitive | Line | Use |
|-----------|------|-----|
| `is_json_on() -> bool` | `output.rs:93` | Fork gate — check FIRST. |
| `emit_json<T: Serialize>(&T) -> Result<()>` | `output.rs:124` | The ONLY sanctioned primary-output serializer; no-BOM, trailing `\n`, tees whole doc to clip under `--clip`. |
| `out_line(&str)` | `output.rs:143` | Replaces the human `println!`; prints + tees the line to `CLIP_BUF`. |
| `flush_clip()` | `output.rs:160` | Called by `main.rs` (NOT per command). |
| `is_color_on() -> bool` | `output.rs:34` | Existing per-command color gate — UNCHANGED. |
| `human_size(u64) -> String` | `output.rs:314` | Size formatting (human path only). |
| `format_row(...)` | `output.rs:225` | Human row layout — **never serialize its output**; serialize raw fields. |
| `RowStatus {Copy,Rename,Skip}` + `.glyph()` | `output.rs:180-199` | flatten/bulk-rename `action` source of truth (lowercase the variant name for JSON). |
| `dry_run_summary` / `real_run_summary` | `output.rs:344,357` | Human summary strings (human path only). |

> ⚠ **`qr` (D-15) is the ONE place Phase 7 may add a primitive here:** a `clip_feed(&str)` (or equivalent) that pushes to `CLIP_BUF` ONLY when `--clip` is on, **without printing** — because `out_line` cannot express "print glyphs X, copy text Y". Mirror `out_line`'s tee half (`output.rs:145-149`) and the `out_line_tees` unit test (`output.rs:490`). See `qr` assignment + Shared Pattern E.

### Test templates — `tests/uuid.rs`

| Template | Line | Copy for |
|----------|------|----------|
| `json_purity` | `tests/uuid.rs:135-179` | All 16 commands (adapt the shape assertion to each schema). |
| `json_count_multi` | `tests/uuid.rs:184-209` | `passgen` (`--count N` → N results). |
| `human_output_unchanged` | `tests/uuid.rs:214-230` | Regression backstop for any command whose human render changes (`du`/`tree`/etc. keep theirs byte-stable). |
| `clip_roundtrip` (`#[ignore]`) | `tests/uuid.rs:237-269` | The 6 SPINE-04 commands. `qr`'s variant asserts `pasted == INPUT`, not the glyphs (D-15). |

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality | Surprise? |
|-------------------|------|-----------|----------------|---------------|-----------|
| `src/commands/base64/mod.rs` | command (transform) | transform | `hash` (scalar shape) | role-match | ⚠ non-UTF-8 decode (A1/Pitfall 6) |
| `src/commands/epoch/mod.rs` | command (transform) | transform | `hash` (scalar) | role-match | unify 3 modes → one shape (D-17) |
| `src/commands/color/mod.rs` | command (transform) | transform | `uuid` (struct pair) | role-match | nested sub-objects (D-17) |
| `src/commands/passgen/mod.rs` | command (generator) | batch (N lines) | `uuid` (multi-capable) | **exact** | none — pure copy |
| `src/commands/eight_ball/mod.rs` | command (random pick) | transform | `hash` (scalar) | role-match | none |
| `src/commands/fortune/mod.rs` | command (random pick) | transform | `hash` (scalar) | role-match | emit UNWRAPPED string |
| `src/commands/roast/mod.rs` | command (random pick) | transform | `hash` (scalar) | role-match | emit UNWRAPPED string |
| `src/commands/cowsay/mod.rs` | command (transform) | transform | `hash` (scalar) | role-match | mixed print!/println!; --json-only (no clip) |
| `src/commands/du/mod.rs` | command (fs read) | CRUD / buffered rows | `uuid` (`{results,count}`) | **exact (model exists)** | none — `Row` already buffered+sorted |
| `src/commands/tree/mod.rs` | command (fs read) | recursive walk | `uuid` (shape) + D-17 recursion | role-match | ⚠ NEW recursive node builder (A4) |
| `src/commands/dupes/mod.rs` | command (fs read) | CRUD / buffered groups | `uuid` (`{results,count}`) | **exact (model exists)** | none — `DupeGroup` exists |
| `src/commands/flatten/mod.rs` | command (fs write) | batch / plan-then-execute | `uuid` + D-12/D-13 | role-match | dry_run flag + plan projection |
| `src/commands/bulk_rename/mod.rs` | command (fs write) | batch / plan-then-execute | `uuid` + D-12/D-13 | role-match | ⚠ --force emits rows; abort→empty stdout (A3/Pitfall 5) |
| `src/commands/json/mod.rs` | command (transform) | identity passthrough | `core::output::emit_json` direct | partial (root-rule exception) | ⚠ D-16 passthrough; --clip routes plain path through out_line |
| `src/commands/qr/mod.rs` | command (transform) | encode | `hash` (scalar) + spine | partial | ⚠ D-14 metadata not glyphs; D-15 clip copies text (A2/Pitfall 4) |
| `src/commands/weather/mod.rs` | command (HTTP fetch) | request-response | `hash` (scalar) | role-match | f64 fields; unit from `current_units` (Pitfall WTHR-3) |

**Test files (Wave 0 / per-plan):** 15 existing `tests/<cmd>.rs` get a `json_purity` (+ `clip_roundtrip` for the 6 SPINE-04). **NEW: `tests/cowsay.rs`** — cowsay has only `tests/cmd/*.trycmd` today (confirmed: no `tests/cowsay.rs` on disk).

---

## Pattern Assignments

### Wave 7a — pure transforms

#### `src/commands/base64/mod.rs` (transform, scalar) ⚠ SURPRISE

**Analog:** `hash` (scalar flat object). **Schema (discretion):** `{output, mode}` (+ optional `input`); `mode` = `"encode"`/`"decode"`.

**Current compute vs render** (`base64/mod.rs:48-58`):
```rust
if self.decode {
    let decoded = decode(&bytes, self.url_safe)?;
    std::io::stdout().write_all(&decoded)?;            // raw BYTES — binary-safe, may be non-UTF-8
} else {
    println!("{}", encode(&bytes, self.url_safe));     // ASCII — always safe
}
```
- **Encode path:** trivial — fork, `out_line(&encoded)` on human, `emit_json(&Base64Output{ output, mode:"encode" })` on json.
- ⚠ **Decode path (A1 / Pitfall 6):** decoded bytes can be arbitrary non-UTF-8; a JSON string can't hold them. Planner must pick a policy: (a) under `--json`, emit decode output as a base64 string field (lossless, round-trippable — **recommended**); or (b) `to_string_lossy` + documented marker; or (c) refuse non-UTF-8 decode under `--json` (exit 1, empty stdout). **`--clip` of binary decode** is also undefined (the line-oriented `out_line` can't carry binary) — decide alongside.

---

#### `src/commands/epoch/mod.rs` (transform, scalar)

**Analog:** `hash` (scalar). **Schema (D-17 LOCKED):** unified `{epoch, utc, local}` for ALL three input modes — **no branching on input direction**.

**Current compute vs render — three divergent print sites** (`epoch/mod.rs:43-60`):
```rust
None        => println!("{}", Utc::now().timestamp()),          // now mode: 1 int line
Some(int)   => { let (local_line, utc_line) = format_timestamp(secs)?;
                 println!("{local_line}"); println!("{utc_line}"); }  // 2 labeled lines
Some(date)  => println!("{}", parse_date(s)?),                  // ts only
```
**The cost:** compute `epoch: i64` + `utc: String` + `local: String` ONCE regardless of mode (now → epoch=now; int → epoch=int; string → epoch=parsed), then derive utc/local from epoch. Human path keeps its mode-specific lines via `out_line`; JSON path emits the one unified struct. Reuse the existing `DateTime::from_timestamp` / `with_timezone(&Local)` math from `format_timestamp` (`epoch/mod.rs:99-105`). **Not in SPINE-04** (no `--clip`), but route human through `out_line` anyway for consistency.

---

#### `src/commands/color/mod.rs` (transform, scalar) — SPINE-04

**Analog:** `uuid` (nested struct pair). **Schema (D-17 LOCKED):** `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` — nested sub-objects (PS7 `.rgb.r`).

**Current compute vs render** (`color/mod.rs:39-58`):
```rust
let (r, g, b) = parse_color(raw.trim())?;
let (h, s, l) = rgb_to_hsl(r, g, b);
println!("  Hex   : #{r:02X}{g:02X}{b:02X}");   // NOTE: current hex is UPPERCASE
println!("  RGB   : rgb({r}, {g}, {b})");
// ... Tuple, HSL lines, then the swatch (gated on is_color_on) ...
```
**The cost:** add `Rgb{r,g,b}` + `Hsl{h,s,l}` + `ColorOutput{hex,rgb,hsl}` structs. The `(r,g,b)`/`(h,s,l)` are already computed — feed both paths. Swatch is display-only → omit from JSON. **Lock the hex case** in the struct (current human render is `#{r:02X}` = uppercase; discretion to keep or lowercase — pin one for a deterministic test). Human path → `out_line` (SPINE-04 clip tees the block).

---

#### `src/commands/passgen/mod.rs` (generator, batch) — SPINE-04 — **EXACT MATCH**

**Analog:** `uuid` verbatim (multi-capable, `--count` N lines). **Schema:** `{results:[{password}], count}` (row field name discretion).

**Current compute vs render — two loops, both `println!`** (`passgen/mod.rs:101-128`):
```rust
if let Some(n) = self.words {
    for _ in 0..self.count { /* phrase */ println!("{}", phrase.join(".")); }
} else {
    for _ in 0..self.count { /* pw */ println!("{pw}"); }
}
```
**The cost:** collect each generated line into `Vec<PassgenRow>` (mirroring `uuid`'s `(0..count).map(...).collect()`), then the exact `uuid` fork. Human path → `out_line` per line. ⚠ **Security note (not new code):** `passgen --clip` copies a secret — already opt-in (user typed `--clip`); the tee is automatic once the human path uses `out_line`. Copy `tests/uuid.rs::json_count_multi` for the N>1 case.

---

#### `src/commands/eight_ball/mod.rs` (random pick, scalar)

**Analog:** `hash` (scalar). **Schema (discretion):** `{text}` (or `{answer}`).

**Current compute vs render** (`eight_ball/mod.rs:74-77`):
```rust
let answer = *EIGHT_BALL_ANSWERS.choose(&mut rng).expect(...);
println!("{answer}");
```
**The cost:** trivial — fork, `emit_json(&{text: answer})` / `out_line(answer)`. The `question` arg is display-only/ignored (`:69`) — **do not include it** in JSON. Not in SPINE-04.

---

#### `src/commands/fortune/mod.rs` (random pick, scalar)

**Analog:** `hash` (scalar). **Schema (discretion + LOCKED behavior):** `{text}` — **emit the UNWRAPPED `chosen` string** (wrapping is human-only).

**Current compute vs render — wrap happens ONLY in the human branch** (`fortune/mod.rs:43-54`):
```rust
let chosen = *list.choose(&mut rng).expect(...);
let width = crate::core::output::terminal_width();
if chosen.chars().count() <= width { println!("{chosen}"); }
else { for line in soft_wrap(chosen, width) { println!("{line}"); } }   // wrap = human concern
```
**The cost:** fork BEFORE the width/wrap logic — `emit_json(&{text: chosen})` emits `chosen` verbatim; the human branch keeps its existing soft-wrap printing (route through `out_line` for consistency, though not in SPINE-04).

---

#### `src/commands/roast/mod.rs` (random pick, scalar)

**Analog:** `fortune` (identical shape) → `hash`. **Schema:** `{text}` unwrapped. Same fork-before-wrap as fortune (`roast/mod.rs:38-48`). Pure copy of the fortune assignment.

---

#### `src/commands/cowsay/mod.rs` (transform, scalar) ⚠ SURPRISE (mixed print) — **--json-only, NO clip**

**Analog:** `hash` (scalar). **Schema (A6, discretion):** `{text}` = the spoken message (the ASCII bubble/cow is a *visual*, like qr glyphs).

**Current compute vs render — THREE print calls incl. a bare `print!`** (`cowsay/mod.rs:57-61`):
```rust
let lines = wrap(raw.trim(), self.width);
println!("{}", bubble(&lines));    // bubble
print!("{COW}");                   // bare print! for the cow
println!();
```
**The cost:** fork FIRST — under `--json`, emit `{text: <raw input or wrapped message>}` (recommend the raw input string; A6). Because cowsay is **NOT in SPINE-04**, the human bubble path may keep `println!`/`print!` as-is (no clip tee required — `out_line` not needed). **NEW test file required:** create `tests/cowsay.rs` with the `json_purity` test (no `tests/<cmd>.rs` exists today — only `tests/cmd/*.trycmd`).

---

### Wave 7b — filesystem buffered-rows

#### `src/commands/du/mod.rs` (fs read, buffered rows) — model already exists

**Analog:** `uuid` (`{results,count}`, D-11). **Schema:** `{results:[{name, size, is_dir}], count}` + optional sibling `total_bytes`/`total_children` (discretion).

**Existing data model — already collected + sorted before render** (`du/mod.rs:67-72, 96-128`):
```rust
struct Row { name: String, is_dir: bool, size: u64 }       // :67 — add #[derive(Serialize)]
// ...
let mut rows = collect_rows(&root, self.depth)?;           // :96 buffered
let total: u64 = rows.iter().map(|r| r.size).sum();        // :101 sibling candidate
let total_children = rows.len();                            // :102 sibling candidate
sort_rows(&mut rows);                                       // :106 sorted BEFORE render
// human render loop:
for (row, size_str) in rows.iter().zip(...) {
    println!("{size_col}  {}{slash}", row.name);           // :127 + blank + summary :132-136
}
```
**The cost:** add `#[derive(serde::Serialize)]` to `Row` (rename/confirm `is_dir` vs a `type` string — discretion). Fork FIRST (Pitfall 1 — `du` has THREE human stdout writes: rows + blank + summary, all must be behind the `else`). `size: u64` bare per D-3 (document the >2^53 caveat). Empty dir under `--json` → `{results:[], count:0}`, NOT the human summary line.

---

#### `src/commands/tree/mod.rs` (fs read, recursive walk) ⚠ SURPRISE (A4 — biggest 7b cost)

**Analog:** D-17 recursive node (root-rule EXCEPTION, not `{results,count}`). **Schema (D-17 LOCKED):** `{name, type:"dir"|"file", size?, children:[]}` recursively (`size` for files only).

**Current model is a FLAT printing recursion — NO node tree is ever built** (`tree/mod.rs:64-71, 119-179`):
```rust
struct Child { name: String, is_dir: bool, size: Option<u64>, path: PathBuf }  // :66 per-LEVEL only
// render_dir walks + println!s as it descends — never assembles a tree:
println!("{prefix}{branch}{name}{size_col}");   // :158 — inline print, no return value
```
**The cost (REAL new work):** write a parallel `fn build_node(dir) -> Node` that recurses and collects `children: Vec<Node>`, using the **same `read_children` + `sort_children`** (`tree/mod.rs:184-224`) the printer uses (no-drift). `Node{ name, type, size: Option<u64>, children: Vec<Node> }`; `type` = `"dir"`/`"file"`; root node = the target dir (its label is `self.path` per `:102`). Fork FIRST: under `--json` build + `emit_json(&root_node)`; else the existing `render_dir`. Not in SPINE-04.

---

#### `src/commands/dupes/mod.rs` (fs read, buffered groups) — model already exists

**Analog:** `uuid` shape + D-17. **Schema (D-17 LOCKED):** `{results:[{size, paths:[…]}], count, wasted_bytes}`.

**Existing data model — grouped + sorted before `render`** (`dupes/mod.rs:53-56, 106-108, 199-233`):
```rust
struct DupeGroup { size: u64, paths: Vec<PathBuf> }    // :53 — add #[derive(Serialize)] + paths→to_string_lossy
// ...
let groups = group_duplicates(hashed);                 // :106
render(&groups);                                        // :108 human-only
// wasted_space(groups) already computes the sibling field: :199
```
**The cost:** add serde to `DupeGroup`; `paths: Vec<PathBuf>` must serialize via `to_string_lossy()` per D-4 (NOT `to_str().unwrap()`). `count` = number of groups; `wasted_bytes` = existing `wasted_space(&groups)` (`:199`). Fork FIRST — `render` (`:210-233`) has the empty-case "No duplicate files found." human line (`:212`) + group lines + summary, ALL human chrome; empty under `--json` → `{results:[], count:0, wasted_bytes:0}`. Not in SPINE-04.

---

#### `src/commands/flatten/mod.rs` (fs write, plan-then-execute) ⚠ D-12/D-13

**Analog:** `uuid` (`{results,count}`) + D-12/D-13 sibling fields. **Schema:** `{results:[{src, dst, action, reason}], count, dry_run:bool, copied, renamed, skipped, total_bytes}`.

**Existing data model — ONE plan feeds both dry-run + real-run already** (`flatten/mod.rs:48-91, 138-202`):
```rust
enum ItemKind { Copy, Rename, Skip }                                  // :50 → action string
impl ItemKind { fn status(self) -> RowStatus { ... } }               // :60 (reuse, lowercase the variant)
struct PlanItem { src: PathBuf, src_label: String, dst_name: Option<String>, kind: ItemKind, reason: Option<String> }  // :72
struct Plan { items: Vec<PlanItem>, to_copy: usize, renamed: usize, skipped: usize }  // :86
// dry-run branch :138-146 (print_plan + dry_run_summary);
// real branch computes copied + bytes_written ONLY on execution :150-151, 175-176
```
**The cost (projection, not new model):**
- D-12: `--json` is orthogonal to `--force`/dry-run — dry-run+json → the PLAN; real+json → the executed result (capture the real-path `copied`/`bytes_written` at `:150-151,175-176`).
- D-13: project each `PlanItem` → JSON row `{src, dst, action, reason}` — rename `dst_name`→`dst` (`None` for skips), `kind`→`action` (lowercase `RowStatus`: `"copy"`/`"rename"`/`"skip"` — copy hash's `rename_all="lowercase"` enum idiom). `dry_run` from `self.dry_run`. Summary counts (`to_copy`/`renamed`/`skipped`, real-run `copied`/`total_bytes`) as siblings.
- ⚠ **Do NOT serialize `format_row` output** (`:155-187`) — that is human layout; serialize the raw fields. Not in SPINE-04.

---

#### `src/commands/bulk_rename/mod.rs` (fs write, plan-then-execute) ⚠ SURPRISE (A3/Pitfall 5)

**Analog:** `flatten` (sibling shape) → `uuid` + D-12/D-13. **Schema:** `{results:[{src, dst, action, reason}], count, dry_run, renamed/to_rename, unchanged, skipped}`.

**Existing data model** (`bulk_rename/mod.rs:98-115, 480-487`):
```rust
struct PlanItem { src, parent, old_name, src_label, new_name: Option<String>, kind: ItemKind, reason: Option<String> }  // :101
struct Plan { items: Vec<PlanItem>, to_rename, unchanged, skipped }  // :482
```
**Two behavioral forks the planner MUST specify:**
1. ⚠ **D-12 override** — `bulk-rename --force --json` MUST emit the applied renames, **overriding** the current silent-on-success (`:380-381` only prints a "Done:" summary). The human `--force` path STAYS silent; only `--json` emits rows. Project `PlanItem` → `{src, dst (=new_name), action, reason}`.
2. ⚠ **A3 / Pitfall 5 — abort path must keep stdout EMPTY (D-09).** The current conflict path prints the plan-with-conflicts to **stdout** then `bail!`s (`:317-322`):
   ```rust
   if !conflicts.is_empty() {
       print_plan_with_conflicts(&plan, &conflicts, arrow_col, width);   // :320 → stdout (WRONG under --json)
       println!();
       bail!("{}", abort_summary(&conflicts));                            // :322
   }
   ```
   Under `--json`, this fork must NOT write to stdout — keep stdout empty, send the conflict explanation to stderr (or just the `bail!` error), exit 1. Lock with a `json_abort_empty_stdout` test. Not in SPINE-04.

---

### Wave 7c — odd-fits

#### `src/commands/json/mod.rs` (transform, identity passthrough) ⚠ D-16 — SPINE-04

**Analog:** **`core::output::emit_json` directly** (root-rule EXCEPTION — the ONE sanctioned direct-serde command). **Schema (D-16 LOCKED):** identity passthrough — emit the parsed `Value` VERBATIM, NOT wrapped in `{results,count}`.

**Current compute vs render — THREE human paths, none through `out_line`** (`json/mod.rs:51-77`):
```rust
match serde_json::from_str::<Value>(&text) {
    Err(e) => anyhow::bail!("at line {} column {}: {e}", e.line(), e.column()),  // :60 invalid → exit 1, empty stdout (UNCHANGED, D-09)
    Ok(value) => {
        if self.compact      { println!("{}", serde_json::to_string(&value)?); }       // :64
        else if is_color_on(){ print!("{}", colorize(&value, 0)); }                    // :69 colored
        else                 { println!("{}", serde_json::to_string_pretty(&value)?); } // :73 plain
    }
}
```
**The cost:**
- Under `--json`: `emit_json(&value)` (pure, no-BOM, trailing `\n`). `--json --clip` copies that doc via `emit_json`'s tee.
- ⚠ **`--clip` WITHOUT `--json` (SPINE-04):** none of the three human paths feed the clip buffer today. Under `--clip`, `init_output` forces `COLOR_ON=false` (`output.rs:109-112`), so the `colorize` branch (`:69`) is never taken — the plain `to_string_pretty` branch (`:73`) runs; **route THAT through `out_line`**. `--compact --clip` → copy the compact form (route `:64` through `out_line` too). The `value` it already holds IS the document — no new model.

---

#### `src/commands/qr/mod.rs` (transform, encode) ⚠ SURPRISE D-14 + D-15 (A2/Pitfall 4) — SPINE-04

**Analog:** `hash` (scalar) for the JSON struct; **the one `core::output` PRIMITIVE ADDITION** for clip. **Schema (D-14 LOCKED):** `{text, error_correction}` — METADATA, not glyphs. `error_correction` = the fixed `"M"` literal (`qr/mod.rs:80`).

**Current compute vs render** (`qr/mod.rs:62-66`):
```rust
let input = crate::core::input::read_input(self.input)?;
let rendered = render_qr(&input)?;       // ▀▄█ half-block glyphs — a VISUAL
println!("{rendered}");                  // glyphs to stdout (no color path, D-03)
```
**Two LOCKED breaks from the uniform analog:**
1. **D-14** — under `--json`, do NOT emit glyphs; `emit_json(&{text: input, error_correction: "M"})`. Don't even need `render_qr` under `--json` (but keep it for the human path).
2. ⚠ **D-15 (A2 / Pitfall 4) — `qr --clip` copies the SOURCE TEXT, not the glyphs.** Routing `rendered` through `out_line` would copy half-blocks (garbage as clipboard text). `out_line` cannot express "print glyphs, copy text". **Planner must add a tiny `core::output` primitive** (e.g. `clip_feed(&str)` that pushes to `CLIP_BUF` only when `--clip`, no stdout — mirror `out_line`'s tee half at `output.rs:145-149`), then: `println!("{rendered}")` for display + `clip_feed(&input)` for the clip payload. **This is the only sanctioned `core::output` change in Phase 7.** Lock with a test asserting `pasted == input` (not the block).

---

#### `src/commands/weather/mod.rs` (HTTP fetch, request-response)

**Analog:** `hash` (scalar). **Schema (D-17 LOCKED, current-only):** `{location, temperature, unit, conditions, …}` — units from `current_units`, **NEVER hardcoded**.

**Current compute vs render** (`weather/mod.rs:94-123`):
```rust
eprintln!("Resolved \"{}\" → {label} ...", self.location);   // :94 stderr — GOOD, no --json contamination
let conditions = wmo_to_str(forecast.current.weather_code);
let temp_unit = &forecast.current_units.temperature_2m;      // :106 AUTHORITATIVE unit (NOT the request param)
let wind_unit = &forecast.current_units.wind_speed_10m;      // :107
let temp = forecast.current.temperature_2m;                  // :108 f64
// ... aligned block: Conditions/Temperature/Wind/Humidity, println! :116-122
```
**Response model already parsed** (`weather/mod.rs:284-307`): `ForecastResp{current: Current, current_units: CurrentUnits}`; `Current` has `temperature_2m/relative_humidity_2m/wind_speed_10m: f64`.
**The cost:** small `WeatherOutput` struct from the already-parsed `forecast`. ⚠ **Pitfall WTHR-3:** `unit` MUST read from `forecast.current_units` (`:106-107`) — imperial wind label is `mp/h`, not `mph`. ⚠ **f64 watch (Pitfall 2):** NaN/Inf is invalid JSON; real API data is finite (low risk) — never serialize a hand-computed f64. The stderr echo (`:94`) is already off the `--json` channel — no change. Not in SPINE-04.

---

## Shared Patterns

### A. The `is_json_on()` fork (applies to ALL 16)
**Source:** `uuid/mod.rs:66-80`, spine `output.rs:93`.
Fork on `is_json_on()` **BEFORE any stdout write** (Pitfall 1). The ONLY stdout write reachable under `--json` is `emit_json`. Commands with MULTIPLE human stdout writes (`du` rows+blank+summary `:127-136`; `tree` label+tree+blank+summary `:103-109`; `dupes` groups+summary `:210-233`; `flatten`/`bulk-rename` rows+summary) must put ALL of them behind the `else`.

### B. `out_line` replaces the primary `println!` (the 6 SPINE-04 commands)
**Source:** `output.rs:143`, `uuid/mod.rs:78`.
**Apply to:** `passgen`, `color`, `base64` (encode), `epoch`, `json` (plain/compact path). The tee to `CLIP_BUF` is automatic. **Exceptions:** `qr` (D-15 — see E), `base64 --decode` binary path (line-oriented `out_line` can't carry binary).

### C. Serde enum with `rename_all = "lowercase"`
**Source:** `hash/mod.rs:88-99`.
**Apply to:** `flatten` + `bulk-rename` `action` field (project `ItemKind`/`RowStatus` → `"copy"`/`"rename"`/`"skip"`).

### D. Path fields via `to_string_lossy()` (D-4 — NEVER `to_str().unwrap()`)
**Source:** `hash/mod.rs:274` (`label.clone()` lossy-safe); existing convention `du/mod.rs:158`, `dupes/mod.rs:223`, etc.
**Apply to:** every `PathBuf`/path field in `du`, `tree`, `dupes`, `flatten`, `bulk-rename` JSON output. The human renders already use `to_string_lossy()`/`.display()`; the JSON path must match.

### E. ⚠ The one `core::output` ADDITION — `clip_feed(&str)` for qr (D-15)
**Source to mirror:** `out_line`'s tee half (`output.rs:145-149`) + the `out_line_tees` unit test (`output.rs:490-513`).
**Apply to:** `qr` only. Pushes the source text to `CLIP_BUF` when `CLIP_ON`, no stdout write — enables "print glyphs, copy text". This is the SOLE sanctioned spine change this phase; gate it behind A2 confirmation.

### F. JSON-purity test (ALL 16) + clip round-trip (the 6 SPINE-04)
**Source:** `tests/uuid.rs:135` (`json_purity`), `:184` (`json_count_multi` for passgen), `:237` (`clip_roundtrip`, `#[ignore]`).
Every `--json` command: copy `json_purity`, adapt the shape assertion (`tests/uuid.rs:145-166`) to the command's schema. The two purity assertions are frozen verbatim: no `0x1B` (`:168-172`), no BOM (`:173-178`). qr's clip test asserts `pasted == input` (D-15), not the glyphs.

### G. Color/progress hygiene — UNCHANGED, do not touch
**Source:** `output.rs:106-113` (`init_output` already forces `COLOR_ON=false` under json/clip).
Every command keeps its existing `is_color_on()` gate (`color:54`, `du:209`, `tree:229`, `dupes:239`, `weather:115`, `json:65`). No per-command color change. No command has a progress bar yet (those are Phase 8).

---

## No Analog Found

None. All 16 commands map to the `uuid`/`hash` pilots or the spine primitives. The structurally-novel work is bounded and listed as surprises above:

| File | Why it deviates from a clean pilot copy | Planner action |
|------|------------------------------------------|----------------|
| `src/commands/tree/mod.rs` | No node tree exists today (flat printing recursion) — needs a NEW `build_node` recursion (A4). | Build parallel recursion reusing `read_children`/`sort_children`. |
| `src/commands/json/mod.rs` | D-16 identity passthrough — root-rule exception; the ONLY direct-serde command. | `emit_json(&value)` verbatim; route plain path through `out_line` for `--clip`. |
| `src/commands/qr/mod.rs` | D-14 metadata (not glyphs) + D-15 clip-copies-text needs a NEW `clip_feed` primitive (A2). | Add `core::output::clip_feed`; emit `{text, error_correction}`. |
| `src/commands/bulk_rename/mod.rs` | D-12 `--force --json` emits rows (overrides silent-on-success) + abort path must keep stdout empty (A3). | Two explicit `--json` behavioral forks. |
| `src/commands/base64/mod.rs` | Non-UTF-8 decode can't go in a JSON string (A1). | Pick base64-string-field / lossy+marker / refuse policy. |

---

## Metadata

**Analog search scope:** `src/commands/{16 targets}/mod.rs`, `src/commands/uuid/mod.rs`, `src/commands/hash/mod.rs`, `src/core/output.rs`, `tests/uuid.rs`, `tests/` listing.
**Files scanned:** 20 source files (16 targets + 2 pilots + spine + test template) read in full; `flatten`/`bulk_rename` read via targeted grep + section reads (struct + run/render sites).
**Pattern extraction date:** 2026-06-25
**Key confirmations:** no `tests/cowsay.rs` on disk (cowsay needs a NEW test file); weather fixtures present (`forecast_imperial.json`, `forecast_metric.json`, `geocode_hit.json`, `geocode_no_match.json`) for the offline `--json` test; all spine primitives live and consumed.
