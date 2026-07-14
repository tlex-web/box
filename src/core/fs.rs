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
//! - [`safe_copy`] — a create-new copy (refuses to overwrite an existing dst, so
//!   a missed in-memory collision fails loudly instead of silently clobbering;
//!   WR-02) plus timestamp preservation via `std::fs::FileTimes`, with
//!   `.context(...)` on every fallible call so deep-path (>260 char) failures
//!   surface loudly per-file (FOUND-06, Pitfalls 5 & 6).

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
pub fn normalize_path(p: &Path) -> std::io::Result<PathBuf> {
    dunce::canonicalize(p)
}

/// `walkdir` `filter_entry` predicate: is this entry hidden and therefore prunable
/// (D-12)?
///
/// The **root** entry (`depth() == 0`) is never hidden — otherwise passing a
/// dotted directory as the source root would prune the entire walk to nothing
/// (walkdir#142, Pitfall 8). Any deeper entry is hidden when its base name starts
/// with `.` **or** (on Windows) it carries `FILE_ATTRIBUTE_HIDDEN`. Applied in
/// `filter_entry`, a hidden *directory* prunes its whole subtree cheaply.
pub fn is_hidden(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false; // never prune the root (walkdir#142)
    }
    let dot = entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'));
    #[cfg(windows)]
    let attr = entry
        .metadata()
        .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0);
    #[cfg(not(windows))]
    let attr = false;
    dot || attr
}

/// Copy `src` to `dst`, preserving the source's modified and accessed times,
/// returning the number of bytes copied.
///
/// **Refuses to overwrite an existing `dst`** (`create_new`): if a destination is
/// already present the copy fails loudly with `AlreadyExists` rather than silently
/// clobbering it. `flatten`'s in-memory `occupied` name-set is the primary
/// collision guard, but it can drift from on-disk reality — a Windows trailing
/// dot/space or non-ASCII-case name collapse the set missed, or a file appearing in
/// the output dir between the `read_dir` seed and the copy. `create_new` is the
/// defense-in-depth backstop for those, upholding the "nothing is silently
/// overwritten" guarantee (WR-02, FLAT-02 / D-14).
///
/// `std::io::copy` (unlike `fs::copy`) does **not** preserve timestamps on Windows
/// (Pitfall 6); we read the source metadata and apply `std::fs::FileTimes` to the
/// freshly created handle afterward. Every fallible I/O call carries `.context(...)`
/// so a deep-path (>260 char) `NotFound`-style failure surfaces loudly per-file
/// rather than being silently dropped (FOUND-06, Pitfall 5).
pub fn safe_copy(src: &Path, dst: &Path) -> anyhow::Result<u64> {
    let mut reader =
        std::fs::File::open(src).with_context(|| format!("opening source {}", src.display()))?;
    // create_new: fail with AlreadyExists instead of clobbering an existing file.
    let mut writer = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(dst)
        .with_context(|| {
            format!(
                "creating destination {} (refusing to overwrite)",
                dst.display()
            )
        })?;
    let bytes = std::io::copy(&mut reader, &mut writer)
        .with_context(|| format!("copying {} -> {}", src.display(), dst.display()))?;

    let meta = std::fs::metadata(src)
        .with_context(|| format!("reading source metadata for {}", src.display()))?;
    let modified = meta
        .modified()
        .with_context(|| format!("reading modified time for {}", src.display()))?;
    let times = std::fs::FileTimes::new().set_modified(modified);
    // Accessed time is best-effort: some filesystems don't report it. Only add it
    // when available so the copy still succeeds (and mtime is still preserved)
    // where atime is missing (Assumption A3).
    let times = match meta.accessed() {
        Ok(accessed) => times.set_accessed(accessed),
        Err(_) => times,
    };
    // Set times on the handle we just wrote (opened write(true)) — no reopen needed.
    writer
        .set_times(times)
        .with_context(|| format!("setting timestamps on {}", dst.display()))?;

    Ok(bytes)
}

/// Atomically write `contents` to `path` (CFG-01 / T-11-01): the `config set` write
/// primitive.
///
/// Unlike [`safe_copy`] (which uses `create_new` to refuse an overwrite), this
/// **replaces** an existing target — the correct semantics for rewriting a config
/// file. The write is crash-safe: `contents` goes to a temp sibling (`<path>.tmp`)
/// first, then `std::fs::rename` swaps it over `path` in one atomic same-volume
/// operation (Windows `MoveFileEx`-style replace), so a crash mid-write can never
/// leave a torn/partial `config.toml` that would brick every subsequent startup.
/// The parent directory (`%APPDATA%\box\`) is created if absent. Every fallible I/O
/// call carries `.context(...)`, mirroring [`safe_copy`]'s per-call discipline.
///
/// Live as of Plan 11-01: `config set` (via [`crate::core::config::set_value`]) is
/// the live consumer.
pub fn atomic_write(path: &Path, contents: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    // Temp sibling on the SAME volume as `path` so the rename is a true atomic
    // replace (a cross-volume rename would fall back to copy+delete, losing
    // atomicity). Appending `.tmp` keeps it next to the target without colliding
    // with a real `config.toml`.
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);

    std::fs::write(&tmp, contents)
        .with_context(|| format!("writing temp file {}", tmp.display()))?;
    // rename OVERWRITES an existing target (unlike safe_copy's create_new) — exactly
    // what a config replace needs.
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
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

    #[test]
    fn safe_copy_refuses_to_overwrite_existing() {
        // WR-02: an already-present destination must error (AlreadyExists) and leave
        // the original untouched — never a silent clobber.
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        std::fs::write(&src, b"incoming").unwrap();
        std::fs::write(&dst, b"original-keep-me").unwrap();

        let err = safe_copy(&src, &dst).expect_err("safe_copy must refuse an existing destination");

        // The original destination content is preserved byte-for-byte.
        assert_eq!(
            std::fs::read(&dst).unwrap(),
            b"original-keep-me",
            "existing destination must not be overwritten"
        );
        // The error chain explains the refusal.
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("refusing to overwrite") || msg.contains("already"),
            "expected an overwrite-refusal error, got: {msg}"
        );
    }

    /// CFG-01 / T-11-01 — `atomic_write` creates a MISSING parent dir (the
    /// `%APPDATA%\box\` case), lands the exact bytes at the target after the rename,
    /// and leaves no leftover `.tmp` sibling.
    #[test]
    fn atomic_write_creates_parent_and_lands_bytes() {
        let dir = tempfile::tempdir().unwrap();
        // The `box/` parent does NOT exist yet — atomic_write must create it.
        let target = dir.path().join("box").join("config.toml");
        assert!(!target.parent().unwrap().exists());

        let contents = "[weather]\nunits = \"imperial\"\n";
        atomic_write(&target, contents).unwrap();

        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            contents,
            "atomic_write must land the exact bytes at the target"
        );
        // No leftover temp sibling after the rename.
        let mut tmp = target.as_os_str().to_os_string();
        tmp.push(".tmp");
        assert!(
            !PathBuf::from(tmp).exists(),
            "the temp sibling must be gone after the rename"
        );
    }

    /// CFG-01 — `atomic_write` REPLACES an existing target (unlike `safe_copy`'s
    /// create-new refusal) — the correct semantics for rewriting a config file.
    #[test]
    fn atomic_write_replaces_existing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("config.toml");
        std::fs::write(&target, "old = 1\n").unwrap();

        atomic_write(&target, "new = 2\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "new = 2\n",
            "atomic_write must overwrite an existing target"
        );
    }
}
