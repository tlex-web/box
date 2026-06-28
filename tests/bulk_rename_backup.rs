//! Wave-2 integration tests for `box bulk-rename --backup` (RENM-V2-02,
//! DESTRUCTIVE — the JSON undo manifest).
//!
//! Black-box: each test builds a real directory tree in a temp dir, points the
//! command's `LOCALAPPDATA` at a SECOND temp dir (so the manifest location is
//! observable and isolated), runs the compiled `box bulk-rename --backup` binary,
//! and asserts the recoverability surface the mandatory Code-review gate requires
//! (D-22 + RESEARCH Pitfall 8):
//!
//! - `--backup` writes a JSON undo MANIFEST (a serde projection of the
//!   pre-flight-cleared `Plan`), NOT file copies — a pure rename is reversible
//!   from the `{old → new}` name map alone.
//! - The manifest is written to `%LOCALAPPDATA%\box\undo\<id>.json` (OUTSIDE the
//!   renamed tree so `--recursive` never re-walks it and it survives renaming the
//!   target dir); its path is echoed to stderr.
//! - The FULL manifest (every entry `applied:false`) is written + `fsync`'d
//!   BEFORE the first `std::fs::rename`; each entry flips `applied:true` as its
//!   rename returns, so a mid-batch I/O error leaves a manifest whose `applied`
//!   flags EXACTLY partition done-vs-pending → the directory is reconcilable.
//! - `--backup` is orthogonal to and only meaningful with `--force`: on a dry-run
//!   it is a clean no-op (no manifest, nothing renamed).
//! - The pre-flight abort path (abort-all-before-any) writes NEITHER the manifest
//!   NOR any rename — nothing on disk changes (snapshot-the-tree-unchanged).
//!
//! Every test forces `NO_COLOR=1` so ANSI never leaks into an assertion.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::prelude::*;

/// Run `box bulk-rename <dir> <pattern> <replacement> [args]` with plain output
/// forced and `LOCALAPPDATA` pointed at `localappdata` (so the undo manifest lands
/// in an isolated, observable location). Returns the raw captured output.
fn run_backup(
    dir: &Path,
    localappdata: &Path,
    pattern: &str,
    replacement: &str,
    args: &[&str],
) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("bulk-rename")
        .arg(dir)
        .arg(pattern)
        .arg(replacement);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.env("LOCALAPPDATA", localappdata);
    cmd.output().expect("run box bulk-rename --backup")
}

/// Find the single `<id>.json` undo manifest under `$LOCALAPPDATA/box/undo/`, if
/// any was written. Returns `None` if the undo dir does not exist or holds no
/// `.json` file (the dry-run / abort no-op cases).
fn find_manifest(localappdata: &Path) -> Option<PathBuf> {
    let undo = localappdata.join("box").join("undo");
    let rd = fs::read_dir(&undo).ok()?;
    rd.flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "json").unwrap_or(false))
}

/// Snapshot every regular file name directly inside `dir` (non-recursive) as
/// `name -> bytes`. Used by the abort/dry-run paths to assert the directory is
/// byte-for-byte unchanged. Copied from `tests/bulk_rename.rs:63`.
fn snapshot_names(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut map = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let ft = entry.file_type().unwrap();
        if ft.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            let bytes = fs::read(entry.path()).unwrap();
            map.insert(name, bytes);
        }
    }
    map
}

