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
/// (D-15 / FLAT-V2-01): split the path into segments on BOTH real separators,
/// drop empty / `..` / `.` segments, join the survivors with `sep`, then run the
/// result through [`sanitize_reserved`]. `sep` is the collision-encoding join
/// character (`_` by default; overridable via `flatten --separator`).
///
/// Splitting on the real path separators — never on `sep` — keeps this correct
/// for a multi-character or unusual separator, and means a segment that itself
/// contains the old default `_` is no longer split (a behavior superset of the
/// v1 form, which round-tripped to the same result for `_`).
///
/// The output is guaranteed to contain no `\`, no `/`, and no `..` component, so
/// a maliciously deep or `..`-laden source path can never produce a name that
/// escapes the output directory (T-03-pathinject). The caller additionally rejects
/// a `sep` containing `/`/`\` before reaching here (T-8-01).
pub fn encode_relative(rel: &Path, sep: &str) -> String {
    // A `..` or `.` parent/current-dir token must never survive (T-03-pathinject):
    // splitting on the real separators isolates each segment so the filter can drop
    // every `..`/`.`/empty one before the surviving segments are joined with `sep`.
    let lossy = rel.to_string_lossy();
    let cleaned: Vec<&str> = lossy
        .split(['\\', '/'])
        .filter(|seg| !seg.is_empty() && *seg != ".." && *seg != ".")
        .collect();
    let joined = cleaned.join(sep);
    sanitize_reserved(&joined)
}

/// Make `name` safe to write on Windows: if its stem (case-insensitive, with or
/// without an extension) is a reserved device name, append `_` to the stem; and
/// trim trailing dots/spaces from the stem (Windows silently trims these, which
/// would create hidden collisions). Pitfall 7 / T-03-reserved.
pub fn sanitize_reserved(name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s, Some(e)),
        None => (name, None),
    };
    // Windows silently trims trailing dots/spaces from the stem (creating hidden
    // collisions), so trim them ourselves FIRST, then test the trimmed stem for a
    // reserved match — `"con "` must be recognised as the reserved `CON`.
    let mut stem = stem.trim_end_matches(['.', ' ']).to_string();
    let is_reserved = RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&stem));
    if is_reserved {
        stem.push('_');
    }
    let rebuilt = match ext {
        Some(e) => format!("{stem}.{e}"),
        None => stem,
    };
    // Windows also strips trailing dots/spaces from the WHOLE name on write — not
    // just the stem. A trailing dot/space AFTER the final `.` (`report.txt `), or a
    // no-extension trailing dot (`report.`), survives the stem-only trim above, so
    // two distinct encoded names (`report` and `report.`) would key differently yet
    // land on the SAME physical file -> silent overwrite (CR-01). Trim the rebuilt
    // name so the occupied-set key matches the real on-disk name.
    let trimmed = rebuilt.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        // The name was nothing but dots/spaces (e.g. `...`): give it a stable,
        // writable name so it can never collapse to ""/"."/".." as a copy target.
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Whether `name`'s Windows-effective component is a reserved DEVICE name
/// (`CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9`, case-insensitive, with or without an
/// extension, ignoring the trailing dots/spaces Windows trims). Shares the
/// [`RESERVED`] list with [`sanitize_reserved`] so the two destructive commands
/// agree on one device-name model (WR-04 / CR-01).
///
/// `flatten` *rewrites* a reserved name through [`sanitize_reserved`] (`CON` ->
/// `CON_`); `bulk-rename` instead *refuses* such a target outright (abort), so it
/// needs the predicate, not the rewrite.
pub fn is_reserved_device_name(name: &str) -> bool {
    let stem = match name.rsplit_once('.') {
        Some((s, _)) => s,
        None => name,
    };
    let stem = stem.trim_end_matches(['.', ' ']);
    RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem))
}

