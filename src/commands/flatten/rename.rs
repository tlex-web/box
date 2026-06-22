//! Pure, unit-tested collision-rename encoding for `flatten` (D-15, FLAT-02).
//!
//! Three pure functions, no I/O, so the dangerous Windows edge cases (reserved
//! device names, path-separator injection, NTFS case-insensitive collisions) are
//! testable in isolation:
//!
//! - [`encode_relative`] — turn a source-relative path into a flat, separator-free
//!   filename (`docs\sub\report.txt` -> `docs_sub_report.txt`), then sanitize it.
//!   No output of this function ever contains `\`, `/`, or a `..` component, so an
//!   encoded name can never escape the output directory (T-03-pathinject).
//! - [`sanitize_reserved`] — neutralize Windows reserved device stems
//!   (`CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9`, case-insensitive, with or without
//!   an extension) and trailing dots/spaces, so such a source file is renamed, not
//!   silently lost to a device (T-03-reserved, Pitfall 7).
//! - [`dedupe`] — append `_1`, `_2`, … before the extension until the name is free
//!   in the occupied set, keying case-insensitively to match NTFS (T-03-overwrite).

use std::collections::HashSet;
use std::path::Path;

/// Windows reserved device names (case-insensitive), which cannot be used as a
/// file name even with an extension (`CON.txt` still targets the console device).
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Encode a path **relative to the canonical source root** into a flat filename
/// (D-15): replace every `\` and `/` separator with `_`, drop any leading
/// separator artifact, then run the result through [`sanitize_reserved`].
///
/// The output is guaranteed to contain no `\`, no `/`, and no `..` component, so
/// a maliciously deep or `..`-laden source path can never produce a name that
/// escapes the output directory (T-03-pathinject).
pub fn encode_relative(_rel: &Path) -> String {
    unimplemented!("RED: encode_relative not yet implemented")
}

/// Make `name` safe to write on Windows: if its stem (case-insensitive, with or
/// without an extension) is a reserved device name, append `_` to the stem; and
/// trim trailing dots/spaces from the stem (Windows silently trims these, which
/// would create hidden collisions). Pitfall 7 / T-03-reserved.
pub fn sanitize_reserved(_name: &str) -> String {
    unimplemented!("RED: sanitize_reserved not yet implemented")
}

