//! The `hash` command: compute and verify file checksums (HASH-01 / HASH-V2-01).
//!
//! Streaming, enum-dispatch hasher (D-03): BLAKE3 (the v2 COMPUTE default — D-04),
//! SHA-256, SHA-512, and MD5 selected by `--algo`. Input is a path / stdin /
//! `--file` routed through [`crate::core::input::read_file_or_stdin`] (the deferred
//! `--file` layer this command is the first consumer of, D-05), and is **streamed**
//! into the hasher — never buffered whole (no `read_to_end` of a multi-GB payload,
//! T-03-03).
//!
//! **BLAKE3-default (HASH-V2-01, breaking — COMPUTE only):** `box hash <file>` with
//! no `--algo` now emits BLAKE3 where v1 emitted SHA-256. The precedence is
//! CLI > env (`BOX_HASH_DEFAULT_ALGO`) > config (`default_hash_algo`) > builtin
//! BLAKE3 (SPINE-05), so `--algo sha256` or a config key restore SHA-256. The
//! `--verify` length table is UNCHANGED (a bare 64-hex still maps to sha256), so no
//! stored SHA-256 baseline silently breaks; a 64-hex mismatch with no `--algo`
//! prints a BLAKE3-fallback diagnostic hint on stderr (D-05).
//!
//! **Multi-file (HASH-V2-02):** `box hash a.bin b.bin` hashes every positional
//! path in argument order, printing one coreutils text-mode `{digest}␣␣{label}`
//! row (TWO spaces) per readable file and, under `--json`, ONE
//! `{"results":[…],"count":N}` document. The partial-failure policy is coreutils
//! best-effort: an unreadable file logs `error: …` on stderr and forces a final
//! `exit 1`, but the other files are still hashed and printed. Under `--json` the
//! document carries only the SUCCESSFUL rows and the process still exits 1 — a
//! deliberate partial-success refinement of D-09 (whose empty-stdout rule targets
//! TOTAL failure; A1). `--verify` stays single-input (the first/only path); the
//! fan-out applies to the compute path only this phase. A stderr-only file-count
//! progress bar appears for batches above [`PROGRESS_FILE_THRESHOLD`], never under
//! `--json` (Pitfall 2).
//!
//! `--json` (SPINE-01 / D-02): `box hash <file> --json` emits one
//! `{"results":[{"path":…,"algo":…,"digest":…}],"count":1}` document; `--clip`
//! tees the digest to the clipboard via `out_line` (SPINE-03 / D-07).
//!
//! `--verify EXPECTEDHASH` (D-04):
//! - the algorithm is the explicit `--algo` if set, else auto-detected by the hex
//!   length: 32→md5, 64→**sha256** (wins the sha256/blake3 tie — UNCHANGED), 128→sha512;
//!   any other length is a usage error → exit 2 via [`BoxError::UnsupportedHashLength`].
//! - comparison is case-insensitive and plain (`eq_ignore_ascii_case`), NOT
//!   constant-time: a checksum is a PUBLIC integrity value, not a secret (T-03-01).
//! - match → exit 0 (no output); mismatch → a clear stderr message → exit 1 (a
//!   plain `anyhow` error, NOT the exit-2 variant, RESEARCH Pitfall 1). A 64-hex
//!   mismatch with no `--algo` (not under `--json`) also prints the D-05 hint.
//!
//! Hex encoding (open item resolved): RustCrypto arms use `const-hex::encode`
//! (already on hand — no `base16ct` `alloc` feature toggle needed); blake3
//! self-hexes via `Hash::to_hex()` (already lowercase 64-hex). The RustCrypto
//! `digest::Digest` 0.11 `finalize()` output is a hybrid-array that satisfies
//! `AsRef<[u8]>`, so it passes straight to `const_hex::encode` with no
//! `.as_slice()` (digest 0.11 dropped `GenericArray`).

use std::io::Read;

