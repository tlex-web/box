//! Wave-0 integration tests for `box flatten --move` (FLAT-V2-02, DESTRUCTIVE).
//!
//! Black-box: each test builds a real source tree in a temp dir, runs the
//! compiled `box flatten --move` binary, and asserts on the relocation behavior
//! AND — for every abort path — that the SOURCE tree is byte-for-byte unchanged.
//! These map 1:1 to the 08-04 FLAT-V2-02 acceptance rows and the Code-review gate.
//!
//! Safety model under test (D-5 destructive template + Pitfall 5):
//! - `--move` is dry-run-DEFAULT: it writes NOTHING and previews the relocation
//!   plan unless `--force` is given (distinct from copy mode, where `--dry-run` is
//!   opt-in).
//! - `--force` relocates each file via copy (`safe_copy`, create-new — never
//!   clobbers) → verify (destination exists + size matches the source) → delete
//!   source. A delete is NEVER performed on a failed/short copy.
//! - Every abort path (containment refusal, dry-run, a mid-batch copy error)
//!   leaves the source tree byte-for-byte unchanged. Empty source DIRECTORIES are
//!   left in place after a move (only files relocate).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use assert_cmd::Command;
use assert_fs::prelude::*;

/// Run `box flatten <src> <out> [args]` with plain output forced; return the raw
/// `Output` (status + bytes), so the JSON / exit-code / snapshot assertions can
/// inspect everything. Mirrors the `flatten_output` runner in `tests/flatten.rs`.
fn run(src: &Path, out: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("flatten").arg(src).arg(out);
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box flatten --move")
}

/// Count the regular files directly inside `dir` (non-recursive).
fn files_in(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

/// Snapshot every regular file under `dir` (recursive) as
/// `relative_path -> (bytes, mtime)`. Copied verbatim from `tests/dupes.rs:46` —
/// the shared "snapshot-the-tree-unchanged" helper the Code-review gate requires
/// for every destructive abort path.
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

/// FLAT-V2-02 — `--move` with NO `--force` is dry-run-DEFAULT: it previews the
/// relocation plan and writes NOTHING. The source tree snapshot before == after
/// and the output dir stays empty.
#[test]
fn move_dryrun_default_no_write() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("a.txt").write_str("aaa").unwrap();
    src.child("sub/b.txt").write_str("bbbb").unwrap();

    let before = snapshot_tree(src.path());

    let output = run(src.path(), out.path(), &["--move"]); // NO --force
    assert!(
        output.status.success(),
        "flatten --move (dry-run default) should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The relocation plan is previewed on stdout (the planned files are listed).
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("a.txt"),
        "the dry-run preview must list the planned files:\n{stdout}"
    );

    // Source tree byte-for-byte unchanged; output dir empty (nothing relocated).
    assert_eq!(
        snapshot_tree(src.path()),
        before,
        "--move without --force must not touch the source tree"
    );
    assert_eq!(
        files_in(out.path()),
        0,
        "--move dry-run must write nothing to the output dir"
    );
}

/// FLAT-V2-02 — `--move --force` relocates each file: copy → verify → delete
/// source. Afterwards the output dir holds the flattened files (contents
/// preserved), the source FILES are gone, and the (now empty) source DIRECTORIES
/// remain in place (only files relocate).
#[test]
fn move_force_relocates() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("one.txt").write_str("1").unwrap();
    src.child("sub/two.txt").write_str("22").unwrap();
    src.child("sub/deep/three.txt").write_str("333").unwrap();

    let output = run(src.path(), out.path(), &["--move", "--force"]);
    assert!(
        output.status.success(),
        "flatten --move --force should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // All three files relocated flat into the output dir, contents preserved.
    assert!(out.path().join("one.txt").exists(), "one.txt relocated");
    assert!(out.path().join("two.txt").exists(), "two.txt relocated");
    assert!(out.path().join("three.txt").exists(), "three.txt relocated");
    assert_eq!(files_in(out.path()), 3, "all three files relocated flat");
    assert_eq!(fs::read_to_string(out.path().join("one.txt")).unwrap(), "1");
    assert_eq!(fs::read_to_string(out.path().join("two.txt")).unwrap(), "22");
    assert_eq!(
        fs::read_to_string(out.path().join("three.txt")).unwrap(),
        "333"
    );

    // Source FILES are deleted (the copy→verify→delete completed for each).
    assert!(
        !src.path().join("one.txt").exists(),
        "source file must be deleted after a move"
    );
    assert!(!src.path().join("sub").join("two.txt").exists());
    assert!(!src.path().join("sub").join("deep").join("three.txt").exists());

    // Source DIRECTORIES remain (empty source dirs are left in place — only files
    // relocate; flatten never prunes the source tree structure).
    assert!(
        src.path().join("sub").is_dir(),
        "an emptied source dir must remain in place"
    );
    assert!(
        src.path().join("sub").join("deep").is_dir(),
        "a nested emptied source dir must remain in place"
    );
}

