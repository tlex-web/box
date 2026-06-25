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
use md5::Md5;
use sha2::{Digest, Sha256, Sha512};

use crate::commands::RunCommand;
use crate::core::errors::BoxError;
use crate::core::input::read_file_or_stdin;

/// Streaming read buffer for the RustCrypto incremental `update` loop. blake3
/// manages its own SIMD-sized internal buffer via `update_reader`.
const READ_BUF: usize = 64 * 1024;

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

    /// File to hash; omit (or `-`) to read from piped stdin (labelled `-`).
    pub path: Option<String>,
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
        // Capture the explicit `--algo` and the original path string BEFORE
        // `read_file_or_stdin` consumes `self.path`. `path_for_probe` is `Some`
        // only for a real on-disk path (NOT stdin/`-`), which is exactly the D-05
        // re-open precondition: the streaming `input.reader` is single-pass
        // (`Box<dyn Read>`, consumed by `digest_reader`), so the BLAKE3 probe must
        // re-open the file — and there is no second read for piped stdin.
        let cli_algo = self.algo;
        let path_for_probe = match self.path.as_deref() {
            Some(p) if p != "-" => Some(p.to_string()),
            _ => None,
        };

        // Acquire a STREAMING input source: path / `--file` (ahead of stdin) /
        // piped stdin / exit-2-on-no-arg-TTY — all inherited from core::input.
        let input = read_file_or_stdin(self.path)?;
        let label = input.label;

        match self.verify {
            // --verify: pick the algorithm — an EXPLICIT --algo ALWAYS wins; only
            // a truly-unset --algo falls back to length auto-detect (WR-01). The
            // verify length table (algo_from_len) is UNCHANGED by the v2 flip: a
            // bare 64-hex still maps to sha256, so stored SHA-256 baselines never
            // silently break (D-04, the #1 v2 data-risk backstop).
            Some(expected) => {
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
            // No --verify: COMPUTE the digest. Precedence is CLI > env > config >
            // builtin (SPINE-05): an explicit `--algo`, else `BOX_HASH_DEFAULT_ALGO`,
            // else the config `default_hash_algo`, else the v2 builtin BLAKE3 (D-04
            // — the breaking compute-default flip; v1 defaulted to SHA-256). The
            // env tier is wired live here (06-02), reusing `parse_algo` so env and
            // config share ONE spelling parser.
            None => {
                let algo = cli_algo
                    .or_else(|| {
                        std::env::var("BOX_HASH_DEFAULT_ALGO")
                            .ok()
                            .and_then(|s| parse_algo(&s))
                    })
                    .or(crate::core::config::config().default_hash_algo)
                    .unwrap_or(Algo::Blake3);
                let computed = digest_reader(algo, input.reader)?;
                // Fork on is_json_on() FIRST (Pitfall 1): the only stdout write
                // under --json is emit_json. The human path keeps the two-space
                // coreutils `<hash>  <label>` row (D-01) and routes through
                // out_line so --clip tees the digest (D-07).
                if crate::core::output::is_json_on() {
                    let doc = HashOutput {
                        count: 1,
                        results: vec![HashRow {
                            // `label` is already an owned String (the path, or `-`
                            // for stdin). Never `to_str().unwrap()` — a non-UTF-8
                            // path policy is `to_string_lossy()` (D-4), but `label`
                            // is already lossy-safe here.
                            path: label.clone(),
                            algo,
                            digest: computed,
                        }],
                    };
                    crate::core::output::emit_json(&doc)?;
                } else {
                    crate::core::output::out_line(&format!("{computed}  {label}"));
                }
                Ok(())
            }
        }
    }
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