/// RENM-V2-02 — `--backup --force` on a clean rename plan writes a `<id>.json`
/// manifest under `$LOCALAPPDATA/box/undo/`: valid JSON, one `{old, new, applied}`
/// record per renamed file with ABSOLUTE paths, every entry `applied:true` after a
/// successful run, the manifest OUTSIDE the renamed tree, and its path echoed to
/// stderr.
#[test]
fn backup_manifest_written() {
    let dir = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let out = run_backup(
        dir.path(),
        local.path(),
        r"IMG_(\d+)",
        "img_$1",
        &["--backup", "--force"],
    );
    assert!(
        out.status.success(),
        "--backup --force should exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The manifest exists under $LOCALAPPDATA/box/undo/<id>.json.
    let manifest_path =
        find_manifest(local.path()).expect("a manifest must be written under box/undo/");

    // Its path is echoed to stderr.
    let stderr = String::from_utf8_lossy(&out.stderr);
    let fname = manifest_path.file_name().unwrap().to_string_lossy();
    assert!(
        stderr.contains(&*fname),
        "the manifest path must be echoed to stderr, got: {stderr}"
    );

    // Valid JSON: one {old,new,applied} per renamed file, absolute paths, all true.
    let raw = fs::read_to_string(&manifest_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).expect("the manifest must be valid JSON");
    let entries = v
        .get("entries")
        .and_then(|e| e.as_array())
        .expect("`.entries` must be an array");
    assert_eq!(entries.len(), 2, "one entry per renamed file");
    for e in entries {
        let old = e.get("old").and_then(|x| x.as_str()).expect("`old` string");
        let new = e.get("new").and_then(|x| x.as_str()).expect("`new` string");
        assert!(
            Path::new(old).is_absolute(),
            "`old` must be an absolute path: {old}"
        );
        assert!(
            Path::new(new).is_absolute(),
            "`new` must be an absolute path: {new}"
        );
        assert_eq!(
            e.get("applied").and_then(|x| x.as_bool()),
            Some(true),
            "every entry must be applied:true after a successful run"
        );
    }

    // The renames happened on disk.
    assert!(dir.path().join("img_0042.jpg").exists());
    assert!(dir.path().join("img_0043.jpg").exists());

    // The manifest lives OUTSIDE the renamed tree (Pitfall 8).
    assert!(
        !manifest_path.starts_with(dir.path()),
        "the manifest must live outside the renamed tree, got {}",
        manifest_path.display()
    );
}

/// RENM-V2-02 / WR-02 — when invoked with a RELATIVE target dir, the manifest
/// still records ABSOLUTE `old`/`new` paths. The manifest lives in a
/// cwd-independent location (`%LOCALAPPDATA%`), so cwd-relative entries would be
/// unreconcilable once the cwd changes; the command canonicalizes the target dir
/// before planning. Guards non-UTF-8 names via `to_string_lossy` (D-4).
#[test]
fn backup_manifest_absolute_paths_for_relative_dir() {
    let root = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    // The renamed files live in <root>/work; we invoke with cwd=<root> and pass the
    // RELATIVE dir "work" to exercise the cwd-relative path bug.
    let work = root.child("work");
    work.create_dir_all().unwrap();
    work.child("IMG_0042.jpg").write_str("a").unwrap();

    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.current_dir(root.path()) // cwd = <root>
        .arg("bulk-rename")
        .arg("work") // RELATIVE dir argument
        .arg(r"IMG_(\d+)")
        .arg("img_$1")
        .arg("--backup")
        .arg("--force")
        .env("NO_COLOR", "1")
        .env("LOCALAPPDATA", local.path());
    let out = cmd.output().expect("run box bulk-rename --backup");
    assert!(
        out.status.success(),
        "--backup --force with a relative dir should exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest_path =
        find_manifest(local.path()).expect("a manifest must be written for a relative dir");
    let raw = fs::read_to_string(&manifest_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).expect("the manifest must be valid JSON");
    let entries = v
        .get("entries")
        .and_then(|e| e.as_array())
        .expect("`.entries` must be an array");
    assert_eq!(entries.len(), 1, "one entry per renamed file");
    for e in entries {
        let old = e.get("old").and_then(|x| x.as_str()).expect("`old` string");
        let new = e.get("new").and_then(|x| x.as_str()).expect("`new` string");
        assert!(
            Path::new(old).is_absolute(),
            "`old` must be ABSOLUTE even for a relative dir argument, got: {old}"
        );
        assert!(
            Path::new(new).is_absolute(),
            "`new` must be ABSOLUTE even for a relative dir argument, got: {new}"
        );
    }
    // The rename happened on disk.
    assert!(work.path().join("img_0042.jpg").exists());
}

/// RENM-V2-02 — `--backup` WITHOUT `--force` is a clean no-op: it writes NO
/// manifest and renames nothing (the manifest is a property of an EXECUTED run).
#[test]
fn backup_dryrun_noop() {
    let dir = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let before = snapshot_names(dir.path());

    let out = run_backup(
        dir.path(),
        local.path(),
        r"IMG_(\d+)",
        "img_$1",
        &["--backup"], // NO --force
    );
    assert!(
        out.status.success(),
        "--backup dry-run should exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // No manifest written on a dry-run.
    assert!(
        find_manifest(local.path()).is_none(),
        "--backup without --force must write no manifest"
    );
    // Nothing renamed — the tree is byte-for-byte unchanged.
    assert_eq!(
        before,
        snapshot_names(dir.path()),
        "a --backup dry-run must rename nothing"
    );
}

/// RENM-V2-02 / WR-03 — `write_manifest` persists via temp-file-then-atomic-rename,
/// so a successful run leaves exactly the final `<id>.json` and NO `<id>.json.tmp`
/// scratch file behind. This confirms the atomic-replace path ran (a failed write
/// can never truncate the last good manifest in place).
#[test]
fn backup_manifest_atomic_no_tmp_leftover() {
    let dir = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let out = run_backup(
        dir.path(),
        local.path(),
        r"IMG_(\d+)",
        "img_$1",
        &["--backup", "--force"],
    );
    assert!(
        out.status.success(),
        "--backup --force should exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let undo = local.path().join("box").join("undo");
    let mut json = 0;
    let mut tmp = 0;
    for e in fs::read_dir(&undo).unwrap().flatten() {
        match e.path().extension().and_then(|x| x.to_str()) {
            Some("json") => json += 1,
            Some("tmp") => tmp += 1,
            _ => {}
        }
    }
    assert_eq!(json, 1, "exactly one final manifest must remain");
    assert_eq!(
        tmp, 0,
        "no <id>.json.tmp scratch file may be left behind (atomic rename consumes it)"
    );
}

/// RENM-V2-02 — a plan the pre-flight REJECTS (a collision) with `--backup
/// --force` writes NEITHER the manifest NOR any rename: abort-all-before-any still
/// wins and nothing on disk changes (snapshot-the-tree-unchanged). The manifest
/// write is strictly AFTER a clean pre-flight.
#[test]
fn backup_only_with_force_writes_nothing_on_abort() {
    let dir = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    // Both files collapse to `dup.txt` -> a collision aborts the whole batch.
    dir.child("a1.txt").write_str("aaa").unwrap();
    dir.child("b1.txt").write_str("bbb").unwrap();

    let before = snapshot_names(dir.path());

    let out = run_backup(
        dir.path(),
        local.path(),
        r".+",
        "dup.txt",
        &["--backup", "--force"],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "a colliding plan must abort with exit 1, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The abort path writes NEITHER manifest NOR any rename.
    assert!(
        find_manifest(local.path()).is_none(),
        "a pre-flight abort must write no manifest"
    );
    assert_eq!(
        before,
        snapshot_names(dir.path()),
        "a pre-flight abort must leave the tree byte-for-byte unchanged"
    );
}

/// RENM-V2-02 / RESEARCH Pitfall 8 — a mid-batch I/O error leaves a manifest whose
/// `applied` flags EXACTLY partition done-vs-pending, so the directory is
/// reconcilable: every `applied:true` entry's `new` exists and its `old` is gone;
/// every `applied:false` entry's `old` still exists.
///
/// Windows: holding an EXCLUSIVE handle (`share_mode(0)`) to one source file forces
/// that file's `std::fs::rename` to fail with a sharing violation mid-batch, so the
/// executor `?`-propagates (exit 1) after the FULL manifest was written + fsync'd
/// before the first rename. This proves the write-order contract AND the
/// applied-partition reconcilability with a real mid-batch failure.
#[cfg(windows)]
#[test]
fn backup_partition_recoverable() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = assert_fs::TempDir::new().unwrap();
    let local = assert_fs::TempDir::new().unwrap();
    // Four files, all renamed by the plan (f01.dat -> g01.dat, ...).
    for n in ["01", "02", "03", "04"] {
        dir.child(format!("f{n}.dat")).write_str(n).unwrap();
    }

    // Lock one source file EXCLUSIVELY so its rename fails with a sharing violation
    // mid-batch -> the executor exits 1, leaving an applied-partitioned manifest.
    let locked_path = dir.path().join("f02.dat");
    let guard = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_path)
        .expect("open one source file exclusively");

    let out = run_backup(
        dir.path(),
        local.path(),
        r"^f(\d+)\.dat$",
        "g$1.dat",
        &["--backup", "--force"],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "the locked rename must fail mid-batch (exit 1), stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The manifest exists: it was written + fsync'd BEFORE the first rename.
    let manifest_path =
        find_manifest(local.path()).expect("the manifest must exist before the first rename");
    let raw = fs::read_to_string(&manifest_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let entries = v.get("entries").and_then(|e| e.as_array()).unwrap();
    assert_eq!(entries.len(), 4, "all four renames are in the manifest");

    // Release the lock before checking on-disk reality.
    drop(guard);

    // The `applied` flags EXACTLY partition done-vs-pending.
    let mut pending = 0;
    for e in entries {
        let old = e.get("old").and_then(|x| x.as_str()).unwrap();
        let new = e.get("new").and_then(|x| x.as_str()).unwrap();
        let applied = e.get("applied").and_then(|x| x.as_bool()).unwrap();
        if applied {
            assert!(
                Path::new(new).exists(),
                "an applied entry's `new` must exist on disk: {new}"
            );
            assert!(
                !Path::new(old).exists(),
                "an applied entry's `old` must be gone: {old}"
            );
        } else {
            pending += 1;
            assert!(
                Path::new(old).exists(),
                "a pending entry's `old` must still exist: {old}"
            );
        }
    }
    assert!(
        pending >= 1,
        "at least the locked file must be a pending (applied:false) entry"
    );
    // The locked source file specifically survived its failed rename.
    assert!(
        locked_path.exists(),
        "the locked source must survive the failed rename (it is reconcilable)"
    );
}