/// FLAT-V2-02 / Pitfall 4 — the containment guard refuses `--move` when the output
/// dir is inside the source dir, BEFORE any I/O. The refusal exits non-zero and
/// the source tree snapshot is byte-for-byte unchanged.
#[test]
fn move_abort_containment_snapshot_unchanged() {
    let src = assert_fs::TempDir::new().unwrap();
    src.child("file.txt").write_str("data").unwrap();
    src.child("sub/nested.txt").write_str("nested").unwrap();
    let out_inside = src.path().join("flat"); // output dir INSIDE the source dir

    let before = snapshot_tree(src.path());

    let output = run(src.path(), &out_inside, &["--move", "--force"]);
    assert!(
        !output.status.success(),
        "containment refusal must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("inside source"),
        "the refusal must explain the containment guard: {stderr}"
    );

    // The source tree is byte-for-byte unchanged after the refusal.
    assert_eq!(
        snapshot_tree(src.path()),
        before,
        "a containment refusal must leave the source tree unchanged"
    );
}

/// FLAT-V2-02 / Pitfall 5 — a mid-batch copy error leaves the source tree
/// byte-for-byte unchanged: NO source file is deleted before the error (already
/// copied dests may exist in the output dir, but every source survives). The copy
/// error is induced deterministically by holding an EXCLUSIVE handle (Windows
/// `share_mode(0)`) to one source file so `box`'s copy of it fails with a sharing
/// violation — the pre-existing-destination path instead RENAMES via the occupied
/// seed, so a lock is the portable way to force `safe_copy` to error mid-run.
#[cfg(windows)]
#[test]
fn move_abort_midbatch_copy_error_snapshot_unchanged() {
    use std::os::windows::fs::OpenOptionsExt;

    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("alpha.bin").write_str("alpha-contents").unwrap();
    src.child("locked.bin").write_str("locked-contents").unwrap();
    src.child("gamma.bin").write_str("gamma-contents").unwrap();

    let before = snapshot_tree(src.path());

    // Hold an exclusive handle to one source file (no sharing) so any other-process
    // open of it fails for the duration of the box run.
    let locked_path = src.path().join("locked.bin");
    let guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_path)
        .expect("open the source file exclusively");

    let output = run(src.path(), out.path(), &["--move", "--force"]);
    // A runtime copy error → exit 1 (the RUNTIME/DATA code), NOT clap's usage 2.
    assert_eq!(
        output.status.code(),
        Some(1),
        "a mid-batch copy error must exit 1 (runtime), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Release the lock BEFORE snapshotting (the snapshot reads every source file).
    drop(guard);

    // The core data-loss invariant: NO source file was deleted before the error,
    // so the SOURCE tree is byte-for-byte unchanged. (copy→verify→delete defers
    // every delete until all copies+verifies succeed; a copy error aborts with
    // zero deletes. Already-copied dests may exist in `out`, but every source
    // survives.)
    assert_eq!(
        snapshot_tree(src.path()),
        before,
        "a mid-batch copy error must leave the source tree byte-for-byte unchanged"
    );
}

/// FLAT-V2-02 — `--move --json` (dry-run) emits the plan with `dry_run:true` and
/// writes nothing; `--move --force --json` emits the executed result with
/// `dry_run:false` and real `copied`/`total_bytes`. Both are exactly one parseable
/// JSON value with no ANSI (0x1B).
#[test]
fn move_json_plan_and_executed() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("one.txt").write_str("1").unwrap();
    src.child("sub/two.txt").write_str("22").unwrap();

    let before = snapshot_tree(src.path());

    // --- Part A: --move --json (dry-run) → plan projection, dry_run:true ---
    let plan_out = run(src.path(), out.path(), &["--move", "--json"]);
    assert!(
        plan_out.status.success(),
        "move --json (dry-run) should exit 0: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&plan_out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(true)),
        "the plan projection must carry dry_run:true: {v}"
    );
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be a plan array");
    assert_eq!(results.len(), 2, "two planned items");
    assert!(
        !plan_out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );

    // The dry-run wrote nothing: source unchanged, output empty.
    assert_eq!(
        snapshot_tree(src.path()),
        before,
        "move --json dry-run must not touch the source"
    );
    assert_eq!(
        files_in(out.path()),
        0,
        "move --json dry-run must write nothing"
    );

    // --- Part B: --move --force --json → executed result, dry_run:false ---
    let exec_out = run(src.path(), out.path(), &["--move", "--force", "--json"]);
    assert!(
        exec_out.status.success(),
        "move --force --json should exit 0: {}",
        String::from_utf8_lossy(&exec_out.stderr)
    );
    let v2: serde_json::Value =
        serde_json::from_slice(&exec_out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v2.get("dry_run"),
        Some(&serde_json::json!(false)),
        "the executed result must carry dry_run:false: {v2}"
    );
    assert_eq!(
        v2.get("copied"),
        Some(&serde_json::json!(2)),
        "`.copied` must reflect the real relocation count"
    );
    assert!(
        v2.get("total_bytes").and_then(|t| t.as_u64()).is_some(),
        "`.total_bytes` must be numeric on a real run"
    );
    assert!(
        !exec_out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );

    // The force run actually relocated the files: dests present, sources gone.
    assert_eq!(files_in(out.path()), 2, "both files relocated flat");
    assert!(
        !src.path().join("one.txt").exists(),
        "source file must be deleted after a move"
    );
}
