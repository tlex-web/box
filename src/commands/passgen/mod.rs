//! The `passgen` command: generate cryptographically secure passwords and
//! EFF-wordlist passphrases (PASS-01). This is the one real threat surface in
//! Phase 2, so the RNG choices are non-negotiable and code-review-gated (T-V6):
//!
//! - **CSPRNG source:** randomness comes from [`OsRng`] (getrandom → ProcessPrng
//!   on Windows) wrapped via `OsRng.unwrap_err()` into an infallible `Rng`
//!   adapter (D-08 literal reading; `rand::TryRngCore` resolves under rand 0.9,
//!   verified in plan Task 1 — no `rand_core` dep needed). NO fixed/manual seed
//!   anywhere.
//! - **Unbiased selection:** every character/word is picked with
//!   [`IndexedRandom::choose`] (uniform, `Option<&T>`). Modulo-indexed selection
//!   is explicitly banned (D-02) because it introduces modulo bias; the
//!   acceptance grep confirms no such indexing appears in the code.
//! - **Output channel:** generated secrets are written to **stdout only** and
//!   are never logged to stderr (D-14 information-disclosure mitigation).
//!
//! Charset (D-14): the default four-class set is lowercase, uppercase, digits,
//! and a *curated paste-safe* symbol subset that excludes shell/quoting-hostile
//! characters (backtick, quotes, `$`, `\`, `;`, `|`, `&`, angle brackets,
//! parens, space) so a generated password pastes cleanly into a PowerShell 7
//! prompt. `--no-symbols` drops the symbol class. Passphrase mode (`--words N`)
//! draws from the embedded EFF Large wordlist (7776 words, CC-BY 3.0 US).
//!
//! Flow (Pattern 1 — thin orchestrator over pure helpers): `run()` builds the
//! requested charset (or the wordlist), constructs the OsRng-backed `rng`, and
//! prints `--count` results, each on its own stdout line.

use std::io::IsTerminal;

use clap::Args;
use rand::rngs::OsRng;
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)
use rand::TryRngCore; // brings OsRng.unwrap_err(); resolves via rand 0.9 (plan A1)

use crate::commands::RunCommand;

/// One row of `box passgen --json` output. Field name (discretion): `password`
/// carries the generated password or passphrase line. SAME value the human path
/// prints (no-drift).
#[derive(serde::Serialize)]
struct PassgenRow {
    password: String,
}

/// The `box passgen --json` document (D-01 multi-capable → always-wrapped
/// `{results, count}`, EXACT uuid copy — passgen can return N lines via `--count`).
/// SPINE-04 (the human path tees each password to the clipboard via `out_line`).
///
/// `entropy_bits` (PASS-V2-01) is a per-CONFIG estimate (not per-row): the
/// theoretical pool-based entropy of the chosen length/charset (or word count),
/// the SAME number the human path prints to STDERR. It is a top-level field — the
/// secret stays the sole stdout content; the entropy is metadata about the run.
#[derive(serde::Serialize)]
struct PassgenOutput {
    results: Vec<PassgenRow>,
    count: usize,
    entropy_bits: f64,
}

/// The embedded EFF Large (Diceware) wordlist — 7776 words, one per line
/// (dice codes already stripped). CC-BY 3.0 US, © Electronic Frontier
/// Foundation (see LICENSE-THIRD-PARTY.md). `include_str!` is relative to THIS
/// source file: `src/commands/passgen/mod.rs` → `src/data/...`.
const EFF_WORDLIST_RAW: &str = include_str!("../../data/eff_large_wordlist.txt");

/// Curated paste-safe symbol set (D-14). Deliberately EXCLUDES shell/quoting
/// hostile characters so a password pastes cleanly into a PowerShell 7 prompt:
/// no backtick, single quote, double quote, dollar, backslash, semicolon, pipe,
/// ampersand, angle brackets, parens, or space. Each of these is safe inside an
/// unquoted or single-quoted PS7 token.
const SYMBOLS: &[char] = &[
    '!', '#', '%', '*', '+', '-', '=', '?', '@', '^', '_', '~', '.', ':', '/',
];

