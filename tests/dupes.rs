//! Wave-0 integration tests for `box dupes` (DUPE-01).
//!
//! Black-box: each test builds a real directory tree in a temp dir, runs the
//! compiled `box dupes` binary, and asserts on the content-duplicate grouping,
//! the size-pre-filter-then-content-hash identity, the wasted-space summary, the
//! deterministic group ordering, and the strictly read-only (never-writes)
//! invariant. The four named tests map 1:1 to the VALIDATION DUPE-01 rows. Every
//! test forces `NO_COLOR=1` so ANSI never leaks into the assertions (RESEARCH
//! determinism rule).
//!
//! Identity model under test (D-13):
//! - Files are first bucketed by `metadata().len()`; only same-size buckets of
//!   `>= 2` files are content-hashed (BLAKE3). Unique-size files are NEVER hashed
//!   and NEVER appear in a group.
//! - Two files of the SAME byte size but DIFFERENT content must NOT be grouped —
//!   that proves the size pre-filter is followed by a REAL content hash, not a
//!   size-only heuristic.
//! - Wasted space = sum over each duplicate group of `(group_len - 1) * file_size`
//!   (the bytes occupied by the redundant copies).
//! - `dupes` is strictly READ-ONLY: the fixture's file set + contents + mtimes are
//!   byte-for-byte unchanged after a run (T-03-13, locked Out of Scope).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use assert_cmd::Command;

/// Run `box dupes <path> [extra args]` with plain output forced and return the
/// assert handle. Mirrors `tests/du.rs` / `tests/flatten.rs` runner shape.
fn dupes(path: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("dupes").arg(path);
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Snapshot every regular file under `dir` (recursive) as
/// `relative_path -> (bytes, mtime)`. Used by the read-only invariant to assert
/// the tree is byte-for-byte unchanged after a `dupes` run.
fn snapshot_tree(dir: &Path) -> BTreeMap<String, (Vec<u8>, SystemTime)> {
    let mut map = BTreeMap::new();
    snapshot_into(dir, dir, &mut map);
    map
}

fn snapshot_into(root: &Path, dir: &Path, map: &mut BTreeMap<String, (Vec<u8>, SystemTime)>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let ft = entry.file_type().unwrap();
        if ft.is_dir() {
            snapshot_into(root, &path, map);
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let bytes = fs::read(&path).unwrap();
            let mtime = fs::metadata(&path).unwrap().modified().unwrap();
            map.insert(rel, (bytes, mtime));
        }
    }
}

/// DUPE-01 / `dupes_groups_identical` — two files with IDENTICAL content are
/// grouped; a file with UNIQUE content is never reported.
#[test]
fn dupes_groups_identical() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Two byte-identical files (must be grouped) ...
    let dup_payload = b"DUPLICATE CONTENT PAYLOAD - identical bytes in both files\n";
    fs::write(root.join("copy_one.txt"), dup_payload).unwrap();
    fs::write(root.join("copy_two.txt"), dup_payload).unwrap();
    // ... and a unique file (must never be grouped).
    fs::write(
        root.join("unique.txt"),
        b"a one-of-a-kind payload, no twin\n",
    )
    .unwrap();

    let out = dupes(root, &[]).success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();

    // Both members of the duplicate group are named in the output.
    assert!(
        stdout.contains("copy_one.txt"),
        "duplicate member copy_one.txt must be listed:\n{stdout}"
    );
    assert!(
        stdout.contains("copy_two.txt"),
        "duplicate member copy_two.txt must be listed:\n{stdout}"
    );
    // The unique file is never part of any group.
    assert!(
        !stdout.contains("unique.txt"),
        "unique.txt must NOT appear in any duplicate group:\n{stdout}"
    );
}

/// DUPE-01 / `dupes_size_then_hash` — two files of the SAME byte size but
/// DIFFERENT content must NOT be grouped (proves the size pre-filter is followed
/// by a real content hash, not size-only).
#[test]
fn dupes_size_then_hash() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Same length (32 bytes each) but differing content — same size bucket, but
    // distinct BLAKE3 hashes, so they must split and NOT be grouped.
    let a = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"; // 32 'A'
    let b = b"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"; // 32 'B'
    assert_eq!(a.len(), b.len(), "fixture must be same-size");
    fs::write(root.join("same_size_x.txt"), a).unwrap();
    fs::write(root.join("same_size_y.txt"), b).unwrap();

    let out = dupes(root, &[]).success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();

    // Neither same-size-but-different file may appear — they are not duplicates.
    assert!(
        !stdout.contains("same_size_x.txt") && !stdout.contains("same_size_y.txt"),
        "same-size/different-content files must NOT be grouped (content hash splits them):\n{stdout}"
    );
}

