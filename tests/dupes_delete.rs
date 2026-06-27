//! Wave-0 integration tests for `box dupes --delete` (DUPE-V2-02, DESTRUCTIVE).
//!
//! Black-box: each test builds a real directory tree in a temp dir, runs the
//! compiled `box dupes --delete` binary, and asserts on the destructive
//! deduplication behavior AND — for the dry-run and every abort path — that the
//! tree is byte-for-byte unchanged. These map 1:1 to the 08-05 DUPE-V2-02
//! acceptance rows and the mandatory adversarial Code-review gate.
//!
//! Safety model under test (D-5 destructive template + RESEARCH Pitfall 6):
//! - `--delete` is dry-run-DEFAULT: it writes NOTHING and previews which copies it
//!   would remove unless `--force` is also given.
//! - `--delete --force` removes duplicates non-interactively, keeping at least ONE
//!   copy per group — the FIRST member in the deterministic sorted path order
//!   (keep-first). A group can never lose its last real copy.
//! - A hardlink ALIAS of the kept member is NEVER a deletion candidate: a member
//!   sharing the kept member's `(volume_serial, file_index)` is kept on disk
//!   (deleting it frees nothing and destroys a name).
//! - The pre-flight computes the WHOLE deletion plan and aborts the ENTIRE
//!   operation before any `remove_file` if it finds a problem (abort-all-before-
//!   any), leaving the tree byte-for-byte unchanged.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use assert_cmd::Command;

/// Run `box dupes <path> [args]` with plain output forced; return the assert
/// handle. Mirrors the `dupes` runner in `tests/dupes.rs`.
fn dupes(path: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("dupes").arg(path);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Capture `box dupes <path> [args]` raw stdout/stderr bytes + exit status, for the
/// JSON-purity and exit-code assertions. Forces `NO_COLOR=1`.
fn dupes_output(path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("dupes").arg(path);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box dupes --delete")
}

/// Snapshot every regular file under `dir` (recursive) as
/// `relative_path -> (bytes, mtime)`. Copied verbatim from `tests/dupes.rs:46` —
/// the shared "snapshot-the-tree-unchanged" helper the Code-review gate requires
/// for the dry-run and every destructive abort path.
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

/// DUPE-V2-02 — `--delete` with NO `--force` is dry-run-DEFAULT: it previews which
/// copies it would remove and writes NOTHING. The tree snapshot before == after,
/// and every duplicate file still exists.
#[test]
fn delete_dryrun_default_no_write() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let payload = b"DUPLICATE PAYLOAD - identical bytes in three files\n";
    fs::write(root.join("aaa.bin"), payload).unwrap();
    fs::write(root.join("bbb.bin"), payload).unwrap();
    fs::write(root.join("ccc.bin"), payload).unwrap();
    fs::write(root.join("solo.bin"), b"a one-of-a-kind payload\n").unwrap();

    let before = snapshot_tree(root);

    let out = dupes_output(root, &["--delete"]); // NO --force
    assert!(
        out.status.success(),
        "dupes --delete (dry-run default) should exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The preview names the files it WOULD remove (the non-kept copies).
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("bbb.bin") || stdout.contains("ccc.bin"),
        "the dry-run preview must list the copies it would delete:\n{stdout}"
    );

    // Nothing deleted: every file still present, tree byte-for-byte unchanged.
    assert!(root.join("aaa.bin").exists());
    assert!(root.join("bbb.bin").exists());
    assert!(root.join("ccc.bin").exists());
    assert!(root.join("solo.bin").exists());
    assert_eq!(
        snapshot_tree(root),
        before,
        "--delete without --force must not touch the tree"
    );
}

/// DUPE-V2-02 — `--delete --force` over a 3-copy duplicate group leaves exactly
/// ONE copy: the FIRST in sorted path order (keep-first). The other two are
/// removed; a non-duplicate file is untouched.
#[test]
fn delete_keep_first() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Three byte-identical copies. Sorted ascending by path -> aaa < bbb < ccc, so
    // keep-first keeps aaa.bin and deletes bbb.bin + ccc.bin.
    let payload = b"keep-first duplicate payload - three identical copies\n";
    fs::write(root.join("aaa.bin"), payload).unwrap();
    fs::write(root.join("bbb.bin"), payload).unwrap();
    fs::write(root.join("ccc.bin"), payload).unwrap();
    // A unique file that must survive untouched.
    fs::write(root.join("unique.bin"), b"distinct content\n").unwrap();

    dupes(root, &["--delete", "--force"]).success();

    // Exactly the first sorted member survives; the rest are deleted.
    assert!(
        root.join("aaa.bin").exists(),
        "keep-first must keep the first sorted copy (aaa.bin)"
    );
    assert!(
        !root.join("bbb.bin").exists(),
        "the second copy must be deleted"
    );
    assert!(
        !root.join("ccc.bin").exists(),
        "the third copy must be deleted"
    );
    // The kept copy still has the original content.
    assert_eq!(fs::read(root.join("aaa.bin")).unwrap(), payload);
    // The unique file is never touched.
    assert!(
        root.join("unique.bin").exists(),
        "a non-duplicate file must be untouched"
    );
}

