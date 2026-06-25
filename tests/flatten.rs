//! Wave-0 integration tests for `box flatten` (FLAT-01..04, FOUND-06).
//!
//! Black-box: each test builds a real source tree in a temp dir, runs the
//! compiled `box flatten` binary, and asserts on the output dir, exit codes, and
//! preserved metadata. The eight named tests map 1:1 to the VALIDATION matrix.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Run `box flatten <src> <out> [--dry-run]` and return the assert handle.
fn flatten(src: &Path, out: &Path, dry_run: bool) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("flatten").arg(src).arg(out);
    if dry_run {
        cmd.arg("--dry-run");
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.assert()
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

/// FLAT-02/03 / SC4 — dry-run on a nested tree with duplicate base names prints a
/// collision-renamed plan and writes ZERO bytes to the output dir.
#[test]
fn dry_run_plans_collisions_writes_nothing() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    // Two files with the SAME base name in different subdirs -> a collision.
    src.child("a/report.txt").write_str("aaa").unwrap();
    src.child("b/report.txt").write_str("bbb").unwrap();
    src.child("top.md").write_str("top").unwrap();

    flatten(src.path(), out.path(), true)
        .success()
        .stdout(predicate::str::contains("Dry run: nothing was copied."))
        .stdout(predicate::str::contains("renamed for collisions"))
        // The renamed plan row shows the encoded collision name + reason.
        .stdout(predicate::str::contains("[collision]"));

    // The output dir must be empty — dry-run wrote nothing.
    assert_eq!(files_in(out.path()), 0, "dry-run must not write any files");
}

/// FLAT-01 / SC5 — a real run copies every (non-hidden, non-symlink) source file
/// flat into the output dir; no subdirectories are created in the output.
#[test]
fn flatten_copies_all_files_flat() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("one.txt").write_str("1").unwrap();
    src.child("sub/two.txt").write_str("2").unwrap();
    src.child("sub/deep/three.txt").write_str("3").unwrap();

    flatten(src.path(), out.path(), false)
        .success()
        .stdout(predicate::str::contains("Done: copied 3 files"));

    // 3 files, all flat in the output root.
    assert_eq!(files_in(out.path()), 3, "all 3 files must be copied flat");
    // No subdirectories created in the output.
    let subdirs = fs::read_dir(out.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count();
    assert_eq!(subdirs, 0, "output must contain no subdirectories");
}

/// FLAT-04 / SC5 — each copied file's modified time equals its source's.
#[test]
fn preserves_mtime() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    let f = src.child("dated.txt");
    f.write_str("payload").unwrap();

    // Backdate the source mtime so an unpreserved copy would visibly differ.
    let backdated =
        std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 60 * 24 * 365);
    fs::File::options()
        .write(true)
        .open(f.path())
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(backdated))
        .unwrap();
    let src_mtime = fs::metadata(f.path()).unwrap().modified().unwrap();

    flatten(src.path(), out.path(), false).success();

    let dst_mtime = fs::metadata(out.path().join("dated.txt"))
        .unwrap()
        .modified()
        .unwrap();
    let diff = src_mtime
        .duration_since(dst_mtime)
        .or_else(|_| dst_mtime.duration_since(src_mtime))
        .unwrap();
    assert!(
        diff < std::time::Duration::from_secs(2),
        "copied mtime must match source (diff {diff:?})"
    );
}

/// FLAT-01/04 / SC5 — the source tree is byte-identical after the run (copy, not
/// move; originals untouched).
#[test]
fn originals_untouched() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("keep1.txt").write_str("hello").unwrap();
    src.child("nested/keep2.txt").write_str("world").unwrap();

    flatten(src.path(), out.path(), false).success();

    // Sources still present with identical contents.
    src.child("keep1.txt").assert("hello");
    src.child("nested/keep2.txt").assert("world");
    assert!(src.child("keep1.txt").path().exists());
    assert!(src.child("nested/keep2.txt").path().exists());
}

/// FLAT-04 / SC5 — a name already present in the output dir BEFORE the run is not
/// overwritten; the incoming file is collision-renamed instead (D-14).
#[test]
fn no_silent_overwrite() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    // Pre-existing output file with the same base name as an incoming file.
    out.child("report.txt").write_str("ORIGINAL").unwrap();
    src.child("report.txt").write_str("incoming").unwrap();

    flatten(src.path(), out.path(), false).success();

    // The pre-existing file must be untouched.
    out.child("report.txt").assert("ORIGINAL");
    // The incoming file landed under a renamed name (encoded source-relative).
    assert_eq!(
        files_in(out.path()),
        2,
        "incoming file must be renamed, not overwrite the existing one"
    );
}