/// Append `_1`, `_2`, … before the extension until the (case-folded) name is free
/// in `occupied`. NTFS is case-insensitive over the FULL Unicode case table, so
/// keying is done on `to_lowercase()` (not `to_ascii_lowercase()`) to catch both
/// `README.TXT` vs `readme.txt` AND non-ASCII pairs like `RÉSUMÉ.txt` vs
/// `résumé.txt` (T-03-overwrite, WR-01). NTFS uses an OS-version-specific uppercase
/// table, so `to_lowercase()` is a close-but-imperfect superset of the ASCII-only
/// check — strictly safer, and it removes the common silent-overwrite. Callers MUST
/// key `occupied` the same way (also `to_lowercase()`).
///
/// Returns `name` unchanged when it does not collide.
///
/// The numeric disambiguation suffix is always `_{n}` (e.g. `readme_1.txt`) and is
/// deliberately NOT affected by `flatten --separator` (FLAT-V2-01): the separator
/// controls the path-segment join in [`encode_relative`], whereas this suffix is a
/// within-output uniqueness counter — keeping it stable avoids a separator like
/// `-` producing ambiguous `name-1` vs a real `name-1` source file.
pub fn dedupe(name: &str, occupied: &HashSet<String>) -> String {
    let key = name.to_lowercase();
    if !occupied.contains(&key) {
        return name.to_string();
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    for n in 1.. {
        let cand = format!("{stem}_{n}{ext}");
        if !occupied.contains(&cand.to_lowercase()) {
            return cand;
        }
    }
    unreachable!("the numeric suffix space is unbounded")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn occupied(names: &[&str]) -> HashSet<String> {
        // Fold the same way real callers seed `occupied` (full Unicode, WR-01).
        names.iter().map(|s| s.to_lowercase()).collect()
    }

    #[test]
    fn encode_relative_replaces_separators() {
        assert_eq!(
            encode_relative(Path::new("docs/sub/report.txt"), "_"),
            "docs_sub_report.txt"
        );
        assert_eq!(
            encode_relative(Path::new("docs\\sub\\report.txt"), "_"),
            "docs_sub_report.txt"
        );
    }

    /// FLAT-V2-01 — `encode_relative` joins path segments with the supplied
    /// separator (default `_`), so `flatten --separator -` yields `a-b-c.txt`.
    /// Both real separators (`/` and `\`) collapse to the chosen join char, and a
    /// `..`/`.` traversal token is still dropped regardless of the separator.
    #[test]
    fn encode_relative_honors_separator() {
        assert_eq!(
            encode_relative(Path::new("docs/sub/report.txt"), "-"),
            "docs-sub-report.txt"
        );
        assert_eq!(
            encode_relative(Path::new("docs\\sub\\report.txt"), "-"),
            "docs-sub-report.txt"
        );
        // The default `_` still works (parity with the v1 hardcoded join).
        assert_eq!(encode_relative(Path::new("a/b/c.txt"), "_"), "a_b_c.txt");
        // A `..` segment is dropped before the join, so no traversal token survives
        // even with a custom separator.
        assert_eq!(
            encode_relative(Path::new("..\\escape.txt"), "-"),
            "escape.txt"
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
            let encoded = encode_relative(Path::new(input), "_");
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
    fn sanitize_trims_whole_name_trailing_junk() {
        // CR-01: a trailing dot/space AFTER the final `.` (or a no-extension
        // trailing dot) must be trimmed from the WHOLE name. Otherwise `report`
        // and `report.` produce different occupied keys yet collapse to the same
        // file on Windows -> silent overwrite. Each of these must fold to the bare
        // on-disk name Windows would actually create.
        assert_eq!(sanitize_reserved("report."), "report");
        assert_eq!(sanitize_reserved("report. "), "report");
        assert_eq!(sanitize_reserved("report.txt."), "report.txt");
        assert_eq!(sanitize_reserved("report.txt "), "report.txt");
        assert_eq!(sanitize_reserved("data."), "data");
        // Reserved stem + trailing dot: suffixed AND trimmed, no dangling dot.
        assert_eq!(sanitize_reserved("CON."), "CON_");
        // A name that is nothing but dots/spaces collapses to a stable placeholder —
        // never "", ".", or ".." (a degenerate copy target; IN-01).
        assert_eq!(sanitize_reserved("..."), "_");
        assert_eq!(sanitize_reserved(".  "), "_");
        assert_eq!(sanitize_reserved(""), "_");
    }

    /// CR-01 end-to-end at the pure-function layer: `report` and `report.` must
    /// land on the SAME occupied key so the second is deduped, not silently lost.
    #[test]
    fn trailing_dot_collides_with_bare_name() {
        let a = sanitize_reserved("report");
        let b = sanitize_reserved("report.");
        assert_eq!(a, "report");
        assert_eq!(b, "report");
        // First claims the name; the second must be forced to dedupe.
        let occ = occupied(&[&a]);
        assert_ne!(
            dedupe(&b, &occ),
            b,
            "report. must dedupe against an existing report, not reuse the name"
        );
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
    fn dedupe_is_case_insensitive_unicode() {
        // WR-01: NTFS folds the FULL Unicode case table, not just ASCII. `RÉSUMÉ.txt`
        // already occupies the slot for `résumé.txt`, so the latter must dedupe — an
        // ASCII-only fold would have missed this and silently overwritten the file.
        let occ = occupied(&["RÉSUMÉ.txt"]);
        assert_eq!(dedupe("résumé.txt", &occ), "résumé_1.txt");
    }

    #[test]
    fn dedupe_returns_unchanged_when_free() {
        let occ = occupied(&["other.txt"]);
        assert_eq!(dedupe("readme.txt", &occ), "readme.txt");
    }

    /// WR-04 — `is_reserved_device_name` recognizes every reserved device stem
    /// (case-insensitive, with/without an extension, ignoring trailing dots/spaces
    /// Windows trims) and leaves ordinary names alone. `bulk-rename` shares this
    /// predicate to REFUSE such a target instead of rewriting it.
    #[test]
    fn is_reserved_device_name_matches_reserved_set() {
        for bad in [
            "CON", "con", "NUL", "nul.txt", "PRN.log", "aux", "COM1", "com9.dat", "LPT3",
            "lpt9.bin", "Con ", "NUL.",
        ] {
            assert!(is_reserved_device_name(bad), "{bad:?} must be reserved");
        }
        for ok in [
            "report.txt",
            "console.txt",
            "contact",
            "com10",
            "lpt0",
            "con_",
            "scon",
        ] {
            assert!(!is_reserved_device_name(ok), "{ok:?} must NOT be reserved");
        }
    }
}