/// Lowercase ASCII letters a-z.
const LOWER: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Uppercase ASCII letters A-Z.
const UPPER: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// ASCII digits 0-9.
const DIGITS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// `box passgen` — generate secure passwords and passphrases (PASS-01).
///
/// By default prints one 16-character password drawn from a four-class curated
/// set (lower + upper + digits + paste-safe symbols) using the OS CSPRNG. The
/// EFF wordlist attribution is appended to this command's `--help` footer
/// (`after_help`) so the CC-BY 3.0 US notice travels with the binary (D-09).
#[derive(Debug, Args)]
#[command(
    after_help = "Passphrase wordlist: EFF Long Wordlist, \u{00A9} Electronic Frontier Foundation, CC-BY 3.0 US."
)]
pub struct PassgenArgs {
    /// Password length in characters (ignored in passphrase mode).
    #[arg(long, default_value_t = 16)]
    pub length: usize,

    /// How many passwords/passphrases to generate, one per line.
    #[arg(long, default_value_t = 1)]
    pub count: usize,

    /// Generate a passphrase of N words from the EFF wordlist instead of a
    /// character password.
    #[arg(long)]
    pub words: Option<usize>,

    /// Exclude symbol characters from the password charset.
    #[arg(long = "no-symbols")]
    pub no_symbols: bool,

    /// Exclude visually-similar characters (i l 1 L o 0 O) from the charset.
    #[arg(long = "no-similar")]
    pub no_similar: bool,

    /// Word separator for passphrase mode (default `.`).
    #[arg(long, default_value = ".")]
    pub separator: String,
}

impl RunCommand for PassgenArgs {
    fn run(self) -> anyhow::Result<()> {
        // CSPRNG: wrap the fallible OS RNG into an infallible Rng-implementing
        // adapter (D-08 literal). Every draw below hits this OS-backed source;
        // there is no fixed/manual seed anywhere (T-V6).
        let mut rng = OsRng.unwrap_err();

        // Build the rows ONCE (mirroring uuid's `.map().collect()`) so the SAME
        // values feed the human and JSON paths (no-drift). Each generated line
        // becomes one `PassgenRow`. `bits` is the per-config entropy estimate the
        // human path prints to STDERR / the JSON path carries as `entropy_bits`.
        let (rows, bits): (Vec<PassgenRow>, f64) = if let Some(n) = self.words {
            // Passphrase mode: draw `n` words per line, unbiased via `choose`.
            let wordlist = eff_wordlist();
            let rows = (0..self.count)
                .map(|_| {
                    let phrase: Vec<&str> = (0..n)
                        .map(|_| {
                            *wordlist
                                .choose(&mut rng)
                                .expect("EFF wordlist is non-empty")
                        })
                        .collect();
                    // `--separator` overrides the default `.` join (PASS-V2-01). A
                    // dot is paste-safe in PS7 and — unlike a hyphen — never appears
                    // inside an EFF word (some are hyphenated, e.g. `t-shirt`), so
                    // the default phrase stays one unambiguous token.
                    PassgenRow {
                        password: phrase.join(&self.separator),
                    }
                })
                .collect();
            // Passphrase entropy: each word contributes log2(7776) ≈ 12.925 bits.
            (rows, entropy_bits(n, wordlist.len()))
        } else {
            // Character mode: build the requested charset, sample every char
            // unbiased via `choose` — never by modulo-indexing the charset (D-02).
            // `--no-similar` prunes the look-alikes AND shrinks the entropy pool.
            let mut charset = build_charset(self.no_symbols);
            if self.no_similar {
                drop_similar(&mut charset);
            }
            let pool_len = charset.len();
            let rows = (0..self.count)
                .map(|_| {
                    let pw: String = (0..self.length)
                        .map(|_| *charset.choose(&mut rng).expect("charset is non-empty"))
                        .collect();
                    PassgenRow { password: pw }
                })
                .collect();
            (rows, entropy_bits(self.length, pool_len))
        };

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the
        // always-wrapped `{results, count, entropy_bits}` document; otherwise print
        // each password via `out_line` (NOT println!) so `--clip` tees every line
        // (SPINE-04). The entropy NEVER touches stdout (D-14): under --json it is a
        // structured field; on the human path it goes to STDERR only.
        if crate::core::output::is_json_on() {
            let doc = PassgenOutput {
                count: rows.len(),
                results: rows,
                entropy_bits: bits,
            };
            crate::core::output::emit_json(&doc)?;
        } else {
            for r in &rows {
                crate::core::output::out_line(&r.password);
            }
            // Entropy summary to STDERR ONLY, TTY-gated exactly like the clip
            // confirmation (so `box passgen 2>log` does not log it and the secret
            // stays the sole stdout content — D-14 / Pitfall 5). Never reached under
            // --json (handled above).
            if std::io::stderr().is_terminal() {
                eprintln!("entropy: ~{bits:.1} bits");
            }
        }
        Ok(())
    }
}