/// Append `_1`, `_2`, … before the extension until the (lowercased) name is free
/// in `occupied`. NTFS is case-insensitive, so keying is always done on
/// `to_ascii_lowercase()` to catch `README.TXT` vs `readme.txt` (T-03-overwrite).
///
/// Returns `name` unchanged when it does not collide.
pub fn dedupe(_name: &str, _occupied: &HashSet<String>) -> String {
    unimplemented!("RED: dedupe not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn occupied(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_ascii_lowercase()).collect()
    }

    #[test]
    fn encode_relative_replaces_separators() {
        assert_eq!(
            encode_relative(Path::new("docs/sub/report.txt")),
            "docs_sub_report.txt"
        );
        assert_eq!(
            encode_relative(Path::new("docs\\sub\\report.txt")),
            "docs_sub_report.txt"
        );
    }

    /// Property: no encoded name — for ANY input, including nested paths and a
    /// `..` segment — contains a path separator or a `..` traversal component
    /// (T-03-pathinject). The encoded name can never escape the output dir.
    #[test]
    fn encode_no_separator() {
        let inputs = [
            "docs/sub/report.txt",
            "docs\\sub\\report.txt",
            "../escape.txt",
            "a/../../b/c.txt",
            "..\\..\\windows\\system32\\evil.dll",
            "/leading/sep.txt",
            "\\leading\\sep.txt",
            "just_a_file.txt",
        ];
        for input in inputs {
            let encoded = encode_relative(Path::new(input));
            assert!(
                !encoded.contains('\\'),
                "encoded {input:?} -> {encoded:?} must not contain a backslash"
            );
            assert!(
                !encoded.contains('/'),
                "encoded {input:?} -> {encoded:?} must not contain a forward slash"
            );
            // No path component is exactly ".." (the separators are gone, but
            // assert on components too for defense in depth).
            assert!(
                !Path::new(&encoded)
                    .components()
                    .any(|c| c.as_os_str() == ".."),
                "encoded {input:?} -> {encoded:?} must not contain a `..` component"
            );
            // And the literal ".." cannot survive as a traversal token because all
            // separators became `_`; the only `.` left is the extension dot.
            assert!(
                !encoded.contains(".."),
                "encoded {input:?} -> {encoded:?} must not contain `..`"
            );
        }
    }

    #[test]
    fn sanitize_reserved_covers_every_class() {
        // CON/PRN/AUX/NUL with and without extension, mixed case.
        assert_eq!(sanitize_reserved("CON.txt"), "CON_.txt");
        assert_eq!(sanitize_reserved("con"), "con_");
        assert_eq!(sanitize_reserved("PRN.log"), "PRN_.log");
        assert_eq!(sanitize_reserved("aux"), "aux_");
        assert_eq!(sanitize_reserved("nul.dat"), "nul_.dat");
        assert_eq!(sanitize_reserved("Nul"), "Nul_");

        // COM1..COM9 and LPT1..LPT9, with and without an extension, mixed case.
        for n in 1..=9 {
            let com = format!("com{n}");
            assert_eq!(sanitize_reserved(&com), format!("com{n}_"));
            let com_ext = format!("COM{n}.txt");
            assert_eq!(sanitize_reserved(&com_ext), format!("COM{n}_.txt"));

            let lpt = format!("LPT{n}");
            assert_eq!(sanitize_reserved(&lpt), format!("LPT{n}_"));
            let lpt_ext = format!("lpt{n}.dat");
            assert_eq!(sanitize_reserved(&lpt_ext), format!("lpt{n}_.dat"));
        }
    }

    #[test]
    fn sanitize_leaves_non_reserved_alone() {
        assert_eq!(sanitize_reserved("report.txt"), "report.txt");
        assert_eq!(sanitize_reserved("console.txt"), "console.txt");
        // COM10 / LPT0 are NOT reserved.
        assert_eq!(sanitize_reserved("com10.txt"), "com10.txt");
        assert_eq!(sanitize_reserved("lpt0"), "lpt0");
    }

    #[test]
    fn sanitize_trims_trailing_dots_and_spaces() {
        // Windows silently trims trailing dots/spaces from the stem, which would
        // create hidden collisions; we trim them ourselves.
        assert_eq!(sanitize_reserved("report .txt"), "report.txt");
        assert_eq!(sanitize_reserved("report..txt"), "report.txt");
        // A reserved stem with trailing junk is both trimmed and suffixed.
        assert_eq!(sanitize_reserved("con .txt"), "con_.txt");
    }

    #[test]
    fn dedupe_numeric_fallback() {
        let occ = occupied(&["readme.txt"]);
        assert_eq!(dedupe("readme.txt", &occ), "readme_1.txt");

        let occ = occupied(&["readme.txt", "readme_1.txt", "readme_2.txt"]);
        assert_eq!(dedupe("readme.txt", &occ), "readme_3.txt");

        // No extension.
        let occ = occupied(&["readme"]);
        assert_eq!(dedupe("readme", &occ), "readme_1");
    }

    #[test]
    fn dedupe_is_case_insensitive() {
        // NTFS: README.TXT already occupies the slot for readme.txt.
        let occ = occupied(&["README.TXT"]);
        assert_eq!(dedupe("readme.txt", &occ), "readme_1.txt");
    }

    #[test]
    fn dedupe_returns_unchanged_when_free() {
        let occ = occupied(&["other.txt"]);
        assert_eq!(dedupe("readme.txt", &occ), "readme.txt");
    }
}