use anyhow::{bail, Context};
use clap::{Args, ValueEnum};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use md5::Md5;
use sha2::{Digest, Sha256, Sha512};

use crate::commands::RunCommand;
use crate::core::errors::BoxError;
use crate::core::input::read_file_or_stdin;

/// Streaming read buffer for the RustCrypto incremental `update` loop. blake3
/// manages its own SIMD-sized internal buffer via `update_reader`.
const READ_BUF: usize = 64 * 1024;

/// "Large input" cutoff for the HASH-V2-02 stderr progress bar (Claude's
/// Discretion): a file-count bar is shown only when MORE than this many files are
/// requested. Below it (the overwhelmingly common single-file case) no bar is
/// drawn, so the default `box hash <file>` stderr stays empty and the existing
/// snapshots are unaffected. The bar is always stderr-only and never constructed
/// under `--json` (Pitfall 2).
const PROGRESS_FILE_THRESHOLD: usize = 8;

/// `box hash [--algo ALGO] [--verify HASH] [PATH]` — compute or verify a file
/// checksum (HASH-01). Reads PATH, piped stdin, or `-` via the shared input layer.
#[derive(Debug, Args)]
pub struct HashArgs {
    /// Hash algorithm. Unset means BLAKE3 when computing (the v2 default — D-04;
    /// pass `--algo sha256` or set `default_hash_algo` in the config to restore
    /// SHA-256), or (under `--verify`) auto-detect by the digest's hex length. An
    /// EXPLICIT `--algo` ALWAYS wins — it is never overridden by length
    /// auto-detection (WR-01).
    #[arg(long, value_enum)]
    pub algo: Option<Algo>,

    /// Verify the input against this expected hex digest; exit 0 on match, 1 on
    /// mismatch, 2 on an unsupported length. Without `--algo`, the algorithm is
    /// auto-detected from the hex length (32→md5, 64→sha256, 128→sha512); WITH an
    /// explicit `--algo`, that algorithm is used verbatim (WR-01).
    #[arg(long)]
    pub verify: Option<String>,

    /// Files to hash; pass several for a coreutils-style batch (one
    /// `<digest>  <filename>` line each, in argument order). Omit (or `-`) to read
    /// from piped stdin (labelled `-`). Under `--verify`, only the FIRST path is
    /// checked (verify stays single-input this phase).
    #[arg(value_name = "PATH")]
    pub paths: Vec<String>,
}

/// The supported hash algorithms (D-02). Value spellings are locked to
/// `sha256`/`blake3`/`sha512`/`md5` (Discretion D); excludes sha1/sha224/sha384.
///
/// Round-trips BOTH directions of the spine (06-02):
/// - `serde::Deserialize` (+ `#[serde(rename_all = "lowercase")]`, added in 06-01)
///   lets the config value `default_hash_algo = "sha256"` parse into `Algo::Sha256`;
/// - `serde::Serialize` (added here, 06-02) lets the `--json` output serialize
///   `Algo::Blake3` to the lowercase `"blake3"` literal in the `HashRow.algo` field.
///
/// The lowercase spellings also match the `ValueEnum` variants, so [`parse_algo`]
/// can reuse `ValueEnum::from_str` as the ONE env+config string→`Algo` parse path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algo {
    /// SHA-256 (the v1 default; restored via `--algo sha256` or the config key).
    Sha256,
    /// BLAKE3 (the v2 COMPUTE default — D-04).
    Blake3,
    /// SHA-512.
    Sha512,
    /// MD5 (legacy interop only — not a security guarantee).
    Md5,
}

/// Parse an environment-variable (or any string) value into an [`Algo`], reusing
/// the `ValueEnum` parser so env, config, and `--algo` all share ONE spelling
/// table (`sha256`/`blake3`/`sha512`/`md5`, case-insensitive). Returns `None` for
/// an unrecognized value — the env tier then simply falls through to the next
/// precedence tier rather than erroring a normal `box hash` (Anti-Pattern 3).
fn parse_algo(s: &str) -> Option<Algo> {
    Algo::from_str(s, true).ok()
}