/// FLAT-04 / Pitfall 4 — `flatten ./p ./p/flat` (output inside source) errors with
/// a non-zero exit BEFORE any copy.
#[test]
fn out_inside_src_aborts() {
    let src = assert_fs::TempDir::new().unwrap();
    src.child("file.txt").write_str("data").unwrap();
    let out_inside = src.path().join("flat");

    flatten(src.path(), &out_inside, false)
        .failure()
        .code(1)
        .stderr(predicate::str::contains("inside source"));

    // Nothing copied into the inner output dir.
    assert_eq!(
        files_in(&out_inside),
        0,
        "no files may be copied when the guard aborts"
    );
}

/// FLAT-04 / Pitfall 8 — a symlink in the source is skipped (counted under
/// "skipped"), with no infinite loop.
#[test]
fn skips_symlinks() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("real.txt").write_str("real").unwrap();

    let link = src.path().join("link.txt");
    let target = src.path().join("real.txt");
    let made = make_file_symlink(&target, &link);
    if !made {
        // Symlink creation needs privilege/Developer Mode on Windows; skip the
        // assertion rather than fail the suite on an unprivileged runner.
        eprintln!("skipping skips_symlinks: could not create a symlink on this host");
        return;
    }

    flatten(src.path(), out.path(), false)
        .success()
        .stdout(predicate::str::contains("skipped 1"))
        .stdout(predicate::str::contains("(skipped: symlink)"));

    // The symlink must NOT have been copied into the output dir.
    assert!(
        !out.path().join("link.txt").exists(),
        "symlink must not be copied"
    );
    // The real file still copied.
    assert!(out.path().join("real.txt").exists());
}

/// FOUND-06 — a >260-char source path fails loudly with a clear error, never
/// silently dropped. We synthesize the long path and assert the file is either
/// copied (if long paths are enabled) or the run reports a clear error — never a
/// silent success that drops the file.
#[test]
fn deep_path_no_silent_loss() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();

    // Build nested dirs until the total path exceeds the 260-char MAX_PATH.
    let mut deep = src.path().to_path_buf();
    let segment = "abcdefghij_klmnopqrst"; // 21 chars per level
    let mut created = true;
    while deep.to_string_lossy().chars().count() < 300 {
        deep.push(segment);
        if fs::create_dir(&deep).is_err() {
            created = false;
            break;
        }
    }

    if !created {
        // The OS refused to create the deep dir (MAX_PATH without long-path
        // support). FOUND-06 is about flatten not silently dropping files it CAN
        // see; if the fixture itself can't exist there is nothing to flatten.
        eprintln!("skipping deep_path_no_silent_loss: host rejected the >260-char fixture dir");
        return;
    }

    let deep_file = deep.join("buried.txt");
    if fs::write(&deep_file, b"buried").is_err() {
        eprintln!("skipping deep_path_no_silent_loss: host rejected the >260-char fixture file");
        return;
    }
    // Also a shallow file so a partial/aborted run is observable.
    src.child("shallow.txt").write_str("shallow").unwrap();

    let assert = flatten(src.path(), out.path(), false);
    let output = assert.get_output().clone();

    if output.status.success() {
        // Long paths are enabled on this host — the buried file MUST be present
        // (no silent loss): one shallow + one buried = 2 files.
        assert_eq!(
            files_in(out.path()),
            2,
            "a successful run must include the deep file (no silent loss)"
        );
    } else {
        // A failure is acceptable per FOUND-06 — but it must be LOUD: a clear
        // error on stderr, not a silent zero-exit drop.
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error:") || stderr.contains("flattening"),
            "deep-path failure must surface a clear error, got stderr: {stderr:?}"
        );
    }
}