/// DUPE-01 / `dupes_wasted_space_sorted` — the wasted-space figure equals the sum
/// over groups of `(group_len - 1) * file_size`, and the group output is
/// deterministically ordered (distinct contents so the `(hash, path)` sort is a
/// total order, never the arbitrary rayon order).
#[test]
fn dupes_wasted_space_sorted() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Group 1: three identical 1024-byte files -> wasted = (3-1) * 1024 = 2048.
    let g1 = vec![b'1'; 1024];
    fs::write(root.join("g1_a.bin"), &g1).unwrap();
    fs::write(root.join("g1_b.bin"), &g1).unwrap();
    fs::write(root.join("g1_c.bin"), &g1).unwrap();
    // Group 2: two identical 2048-byte files -> wasted = (2-1) * 2048 = 2048.
    let g2 = vec![b'2'; 2048];
    fs::write(root.join("g2_a.bin"), &g2).unwrap();
    fs::write(root.join("g2_b.bin"), &g2).unwrap();
    // A unique file (never grouped, contributes 0 wasted).
    fs::write(root.join("solo.bin"), vec![b'0'; 777]).unwrap();

    // Total wasted = 2048 + 2048 = 4096 bytes = "4.0 KB" (human_size, 1024-based).
    let out = dupes(root, &[]).success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();

    assert!(
        stdout.contains("4.0 KB"),
        "wasted-space summary must report 4.0 KB ((3-1)*1KB + (2-1)*2KB):\n{stdout}"
    );

    // Determinism: a second run produces byte-identical stdout (the (hash, path)
    // sort is total over distinct-content groups, so rayon order can't flap).
    let out2 = dupes(root, &[]).success().get_output().stdout.clone();
    assert_eq!(
        stdout.as_bytes(),
        out2.as_slice(),
        "dupes output must be deterministic across runs (sorted before printing)"
    );

    // The solo file is never grouped.
    assert!(
        !stdout.contains("solo.bin"),
        "unique-size file must not be grouped:\n{stdout}"
    );
}

/// DUPE-01 / `dupes_never_writes` — the fixture's file set + contents + mtimes are
/// byte-for-byte unchanged after a run (strictly read-only, T-03-13, locked Out of
/// Scope).
#[test]
fn dupes_never_writes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // A tree that exercises both the duplicate and the unique path.
    let payload = b"read-only invariant payload\n";
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("first.txt"), payload).unwrap();
    fs::write(root.join("sub").join("second.txt"), payload).unwrap();
    fs::write(root.join("loner.txt"), b"distinct content\n").unwrap();

    let before = snapshot_tree(root);

    dupes(root, &[]).success();

    let after = snapshot_tree(root);

    // Exact same file set, contents, and mtimes — nothing created, deleted, or
    // touched (dupes has NO write path).
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "dupes must not add or remove any file"
    );
    for (rel, (bytes, mtime)) in &before {
        let (abytes, amtime) = after.get(rel).expect("file must still exist");
        assert_eq!(bytes, abytes, "contents of {rel} must be unchanged");
        assert_eq!(
            mtime, amtime,
            "mtime of {rel} must be unchanged (read-only)"
        );
    }
}

/// DUPE-01 / WR-02 — pointing `box dupes` at a FILE (not a directory) is a clear
/// error, not silent empty output. Previously a file argument printed `No
/// duplicate files found.` with exit 0.
#[test]
fn dupes_file_argument_errors() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lonely.txt");
    fs::write(&file, b"content").unwrap();

    let output = dupes(&file, &[]).failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is not a directory"),
        "a file argument must produce a clear 'is not a directory' error, got: {stderr:?}"
    );
}

/// DUPE-01 / WR-03 — a non-existent path yields a clear "no such directory: X"
/// message naming the path, not dunce's raw `(os error 3)`.
#[test]
fn dupes_missing_path_friendly_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");

    let output = dupes(&missing, &[]).failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no such directory") && stderr.contains("does-not-exist"),
        "a missing path must produce a clear 'no such directory' error naming the path, got: {stderr:?}"
    );
}
