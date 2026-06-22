# Phase 3: Filesystem Power Tools - Research

**Researched:** 2026-06-22
**Domain:** Rust CLI filesystem tools — streaming hashers (blake3 + RustCrypto digest 0.11), parallel content-dedup (rayon), regex rename with Windows-safe pre-flight collision detection, walkdir traversal, deterministic snapshot testing
**Confidence:** HIGH (every new-crate API shape verified against docs.rs / official std docs this session; all 19 design decisions are LOCKED in CONTEXT.md and were NOT relitigated)

<user_constraints>
## User Constraints (from CONTEXT.md)

> All 19 decisions (D-01..D-19) are LOCKED. The planner MUST honor these verbatim. This research de-risks EXECUTION of these decisions; it does not propose alternatives.

### Locked Decisions

**hash (HASH-01)**
- **D-01:** Default algorithm = **SHA-256** (`--algo blake3` switches). Output format `<hash>  <filename>` (two spaces, coreutils style). ⚠️ The STATE.md / CLAUDE.md "BLAKE3 as default" lines are **SUPERSEDED** — read as "BLAKE3 via `--algo blake3`".
- **D-02:** `--algo` set = `{sha256` (default)`, blake3, sha512, md5}`. Crates: `sha2` 0.11.0 (sha256+sha512), `blake3` 1.8.5, `md-5` (RustCrypto, hyphenated). Net-new dep vs locked stack: only `md-5`. Excluded: sha1, sha224, sha384.
- **D-03:** Implementation = **enum-dispatch hasher, NOT a unified `dyn Digest`**. RustCrypto algos (sha256/sha512/md5) share one `digest::Digest` code path; **blake3 gets its own arm on the stable native `blake3::Hasher`** (NOT `traits-preview`). Stream every algorithm (no whole-file buffering): blake3 via `Hasher::update_reader`, RustCrypto via incremental `update`.
- **D-04:** `--verify EXPECTEDHASH` = auto-detect algorithm by hex length (32→md5, 64→**sha256** wins the sha256/blake3 tie, 128→sha512), `--algo` is the explicit override. **Case-insensitive, plain `==` (NOT constant-time)**. Exit 0 match / 1 mismatch; unsupported length → exit 2.
- **D-05:** Input = `core::input` + implement the deferred `--file PATH` layer (Phase 2 D-06). stdin filename label = `-`. Byte-exact reads.

**Shared traversal (tree / du / dupes)**
- **D-06:** All three reuse `walkdir` + `core::fs::is_hidden` — skip hidden by default. `follow_links(false)`. `--all`/`-a` is NOT this phase.
- **D-07:** **No noise-directory skip list and NO `ignore` crate in Phase 3.** `node_modules`/`target` shown as literal truth.

**tree (TREE-01)**
- **D-08:** Sort = directories first, then files, each case-insensitive alphabetical. Depth-first.
- **D-09:** Glyphs = standard Unicode box-drawing `├──`, `└──`, `│  `, `   `.
- **D-10:** `--sizes` shows per-file size ONLY (dirs blank). `--depth N` limits displayed depth. Summary `N directories, M files` to stdout. Color: directory names only, gated through `is_color_on()`.

**du (DU-01)**
- **D-11:** One row per IMMEDIATE CHILD; dirs show recursive total, files own size; sorted biggest-first. `--depth N` = aggregation cap; `--top N` = post-sort truncation. Trailing `/` marks dirs. Summary always reflects FULL scan total: `{X} of {Y} entries shown. {TOTAL} total.` Color: size value only.

**Size formatting (shared)**
- **D-12:** Promote flatten's `human_size` into `core::output` and reuse it. 1024-based, decimal-style labels (`B`/`KB`/`MB`/`GB`/`TB`). Do NOT add the `humansize` crate. `du` right-aligns the size column.

**dupes (DUPE-01)**
- **D-13:** Identity = size pre-filter THEN content hash. Content-equality hash = **BLAKE3**. Use **`rayon` 1.12** for the parallel content-hash phase. Output = groups + wasted-space summary. No deletion.

