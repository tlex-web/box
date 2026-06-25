---
phase: 06-scriptable-core-foundation
reviewed: 2026-06-25T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/cli.rs
  - src/commands/hash/mod.rs
  - src/commands/uuid/mod.rs
  - src/core/config.rs
  - src/core/errors.rs
  - src/core/mod.rs
  - src/core/output.rs
  - src/main.rs
  - tests/config.rs
  - tests/hash.rs
  - tests/uuid.rs
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-06-25
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Reviewed the Phase-6 scriptable spine: `core::output` (`emit_json`/`out_line`/`flush_clip` + the `JSON_ON`/`CLIP_ON`/`CLIP_BUF` globals), the hand-rolled `core::config` TOML resolver, `BoxError::Config`, the global `--json`/`--clip` flags in `cli.rs`/`main.rs`, and the first two consumers (`uuid`, `hash`, including the BLAKE3-default flip).

The implementation is careful and well-tested. JSON purity (no BOM via `to_writer_pretty`, no ANSI because `init_output` force-disables color, single trailing `\n`) holds. The config precedence (CLI > env > config > builtin) is correctly built with `Option::or` chains, and the `--verify` length table is genuinely left intact so stored SHA-256 baselines do not silently break. The arboard main-thread discipline is respected (`flush_clip` runs once in `main()` after dispatch).

No BLOCKER-class defects were found. However, there are several correctness and robustness gaps worth fixing — most notably an unflushed-stdout ordering hazard between `emit_json` and the `flush_clip` stderr confirmation, a `CLIP_BUF` semantics gap that means the `--json --clip` clipboard payload can lose a trailing brace difference vs. stdout, a silent no-output edge case for `box uuid -n 0`, and several stale/misleading doc comments that will mislead the Phase-7 commands that copy this code as their template.

## Warnings

### WR-01: `emit_json` writes to a locked stdout handle that is never explicitly flushed before `flush_clip`/exit

**File:** `src/core/output.rs:124-134`
**Issue:** `emit_json` takes `std::io::stdout().lock()`, calls `serde_json::to_writer_pretty(&mut out, …)` and `out.write_all(b"\n")`, then returns — the `StdoutLock` is dropped without an explicit `flush()`. `StdoutLock` is buffered (a `LineWriter` over a `BufWriter`); `to_writer_pretty` emits a multi-line document with no terminating newline, and the final `write_all(b"\n")` does push a newline (which a `LineWriter` flushes on). In practice the line-buffer flushes, but this relies on an implementation detail of `Stdout`'s `LineWriter`. The `out_line` path (`println!`) and the `emit_json` path use different stdout handles, and `flush_clip` then writes to **stderr**. If stdout buffering ever changes (e.g. piping fully buffers), JSON could be emitted after the stderr "Copied to clipboard" line or interleave oddly. This is the exact "JSON output purity / ordering" risk the phase flags.
**Fix:** Flush explicitly before returning, so the contract does not depend on `LineWriter` internals:
```rust
pub fn emit_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value).context("serializing --json output")?;
    out.write_all(b"\n")?;
    out.flush().context("flushing --json output")?;
    // ... clip tee unchanged
}
```

### WR-02: `--json --clip` copies a clipboard payload that differs from stdout (no trailing newline), and the docs claim they match

**File:** `src/core/output.rs:118-134` and `:160-175`
**Issue:** Under `--json --clip`, `emit_json` writes `to_writer_pretty(...) + "\n"` to stdout but tees `to_string_pretty(value)` (no trailing `\n`) into `CLIP_BUF`. `flush_clip` then `trim_end()`s the buffer. The net clipboard content is the pretty JSON with no trailing newline — which happens to be fine, but the doc comment at line 119-121 says it "tees the whole document," implying parity with stdout. More importantly, `emit_json` serializes the value **twice** (`to_writer_pretty` for stdout, `to_string_pretty` for the buffer). For `uuid`/`hash` the value is deterministic, so the two serializations agree — but the "no-drift, single serializer" guarantee the spine advertises (one serialize feeding both channels) is not actually upheld here: a future value whose `Serialize` impl is non-deterministic (e.g. a `HashMap` field) would put a *different* document on the clipboard than on stdout. The no-drift invariant should be enforced by serializing once.
**Fix:** Serialize once into a `String`, write that to stdout, and tee the same bytes:
```rust
pub fn emit_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    let s = serde_json::to_string_pretty(value).context("serializing --json output")?;
    let mut out = std::io::stdout().lock();
    out.write_all(s.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    if CLIP_ON.load(Ordering::Relaxed) {
        CLIP_BUF.lock().unwrap().push_str(&s);
    }
    Ok(())
}
```