/// Best-effort file symlink creation; returns false if the platform/permissions
/// disallow it (so the symlink test can skip rather than fail).
#[cfg(windows)]
fn make_file_symlink(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(not(windows))]
fn make_file_symlink(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

// --- Scriptable spine (SPINE-02, Wave-7b, D-12/D-13) — from tests/uuid.rs:135 ---
//
// `box flatten <src> <out> --json` carries a `dry_run` boolean (D-13) and a
// `{results:[{src,dst,action,reason}], count, dry_run, copied, renamed, skipped,
// total_bytes}` document. `--json` is orthogonal to `--force`/dry-run (D-12):
// dry-run+json emits the PLAN, real+json emits the EXECUTED result. Purity: one
// JSON value, no 0x1B, no BOM.

/// Capture `box flatten <src> <out> [args]` raw stdout bytes + exit status, for
/// the JSON assertions (raw bytes, not a trimmed String). Forces `NO_COLOR=1`.
fn flatten_output(src: &Path, out: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("flatten").arg(src).arg(out);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box flatten")
}

/// SPINE-02 — `box flatten <src> <out> --json` (dry-run by default here is a real
/// run; we use --dry-run explicitly elsewhere) emits exactly one well-formed JSON
/// document with `.results`, `.count`, and `.dry_run`, no ANSI, no BOM. Runnable
/// via `cargo test --test flatten json_purity`.
#[test]
fn json_purity() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("one.txt").write_str("1").unwrap();
    src.child("sub/two.txt").write_str("22").unwrap();

    let output = flatten_output(src.path(), out.path(), &["--json"]);
    assert!(output.status.success(), "box flatten --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");

    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 2, "two files planned/copied");
    assert_eq!(v.get("count"), Some(&serde_json::json!(2)));
    assert!(
        v.get("dry_run").and_then(|d| d.as_bool()).is_some(),
        "`.dry_run` must be a boolean"
    );
    // Each row carries {src, action} (dst/reason may be null).
    for row in results {
        assert!(
            row.get("src").and_then(|s| s.as_str()).is_some(),
            "row carries a string `src`: {row}"
        );
        let action = row
            .get("action")
            .and_then(|a| a.as_str())
            .expect("row carries a string `action`");
        assert!(
            ["copy", "rename", "skip"].contains(&action),
            "action is the lowercased RowStatus, got: {action}"
        );
    }

    assert!(
        !output.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    assert_ne!(
        &output.stdout[..3.min(output.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-02 / D-12/D-13 — `box flatten --dry-run --json` yields `.dry_run == true`
/// and a `.results` PLAN array, writing nothing. Runnable via
/// `cargo test --test flatten json_dry_run`.
#[test]
fn json_dry_run() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("a/report.txt").write_str("aaa").unwrap();
    src.child("b/report.txt").write_str("bbb").unwrap();
    src.child("top.md").write_str("top").unwrap();

    let output = flatten_output(src.path(), out.path(), &["--dry-run", "--json"]);
    assert!(output.status.success(), "dry-run --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(true)),
        "`.dry_run` must be true on a --dry-run"
    );
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be a plan array");
    assert_eq!(results.len(), 3, "three files in the plan");
    // A collision rename must be present in the plan.
    assert!(
        results
            .iter()
            .any(|r| r.get("action").and_then(|a| a.as_str()) == Some("rename")),
        "the colliding report.txt produces a rename row: {v}"
    );

    // Dry-run wrote nothing.
    assert_eq!(files_in(out.path()), 0, "dry-run must not write any files");
}

/// SPINE-02 / D-12 — `box flatten <src> <out> --json` (a REAL run, no --dry-run)
/// yields `.dry_run == false` and real copied counts; the files are on disk.
/// Runnable via `cargo test --test flatten json_force_run`.
#[test]
fn json_force_run() {
    let src = assert_fs::TempDir::new().unwrap();
    let out = assert_fs::TempDir::new().unwrap();
    src.child("one.txt").write_str("1").unwrap();
    src.child("sub/two.txt").write_str("22").unwrap();

    let output = flatten_output(src.path(), out.path(), &["--json"]);
    assert!(output.status.success(), "real --json run should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(false)),
        "`.dry_run` must be false on a real run"
    );
    assert_eq!(
        v.get("copied"),
        Some(&serde_json::json!(2)),
        "`.copied` reflects the real-run copy count"
    );
    assert!(
        v.get("total_bytes").and_then(|t| t.as_u64()).is_some(),
        "`.total_bytes` must be a number on a real run"
    );

    // The real run actually copied the files flat.
    assert_eq!(files_in(out.path()), 2, "both files copied flat");
}