/// DUPE-V2-02 — multiple duplicate groups each retain at least ONE member after
/// `--force` (keep-first guarantees a group can never lose its last copy).
#[test]
fn delete_keeps_at_least_one_per_group() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Group 1: two identical copies of payload X.
    let x = b"group-one payload XXXXXXXXXXXXXXXXXXXX\n";
    fs::write(root.join("g1_a.bin"), x).unwrap();
    fs::write(root.join("g1_b.bin"), x).unwrap();
    // Group 2: three identical copies of a DIFFERENT payload Y.
    let y = b"group-two payload YYYYYYYYYYYYYYYYYYYYYYYYYYYY\n";
    fs::write(root.join("g2_a.bin"), y).unwrap();
    fs::write(root.join("g2_b.bin"), y).unwrap();
    fs::write(root.join("g2_c.bin"), y).unwrap();

    dupes(root, &["--delete", "--force"]).success();

    // Each group keeps exactly its first sorted member; nothing else.
    assert!(root.join("g1_a.bin").exists(), "group 1 keeps g1_a.bin");
    assert!(!root.join("g1_b.bin").exists(), "group 1 deletes g1_b.bin");
    assert!(root.join("g2_a.bin").exists(), "group 2 keeps g2_a.bin");
    assert!(!root.join("g2_b.bin").exists(), "group 2 deletes g2_b.bin");
    assert!(!root.join("g2_c.bin").exists(), "group 2 deletes g2_c.bin");

    // Both groups retained content (the kept copies are intact).
    assert_eq!(fs::read(root.join("g1_a.bin")).unwrap(), x);
    assert_eq!(fs::read(root.join("g2_a.bin")).unwrap(), y);
}

/// DUPE-V2-02 / RESEARCH Pitfall 6 — a hardlink ALIAS of the kept member is NEVER
/// deleted (a member sharing the kept member's inode is skipped). A real
/// `std::fs::hard_link` fixture: the alias survives, the inode's link count is not
/// driven to zero, and a genuinely distinct duplicate copy IS removed (dedup still
/// works).
#[test]
fn delete_hardlink_alias_never_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let payload = vec![b'H'; 4096];
    // alias.bin and original.bin share ONE inode (a hardlink pair); copy.bin is a
    // distinct on-disk file with identical content. Sorted ascending:
    // alias.bin < copy.bin < original.bin -> keep-first keeps alias.bin.
    let original = root.join("original.bin");
    let alias = root.join("alias.bin");
    let copy = root.join("copy.bin");
    fs::write(&original, &payload).unwrap();
    fs::write(&copy, &payload).unwrap();
    // A real hardlink: same inode, two names. If the platform/volume refuses (e.g.
    // a non-NTFS temp dir), skip — hardlink identity is NTFS-specific here.
    if fs::hard_link(&original, &alias).is_err() {
        eprintln!("skipping delete_hardlink_alias_never_deleted: hard_link unsupported");
        return;
    }

    dupes(root, &["--delete", "--force"]).success();

    // The kept member (alias.bin, sorted first) survives.
    assert!(
        alias.exists(),
        "the kept member (alias.bin) must survive"
    );
    // original.bin shares alias.bin's inode -> it is a hardlink alias of the kept
    // member and must NEVER be deleted (deleting it frees nothing, destroys a name).
    // Both names still resolving => the inode's link count was not driven to zero.
    assert!(
        original.exists(),
        "a hardlink alias of the kept member must never be deleted (link count must not hit zero)"
    );
    assert_eq!(
        fs::read(&alias).unwrap(),
        payload,
        "the kept inode's content is intact via the alias"
    );
    assert_eq!(
        fs::read(&original).unwrap(),
        payload,
        "the kept inode's content is intact via the original name"
    );
    // The genuinely distinct duplicate copy IS removed (dedup still happens).
    assert!(
        !copy.exists(),
        "a distinct duplicate copy (different inode) must still be deleted"
    );
}

/// DUPE-V2-02 — the dry-run leaves the tree byte-for-byte unchanged (snapshot
/// before == after, including mtimes). The read-only invariant of `--delete`
/// without `--force`.
#[test]
fn delete_abort_dryrun_snapshot_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let payload = b"dry-run snapshot payload - must be unchanged\n";
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("first.bin"), payload).unwrap();
    fs::write(root.join("sub").join("second.bin"), payload).unwrap();
    fs::write(root.join("loner.bin"), b"distinct content\n").unwrap();

    let before = snapshot_tree(root);

    dupes(root, &["--delete"]).success(); // dry-run default

    let after = snapshot_tree(root);
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "a --delete dry-run must not add or remove any file"
    );
    assert_eq!(
        before, after,
        "a --delete dry-run must leave the tree byte-for-byte unchanged (contents + mtimes)"
    );
}