### WR-03: `box uuid -n 0` (and `hash` count semantics) silently produces no output and exits 0

**File:** `src/commands/uuid/mod.rs:45-46, 59-81`
**Issue:** `count: u8` with `default_value_t = 1` accepts `-n 0`. With `count == 0` the `rows` vec is empty: the human path prints nothing, the JSON path emits `{"results":[],"count":0}`, and under `--clip` `flush_clip` no-ops. The command exits 0 having done nothing the user could plausibly want. For a "generate a UUID" tool, `-n 0` is almost certainly a user mistake that should be a usage error (exit 2) or be clamped to 1, not a silent success. The same class of silent-empty applies if a future caller wires a 0 count. This is a correctness/UX defect (regression-class: a script checking "did I get a UUID?" gets exit 0 + empty stdout).
**Fix:** Reject `0` at parse time with a clap `value_parser` range, or guard in `run`:
```rust
#[arg(short = 'n', long = "count", default_value_t = 1,
      value_parser = clap::value_parser!(u8).range(1..))]
pub count: u8,
```

### WR-04: `flush_clip` trims ALL trailing whitespace, not "exactly one trailing newline" as documented

**File:** `src/core/output.rs:152-175`
**Issue:** The doc comment (line 158) states "the trailing whitespace is trimmed exactly once (D-07, reusing `clip/mod.rs`'s single-shot arboard flow)." The implementation uses `text.trim_end()` (lines 165, 169), which strips *every* trailing whitespace character (all newlines, spaces, tabs), not exactly one. `clip/mod.rs::trim_one_trailing_newline` strips at most one `\n`/`\r\n` — the opposite policy. For the `out_line` path each line ends in `\n`, so a multi-line `uuid -n 5 --clip` buffer ends with a single `\n` and `trim_end` removes just that one — coincidentally matching the doc. But if any command's last `out_line` argument itself ends in whitespace (e.g. a value with a trailing space, or a blank final line), `trim_end` silently eats it from the clipboard while stdout kept it. The clipboard and stdout then disagree on trailing bytes. The comment also wrongly claims it reuses `clip/mod.rs`'s logic; it does not.
**Fix:** Either update the doc to say "all trailing whitespace is stripped" (if that is the intended policy), or strip exactly one terminator to match `clip` semantics and the stated contract. If matching `clip`:
```rust
let trimmed = crate::commands::clip::trim_one_trailing_newline(text.as_bytes().to_vec());
// (requires making that helper pub(crate) and handling the String/Vec<u8> boundary)
```
At minimum, correct the doc comment so Phase-7 authors copying this primitive understand the real behavior.

### WR-05: BLAKE3 probe hint re-opens and fully re-hashes the file on every 64-hex verify mismatch — unbounded work triggered by a failed verify

**File:** `src/commands/hash/mod.rs:298-330` (called from `:234-239`)
**Issue:** On any 64-hex `--verify` mismatch with no `--algo` (the common "I pasted a checksum and it didn't match" case), `emit_blake3_probe_hint` calls `read_file_or_stdin(Some(path))` and `digest_reader(Algo::Blake3, …)` — i.e. it streams and hashes the **entire file a second time** purely to decide which of two stderr hint strings to print. For a multi-GB file (exactly the streaming case this command is built around per T-03-03), a single failed `box hash --verify <wrong-64hex> bigfile` now reads the whole file twice. This is correctness-adjacent: a verify failure is a hot path (CI, download checks), and doubling the I/O to produce a cosmetic hint is a denial-of-effort footgun. The static hint (the `else` branch) already exists and is informative on its own.
**Fix:** Gate the decisive re-hash behind a size threshold, or drop the re-hash entirely and always emit the static transitional hint (the decisive variant is a nicety, not load-bearing):
```rust
// Only attempt the decisive re-hash for small inputs; otherwise emit the static hint.
let decisive = match path {
    Some(p) if std::fs::metadata(p).map(|m| m.len() <= 8 * 1024 * 1024).unwrap_or(false) => {
        // ... existing re-open + blake3 compare
    }
    _ => false,
};
```

## Info

### IN-01: Stale doc comment references a nested config key (`hash.default_algo`) that does not exist

**File:** `src/commands/hash/mod.rs:43` and `src/core/config.rs:43-45`
**Issue:** The `Config` field doc says it is "the `hash.default_algo` escape hatch" and the hash module header (line 12) and arg doc (line 59) correctly use the flat key `default_hash_algo`. The `hash.default_algo` phrasing implies a nested TOML table (`[hash] default_algo = "sha256"`), but `deny_unknown_fields` would *reject* that form. A user reading the field doc and writing `[hash]\ndefault_algo = "sha256"` gets an exit-2 config error.
**Fix:** Replace `hash.default_algo` with the actual flat key `default_hash_algo` in the doc comment at `config.rs:43`.

