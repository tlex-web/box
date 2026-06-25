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

// --- Scriptable spine (SPINE-02, Wave-7b) — copied from tests/uuid.rs:135 -------
//
// `box dupes <dir> --json` emits ONE `{results,count,wasted_bytes}` document
// (D-17): each `.results` group is `{size, paths:[…]}` where `paths` are lossy
// STRINGS (D-4 — never `to_str().unwrap()`), `count` is the number of groups, and
// `wasted_bytes` is the redundant-copy total. Purity: one JSON value, no 0x1B, no
// BOM.

/// Capture `box dupes <path> [args]` raw stdout bytes + exit status, for the
/// JSON-purity assertions (raw bytes, not a trimmed String). Forces `NO_COLOR=1`.
fn dupes_output(path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("dupes").arg(path);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box dupes")
}

/// SPINE-02 — `box dupes <dir> --json` emits exactly one well-formed JSON document
/// `{"results":[{"size":N,"paths":["…","…"]}],"count":N,"wasted_bytes":N}`:
/// `.results[*].paths` is a STRING array (D-4 lossy), `.wasted_bytes` is present,
/// with no ANSI and no BOM. Runnable via `cargo test --test dupes json_purity`.
#[test]
fn json_purity() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // A confirmed duplicate pair (identical content) so there is at least one
    // group; the JSON path must serialize its paths as strings.
    let payload = b"DUPLICATE PAYLOAD - identical bytes\n";
    fs::write(root.join("dup_one.txt"), payload).unwrap();
    fs::write(root.join("dup_two.txt"), payload).unwrap();
    // A unique file (never grouped).
    fs::write(root.join("unique.txt"), b"a one-of-a-kind payload\n").unwrap();

    let out = dupes_output(root, &["--json"]);
    assert!(out.status.success(), "box dupes --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The D-17 shape: `{results, count, wasted_bytes}`.
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "exactly one duplicate group");
    assert_eq!(
        v.get("count"),
        Some(&serde_json::json!(1)),
        "`.count` is the number of groups"
    );

    // 3. `.wasted_bytes` is present and numeric ((2-1)*size for the pair).
    let wasted = v
        .get("wasted_bytes")
        .and_then(|w| w.as_u64())
        .expect("`.wasted_bytes` must be a number");
    assert_eq!(
        wasted,
        payload.len() as u64,
        "wasted = (2-1) * file_size for the single duplicate pair"
    );

    // 4. The group carries a numeric `size` and a `paths` STRING array (D-4 lossy).
    let group = &results[0];
    assert!(
        group.get("size").and_then(|s| s.as_u64()).is_some(),
        "the group carries a numeric `size`: {group}"
    );
    let paths = group
        .get("paths")
        .and_then(|p| p.as_array())
        .expect("`.results[0].paths` must be an array");
    assert_eq!(paths.len(), 2, "the duplicate pair has two paths");
    for p in paths {
        assert!(
            p.as_str().is_some(),
            "every path is serialized as a string (to_string_lossy, D-4): {p}"
        );
    }

    // 5. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 6. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// WR-02 — the documented D-4 lossy path-serialization (`to_string_lossy`) must
/// never panic on an edge file name under `--json`, and must still emit exactly
/// one well-formed JSON document (exit 0). This is the regression guard the
/// review flagged as missing: the base64 decode path has `json_decode_non_utf8`,
/// but the filesystem commands had no equivalent, so a regression that started
/// emitting `to_str().unwrap()` on a non-UTF-8 name would go uncaught.
///
/// On Unix we build a GENUINELY non-UTF-8 file name (a lone 0x80 continuation
/// byte) via `OsStrExt` — exactly the case `to_string_lossy` maps to U+FFFD.
/// On Windows the std API cannot create a non-UTF-8 OS string from raw bytes
/// (NTFS names are UTF-16), so we use a non-ASCII multibyte name (`café_dup`)
/// which exercises the same `to_string_lossy` serialization path with multibyte
/// content. Either way the contract is identical: `--json` exits 0, the document
/// parses, and `.results[].paths` are strings (never a panic, D-4).
#[test]
fn json_lossy_path_name_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // A duplicate pair so there is at least one group whose paths get serialized.
    let payload = b"WR-02 lossy-name duplicate payload\n";

    #[cfg(unix)]
    let (name_a, name_b) = {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        // 0x80 is a lone UTF-8 continuation byte → not valid UTF-8 → U+FFFD on
        // to_string_lossy. The platform (Linux/macOS) allows it in a file name.
        let a = OsStr::from_bytes(b"dup_\x80_a.bin").to_owned();
        let b = OsStr::from_bytes(b"dup_\x80_b.bin").to_owned();
        (a, b)
    };
    #[cfg(not(unix))]
    let (name_a, name_b) = {
        // Windows: a valid-but-multibyte non-ASCII name (UTF-16 on disk). This
        // still drives the to_string_lossy projection, just without an invalid
        // sequence (which the platform disallows here).
        (
            std::ffi::OsString::from("café_dup_a.bin"),
            std::ffi::OsString::from("café_dup_b.bin"),
        )
    };

    fs::write(root.join(&name_a), payload).unwrap();
    fs::write(root.join(&name_b), payload).unwrap();

    let out = dupes_output(root, &["--json"]);
    // The core assertion: the run does NOT panic and exits 0 on the edge name.
    assert!(
        out.status.success(),
        "box dupes --json must exit 0 on an edge file name (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );

    // It still emits exactly one well-formed JSON document with string paths.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "the edge-named pair forms one group");
    let paths = results[0]
        .get("paths")
        .and_then(|p| p.as_array())
        .expect("`.results[0].paths` must be an array");
    for p in paths {
        assert!(
            p.as_str().is_some(),
            "every path is a (lossy) string, never a panic (D-4): {p}"
        );
    }
}

/// WR-05 — the intra-group `.results[*].paths` ordering is a documented contract
/// (the `(hash, path)` sort before grouping, RESEARCH Pitfall 6). This pins that
/// a consumer may rely on a deterministic, ascending path order within each
/// duplicate group, so the `DupeRow` doc's ordering guarantee has a CI gate.
#[test]
fn json_paths_sorted_within_group() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Three byte-identical files whose names sort in a non-creation order, so a
    // sorted result is distinguishable from insertion/walk order.
    let payload = b"WR-05 sorted-paths duplicate payload\n";
    fs::write(root.join("zeta.bin"), payload).unwrap();
    fs::write(root.join("alpha.bin"), payload).unwrap();
    fs::write(root.join("mid.bin"), payload).unwrap();

    let out = dupes_output(root, &["--json"]);
    assert!(out.status.success(), "box dupes --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "the three identical files form one group");

    let paths: Vec<&str> = results[0]
        .get("paths")
        .and_then(|p| p.as_array())
        .expect("`.results[0].paths` must be an array")
        .iter()
        .map(|p| p.as_str().expect("each path is a string"))
        .collect();
    assert_eq!(paths.len(), 3, "the group has all three duplicate paths");

    // The contract: paths are sorted ascending within the group.
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(
        paths, sorted,
        "`.results[*].paths` must be sorted ascending within a group (WR-05): {paths:?}"
    );
}