/// DUPE-V2-02 — the abort-all-before-any pre-flight: when a member cannot be read
/// during planning, the ENTIRE operation aborts before a single `remove_file`, so
/// the tree is byte-for-byte unchanged (no partial deletion).
///
/// Windows: holding an EXCLUSIVE handle (`share_mode(0)`) to one duplicate forces
/// `box`'s read/identity phase to fail with a sharing violation, which aborts the
/// whole plan before the destructive phase — exactly the abort-all-before-any
/// guarantee. The other duplicates must survive.
#[cfg(windows)]
#[test]
fn delete_abort_preflight_snapshot_unchanged() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // A 3-copy duplicate group; one copy is locked exclusively so the plan phase
    // fails before any deletion runs.
    let payload = b"preflight-abort payload - identical across copies\n";
    fs::write(root.join("aaa.bin"), payload).unwrap();
    fs::write(root.join("locked.bin"), payload).unwrap();
    fs::write(root.join("zzz.bin"), payload).unwrap();

    let before = snapshot_tree(root);

    let locked_path = root.join("locked.bin");
    let guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_path)
        .expect("open the duplicate file exclusively");

    let out = dupes_output(root, &["--delete", "--force"]);
    // A runtime read/plan error -> exit 1 (RUNTIME), NOT clap's usage 2.
    assert_eq!(
        out.status.code(),
        Some(1),
        "an abort-all-before-any pre-flight failure must exit 1 (runtime), stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Release the lock BEFORE snapshotting (the snapshot reads every file).
    drop(guard);

    // The core data-loss invariant: NO file was deleted before the abort, so the
    // tree is byte-for-byte unchanged.
    assert_eq!(
        snapshot_tree(root),
        before,
        "an abort-all-before-any pre-flight failure must leave the tree byte-for-byte unchanged"
    );
    // Specifically, the other duplicates survive (no partial deletion).
    assert!(root.join("aaa.bin").exists());
    assert!(root.join("zzz.bin").exists());
    assert!(root.join("locked.bin").exists());
}

/// DUPE-V2-02 — `--delete --json` (dry-run) and `--delete --force --json` each emit
/// exactly one parseable JSON value with NO ANSI (`0x1B`), and a `dry_run` marker
/// reflecting the mode. The dry-run writes nothing; the force run deletes the
/// non-kept copies.
#[test]
fn delete_json_plan_and_executed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Three identical copies -> one group, keep-first keeps aaa.bin, deletes 2.
    let payload = b"json plan/executed payload - three identical copies\n";
    fs::write(root.join("aaa.bin"), payload).unwrap();
    fs::write(root.join("bbb.bin"), payload).unwrap();
    fs::write(root.join("ccc.bin"), payload).unwrap();

    // --- Part A: --delete --json (dry-run) -> dry_run:true, nothing deleted ---
    let plan_out = dupes_output(root, &["--delete", "--json"]);
    assert!(
        plan_out.status.success(),
        "dupes --delete --json (dry-run) should exit 0: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&plan_out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(true)),
        "the dry-run plan must carry dry_run:true: {v}"
    );
    assert!(
        v.get("results").and_then(|r| r.as_array()).is_some(),
        "the plan must carry a results array: {v}"
    );
    assert!(
        !plan_out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // Nothing deleted under the dry-run.
    assert!(root.join("aaa.bin").exists());
    assert!(root.join("bbb.bin").exists());
    assert!(root.join("ccc.bin").exists());

    // --- Part B: --delete --force --json -> dry_run:false, copies deleted ---
    let exec_out = dupes_output(root, &["--delete", "--force", "--json"]);
    assert!(
        exec_out.status.success(),
        "dupes --delete --force --json should exit 0: {}",
        String::from_utf8_lossy(&exec_out.stderr)
    );
    let v2: serde_json::Value =
        serde_json::from_slice(&exec_out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v2.get("dry_run"),
        Some(&serde_json::json!(false)),
        "the executed result must carry dry_run:false: {v2}"
    );
    assert!(
        !exec_out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // The force run deleted the two non-kept copies, kept the first.
    assert!(root.join("aaa.bin").exists(), "keep-first keeps aaa.bin");
    assert!(!root.join("bbb.bin").exists(), "force deletes bbb.bin");
    assert!(!root.join("ccc.bin").exists(), "force deletes ccc.bin");
}