/// One row of `box hash --json` output (D-03 field names: `path`, `algo`,
/// `digest`). `algo` serializes lowercase (`"blake3"`) via the enum's
/// `rename_all`. The `digest` and `algo` come from the SAME compute the human row
/// prints, so the JSON can never report a different digest than the `<hash>
/// <label>` line (no-drift, Pattern 2).
#[derive(serde::Serialize)]
struct HashRow {
    path: String,
    algo: Algo,
    digest: String,
}

/// The `box hash --json` document (D-01/D-02): a `results` array wrapped in an
/// object with a `count`, ALWAYS wrapped even for the single-file Phase-6 case
/// (so the shape stays compatible with Phase-8 multi-file `hash`). Locked field
/// names: `results`, `count`.
#[derive(serde::Serialize)]
struct HashOutput {
    results: Vec<HashRow>,
    count: usize,
}

/// Auto-detect the algorithm for a `--verify` value by its hex length (D-04):
/// 32→md5, 64→sha256 (wins the sha256/blake3 64-tie), 128→sha512. Any other
/// length is a usage error mapped to exit 2 by `main()`.
fn algo_from_len(len: usize) -> Result<Algo, BoxError> {
    match len {
        32 => Ok(Algo::Md5),
        64 => Ok(Algo::Sha256),
        128 => Ok(Algo::Sha512),
        len => Err(BoxError::UnsupportedHashLength { len }),
    }
}