**bulk-rename (RENM-01)**
- **D-14:** Scope = top-level files by default; `--recursive` opt-in (reuse flatten's walk, `min_depth(1)`, files only). Collision detection scoped per containing directory.
- **D-15:** Targets = files only. Dirs and symlinks/junctions skipped, shown as `-` rows.
- **D-16:** Match target = the FULL base name (incl. extension). Extension protection via pattern discipline, NOT stem-splitting.
- **D-17:** Replacement = `Regex::replace` (FIRST match only), `$1`/`${1}` capture syntax. `--all` deferred to v2. Document the `$1abc` foot-gun in `--help`.
- **D-18:** Safety = **pre-flight, in-memory, ABORT-ALL-BEFORE-ANY-RENAME**. ⚠️ `std::fs::rename` SILENTLY OVERWRITES an existing destination on Windows; there is NO `create_new` backstop for moves. (1) case-folded occupied set per directory, (2) check every target vs other planned targets AND pre-existing on-disk names → any clobber aborts the whole batch (exit 1, nothing written), (3) cycles/swaps → DETECT-AND-ABORT, (4) skip no-op renames but a case-only change (`foo`→`Foo`) IS a real rename (compare exact non-folded names).
- **D-19:** UX = dry-run preview is the DEFAULT; `--force` executes. Reuse flatten's glyph output (`~` rename, `-` skip), `format_row`/`arrow_col`, `[collision]` inline reason, parallel summary. Each `std::fs::rename` `.context(...)`-wrapped; stop on first unexpected I/O error.

### Claude's Discretion
- Module layout under `src/commands/<cmd>/mod.rs`; exact `core::output` location/signature for promoted `human_size`; the precise per-directory collision-set data structure.
- Whether `dupes` shows an optional `indicatif` spinner (NOT required v1; keep simple).
- Exact `--algo` value spellings and the hash-algorithm enum names; whether `--file` for hash is positional or a flag (must route through `core::input` extension point).
- Exact color shades, alignment column widths, summary/error wording within the locked families.

### Deferred Ideas (OUT OF SCOPE)
- Reconcile stale "BLAKE3 default" notes in STATE.md / CLAUDE.md to "BLAKE3 via `--algo blake3`" (planner/transition action, not a feature).
- `--all`/`-a` show-hidden flag for tree/du/dupes — v2.
- Noise-directory skipping (hardcoded list or `ignore` crate) — v2; lean entry point is a dep-free hardcoded list scoped to dupes, NOT `ignore`.
- Two-phase temp-name swap/cycle rename — v2 (v1 detects-and-aborts).
- bulk-rename `--all` (replace_all), case transforms, sequential numbering, `--backup`, `--json` — RENM-V2.
- hash: BLAKE3-as-default reconsideration, multi-file, `--json`, progress bar, sha1/sha224/sha384 — HASH-V2.
- du percentage bars / color ranges / apparent-size; tree gitignore/dirs-only/sort-by-size — DU-V2 / TREE-V2.
- indicatif progress for dupes/large-file hash — v2.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HASH-01 | Hash a file (default SHA-256), choose algorithm, hash stdin, verify against a known hash (exit 0 match / 1 mismatch), `HASH  filename` format | Verified: digest 0.11 incremental `update`/`finalize` path for sha256/sha512/md5; native `blake3::Hasher::update_reader`; `base16ct::lower::encode_string` for RustCrypto hex; `Hash::to_hex()` for blake3 (already lowercase 64-hex); `core::input` `-` sentinel + `--file` extension point |
| TREE-01 | Box-drawing tree, optional sizes, depth limit, colored dirs vs files, count summary | Verified: walkdir `filter_entry(!is_hidden)` + `follow_links(false)` + `sort_by`/manual dir-first ordering; box-drawing glyph patterns; `is_color_on()` gate; promoted `human_size` |
| DU-01 | Size-sorted (biggest first), human sizes, depth limit, top-N | Verified: walkdir recursive accumulation per immediate child; `human_size` promotion; right-align column technique; stable `sort_by` (size desc, then name) for determinism |
| DUPE-01 | Find duplicates by content (size pre-filter then hash), groups + wasted-space summary | Verified: `into_par_iter()`/`par_iter()` over same-size groups, collect into a map then sort keys for deterministic output; BLAKE3 equality hash reuses the hash infra |
| RENM-01 | Regex rename with capture-group replacement; dry-run default, `--force` to execute, collision detection aborts before any rename | Verified: `Regex::replace` is first-match-only, returns `Cow<str>`, `${1}` brace syntax + `$1abc` foot-gun; `std::fs::rename` has NO no-overwrite option on Windows → pre-flight detection is the only backstop (D-18) |
</phase_requirements>

## Summary

This phase is a **low-novelty, high-verification** build: five filesystem commands layered onto the proven Phase-1 infrastructure (`walkdir` + `core::fs::is_hidden` + `core::output` color/row helpers + `core::input` precedence). Four of the five (hash, tree, du, dupes) are read-only; only `bulk-rename` mutates the disk, and it carries the single load-bearing safety fact of the phase — **`std::fs::rename` silently overwrites its destination on Windows and offers no `create_new`-style backstop**, so correctness rests entirely on in-memory pre-flight collision detection (D-18).

The only genuinely new ecosystem surface is the **hashing stack**. Two distinct API idioms coexist by design (D-03): the RustCrypto family (`sha2` 0.11 + `md-5` 0.11) all speak the `digest::Digest` 0.11 trait (`new()` → `update(impl AsRef<[u8]>)` → `finalize() -> Output<Self>`), while **blake3 deliberately uses its own stable native `Hasher`** (`new()` → `update_reader(impl Read) -> io::Result` → `finalize() -> Hash`, `Hash::to_hex()` already lowercase 64-hex) to avoid coupling the build to blake3's unstable `traits-preview` feature. The one non-obvious detail confirmed this session: **digest 0.11 migrated its output type from `GenericArray` to `hybrid-array`**, so `finalize()` yields a value that behaves like `[u8; N]`; the RustCrypto-native way to hex-encode it is `base16ct::lower::encode_string(&output)` (a new ~1-line dep), or manual `{:02x}` formatting.

**Primary recommendation:** Build a single `Hasher` enum with four arms (Sha256/Sha512/Md5 sharing a generic RustCrypto streaming helper + a separate Blake3 arm), pin `base16ct = "1"` for RustCrypto hex output (blake3 needs no hex crate), pin `regex = "1.12"`, add `rayon` only with its content-hash use scoped to dupes, and reuse every Phase-1 traversal/output/collision pattern verbatim. Keep snapshot tests deterministic by fixing input bytes (so sizes/hashes are exact) and forcing a stable sort key, and set `NO_COLOR=1` in every `assert_cmd` invocation as the existing tests already do.

## Architectural Responsibility Map

This is a single-binary CLI (no client/server/DB tiers). The relevant "tiers" are the established internal modules; the map below confirms each capability is assigned to the module that already owns that concern, so the planner can sanity-check task placement.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Algorithm dispatch + streaming hash | `commands/hash` (new `Hasher` enum) | — | Hashing logic is command-local; only the *result* (a hex string) is shared output |
| Hex encoding of a digest | `commands/hash` (via `base16ct`) | — | blake3 self-hexes (`to_hex`); only RustCrypto arms call `base16ct` |
| Reading file/stdin input | `core::input` (extend with `--file`) | `commands/hash` (first consumer) | D-05/D-06: the precedence + `-` sentinel already live in `core::input`; `hash` adds the `--file` branch ahead of the stdin branch |
| Path normalization | `core::fs::normalize_path` | all 5 commands | dunce UNC-safety is centralized (Pitfall 1) |
| Hidden-file pruning | `core::fs::is_hidden` | tree/du/dupes/bulk-rename(`--recursive`) | D-06: one Windows-correct predicate, root-safe (walkdir#142) |
| Recursive directory walk | `walkdir` (per-command iterator) | `core::fs::is_hidden` filter | walkdir is configured per command (depth, file-only) but shares the filter |
| Size formatting | `core::output::human_size` (promoted) | tree/du | D-12: one tested fn, no `humansize` crate |
| Color gating | `core::output::is_color_on` | tree (dir names), du (size value) | D-10: one gate so piped output is byte-identical minus ANSI |
| Row/arrow/preview rendering | `core::output::{format_row,arrow_col,terminal_width,truncate_middle}` | bulk-rename | D-19: reuse flatten's preview machinery |
| Parallel content hashing | `rayon` (scoped to dupes) | `commands/hash` infra (BLAKE3) | D-13: only dupes parallelizes; hash itself is single-file/sequential |
| Pre-flight collision detection | `commands/bulk-rename` (case-folded set per dir) | flatten's `occupied`-set pattern (model) | D-18: the only backstop for the no-`create_new`-on-rename hazard |

## Standard Stack

> All versions confirmed via `cargo search` (crates.io) on 2026-06-22. Versions in CONTEXT.md / CLAUDE.md are honored as locked; where CONTEXT.md left a version unspecified, the current stable is recommended below and flagged.

### Core (new dependencies this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `blake3` | 1.8.5 | BLAKE3 hashing — `--algo blake3` (hash) + content-equality hash (dupes) | `[VERIFIED: crates.io]` Native stable `Hasher` API; `to_hex()` returns lowercase 64-hex with zero extra deps `[CITED: docs.rs/blake3/1.8.5]` |
| `sha2` | 0.11.0 | SHA-256 (default) + SHA-512 | `[VERIFIED: crates.io]` RustCrypto; implements `digest::Digest` 0.11 `[CITED: CLAUDE.md]` |
| `md-5` | 0.11.0 | MD5 — legacy checksum interop (`--algo md5`) | `[VERIFIED: crates.io]` Hyphenated RustCrypto crate (the `md5` crate does NOT implement `digest`). **Latest is 0.11.0**, matching `sha2`/`digest` 0.11 — confirmed this session (CLAUDE.md said "latest") |
| `base16ct` | 1.0.0 | Lowercase hex-encode the RustCrypto `finalize()` output | `[VERIFIED: crates.io]` RustCrypto's own recommended hex helper; constant-time, no_std, tiny. `base16ct::lower::encode_string(&output) -> String` `[CITED: lib.rs/crates/sha1 RustCrypto example]` |
| `rayon` | 1.12.0 | Parallel content-hash phase of `dupes` (D-13) | `[VERIFIED: crates.io]` `into_par_iter()`/`par_iter()` work-stealing standard `[CITED: CLAUDE.md]` |
| `regex` | 1.12.4 | `bulk-rename` pattern matching + capture-group replacement (D-17) | `[VERIFIED: crates.io]` CONTEXT.md/CLAUDE.md left the version unspecified — **recommend pinning `regex = "1.12"`** (current stable 1.12.4). Default features only; no `unicode-*` opt-in needed beyond defaults |

### Supporting (already in the manifest — reused, not added)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `walkdir` | 2.5 | Recursive traversal for tree/du/dupes/bulk-rename(`--recursive`) | Already a dep; reuse `follow_links(false)` + `filter_entry(!is_hidden)` |
| `anyhow` | 1.0 | `.context(...)` per-file error surfacing (FOUND-06, D-19) | Every fallible I/O call |
| `thiserror` | 2.0 | Typed `BoxError` (exit-code mapping in `main`) | If a new typed exit-2 variant is needed (e.g. bad `--verify` length) |
| `owo-colors` | 4.3 | Color decoration, gated by `is_color_on()` | tree dir names, du size accent |
| `crossterm` | 0.29 | `terminal_width()` for du column alignment | Reuse via `core::output::terminal_width` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `base16ct` (RustCrypto hex) | `const-hex` 1.19.1 (SIMD, faster) | `const-hex` is faster but heavier; for a per-file CLI the speed is irrelevant and `base16ct` is the RustCrypto-canonical choice with the smallest footprint. **Recommend `base16ct`.** A third option needs no crate at all: `output.iter().map(|b| format!("{b:02x}")).collect::<String>()` — viable but allocates per byte; `base16ct` is cleaner. (Discretion: the hex approach is the planner's call, but `base16ct` is the lowest-risk default.) |
| blake3 native `Hasher` | blake3 `digest::Digest` via `traits-preview` | LOCKED OUT by D-03: `traits-preview` is "unstable, may break between patch versions" — using it would couple the build to an unstable feature. Native `Hasher` is the locked choice. |
| `regex` `replace` | `replace_all` | LOCKED to `replace` (first-match-only) by D-17; `replace_all` is RENM-V2. |
| `humansize` crate | promoted `human_size` fn | LOCKED OUT by D-12. |
| `ignore` crate | plain `walkdir` | LOCKED OUT by D-07 (v2). |

**Installation (Cargo.toml additions):**
```toml
# Hashing (hash + dupes content-equality) — D-02/D-03/D-13
blake3 = "1.8"
sha2 = "0.11"
md-5 = "0.11"
# Lowercase hex-encode the RustCrypto digest output (blake3 self-hexes via to_hex)
base16ct = { version = "1", features = ["alloc"] }   # encode_string needs alloc
# Parallel content-hash phase of dupes — D-13
rayon = "1.12"
# bulk-rename regex pattern + capture-group replacement — D-17
regex = "1.12"
```

> ⚠️ **`base16ct` feature note** `[ASSUMED]` `base16ct::lower::encode_string` returns an owned `String`, which requires the `alloc` feature on `base16ct`. Verify during the hash plan that `encode_string` is gated behind `alloc` (the alternative `encode_str(&mut buf)` writes into a caller buffer and needs no alloc). If the build errors on `encode_string`, either enable `alloc` (shown above) or use the fixed-buffer `encode_str` variant. This is a 1-line build-time check, not a design risk.

**Version verification performed this session:**
```
cargo search blake3   → blake3 = "1.8.5"     ✓
cargo search sha2     → sha2 = "0.11.0"      ✓
cargo search md-5     → md-5 = "0.11.0"      ✓ (CONTEXT/CLAUDE said "latest"; pinned 0.11 to match digest 0.11)
cargo search rayon    → rayon = "1.12.0"     ✓
cargo search regex    → regex = "1.12.4"     ✓ (CONTEXT left unspecified; recommend "1.12")
cargo search base16ct → base16ct = "1.0.0"   ✓
cargo search digest   → digest = "0.11.3"    ✓ (transitive via sha2/md-5; do NOT add directly)
```

## Package Legitimacy Audit

> slopcheck 1.x was available and run this session. All new packages scanned `[OK]`.

| Package | Registry | slopcheck | Source Repo | Disposition |
|---------|----------|-----------|-------------|-------------|
| `blake3` | crates.io | `[OK]` | github.com/BLAKE3-team/BLAKE3 | Approved (official BLAKE3 team) |
| `sha2` | crates.io | `[OK]` | github.com/RustCrypto/hashes | Approved (RustCrypto) |
| `md-5` | crates.io | `[OK]` | github.com/RustCrypto/hashes | Approved (RustCrypto; hyphenated) |
| `rayon` | crates.io | `[OK]` | github.com/rayon-rs/rayon | Approved |
| `regex` | crates.io | `[OK]` | github.com/rust-lang/regex | Approved (rust-lang official) |
| `base16ct` | crates.io | `[OK]` | github.com/RustCrypto/formats | Approved (RustCrypto) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

All six are first-party crates from BLAKE3-team, RustCrypto, rayon-rs, and rust-lang — the highest-credibility sources in the Rust ecosystem. Provenance is authoritative (RustCrypto/BLAKE3-team/rust-lang official repos + CLAUDE.md locked versions), so these are `[VERIFIED: crates.io]`, not merely registry-present.

## Architecture Patterns

### System Architecture Diagram

```
                       box <cmd> [args]  (argv)
                              │
                       clap derive parse  (src/cli.rs)
                              │
                  init_color(no_color)  → COLOR_ON gate  (core::output)
                              │
        ┌────────────┬────────────┬───────────┬────────────┬──────────────┐
        ▼            ▼            ▼           ▼            ▼              ▼
      hash         tree          du        dupes     bulk-rename     (others)
        │            │            │           │            │
  ┌─────┴─────┐  walkdir      walkdir     walkdir       walkdir
  │ core::    │  +is_hidden   +is_hidden  +is_hidden    +is_hidden(rec)
  │ input     │  follow(false)follow(false)follow(false) files-only
  │ (--file/  │     │            │           │            │
  │  stdin/-) │  dirs-first   accumulate  ┌──┴───┐    ┌───┴────────────┐
  └─────┬─────┘  sort+recurse per-child   │group │    │ PLAN (in-mem): │
        │          │  totals  │  by size  │      │    │ src→regex.     │
   Hasher enum     │          │           ▼      │    │ replace→target │
   ┌────┴─────┐    │       sort desc  same-size  │    └───────┬────────┘
   │ Sha256   │    │       biggest-1st groups    │            ▼
   │ Sha512   │ box-draw    │           │        │     PRE-FLIGHT collision
   │ Md5      │ glyphs +   --top/      rayon     │     scan (case-folded
   │ (digest  │ size col   --depth   par_iter    │     set per dir +
   │  0.11)   │    │       truncate  BLAKE3 hash │     on-disk names)
   │ Blake3   │    │          │      per file    │       │
   │ (native  │    │          │         │        │   any clobber/cycle?
   │  Hasher) │    │          │      collect→    │    ──yes──► ABORT (exit 1,
   └────┬─────┘    │          │      map<hash,   │              nothing written)
        │          │          │      [paths]>    │       │ no
  base16ct hex     │          │      sort keys   │       ▼
  (rustcrypto) OR  │          │         │     dry-run? ──default──► preview only
  to_hex (blake3)  │          │     groups +     │       │ --force
        │          │          │     wasted-space │       ▼
        ▼          ▼          ▼      summary      ▼   std::fs::rename per file
   <hash> <name>  tree +   size-sorted  groups   (⚠ NO create_new backstop —
   OR verify→     summary   + summary   + summary  pre-flight is the only guard)
   exit 0/1                                            │
        │                                              ▼
        └──────────── data→stdout, messages→stderr, exit 0/1/2 ───────────┘
                          (FOUND-03, inherited from main.rs)
```

### Recommended Project Structure
```
src/commands/
├── hash/
│   └── mod.rs        # HashArgs + Hasher enum (Sha256/Sha512/Md5/Blake3) + verify logic
├── tree/
│   └── mod.rs        # TreeArgs + recursive box-drawing render + dir-first sort
├── du/
│   └── mod.rs        # DuArgs + per-child recursive totals + sort/top/depth
├── dupes/
│   └── mod.rs        # DupesArgs + size pre-filter + rayon BLAKE3 hash + group output
└── bulk_rename/      # NOTE: module is `bulk_rename` (snake_case); CLI name "bulk-rename"
    └── mod.rs        # BulkRenameArgs + regex plan + pre-flight collision detection + execute
```
(Discretion D — module layout is the planner's call; `core::output` gains `human_size`, `core::input` gains the `--file` branch.)

> ⚠️ **Module-name pitfall (precedent: `8ball`→`eight_ball`):** the CLI name `bulk-rename` is not a valid Rust identifier (the hyphen). Mirror the established `#[command(name = "bulk-rename")]` pattern on a `BulkRename` enum variant (already present in `src/cli.rs:65-66`) and name the Rust module `bulk_rename`. This is identical to the `8ball`→`eight_ball` handling already in the codebase (STATE.md).

### Pattern 1: enum-dispatch streaming Hasher (D-03)
**What:** One enum with four arms. The three RustCrypto arms share a generic streaming helper over `digest::Digest`; blake3 is its own arm on the native `Hasher`. Stream from an `impl Read` so nothing is buffered whole.
**When to use:** The `hash` command's core, and reused by `dupes` for the BLAKE3 content hash.
```rust
// RustCrypto arms — digest 0.11 trait. update() takes impl AsRef<[u8]>; finalize()
// consumes self and returns Output<Self> (a hybrid-array that behaves like [u8; N]).
// Source: https://docs.rs/digest/0.11.3/digest/trait.Digest.html
use digest::Digest;
use std::io::Read;

fn hash_rustcrypto<D: Digest, R: Read>(mut hasher: D, mut reader: R) -> anyhow::Result<String> {
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;     // .context(...) in real code
        if n == 0 { break; }
        hasher.update(&buf[..n]);           // impl AsRef<[u8]> in 0.11
    }
    let out = hasher.finalize();            // Output<D>, behaves like [u8; N] in 0.11
    Ok(base16ct::lower::encode_string(&out)) // lowercase hex String
}

// blake3 arm — native stable Hasher (NOT traits-preview). update_reader takes the
// reader BY VALUE, returns io::Result<&mut Self>. to_hex() is already lowercase 64-hex.
// Source: https://docs.rs/blake3/1.8.5/blake3/struct.Hasher.html
fn hash_blake3<R: Read>(reader: R) -> anyhow::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(reader)?;          // streams internally with a SIMD-sized buffer
    Ok(hasher.finalize().to_hex().to_string()) // ArrayString -> String, lowercase
}
```
Key facts (all verified this session):
- `digest::Digest::update(impl AsRef<[u8]>)` — `&buf[..n]` satisfies it. `[CITED: docs.rs/digest/0.11.3]`
- `finalize(self) -> Output<Self>` consumes the hasher; in 0.11 `Output` is a `hybrid-array` (behaves like `[u8; N]`, can drop `.as_slice()`). `[VERIFIED: github.com/RustCrypto migration notes]`
- `base16ct::lower::encode_string(&out)` → lowercase hex `String` (needs `alloc`). `[CITED: RustCrypto sha1 example]`
- `blake3::Hasher::update_reader(impl Read) -> io::Result<&mut Self>` — reader by value; `std` feature (default-on) required. `[CITED: docs.rs/blake3/1.8.5/blake3/struct.Hasher.html]`
- `blake3::Hash::to_hex() -> ArrayString` ("Encode a Hash in lowercase hexadecimal", 64 chars). `Display`/`to_string()` also lowercase. `[CITED: docs.rs/blake3/1.8.5/blake3/struct.Hash.html]`
- `digest` 0.11.3 is pulled transitively by `sha2`/`md-5` 0.11 — **do NOT add `digest` directly** unless a `use digest::Digest;` import requires it; if so, add `digest = "0.11"` to match.

### Pattern 2: `--verify` length auto-detection (D-04)
**What:** Strip/normalize the expected hash, switch on its hex length to pick the algorithm (unless `--algo` overrides), hash, lowercase-compare with plain `==`.
```rust
// D-04: 32→md5, 64→sha256 (wins the sha256/blake3 tie), 128→sha512.
// --algo is the explicit override (so `--algo blake3 --verify <64hex>` picks blake3).
fn algo_from_len(expected: &str) -> Option<Algo> {
    match expected.trim().len() {
        32  => Some(Algo::Md5),
        64  => Some(Algo::Sha256),   // tie-break: sha256 over blake3 (D-04)
        128 => Some(Algo::Sha512),
        _   => None,                 // unsupported length → exit 2 (bad args, FOUND-03)
    }
}
// Compare: case-insensitive, PLAIN ==, not constant-time (a checksum is public; D-04).
let ok = computed.eq_ignore_ascii_case(expected.trim());
// exit 0 on match, 1 on mismatch (HASH-01). An unsupported length must reach exit 2,
// so route it through a typed BoxError variant (mirror MissingInput→exit 2), NOT a
// plain anyhow::bail! (which main() maps to exit 1). See Pitfall 1 below.
```

### Pattern 3: deterministic dir-first tree render (D-08/D-09)
**What:** Read each directory's children, partition into dirs/files, sort each case-insensitively, print depth-first with box-drawing prefixes. The `└──` (last child) vs `├──` (non-last) choice and the `│  ` vs `   ` continuation are computed from "is this the last entry at this level".
```rust
// Glyphs are Unicode box-drawing (D-09) — distinct from flatten's ASCII +/~/- *status*
// glyphs. Branch structure, not state, so Unicode is correct here.
const TEE: &str  = "├── ";
const ELL: &str  = "└── ";
const PIPE: &str = "│   ";
const GAP: &str  = "    ";
// Sort: directories first, then files, each case-insensitive alphabetical (D-08).
children.sort_by(|a, b| {
    let (ad, bd) = (a.is_dir(), b.is_dir());
    bd.cmp(&ad)  // dirs (true) before files (false)
        .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase()))
});
```
Color only directory names, gated on `is_color_on()` (D-10), exactly like flatten gates its glyph. Summary `N directories, M files` to stdout.

### Pattern 4: deterministic parallel dupes (D-13)
**What:** Walk → group by byte-size → for each multi-member size group, hash members in parallel with rayon (BLAKE3) → group by hash → keep groups with ≥2 members → sort for stable output.
```rust
use rayon::prelude::*;
use std::collections::HashMap;

// size_groups: HashMap<u64, Vec<PathBuf>> — only groups with len >= 2 can possibly dup.
let candidates: Vec<PathBuf> = size_groups.into_values()
    .filter(|v| v.len() >= 2).flatten().collect();

// Parallel content-hash (BLAKE3). Collect (hash, path) pairs; rayon order is
// nondeterministic, so we SORT before output for reproducible snapshots.
let mut hashed: Vec<(String, PathBuf)> = candidates
    .par_iter()
    .map(|p| Ok((hash_file_blake3(p)?, p.clone())))
    .collect::<anyhow::Result<Vec<_>>>()?;   // first error short-circuits

hashed.sort();                               // deterministic: by hash then path
// group consecutive equal hashes; emit groups of size >= 2 + wasted-space summary.
```
**Determinism rule:** rayon parallelism produces results in arbitrary order. Always `collect` then **sort by a stable key** (hash, then path) before printing — otherwise dupes output and its snapshot will flap. (Same rule applies to du: sort by `(size desc, name asc)`, never rely on walk order.)

### Pattern 5: pre-flight collision detection for bulk-rename (D-18)
**What:** Build the entire rename plan in memory, validate it against (i) other planned targets and (ii) pre-existing on-disk names, abort the whole batch on any clobber/cycle BEFORE the first `std::fs::rename`. This is the model from flatten's `occupied`-set, adapted — but unlike flatten there is NO `safe_copy`/`create_new` backstop, so the pre-flight check is the *only* guard.
```rust
// Per containing directory (D-14): collision scope is per-dir.
// 1. Case-folded occupied set = pre-existing on-disk names NOT being renamed away
//    (full Unicode to_lowercase, matching flatten::rename::dedupe — WR-01).
// 2. Each planned target checked vs (a) other planned targets, (b) the occupied set.
// 3. Cycles/swaps: any target equal to another item's SOURCE → detect-and-abort (D-18.3).
// 4. No-op skip: new == old byte-exact → "-" (unchanged). EXCEPTION (D-18.4): a
//    case-only change (foo→Foo) is byte-different and IS a real rename — compare
//    EXACT (non-folded) names to detect it, so it is not falsely self-collided.
//
// ⚠ std::fs::rename has NO create_new analog and SILENTLY OVERWRITES on Windows
//   (MoveFileExW + MOVEFILE_REPLACE_EXISTING). Verified against the std docs and
//   the Rust source. Pre-flight detection is the entire safety story.
```
Each actual `std::fs::rename(src, dst)` is `.context(...)`-wrapped (D-19, FOUND-06); a *predictable* collision never reaches execution because it aborted pre-flight. Stop on the first *unexpected* I/O error.

### Anti-Patterns to Avoid
- **Buffering whole files to hash them.** Stream via `update`/`update_reader` (D-03). A multi-GB file must not be `read`-to-`Vec` first.
- **Using blake3's `digest::Digest` impl.** It is behind `traits-preview` (unstable). Use the native `Hasher` (D-03).
- **Relying on `safe_copy`/`create_new` semantics for renames.** They do not exist for `std::fs::rename`. Pre-flight only (D-18).
- **Constant-time hash compare for `--verify`.** A checksum is a public integrity value; plain `==` is correct (D-04). Constant-time would be cargo-culting.
- **Emitting parallel/walk-order output without a stable sort.** dupes and du snapshots will be non-reproducible. Always sort by an explicit key.
- **ASCII-only case folding for the rename collision set.** Use full-Unicode `to_lowercase` (NTFS folds the full table — WR-01, already the flatten precedent).
- **`replace_all` in bulk-rename.** First-match-only `replace` is locked (D-17); `replace_all` rewrites every match (the `2024_v2.jpg` foot-gun).
- **Pruning the dotted root in tree/du/dupes.** `is_hidden` already exempts depth-0 (walkdir#142); reuse it, never re-implement.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Hex-encode a digest | Manual nibble loop | `base16ct::lower::encode_string` (RustCrypto) or blake3 `to_hex()` | Edge-cases (case, padding, length); RustCrypto's own choice; blake3 self-hexes |
| Stream-hash a reader | Manual chunk loop for blake3 | `blake3::Hasher::update_reader` | Uses a SIMD-sized internal buffer (faster than a fixed 8 KiB loop) |
| Recursive walk + hidden skip | `std::fs::read_dir` recursion | `walkdir` + `core::fs::is_hidden` | Already solved: symlink-loop safety, root-exemption (walkdir#142), Windows hidden-attr |
| Human size formatting | New formatter | promoted `core::output::human_size` | ~13 lines, already unit-tested, locked label style (D-12) |
| Parallel hashing | Thread pool by hand | `rayon` `par_iter` | Work-stealing, automatic core scaling (D-13) |
| Regex capture replacement | Manual `$1` substitution | `regex::Regex::replace` | Handles `${1}` braces, named groups, the `$$` literal — and the foot-gun is documented |
| Rename collision safety | Trust the OS | In-memory pre-flight set | `std::fs::rename` silently overwrites; the OS will NOT stop you (D-18) |
| Path canonicalization | `std::fs::canonicalize` | `core::fs::normalize_path` (dunce) | Avoids the `\\?\` UNC leak (Pitfall 1, FOUND-06) |
| Terminal width for column align | `tput`/env guess | `core::output::terminal_width` | crossterm-backed, 80-col fallback when piped |

**Key insight:** This phase adds almost no new *logic* — it adds new *crate integrations* on top of a fully-built foundation. The risk is not "can we design it" but "did we call the new APIs with their exact current signatures." Every signature in Pattern 1–5 above is verified against the live docs for the pinned versions.

## Runtime State Inventory

> This is a greenfield feature addition to an existing binary (no rename/migration of stored state). Most categories are N/A, but two are worth an explicit note for the planner.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore; all five commands operate on the live filesystem the user points them at. Verified by reading all of `src/` (no DB/embedded store). | none |
| Live service config | None — no external services. Verified: no network/service code in the codebase. | none |
| OS-registered state | None new. The binary is on PATH via `install.ps1` (Phase 1); adding subcommands needs a rebuild + reinstall to be reachable globally, but registers nothing new. | Rebuild + `install.ps1` re-run is a human-verify step (same as every phase), not a data migration |
| Secrets/env vars | None. Commands read `NO_COLOR` (existing gate) only. | none |
| Build artifacts | New deps (`blake3`/`sha2`/`md-5`/`base16ct`/`rayon`/`regex`) added to `Cargo.toml` → `Cargo.lock` will change; the committed lockfile is part of the reproducible-build contract (STATE.md [01-01]). | Commit the updated `Cargo.lock` with the manifest change |

**The canonical question (after every file is updated, what runtime systems still hold old state?):** Nothing — these are net-new read-only/rename commands with no persisted state of their own. The only "registration" is the on-PATH binary, which is a rebuild-and-reinstall, handled by the existing human-verify gate.

## Common Pitfalls

### Pitfall 1: `--verify` unsupported-length must reach exit 2, not exit 1
**What goes wrong:** A `--verify` value whose hex length matches no algorithm (e.g. 40 chars / sha1) is a *usage* error (FOUND-03 → exit 2), but a plain `anyhow::bail!` is mapped to exit 1 by `main()`.
**Why it happens:** `main.rs` only upgrades to exit 2 for `BoxError::MissingInput` (downcast) and clap parse errors; every other `Err` is exit 1 (verified in `src/main.rs:97-103`).
**How to avoid:** Add a typed `BoxError` variant (e.g. `UnsupportedHashLength`) and return it via `.into()`, mirroring the `MissingInput`→exit-2 pattern (STATE.md [02-01]). Then extend the downcast arm in `main.rs`. **A successful-but-mismatched `--verify` is exit 1** (a legitimate negative result, HASH-01), so keep the two cases distinct.
**Warning signs:** A test feeding a 40-char hash expects exit 2 but gets exit 1.

### Pitfall 2: digest 0.11 output is no longer `GenericArray`
**What goes wrong:** Code copied from a digest-0.10 example calls `.as_slice()` on `finalize()` or types the result as `GenericArray<...>`, which won't compile against 0.11.
**Why it happens:** digest 0.11 migrated `Output` from `generic-array` to `hybrid-array`; the value now behaves like `[u8; N]`.
**How to avoid:** Pass `&out` straight to `base16ct::lower::encode_string` (it takes `AsRef<[u8]>`); drop any `.as_slice()`/`.as_ref()`. Pin `sha2`/`md-5`/(`digest` if used directly) all at `0.11` so trait versions match. `[VERIFIED: github.com/RustCrypto migration notes]`
**Warning signs:** `expected GenericArray, found ...` or `no method as_slice` compile errors.

### Pitfall 3: the `$1abc` capture foot-gun in bulk-rename replacements
**What goes wrong:** A user writes `img_$1abc` expecting "img_ + group1 + abc" but gets "img_" + (empty) because `$1abc` parses as a capture group *named* `1abc`, which doesn't exist → empty string.
**Why it happens:** regex's `$name` syntax greedily consumes alphanumerics into the name. `[CITED: docs.rs/regex/1.12.4/regex/struct.Regex.html]`
**How to avoid:** Document in `--help` (D-17) that `${1}abc` (braced) is required to follow a group with literal text, and that a nonexistent group → empty. The dry-run preview (default) shows the user exactly what changes, so the foot-gun is visible before any write.
**Warning signs:** Dry-run preview shows truncated/empty target names.

### Pitfall 4: `std::fs::rename` silently overwrites on Windows
**What goes wrong:** A rename whose target already exists destroys the target — silently, no error.
**Why it happens:** Windows `std::fs::rename` maps to `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`; there is no `create_new` analog for moves. `[CITED: doc.rust-lang.org/std/fs/fn.rename.html]`
**How to avoid:** Pre-flight collision detection (D-18) is mandatory and is the *only* backstop. A target that collides with a pre-existing on-disk name (not itself being renamed away) or with another planned target aborts the whole batch before any write.
**Warning signs:** A "successful" rename run that reduced the file count.

### Pitfall 5: case-only rename (`foo`→`Foo`) falsely flagged as a self-collision
**What goes wrong:** On NTFS, `foo` and `Foo` fold to the same case-insensitive key, so a naive collision check treats `foo→Foo` as "renaming onto an existing file" and aborts.
**Why it happens:** NTFS is case-insensitive/preserving; `std::fs::rename` *does* update the stored casing, so a case-only change is a real, valid rename.
**How to avoid:** Detect it by comparing **exact (non-folded) names** (D-18.4): if old and new differ in exact bytes but fold to the same key AND the colliding "existing" name is the item's own source, it is a legitimate case-rename, not a collision. Skip only when new == old byte-exact.
**Warning signs:** A `foo`→`Foo` rename aborts as `[collision]`.

### Pitfall 6: non-deterministic dupes/du output breaks snapshots
**What goes wrong:** dupes (rayon-parallel) or du emits rows in walk/thread order; snapshot tests flap.
**Why it happens:** rayon order is arbitrary; walkdir order is OS-dependent.
**How to avoid:** Always `collect` then sort by an explicit stable key before printing — dupes by `(hash, path)`, du by `(size desc, name asc)`. Make test fixtures use distinct sizes/contents so the sort is total. (See Validation Architecture.)
**Warning signs:** A snapshot test that passes locally but fails on CI, or fails intermittently.

### Pitfall 7: walkdir root-pruning (already solved — do not regress)
**What goes wrong:** Passing a dotted directory (`.config`) as the tree/du/dupes target prunes the entire walk to zero because the root itself looks "hidden."
**Why it happens:** walkdir#142 — `filter_entry` is applied to the root too.
**How to avoid:** Reuse `core::fs::is_hidden` verbatim; it exempts `depth() == 0` (verified in `src/core/fs.rs:46-49` + the `is_hidden_false_for_root_even_if_dotted` test). Never write a new hidden predicate.
**Warning signs:** `box tree .config` prints nothing.

## Code Examples

### Reusing the input layer for `hash` (`--file` + stdin + `-`) — D-05
```rust
// core::input already resolves arg-vs-stdin-vs-TTY with the `-` sentinel. The new
// `--file PATH` branch slots in AHEAD of the stdin branch without reshaping the
// public signatures (input.rs module docs, lines 24-25). The stdin label is "-".
// hash reads BYTES (read_input_bytes / read_to_end), so hashing is byte-exact and
// works on non-UTF-8 input. Source: src/core/input.rs
```
> Discretion: whether `--file` is a positional or a flag is the planner's call (D), as long as it routes through `core::input` so the precedence/`-` semantics are inherited.

### Promoting `human_size` into `core::output` — D-12
```rust
// Move the EXACT fn from src/commands/flatten/mod.rs:332-344 into core::output and
// re-export; flatten then calls core::output::human_size. Spec (unchanged):
//   bytes < 1024            -> "{bytes} B"
//   else divide by 1024     -> "{value:.1} {unit}"  (B/KB/MB/GB/TB, stop at TB)
// The existing human_size unit test (mod.rs:350-357) moves with it. du right-aligns
// the column to the widest shown value; tree --sizes uses it per-file only (dirs blank).
```

### du immediate-child accumulation — D-11
```rust
// One row per immediate child of the target dir. For each child:
//   - file: its own size
//   - dir : recursive total of all (non-hidden) descendants (walkdir sum)
// --depth N caps how deep totals roll up; --top N truncates the post-sorted list.
// The SUMMARY always uses the FULL scan total, not just shown rows:
//   "{X} of {Y} entries shown. {TOTAL} total."   (X = shown after --top, Y = all children)
// Trailing "/" (ASCII) marks dirs so the distinction survives piping.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| digest `Output` = `GenericArray` (generic-array crate) | digest 0.11 `Output` = `hybrid-array` (behaves like `[u8; N]`) | digest 0.11 (2024–25) | Drop `.as_slice()`; pass `&out` directly to hex encoder (Pitfall 2) |
| `md5` crate for MD5 | `md-5` (RustCrypto, hyphenated) implementing `digest` | RustCrypto era | The `md5` crate is NOT `digest`-interoperable; `md-5` 0.11 shares the trait path (D-02) |
| blake3 hashing via `digest::Digest` | blake3 native `Hasher` (the `digest` impl is `traits-preview`/unstable) | ongoing | D-03 locks the native path to avoid coupling to an unstable feature |
| `generic-array`-based RustCrypto APIs | `hybrid-array`, moving toward const generics | RustCrypto 0.11 line | Simpler ergonomics; only matters at the `finalize()` call site |

**Deprecated/outdated for this phase:**
- blake3 `traits-preview` / `digest`-impl path — explicitly avoided (D-03).
- `humansize` crate — superseded by the promoted local `human_size` (D-12).
- `ignore` crate — deferred to v2 (D-07).
- `GenericArray`/`.as_slice()` digest-0.10 idioms — replaced by hybrid-array `[u8; N]` behavior.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `base16ct::lower::encode_string` requires the `alloc` feature (returns owned `String`) | Standard Stack / Installation note | LOW — 1-line build fix: enable `alloc`, or use `encode_str(&mut buf)` into a fixed buffer (a 64-byte stack buffer suffices for any of these hashes). Caught at compile time. |
| A2 | `regex = "1.12"` default features suffice for `bulk-rename` (no extra `unicode-*` opt-in) | Standard Stack | LOW — default features include Unicode support; if a Unicode class is missing, add the relevant `unicode-*` feature. CONTEXT.md left the version unspecified, so the pin itself is a recommendation, not a locked decision. |
| A3 | `digest` does not need to be added as a direct dep (transitive via `sha2`/`md-5`) | Code Examples / Pattern 1 | LOW — if `use digest::Digest;` fails to resolve, add `digest = "0.11"` (must match the 0.11 line). Compile-time, trivial. |
| A4 | du "recursive total for a directory child" counts file sizes of all non-hidden descendants (not allocated/on-disk size) | du pattern / D-11 | LOW — DU-V2-01 explicitly defers apparent-vs-allocated size; v1 uses logical file size (`metadata().len()`), consistent with the ROADMAP example. Confirm the summary math in the du plan. |

**These four are all LOW-risk, compile-time-or-test-time checks, not design decisions.** None block planning; each is a 1-line resolution flagged so the planner adds a verify step.

## Open Questions

1. **`base16ct` `alloc` feature vs fixed-buffer `encode_str`**
   - What we know: `base16ct::lower::encode_string(&out) -> String` is the RustCrypto-documented helper; `encode_string` needs `alloc`. A no-alloc `encode_str(src, &mut dst) -> &str` exists.
   - What's unclear: whether the planner prefers the owned-`String` ergonomic path (enable `alloc`) or the no-alloc fixed-buffer path.
   - Recommendation: enable `alloc` (shown in the install block) for ergonomics; the per-file CLI has no allocation-pressure reason to avoid it. Either way, verify with a one-line probe in the hash plan.

2. **`digest` import path for the generic RustCrypto helper**
   - What we know: `sha2::Sha256`/`sha2::Sha512`/`md5::Md5` all re-export the `Digest` trait; you can `use sha2::Digest;` (and `md5::Digest;`) without a direct `digest` dep in many cases.
   - What's unclear: whether the chosen generic helper (`fn hash<D: Digest>(...)`) needs `use digest::Digest;` (direct dep) or can use the re-export.
   - Recommendation: try `use sha2::Digest;` first (no new dep); if the generic bound across sha2+md5 needs the canonical trait, add `digest = "0.11"`. Trivial either way.

3. **bulk-rename `--recursive` collision scope across directories**
   - What we know: D-14 locks collision scope to **per containing directory** (two files in different dirs may both become `img_1.jpg`).
   - What's unclear: nothing blocking — this is locked; flagged only so the planner makes the per-dir keying explicit in the data structure (Discretion D: the exact set type).
   - Recommendation: key the occupied set by `(parent_dir, folded_name)` or use one set per directory; mirror flatten's per-run `occupied` but partitioned by parent.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | build all | ✓ | 1.90.0 | — |
| `rustc` | build all | ✓ | 1.90.0 (≥ MSRV for all crates) | — |
| crates.io network (first build to fetch new deps) | blake3/sha2/md-5/base16ct/rayon/regex | ✓ (verified via `cargo search`) | — | offline build fails until deps are vendored/fetched once |
| `x86_64-pc-windows-msvc` target | release build (crt-static) | ✓ (Phase 1 verified the release MSVC + crt-static build) | — | — |
| slopcheck | research legitimacy gate | ✓ | (ran clean) | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none — the only network need is the one-time `cargo fetch` for the six new crates, all confirmed present on crates.io this session.

> Note: `ctx7` (Context7 CLI) was NOT available; documentation was sourced from docs.rs (official crate docs) and doc.rust-lang.org (official std docs) via WebFetch instead — equivalent authority for these crates.

## Validation Architecture

> nyquist_validation is treated as ENABLED (no `.planning/config.json` key set to false was found in scope). This section is consumed by the Nyquist validation-strategy step. The repo's established pattern is: per-command integration tests in `tests/<cmd>.rs` via `assert_cmd` + `assert_fs` + `predicates`, plus CLI-transcript snapshots in `tests/cmd/*.trycmd` (run by the single `trycmd()` test in `tests/cli.rs`). **Every `assert_cmd` invocation sets `NO_COLOR=1`** so output is byte-identical regardless of the runner's TTY (precedent: `tests/flatten.rs:22`).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + `assert_cmd` 2.2, `assert_fs` 1.1, `predicates` 3.1, `tempfile` 3.27, `trycmd` 1.2 (snapshots), `insta` 1.48 (available, used sparingly). All already in `[dev-dependencies]`. |
| Config file | none — standard `tests/` integration layout + `#[cfg(test)]` unit mods in each `mod.rs`/`rename.rs` |
| Quick run command | `cargo test --test hash` (or `tree`/`du`/`dupes`/`bulk_rename`) — single command's suite |
| Full suite command | `cargo test` (all unit + integration) then `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` (the locked Phase-2 gate) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HASH-01 | SHA-256 default on a known-content file → exact `<hash>  <name>` | integration | `cargo test --test hash hash_default_sha256` | ❌ Wave 0 |
| HASH-01 | `--algo blake3` switches; known BLAKE3 of fixed bytes | integration | `cargo test --test hash hash_algo_blake3` | ❌ Wave 0 |
| HASH-01 | `--algo sha512` / `--algo md5` produce known digests | integration | `cargo test --test hash hash_algo_sha512_md5` | ❌ Wave 0 |
| HASH-01 | stdin (no arg / `-`) hashes piped bytes, label `-` | integration | `cargo test --test hash hash_stdin_dash_label` | ❌ Wave 0 |
| HASH-01 | `--verify` correct hash → exit 0; wrong → exit 1 | integration | `cargo test --test hash hash_verify_match_mismatch` | ❌ Wave 0 |
| HASH-01 | `--verify` length auto-detect (32/64/128) picks algo; sha256 wins 64-tie | unit + integration | `cargo test --test hash hash_verify_autodetect` | ❌ Wave 0 |
| HASH-01 | `--verify` unsupported length → exit 2 (Pitfall 1) | integration | `cargo test --test hash hash_verify_bad_len_exit2` | ❌ Wave 0 |
| TREE-01 | box-drawing tree of a fixed tree; dirs-first sort; exact transcript | snapshot | `cargo test --test cli trycmd` (`tests/cmd/tree.trycmd`) | ❌ Wave 0 |
| TREE-01 | `--sizes` shows per-file size, dirs blank; `--depth N` caps | integration | `cargo test --test tree tree_sizes_and_depth` | ❌ Wave 0 |
| TREE-01 | summary `N directories, M files` | integration | `cargo test --test tree tree_count_summary` | ❌ Wave 0 |
| TREE-01 | piped output has no ANSI (color gate) | integration | `cargo test --test tree tree_piped_no_ansi` | ❌ Wave 0 |
| DU-01 | biggest-first order on distinct-size fixture; trailing `/` on dirs | integration | `cargo test --test du du_biggest_first` | ❌ Wave 0 |
| DU-01 | `--top N` truncates shown rows; summary reflects FULL total | integration | `cargo test --test du du_top_and_total_summary` | ❌ Wave 0 |
| DU-01 | `--depth N` aggregation cap | integration | `cargo test --test du du_depth_cap` | ❌ Wave 0 |
| DUPE-01 | identical-content files grouped; unique files not grouped | integration | `cargo test --test dupes dupes_groups_identical` | ❌ Wave 0 |
| DUPE-01 | size pre-filter (same size, different content) → not grouped | integration | `cargo test --test dupes dupes_size_then_hash` | ❌ Wave 0 |
| DUPE-01 | wasted-space summary math; deterministic sorted output | integration | `cargo test --test dupes dupes_wasted_space_sorted` | ❌ Wave 0 |
| DUPE-01 | no file is deleted/modified (read-only) | integration | `cargo test --test dupes dupes_never_writes` | ❌ Wave 0 |
| RENM-01 | dry-run is DEFAULT, writes nothing; preview shows `old -> new` | integration | `cargo test --test bulk_rename renm_dryrun_default_no_write` | ❌ Wave 0 |
| RENM-01 | `--force` executes; capture-group `${1}` replacement | integration | `cargo test --test bulk_rename renm_force_capture_group` | ❌ Wave 0 |
| RENM-01 | collision (two→one name) aborts (exit 1), nothing written, in BOTH dry-run and `--force` | integration | `cargo test --test bulk_rename renm_collision_aborts` | ❌ Wave 0 |
| RENM-01 | cycle/swap (a→b,b→a) detect-and-abort | integration | `cargo test --test bulk_rename renm_cycle_aborts` | ❌ Wave 0 |
| RENM-01 | case-only rename (`foo`→`Foo`) succeeds, not self-collision (Pitfall 5) | integration | `cargo test --test bulk_rename renm_case_only_ok` | ❌ Wave 0 |
| RENM-01 | dirs/symlinks skipped (`-` rows); first-match-only `replace` | integration + unit | `cargo test --test bulk_rename renm_skips_and_first_match` | ❌ Wave 0 |

### Determinism rules for snapshots (Windows-specific)
- **Fix the input bytes.** Write fixtures with exact byte counts/contents so sizes (du, tree `--sizes`) and hashes (hash, dupes) are deterministic. Use distinct sizes so the du/dupes sort key is total.
- **Force `NO_COLOR=1`** on every `assert_cmd` run (existing precedent) so no ANSI leaks into a comparison; add a dedicated "piped has no `\x1b[`" test per styled command (mirror `tests/cli.rs::piped_help_has_no_ansi`).
- **trycmd path normalization:** trycmd rewrites `\`→`/` in stored Windows snapshots (STATE.md [02-04], `tests/cmd/cowsay-single.trycmd`). For `tree`/`du`/`bulk-rename` transcripts, expect forward-slash paths in the `.trycmd` file even though the binary prints backslashes — do not fight the harness; lock the exact byte render with unit tests on the pure formatting fns and use trycmd for the end-to-end shape.
- **Avoid wall-clock/order dependence:** dupes sorts by `(hash, path)`, du by `(size desc, name asc)` BEFORE printing (Pitfall 6). Tree sort is `(dirs-first, name asc)` (D-08). All three are total orders on the fixtures.
- **Known-answer hashes:** for HASH-01, embed the expected lowercase hex of a fixed input (e.g. the SHA-256 / BLAKE3 / SHA-512 / MD5 of `b"box"` or an empty file) as literals so the test is a true known-answer test, not a round-trip.
- **Exit-code coverage:** assert `.code(0)` (match/success), `.code(1)` (verify mismatch, collision abort, I/O error), `.code(2)` (bad `--verify` length / bad args) explicitly — the 0/1/2 contract is inherited from `main.rs` and must be re-verified per command.

### Sampling Rate
- **Per task commit:** `cargo test --test <cmd>` for the command touched, plus `cargo clippy --all-targets -- -D warnings` on the changed crate surface.
- **Per wave merge:** `cargo test` (full unit + integration) + `cargo fmt --check`.
- **Phase gate:** full `cargo test` green + `cargo clippy -- -D warnings` clean + `cargo fmt --check` clean before `/gsd:verify-work` (the exact Phase-2 close-out gate per STATE.md).

### Wave 0 Gaps
- [ ] `tests/hash.rs` — covers HASH-01 (default/algos/stdin/verify/exit-2)
- [ ] `tests/tree.rs` + `tests/cmd/tree.trycmd` — covers TREE-01 (render/sizes/depth/summary/no-ansi)
- [ ] `tests/du.rs` — covers DU-01 (biggest-first/top/depth/total-summary)
- [ ] `tests/dupes.rs` — covers DUPE-01 (groups/size-prefilter/wasted-space/never-writes)
- [ ] `tests/bulk_rename.rs` — covers RENM-01 (dry-run default/force/collision/cycle/case-only/skips)
- [ ] Unit tests in each `mod.rs` for pure fns: `Hasher` known-answers, tree glyph/sort, du human-size/sort, bulk-rename plan + collision detection (testable without I/O, like `flatten::rename`)
- [ ] No new framework install needed — all dev-deps already present.

## Security Domain

> `security_enforcement` defaults to enabled (no `false` found in scope). This is a local-filesystem CLI with no network, auth, or session surface, so most ASVS categories are N/A; the live ones are input validation and the integrity/safety of the destructive `bulk-rename`.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No identity surface |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | Runs with the invoking user's FS permissions; no privilege boundary crossed |
| V5 Input Validation | yes | Path args normalized via `core::fs::normalize_path` (dunce, no `\\?\` leak); `--verify` length-validated → exit 2 on bad input; regex compiled with `Regex::new` and a compile error surfaces as a clean stderr message (never a panic, FOUND-05) |
| V6 Cryptography | partial | Hashing uses vetted crates (`sha2`/`blake3`/`md-5`) — never hand-rolled. ⚠️ `--verify` compare is intentionally PLAIN `==` (D-04): a file checksum is a *public integrity value*, not a secret, so constant-time comparison is NOT required and would be cargo-culting. MD5 is offered for legacy interop only and is NOT presented as a security guarantee. |

### Known Threat Patterns for a local FS CLI
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via crafted rename target | Tampering | bulk-rename replaces only the base name (D-16); a regex replacement that injects `/`/`\` would change the base name only within its own directory — but the planner should ensure a replacement producing a separator is rejected or treated as an invalid (skipped) target, since the rename is scoped to the parent dir. Flag: confirm in the bulk-rename plan that a target containing a path separator is refused (mirror flatten's "no separator survives" invariant). |
| Silent destination overwrite on rename | Tampering / DoS (data loss) | Pre-flight collision detection (D-18) — the *only* backstop; `std::fs::rename` will not stop an overwrite |
| Reserved-name / trailing-dot collapse on Windows | Tampering | bulk-rename's collision set should fold names the way `flatten::rename::sanitize_reserved`/`dedupe` do (full-Unicode `to_lowercase`, trailing-dot awareness) so a target that Windows would silently rewrite cannot collapse onto another file undetected |
| ReDoS via a pathological user regex | DoS | `regex` crate has linear-time matching by construction (no catastrophic backtracking) — inherent mitigation; no extra control needed |
| Panic on bad input (bad path / bad regex / unreadable file) | DoS | All fallible I/O `.context(...)`-wrapped → clean stderr + exit 1/2, never a panic (FOUND-05, D-19) |
| Symlink-loop / following | DoS | `follow_links(false)` on every walker (D-06); symlinks skipped as `-` rows in bulk-rename (D-15) |

> ⚠️ **Planner action (V5):** add an explicit invariant/test that a bulk-rename target containing a path separator (`/` or `\`) — which a careless `${1}` replacement could in principle produce — is refused/skipped, never executed. This mirrors flatten's `encode_relative` "no separator survives" property (`src/commands/flatten/rename.rs` + the `encode_no_separator` test) and closes the only path-injection avenue a destructive rename opens.

## Sources

### Primary (HIGH confidence)
- `docs.rs/blake3/1.8.5/blake3/struct.Hasher.html` — `new()`, `update(&[u8])`, `update_reader(impl Read) -> io::Result<&mut Self>` (reader by value, `std` feature default-on), `update_rayon` (gated by `rayon` feature), `finalize() -> Hash`
- `docs.rs/blake3/1.8.5/blake3/struct.Hash.html` — `to_hex() -> ArrayString` "lowercase hexadecimal", 64 chars; `Display`/`to_string()` lowercase
- `docs.rs/digest/0.11.3/digest/trait.Digest.html` — `new()`, `update(impl AsRef<[u8]>)`, `finalize(self) -> Output<Self>`, `digest()` convenience; `Output` is a type alias (hybrid-array in 0.11)
- `docs.rs/regex/1.12.4/regex/struct.Regex.html` — `replace` = leftmost-first (first match only), returns `Cow<'h, str>`; `${1}` brace syntax + the `$1abc` named-group foot-gun; nonexistent group → empty; `$$` literal
- `doc.rust-lang.org/std/fs/fn.rename.html` — Windows replaces an existing `to`; **no no-overwrite option** (unlike `File::create_new`)
- crates.io via `cargo search` (2026-06-22) — blake3 1.8.5, sha2 0.11.0, md-5 0.11.0, rayon 1.12.0, regex 1.12.4, base16ct 1.0.0, digest 0.11.3
- slopcheck (ran clean) — all six new crates `[OK]`
- Local codebase: `src/core/{fs,input,output}.rs`, `src/commands/flatten/{mod,rename}.rs`, `src/cli.rs`, `src/main.rs`, `tests/{flatten,cli}.rs`, `tests/cmd/cowsay-single.trycmd`, `Cargo.toml`, `CLAUDE.md`, `.planning/{STATE,REQUIREMENTS}.md`, `03-CONTEXT.md`

### Secondary (MEDIUM confidence)
- WebSearch (verified against official sources) — `base16ct::lower::encode_string` as RustCrypto's hex helper; digest 0.11 `GenericArray`→`hybrid-array` migration (cross-referenced to github.com/RustCrypto migration notes); `std::fs::rename`→`MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` (cross-referenced to the official std docs)

### Tertiary (LOW confidence)
- None material — every load-bearing claim was cross-verified against an authoritative source. The four Assumptions Log items are LOW-risk compile/test-time checks, not unverified facts.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions `cargo search`-confirmed, all crate APIs read from docs.rs for the pinned versions, all slopcheck-clean from first-party sources
- Architecture/patterns: HIGH — every command layers on Phase-1 code that was read in full this session; the new API call sites are verified signatures
- Pitfalls: HIGH — the load-bearing rename-overwrite hazard, the digest 0.11 output-type change, and the regex foot-gun are all confirmed against official docs
- Validation: HIGH — mirrors the exact, already-shipped Phase-1/2 test conventions (assert_cmd + trycmd + NO_COLOR) verified by reading the existing tests

**Research date:** 2026-06-22
**Valid until:** ~2026-07-22 (30 days; stable crates, but re-check sha2/md-5/digest/blake3 patch versions and `regex`/`base16ct` if the build is delayed — the 0.11 RustCrypto line is recent and still moving)