/// Build the password character set. The default is the four-class curated set
/// (lower + upper + digits + [`SYMBOLS`]); `no_symbols` drops the symbol class.
fn build_charset(no_symbols: bool) -> Vec<char> {
    let mut set: Vec<char> = Vec::new();
    set.extend_from_slice(LOWER);
    set.extend_from_slice(UPPER);
    set.extend_from_slice(DIGITS);
    if !no_symbols {
        set.extend_from_slice(SYMBOLS);
    }
    set
}

/// The visually-ambiguous characters `--no-similar` removes (lowercase i / L,
/// digit one, uppercase L, lowercase o, digit zero, uppercase O).
const SIMILAR: &str = "il1Lo0O";

/// Remove every [`SIMILAR`] look-alike from a charset in place (`--no-similar`).
/// Pure so the prune (and its effect on the pool size) is unit-testable. RNG
/// untouched (T-V6) — only the charset shrinks, never the draw.
fn drop_similar(set: &mut Vec<char>) {
    set.retain(|c| !SIMILAR.contains(*c));
}

/// Theoretical entropy in bits for drawing `len` independent symbols uniformly
/// from a pool of `pool` symbols: `len * log2(pool)`. Used for both char mode
/// (`length * log2(pool_size)`) and passphrase mode (`words * log2(7776)`). Pure
/// so the formula is unit-testable; this is a DISPLAY estimate, not a security
/// control (the CSPRNG draw is the actual guarantee — T-V6).
fn entropy_bits(len: usize, pool: usize) -> f64 {
    if pool <= 1 {
        return 0.0;
    }
    len as f64 * (pool as f64).log2()
}

