//! Filesystem helpers shared by every command that touches the disk.
//!
//! Three concerns live here, each a documented Windows foot-gun handled once:
//! - [`normalize_path`] — UNC-safe canonicalization via `dunce` (never
//!   `std::fs::canonicalize`, which leaks the `\\?\` verbatim prefix — FOUND-06,
//!   Pitfall 1).
//! - [`is_hidden`] — the `walkdir` `filter_entry` predicate (D-12): a non-root
//!   entry is hidden if its name starts with `.` or carries
//!   `FILE_ATTRIBUTE_HIDDEN`. The root entry (depth 0) is never hidden, so a
//!   dotted source root is not pruned to zero files (walkdir#142, Pitfall 8).
//! - [`safe_copy`] — `fs::copy` plus timestamp preservation via
//!   `std::fs::FileTimes`, with `.context(...)` on every fallible call so
//!   deep-path (>260 char) failures surface loudly per-file (FOUND-06,
//!   Pitfalls 5 & 6).

// Forward-compat surface: flatten (plan 03) is the first caller of these
// helpers. They're exercised by the inline unit tests below but not yet by any
// command, so the binary build reports them as dead code until plan 03 wires
// the first call site (mirrors plan 01-01's RunCommand allow).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::Context;
use walkdir::DirEntry;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

/// Windows `FILE_ATTRIBUTE_HIDDEN` bit.
#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

/// Canonicalize `p` without leaking a `\\?\` UNC/verbatim prefix.
///
/// Wraps `dunce::canonicalize` — **never** `std::fs::canonicalize`, which always
/// prefixes `\\?\` on Windows (rust-lang/rust#42869) and would corrupt
/// containment guards and collision-encoding (FOUND-06, Pitfall 1).
pub fn normalize_path(_p: &Path) -> std::io::Result<PathBuf> {
    unimplemented!("RED: implemented in GREEN")
}

/// `walkdir` `filter_entry` predicate: is this entry hidden and therefore prunable
/// (D-12)?
///
/// The **root** entry (`depth() == 0`) is never hidden — otherwise passing a
/// dotted directory as the source root would prune the entire walk to nothing
/// (walkdir#142, Pitfall 8). Any deeper entry is hidden when its base name starts
/// with `.` **or** (on Windows) it carries `FILE_ATTRIBUTE_HIDDEN`. Applied in
/// `filter_entry`, a hidden *directory* prunes its whole subtree cheaply.
pub fn is_hidden(_entry: &DirEntry) -> bool {
    unimplemented!("RED: implemented in GREEN")
}

/// Copy `src` to `dst`, preserving the source's modified and accessed times,
/// returning the number of bytes copied.
///
/// `fs::copy` does **not** preserve timestamps on Windows (Pitfall 6); we read
/// the source metadata and apply `std::fs::FileTimes` afterward. Every fallible
/// I/O call carries `.context(...)` so a deep-path (>260 char) `NotFound`-style
/// failure surfaces loudly per-file rather than being silently dropped (FOUND-06,
/// Pitfall 5).
pub fn safe_copy(_src: &Path, _dst: &Path) -> anyhow::Result<u64> {
    unimplemented!("RED: implemented in GREEN")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn normalize_path_strips_verbatim_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let norm = normalize_path(dir.path()).unwrap();
        let s = norm.to_string_lossy();
        assert!(
            !s.starts_with(r"\\?\"),
            "normalize_path must not leak the \\\\?\\ prefix: {s}"
        );
    }

    #[test]
    fn is_hidden_false_for_root_even_if_dotted() {
        // Root entry (depth 0) is never hidden, even when its name starts with
        // `.` (walkdir#142 gotcha).
        let dir = tempfile::tempdir().unwrap();
        let dotted = dir.path().join(".dotroot");
        std::fs::create_dir(&dotted).unwrap();
        let root = walkdir::WalkDir::new(&dotted)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(root.depth(), 0);
        assert!(!is_hidden(&root), "root must never be reported hidden");
    }

    #[test]
    fn is_hidden_true_for_nonroot_dotfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".secret"), b"x").unwrap();
        std::fs::write(dir.path().join("visible.txt"), b"y").unwrap();
        let mut saw_dotfile = false;
        let mut saw_visible = false;
        for entry in walkdir::WalkDir::new(dir.path()).into_iter().flatten() {
            if entry.depth() == 0 {
                continue;
            }
            match entry.file_name().to_str().unwrap() {
                ".secret" => {
                    saw_dotfile = true;
                    assert!(is_hidden(&entry), "non-root dotfile must be hidden");
                }
                "visible.txt" => {
                    saw_visible = true;
                    assert!(!is_hidden(&entry), "ordinary file must not be hidden");
                }
                _ => {}
            }
        }
        assert!(saw_dotfile && saw_visible);
    }

    #[cfg(windows)]
    #[test]
    fn is_hidden_true_for_nonroot_hidden_attribute() {
        use std::os::windows::process::CommandExt;
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("plainname.txt");
        std::fs::write(&f, b"z").unwrap();
        // Set FILE_ATTRIBUTE_HIDDEN via `attrib +h` (no dot in the name, so only
        // the attribute can make it hidden).
        let status = std::process::Command::new("attrib")
            .arg("+h")
            .arg(&f)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .status()
            .expect("run attrib +h");
        assert!(status.success(), "attrib +h should succeed");
        let entry = walkdir::WalkDir::new(dir.path())
            .into_iter()
            .flatten()
            .find(|e| e.file_name().to_str() == Some("plainname.txt"))
            .expect("find the hidden file");
        assert!(
            is_hidden(&entry),
            "non-root entry with FILE_ATTRIBUTE_HIDDEN must be hidden"
        );
    }

    #[test]
    fn safe_copy_copies_bytes_and_preserves_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        let payload = b"hello flatten timestamps";
        std::fs::write(&src, payload).unwrap();

        // Backdate the source mtime so an unpreserved copy would visibly differ.
        let backdated = SystemTime::now() - Duration::from_secs(60 * 60 * 24 * 365);
        std::fs::File::options()
            .write(true)
            .open(&src)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(backdated))
            .unwrap();

        let n = safe_copy(&src, &dst).unwrap();
        assert_eq!(n as usize, payload.len(), "byte count must match");
        assert_eq!(std::fs::read(&dst).unwrap(), payload, "contents must match");

        let src_m = std::fs::metadata(&src).unwrap().modified().unwrap();
        let dst_m = std::fs::metadata(&dst).unwrap().modified().unwrap();
        // Allow a 2s tolerance for filesystem timestamp resolution.
        let diff = src_m
            .duration_since(dst_m)
            .or_else(|_| dst_m.duration_since(src_m))
            .unwrap();
        assert!(
            diff < Duration::from_secs(2),
            "dst mtime must match src mtime (diff {diff:?})"
        );
    }
}