### IN-02: `resolve_algo` is dead in production — only the inline `or` chain in `hash` is live

**File:** `src/core/config.rs:127-130`
**Issue:** `resolve_algo` carries `#[allow(dead_code)]` and is exercised only by the `precedence_matrix` unit test. The header comment (line 126) claims "`hash` adopts it as the live resolver in Plan 06-02 (allow removed there)," but `hash/mod.rs:253-260` hand-rolls its own `cli.or_else(env).or(cfg).unwrap_or(Blake3)` chain instead of calling `resolve_algo`. So the canonical resolver is NOT actually the live path, the `#[allow(dead_code)]` was NOT removed, and the comment is inaccurate. The two chains agree today, but having the "canonical" resolver bypassed means a future precedence fix could be applied to one and not the other.
**Fix:** Make `hash` call `crate::core::config::resolve_algo(cli_algo, env_algo, config().default_hash_algo)` and remove the `#[allow(dead_code)]`, OR update the comment to admit the resolver is test-only scaffolding. The former restores the single-source-of-truth the comment promises.

### IN-03: `init_config` discards the `OnceLock::set` result, masking a double-init

**File:** `src/core/config.rs:69-72`
**Issue:** `let _ = CONFIG.set(load()?)` silently ignores the `Result` from `set`. If `init_config` were ever called twice (e.g. a future test harness or a refactor), the second config would be silently dropped and the first would win, with no signal. For a process-global set-once primitive this is usually intended, but discarding it with `let _ =` removes the ability to detect a programming error.
**Fix:** Either `expect` to assert single-init, or document why a redundant init is acceptable:
```rust
CONFIG.set(load()?).map_err(|_| ()).ok(); // benign: only main() inits; ignore re-init
```
This is informational — the current single-caller (`main()`) makes it safe in practice.

### IN-04: `tests/config.rs` module doc says `config_path` uses `dirs::config_dir()`, but the code reads `%APPDATA%` directly first

**File:** `tests/config.rs:8-12`
**Issue:** The test module doc states "`core::config::config_path` resolves `%APPDATA%\box\config.toml` via `dirs::config_dir()`, which reads the `APPDATA` env var on Windows." But `config.rs:108-117` was deliberately changed (per the in-code Rule-1 comment) to read `APPDATA` *directly* first precisely because `dirs::config_dir()` ignores `APPDATA` on Windows. The test comment describes the rejected approach. The tests still pass (they set `APPDATA`), but the rationale documented in the test contradicts the production code.
**Fix:** Update the `tests/config.rs` doc block to state that `config_path` reads `APPDATA` directly (with `dirs::config_dir()` only as the non-Windows / APPDATA-unset fallback).

### IN-05: `algo_from_len`'s shadowing `len => Err(...)` arm reuses the binding name confusingly

**File:** `src/commands/hash/mod.rs:135-142`
**Issue:** The wildcard arm `len => Err(BoxError::UnsupportedHashLength { len })` rebinds `len` to shadow the function parameter `len`. It is correct (the match scrutinee is `len`, so the binding equals the parameter), but reusing the same identifier as a catch-all binding obscures that this is the "everything else" arm. A plain `_` with the outer `len` would read more clearly.
**Fix:** `other => Err(BoxError::UnsupportedHashLength { len: other })`, or `_ => Err(BoxError::UnsupportedHashLength { len })` (the latter relies on the captured outer `len`, which is clearer about intent).

### IN-06: `HashOutput` literal initializes `count` then `results`, opposite to the struct's field/serialization order

**File:** `src/commands/hash/mod.rs:126-130` and `:266-278`
**Issue:** `HashOutput` declares `results` then `count`; serde serializes in declaration order (`{"results":…,"count":…}`, which the tests assert). The constructor at line 267 lists `count: 1` before `results`, and hardcodes `count: 1` rather than deriving it from `results.len()` (as `uuid` does at `uuid/mod.rs:70`). For the single-file Phase-6 case `1` is always correct, but hardcoding the count instead of `results.len()` is a latent drift risk when Phase-8 makes `hash` multi-file: the count and the array length could disagree. `uuid` already does this correctly with `count: rows.len()`.
**Fix:** Derive the count to stay drift-proof and match the `uuid` template: `count: results.len()` with `results` built first (or compute the vec into a local and use its `.len()`).

---

_Reviewed: 2026-06-25_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