/// Parse the embedded EFF wordlist into a `Vec<&'static str>`, one clean word
/// per entry. The asset is stored words-only with the dice codes already
/// stripped, but we defensively `.trim()` each line (tolerating any stray
/// whitespace / `\r` from a CRLF checkout) and drop empties, so the parser still
/// yields exactly 7776 clean words with no digit or tab. `split_whitespace().last()`
/// would also strip a dice-code prefix were the asset ever stored verbatim.
fn eff_wordlist() -> Vec<&'static str> {
    EFF_WORDLIST_RAW
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE authoritative 7776 gate (replaces the cross-platform Task-1 shell
    /// line-count): the parsed EFF wordlist has exactly 7776 entries.
    #[test]
    fn eff_wordlist_has_exactly_7776_entries() {
        assert_eq!(
            eff_wordlist().len(),
            7776,
            "EFF Large wordlist must parse to exactly 7776 words"
        );
    }

    /// No parsed EFF entry contains a digit or a tab (dice codes are fully
    /// stripped; the word field is clean).
    #[test]
    fn eff_words_have_no_digit_or_tab() {
        for w in eff_wordlist() {
            assert!(
                !w.chars().any(|c| c.is_ascii_digit() || c == '\t'),
                "EFF word {w:?} contains a digit or tab — dice code not stripped"
            );
            assert!(!w.is_empty(), "EFF word must not be empty");
        }
    }

    /// The default charset is exactly the four-class curated set and every
    /// member is lower/upper/digit/curated-symbol.
    #[test]
    fn default_charset_is_four_class_curated() {
        let set = build_charset(false);
        // 26 + 26 + 10 + SYMBOLS.len()
        assert_eq!(set.len(), 26 + 26 + 10 + SYMBOLS.len());
        for c in &set {
            let ok = c.is_ascii_lowercase()
                || c.is_ascii_uppercase()
                || c.is_ascii_digit()
                || SYMBOLS.contains(c);
            assert!(ok, "charset member {c:?} not in the four-class set");
        }
    }

    /// `--no-symbols` drops the symbol class: the charset is alphanumeric only.
    #[test]
    fn no_symbols_charset_is_alphanumeric_only() {
        let set = build_charset(true);
        assert_eq!(set.len(), 26 + 26 + 10);
        for c in &set {
            assert!(c.is_ascii_alphanumeric(), "no-symbols charset leaked {c:?}");
        }
    }

    /// `entropy_bits` matches `len * log2(pool)`, and `--no-similar` lowers it by
    /// shrinking the pool (PASS-V2-01). The default four-class set (77 chars) loses
    /// all 7 look-alikes (`il1Lo0O` are all alphanumeric) → a 70-char pool, so the
    /// entropy strictly drops.
    #[test]
    fn entropy_bits_matches_formula_and_drops_with_no_similar() {
        // Char mode: 16 chars over the full 77-symbol pool.
        let full = build_charset(false);
        assert_eq!(full.len(), 26 + 26 + 10 + SYMBOLS.len());
        let full_bits = entropy_bits(16, full.len());
        assert!(
            (full_bits - 16.0 * (full.len() as f64).log2()).abs() < 1e-9,
            "char-mode entropy must equal length * log2(pool)"
        );

        // `--no-similar` removes all 7 of `il1Lo0O` (every one is alphanumeric).
        let mut pruned = build_charset(false);
        drop_similar(&mut pruned);
        assert_eq!(
            pruned.len(),
            full.len() - SIMILAR.chars().count(),
            "no-similar must drop exactly the 7 look-alikes from the default set"
        );
        for c in &pruned {
            assert!(!SIMILAR.contains(*c), "look-alike {c:?} survived the prune");
        }
        let pruned_bits = entropy_bits(16, pruned.len());
        assert!(
            pruned_bits < full_bits,
            "no-similar must lower entropy: {pruned_bits} !< {full_bits}"
        );

        // Passphrase mode: 4 words over the 7776-word EFF pool ≈ 51.7 bits
        // (4 * log2(7776) = 4 * 12.925…).
        let phrase_bits = entropy_bits(4, 7776);
        assert!(
            (phrase_bits - 4.0 * 7776f64.log2()).abs() < 1e-9,
            "passphrase entropy must equal words * log2(7776)"
        );
        assert!(
            (phrase_bits - 51.699).abs() < 0.01,
            "4 EFF words ≈ 51.7 bits, got {phrase_bits}"
        );
    }

    /// The curated SYMBOLS const contains NONE of the shell/quoting-hostile
    /// characters (D-14, paste-safe in PowerShell 7).
    #[test]
    fn symbols_exclude_shell_hostile_chars() {
        let banned = [
            '`', '\'', '"', '$', '\\', ';', '|', '&', '<', '>', '(', ')', ' ',
        ];
        for b in banned {
            assert!(
                !SYMBOLS.contains(&b),
                "SYMBOLS must not contain the shell-hostile char {b:?}"
            );
        }
    }
}