/// Stream `reader` through a RustCrypto `digest::Digest` hasher and return the
/// lowercase-hex digest. Reads in 64 KiB chunks — never the whole payload at once
/// (T-03-03). The `finalize()` output (a digest-0.11 hybrid-array) is `AsRef<[u8]>`,
/// so it hex-encodes with no `.as_slice()`.
fn hash_rustcrypto<D: Digest, R: Read>(mut hasher: D, mut reader: R) -> anyhow::Result<String> {
    let mut buf = vec![0u8; READ_BUF];
    loop {
        let n = reader
            .read(&mut buf)
            .context("failed to read input while hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(const_hex::encode(hasher.finalize()))
}

/// Stream `reader` through the **native stable** `blake3::Hasher` (NOT the
/// `traits-preview` `digest::Digest` impl, D-03). `update_reader` takes the reader
/// by value and streams with a SIMD-sized buffer; `to_hex()` is already lowercase
/// 64-hex.
fn hash_blake3<R: Read>(reader: R) -> anyhow::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(reader)
        .context("failed to read input while hashing")?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Compute the lowercase-hex digest of `reader` under `algo`. Sha256/Sha512/Md5
/// dispatch into the shared RustCrypto path; Blake3 into its native arm.
fn digest_reader<R: Read>(algo: Algo, reader: R) -> anyhow::Result<String> {
    match algo {
        Algo::Sha256 => hash_rustcrypto(Sha256::new(), reader),
        Algo::Sha512 => hash_rustcrypto(Sha512::new(), reader),
        Algo::Md5 => hash_rustcrypto(Md5::new(), reader),
        Algo::Blake3 => hash_blake3(reader),
    }
}

impl RunCommand for HashArgs {
    fn run(self) -> anyhow::Result<()> {
        let cli_algo = self.algo;

        match self.verify {
            // --verify stays SINGLE-INPUT this phase (the first/only path); the
            // multi-file fan-out applies to the COMPUTE path only (HASH-V2-02). Pick
            // the algorithm — an EXPLICIT --algo ALWAYS wins; only a truly-unset
            // --algo falls back to length auto-detect (WR-01). The verify length
            // table (algo_from_len) is UNCHANGED by the v2 flip: a bare 64-hex still
            // maps to sha256, so stored SHA-256 baselines never silently break
            // (D-04, the #1 v2 data-risk backstop).
            Some(expected) => {
                // The first positional path (or None → stdin) is the verify target.
                // Capture `path_for_probe` (Some only for a real on-disk path, NOT
                // stdin/`-`) BEFORE `read_file_or_stdin` consumes it: the streaming
                // reader is single-pass, so the D-05 BLAKE3 probe must re-open the
                // file — and there is no second read for piped stdin.
                let first = self.paths.into_iter().next();
                let path_for_probe = match first.as_deref() {
                    Some(p) if p != "-" => Some(p.to_string()),
                    _ => None,
                };
                let input = read_file_or_stdin(first)?;
                let label = input.label;

                let expected = expected.trim();
                let algo = match cli_algo {
                    // Explicit choice is honored verbatim, even when the digest's
                    // length would map to a DIFFERENT algorithm (e.g. `--algo
                    // sha256 --verify <32-hex>` is sha256, NOT md5).
                    Some(a) => a,
                    // No --algo: auto-detect by length. An unsupported length
                    // returns the typed variant → exit 2 (never a panic).
                    None => algo_from_len(expected.len())?,
                };
                let computed = digest_reader(algo, input.reader)?;
                if computed.eq_ignore_ascii_case(expected) {
                    // Match → exit 0, no extra output (quiet success).
                    Ok(())
                } else {
                    // D-05 BLAKE3-fallback probe: on a 64-hex mismatch with NO
                    // explicit --algo (so the value was verified as sha256) and
                    // NOT under --json (the probe is a human stderr hint — D-09
                    // keeps the JSON channel pure), emit a transitional hint. When
                    // a real path is available we re-open it and compute BLAKE3:
                    // if the value MATCHES the file's blake3 the hint is DECISIVE;
                    // otherwise (or for piped stdin, where no second read exists)
                    // it degrades to the STATIC transitional hint.
                    if expected.len() == 64
                        && cli_algo.is_none()
                        && !crate::core::output::is_json_on()
                    {
                        emit_blake3_probe_hint(expected, path_for_probe.as_deref());
                    }
                    // Mismatch → a plain anyhow error (exit 1), NOT the exit-2
                    // UnsupportedHashLength variant (RESEARCH Pitfall 1). Exit
                    // STAYS 1 — the probe above only adds a stderr hint.
                    bail!("hash mismatch for {label}: expected {expected}, got {computed}");
                }
            }
            // No --verify: COMPUTE digests for one or more files (HASH-V2-02).
            None => run_compute(cli_algo, self.paths),
        }
    }
}

/// Compute and emit digests for `paths` — the HASH-V2-02 multi-file compute path.
///
/// An empty `paths` reads stdin once (label `-`, the unchanged single-input
/// behavior); otherwise each path is hashed in argument order. The human path
/// prints one coreutils two-space `{digest}  {label}` row per readable file via
/// `out_line` (so `--clip` tees each digest, D-07); under `--json` the ONLY stdout
/// write is one `{results,count}` document.
///
/// **Partial-failure policy (A1, coreutils best-effort):** an unreadable file logs
/// `error: …` on stderr and forces a final `exit 1`, but the rest of the batch is
/// still hashed and reported. Under `--json` the document carries only the
/// SUCCESSFUL rows and the process still exits 1 — the deliberate partial-success
/// refinement of D-09 (whose empty-stdout rule targets TOTAL failure).
///
/// Progress: a file-count bar is drawn to STDERR only for batches larger than
/// [`PROGRESS_FILE_THRESHOLD`] and never under `--json` (Pitfall 2) — it never
/// touches stdout.
fn run_compute(cli_algo: Option<Algo>, paths: Vec<String>) -> anyhow::Result<()> {
    // Resolve the algorithm ONCE via the EXISTING CLI > env > config > builtin
    // chain (do not duplicate the resolver): an explicit `--algo`, else
    // `BOX_HASH_DEFAULT_ALGO` (reusing `parse_algo`), else the config
    // `[hash] default_algo` (nested since D-13), else the v2 builtin BLAKE3
    // (D-04). Every file in the batch shares this one algorithm.
    let algo = cli_algo
        .or_else(|| {
            std::env::var("BOX_HASH_DEFAULT_ALGO")
                .ok()
                .and_then(|s| parse_algo(&s))
        })
        .or(crate::core::config::config().hash.default_algo)
        .unwrap_or(Algo::Blake3);

    // Empty Vec → a single stdin target (label `-`); else one target per path.
    let targets: Vec<Option<String>> = if paths.is_empty() {
        vec![None]
    } else {
        paths.into_iter().map(Some).collect()
    };

    // Fork once on --json: under it, no human rows and no progress (Pitfall 1/2).
    let json = crate::core::output::is_json_on();

    // stderr-only file-count progress, only for a batch above the cutoff and only
    // when --json is off; never constructed (and never drawn to stdout) otherwise.
    let progress = if !json && targets.len() > PROGRESS_FILE_THRESHOLD {
        let pb =
            ProgressBar::with_draw_target(Some(targets.len() as u64), ProgressDrawTarget::stderr());
        pb.set_style(
            ProgressStyle::with_template("{bar:30} {pos}/{len} files hashed")
                .unwrap_or_else(|_| ProgressStyle::default_bar()),
        );
        Some(pb)
    } else {
        None
    };

    let mut rows: Vec<HashRow> = Vec::with_capacity(targets.len());
    let mut had_error = false;
    for t in targets {
        match read_file_or_stdin(t).and_then(|inp| {
            let label = inp.label.clone();
            digest_reader(algo, inp.reader).map(|d| (label, d))
        }) {
            Ok((label, digest)) => {
                if !json {
                    // The line-281 two-space coreutils row (D-01), routed through
                    // out_line so --clip tees each digest (D-07).
                    crate::core::output::out_line(&format!("{digest}  {label}"));
                }
                rows.push(HashRow {
                    // `label` is already an owned String (the path, or `-` for
                    // stdin); never `to_str().unwrap()` (D-4).
                    path: label,
                    algo,
                    digest,
                });
            }
            // Best-effort (A1): report the bad file on stderr, keep going.
            Err(e) => {
                eprintln!("error: {e:#}");
                had_error = true;
            }
        }
        if let Some(pb) = &progress {
            pb.inc(1);
        }
    }
    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    // Under --json the ONLY stdout write is one document with the SUCCESSFUL rows
    // (A1 partial-success — distinct from D-09's total-failure empty-stdout rule).
    if json {
        let doc = HashOutput {
            count: rows.len(),
            results: rows,
        };
        crate::core::output::emit_json(&doc)?;
    }

    // Coreutils best-effort: exit 1 if ANY file failed. The successful digests are
    // already flushed above (out_line/emit_json each end on a newline, so stdout is
    // line-flushed before this exit). flush_clip is intentionally skipped on the
    // failure path, matching the existing verify-mismatch error contract.
    if had_error {
        std::process::exit(1);
    }
    Ok(())
}

/// Emit the D-05 BLAKE3-fallback hint on stderr (caller has already checked the
/// 64-hex + no-`--algo` + not-`--json` precondition). When a real `path` is
/// available, re-open it (the streaming reader was single-pass) and compute its
/// BLAKE3: if it equals `expected` (case-insensitive) the hint is DECISIVE
/// ("re-run with `--algo blake3`"); otherwise — or when the source was piped
/// stdin (`path` is `None`), which cannot be re-read — it degrades to the STATIC
/// transitional hint. stderr-only (the `bail!` carries the real error); the
/// emphasis is gated on `is_color_on()` so the plain text is byte-identical minus
/// ANSI (D-05).
fn emit_blake3_probe_hint(expected: &str, path: Option<&str>) {
    use owo_colors::OwoColorize;

    // The literal users grep for (and the test pins) is `--algo blake3`.
    let flag = "--algo blake3";
    let flag_styled = if crate::core::output::is_color_on() {
        flag.yellow().to_string()
    } else {
        flag.to_string()
    };

    // Decisive only when we can re-open a real path AND its blake3 matches.
    let decisive = match path {
        Some(p) => match read_file_or_stdin(Some(p.to_string())) {
            Ok(reopened) => match digest_reader(Algo::Blake3, reopened.reader) {
                Ok(b3) => b3.eq_ignore_ascii_case(expected),
                Err(_) => false,
            },
            Err(_) => false,
        },
        None => false,
    };

    if decisive {
        eprintln!(
            "hint: the digest does not match as sha256, but it MATCHES this file's blake3 — re-run with `{flag_styled}`"
        );
    } else {
        eprintln!(
            "hint: the default hash algorithm is now blake3 — pass `{flag_styled}` if this is a blake3 digest"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known-answer vectors for the byte string `box` (3 bytes), computed
    // independently (coreutils + blake3 reference). Mirrors tests/hash.rs so the
    // arm dispatch is unit-tested without spawning the binary.
    const BOX_SHA256: &str = "26f8567f2569182294c3fa5b9f9cb2270b554eef628b4c149cf82a42888ff4ae";
    const BOX_SHA512: &str = "04bbbafb37d4457e27963dbf55c92613ca2ab936ec506c57cd0d4f6504ae8b448191335ad7a9521b9bf2e2af9cee9361ecaab295df0e834ec07fa03b29a4d1ef";
    const BOX_MD5: &str = "34be958a921e43d813a2075297d8e862";
    const BOX_BLAKE3: &str = "095dfefdedb7f0870e801730da35823caaa8e969078e53b6e262c66f1a5b1c1e";

    /// Each algorithm arm hashes `b"box"` to its known-answer vector — proving the
    /// enum dispatch, the RustCrypto hybrid-array hex path, and the blake3 native
    /// arm all agree with independent references.
    #[test]
    fn known_answer_per_algo() {
        assert_eq!(
            digest_reader(Algo::Sha256, &b"box"[..]).unwrap(),
            BOX_SHA256
        );
        assert_eq!(
            digest_reader(Algo::Sha512, &b"box"[..]).unwrap(),
            BOX_SHA512
        );
        assert_eq!(digest_reader(Algo::Md5, &b"box"[..]).unwrap(), BOX_MD5);
        assert_eq!(
            digest_reader(Algo::Blake3, &b"box"[..]).unwrap(),
            BOX_BLAKE3
        );
    }

    /// `algo_from_len` maps the three supported lengths and rejects everything
    /// else with the typed exit-2 variant (carrying the offending length).
    #[test]
    fn algo_from_len_maps_supported_and_rejects_others() {
        assert_eq!(algo_from_len(32).unwrap(), Algo::Md5);
        assert_eq!(algo_from_len(64).unwrap(), Algo::Sha256);
        assert_eq!(algo_from_len(128).unwrap(), Algo::Sha512);
        // 64 wins the sha256/blake3 tie — there is no length that maps to blake3.
        assert!(matches!(
            algo_from_len(40),
            Err(BoxError::UnsupportedHashLength { len: 40 })
        ));
        assert!(matches!(
            algo_from_len(0),
            Err(BoxError::UnsupportedHashLength { len: 0 })
        ));
    }

    /// Streaming across a multi-chunk payload (> READ_BUF) yields the same digest
    /// as a single shot — proving the 64 KiB read loop accumulates correctly.
    #[test]
    fn streaming_multi_chunk_matches_single_shot() {
        let big = vec![0xABu8; READ_BUF * 2 + 7];
        let mut one = Sha256::new();
        one.update(&big);
        let expected = const_hex::encode(one.finalize());
        assert_eq!(digest_reader(Algo::Sha256, &big[..]).unwrap(), expected);
    }
}
