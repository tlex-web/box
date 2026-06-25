//! Wave-0 integration tests for `box bulk-rename` (RENM-01) — the phase's single
//! DESTRUCTIVE command, so every abort path is asserted to leave the tree
//! byte-for-byte unchanged.
//!
//! Black-box: each test builds a real directory tree in a temp dir, runs the
//! compiled `box bulk-rename` binary (positional `dir pattern replacement`,
//! optional `--force`/`--recursive`), and asserts on the dry-run preview, the
//! `--force` execution, and — critically — the ABORT-ALL-BEFORE-ANY-RENAME
//! safety model (D-18): a collision, a cycle/swap, or a path-separator-injecting
//! replacement aborts the WHOLE batch (exit 1, nothing written) in BOTH dry-run
//! and `--force`. Every test forces `NO_COLOR=1` so ANSI never leaks into an
//! assertion.
//!
//! Safety model under test (D-18, the ENTIRE safety story because
//! `std::fs::rename` SILENTLY OVERWRITES its destination on Windows — there is no
//! `create_new` backstop for moves):
//! - **Collision** — two files renaming to one name, or a target clobbering a
//!   pre-existing on-disk name not renamed away → abort, nothing written.
//! - **Cycle/swap** — `a→b, b→a` (any target equal to another item's source) →
//!   abort (no two-phase temp pass in v1).
//! - **Case-only** — `foo→Foo` is byte-different and IS a real rename; it must
//!   NOT be flagged as a self-collision (Pitfall 5).
//! - **Separator injection** — a `${1}` replacement producing `/` or `\` is
//!   refused/skipped, never executed (mirrors flatten's `encode_no_separator`).
//! - **Skips** — directories and symlinks are `-` rows; a no-op (`new == old`
//!   byte-exact) is a `(unchanged)` `-` row.
//! - **First-match** — `Regex::replace` rewrites only the FIRST match (D-17).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Run `box bulk-rename <dir> <pattern> <replacement> [extra args]` with plain
/// output forced and return the assert handle. Mirrors the `tests/flatten.rs` /
/// `tests/dupes.rs` runner shape (`Command::cargo_bin`, positional args,
/// `NO_COLOR=1`).
fn bulk_rename(
    dir: &Path,
    pattern: &str,
    replacement: &str,
    args: &[&str],
) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("bulk-rename")
        .arg(dir)
        .arg(pattern)
        .arg(replacement);
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Snapshot every regular file name directly inside `dir` (non-recursive) as
/// `name -> bytes`. Used by every abort path to assert the directory is
/// byte-for-byte unchanged after a refused run.
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

/// The EXACT (case-preserving) file names directly inside `dir`. NTFS is
/// case-insensitive but case-PRESERVING, and `Path::exists()` matches
/// case-insensitively — so a rename `IMG_0042.jpg` -> `img_0042.jpg` leaves
/// `Path::new("IMG_0042.jpg").exists()` TRUE. To observe the stored casing (and
/// to assert a name is truly absent vs. present-under-different-case) we must read
/// the directory and compare the listed names byte-exactly.
fn listed_names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}

/// RENM-01 — the DEFAULT run is a dry-run preview: it prints the `~ old -> new`
/// plan plus the dry-run summary (with the `Re-run with --force` hint) and writes
/// NOTHING.
#[test]
fn renm_dryrun_default_no_write() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let before = snapshot_names(dir.path());

    bulk_rename(dir.path(), r"IMG_(\d+)", "img_$1", &[])
        .success()
        .stdout(predicate::str::contains("-> img_0042.jpg"))
        .stdout(predicate::str::contains("-> img_0043.jpg"))
        .stdout(predicate::str::contains("Dry run:"))
        .stdout(predicate::str::contains("Re-run with --force to apply."));

    // Default run is a dry-run — the directory is byte-for-byte unchanged.
    let after = snapshot_names(dir.path());
    assert_eq!(before, after, "default (dry-run) must write nothing");
    // The stored casing is still the ORIGINAL (case-preserving check — NTFS is
    // case-insensitive, so `Path::exists` would falsely match the lowercased name).
    let names = listed_names(dir.path());
    assert!(
        names.iter().any(|n| n == "IMG_0042.jpg"),
        "original casing must be preserved by a dry-run, got {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "img_0042.jpg"),
        "dry-run must not produce the lowercased name, got {names:?}"
    );
}

/// RENM-01 — `--force` applies the FIRST-match capture-group replacement to the
/// full base name (D-16/D-17): `IMG_0042.jpg` -> `img_0042.jpg`.
#[test]
fn renm_force_capture_group() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    bulk_rename(dir.path(), r"IMG_(\d+)", "img_$1", &["--force"]).success();

    // The STORED casing is now lowercase (case-exact listing — `Path::exists` on
    // NTFS would falsely match the old uppercase name against the new lowercase
    // file). Old casing gone, new casing present, contents preserved.
    let names = listed_names(dir.path());
    assert!(
        names.iter().any(|n| n == "img_0042.jpg") && names.iter().any(|n| n == "img_0043.jpg"),
        "--force must store the lowercased names, got {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "IMG_0042.jpg") && !names.iter().any(|n| n == "IMG_0043.jpg"),
        "old uppercase casing must be gone, got {names:?}"
    );
    assert_eq!(fs::read(dir.path().join("img_0042.jpg")).unwrap(), b"a");
    assert_eq!(fs::read(dir.path().join("img_0043.jpg")).unwrap(), b"b");
}

/// RENM-01 / T-RENM-clobber — two files renaming to ONE name aborts the whole
/// batch before any rename (exit 1, nothing written) in BOTH dry-run and
/// `--force`.
#[test]
fn renm_collision_aborts() {
    for force in [false, true] {
        let dir = assert_fs::TempDir::new().unwrap();
        // Both files collapse to `dup.txt` under the replacement.
        dir.child("a1.txt").write_str("aaa").unwrap();
        dir.child("b1.txt").write_str("bbb").unwrap();

        let before = snapshot_names(dir.path());

        let args: &[&str] = if force { &["--force"] } else { &[] };
        // `.+` matches the whole base name, replacing it with the constant `dup.txt`.
        bulk_rename(dir.path(), r".+", "dup.txt", args)
            .failure()
            .code(1)
            .stderr(predicate::str::contains("Aborted").or(predicate::str::contains("collision")))
            .stdout(predicate::str::contains("[collision]"));

        let after = snapshot_names(dir.path());
        assert_eq!(
            before, after,
            "collision abort must leave the tree byte-for-byte unchanged (force={force})"
        );
    }
}

/// RENM-01 / T-RENM-cycle — a swap `a->b, b->a` is detected and aborts the batch
/// (no two-phase temp pass in v1), in BOTH dry-run and `--force`, nothing written.
///
/// Driving a real swap with one `Regex::replace`: files `01.txt` and `10.txt`
/// with pattern `01|10` (first match) and replacement that maps each to the
/// OTHER is not expressible in a single constant replacement. Instead we use the
/// reversible-digit trick: files `12.txt` and `21.txt`, pattern `(1)(2)|(2)(1)`
/// won't reorder either. The robust, expressible swap: name the files after their
/// TARGETS' sources directly — `a.txt` renames to `b.txt` and `b.txt` renames to
/// `a.txt` via a single alternation with backreference-free constant arms is
/// impossible, so we model the cycle with two files where the pattern maps the
/// digit `1`->`2`: `f1.txt` -> `f2.txt` (target = source of `f2.txt`) while
/// `f2.txt` -> `f1.txt` requires `2`->`1` simultaneously. A single replacement
/// can only do one direction, so the swap fixture uses the SAME pattern that maps
/// `o`<->`0` lookalikes: rename `ab.txt` and `ba.txt` by REVERSING via the
/// capture order `(.)(.)(\.txt)` -> `$2$1$3`: `ab.txt`->`ba.txt` and
/// `ba.txt`->`ab.txt` — a genuine swap from ONE pattern.
#[test]
fn renm_cycle_aborts() {
    for force in [false, true] {
        let dir = assert_fs::TempDir::new().unwrap();
        dir.child("ab.txt").write_str("AAA").unwrap();
        dir.child("ba.txt").write_str("BBB").unwrap();

        let before = snapshot_names(dir.path());

        // `(.)(.)(\.txt)` -> `$2$1$3`: ab.txt -> ba.txt AND ba.txt -> ab.txt.
        // Each target equals the OTHER item's source -> a cycle the detector must
        // abort (no two-phase temp pass in v1).
        let args: &[&str] = if force { &["--force"] } else { &[] };
        bulk_rename(dir.path(), r"^(.)(.)(\.txt)$", "$2$1$3", args)
            .failure()
            .code(1)
            .stderr(predicate::str::contains("Aborted").or(predicate::str::contains("cycle")));

        let after = snapshot_names(dir.path());
        assert_eq!(
            before, after,
            "cycle abort must leave the tree byte-for-byte unchanged (force={force})"
        );
    }
}

/// RENM-01 / T-RENM-caseonly — a case-only rename `foo`->`Foo` succeeds under
/// `--force` and is NOT falsely flagged as a self-collision; the stored casing
/// changes.
#[test]
fn renm_case_only_ok() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("payload").unwrap();

    // `^f` -> `F`: foo.txt -> Foo.txt (a real, byte-different rename that folds to
    // the same key as its own source).
    bulk_rename(dir.path(), r"^foo", "Foo", &["--force"]).success();

    // The stored name now carries the new casing; the old casing is gone on a
    // case-insensitive filesystem the entry is the same inode but the listed name
    // is `Foo.txt`.
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n == "Foo.txt"),
        "case-only rename must produce Foo.txt, got {names:?}"
    );
    assert_eq!(fs::read(dir.path().join("Foo.txt")).unwrap(), b"payload");
}

/// RENM-01 / T-RENM-pathinject — a replacement that would inject a path separator
/// (`/` or `\`) is refused/skipped, never executed (mirrors flatten's
/// `encode_no_separator` invariant). Nothing is written and no subdirectory is
/// created.
#[test]
fn renm_separator_refused() {
    for force in [false, true] {
        let dir = assert_fs::TempDir::new().unwrap();
        dir.child("file1.txt").write_str("data").unwrap();

        let before = snapshot_names(dir.path());

        // Replace the name with `sub/evil.txt` — a path-separator-injecting target.
        let args: &[&str] = if force { &["--force"] } else { &[] };
        let assert = bulk_rename(dir.path(), r".+", "sub/evil.txt", args);
        let output = assert.get_output().clone();

        // The injecting target is never executed: no `sub/` dir, no `evil.txt`,
        // the source file is untouched. Either it aborts (exit 1) or it is shown
        // as a refused `-` skip row — in NO case does the separator land on disk.
        assert!(
            !dir.path().join("sub").exists(),
            "separator injection must not create a subdirectory (force={force})"
        );
        let after = snapshot_names(dir.path());
        assert_eq!(
            before, after,
            "separator-refusal must leave the directory unchanged (force={force})"
        );
        // The refusal must be visible (a refused/collision row or an abort).
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            combined.contains("separator")
                || combined.contains("refused")
                || combined.contains("skipped")
                || combined.contains("Aborted"),
            "separator refusal must be reported, got: {combined:?}"
        );
    }
}

/// RENM-01 / CR-01 — a replacement that yields exactly `..` (a directory-escaping
/// target: `parent.join("..")` resolves to the GRANDPARENT) is refused in
/// pre-flight and aborts the whole batch (exit 1, nothing renamed) in BOTH dry-run
/// and `--force`. The escape never reaches the `rename` call: the source file is
/// untouched and no entry appears in the parent or grandparent directory.
#[test]
fn renm_dotdot_target_aborts() {
    for force in [false, true] {
        // A nested layout so we can prove the grandparent is untouched too: the
        // rename runs inside <root>/work, whose parent is <root>.
        let root = assert_fs::TempDir::new().unwrap();
        let work = root.child("work");
        work.create_dir_all().unwrap();
        work.child("a.txt").write_str("payload").unwrap();
        // A pre-existing file in the grandparent (<root>) must be left alone — a
        // `..` escape could otherwise clobber it.
        root.child("grandparent.txt").write_str("keep").unwrap();

        let before_work = snapshot_names(work.path());
        let before_root = snapshot_names(root.path());

        // `.+` matches the whole base name, replacing it with the constant `..`.
        let args: &[&str] = if force { &["--force"] } else { &[] };
        bulk_rename(work.path(), r".+", "..", args)
            .failure()
            .code(1)
            .stderr(predicate::str::contains("Aborted").and(
                predicate::str::contains("separator").or(predicate::str::contains("refused")),
            ));

        // The escaping target never reached `rename`: the work dir AND the
        // grandparent are byte-for-byte unchanged, and `a.txt` still exists.
        assert_eq!(
            before_work,
            snapshot_names(work.path()),
            "a `..` target must leave the work dir byte-for-byte unchanged (force={force})"
        );
        assert_eq!(
            before_root,
            snapshot_names(root.path()),
            "a `..` target must NOT touch the grandparent dir (force={force})"
        );
        assert!(
            work.path().join("a.txt").exists(),
            "the source file must survive the refused `..` rename (force={force})"
        );
    }
}

/// RENM-01 — directories and symlinks become `-` rows, and the replacement is
/// FIRST-match only (D-17): `2024_2024.log` with pattern `2024` -> `x` yields
/// `x_2024.log`, not `x_x.log`.
#[test]
fn renm_skips_and_first_match() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("2024_2024.log").write_str("log").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    // Dry-run preview (default): the dir is a `-` skip row, and the file's preview
    // shows ONLY the first `2024` replaced.
    bulk_rename(dir.path(), r"2024", "x", &[])
        .success()
        .stdout(predicate::str::contains("-> x_2024.log"))
        .stdout(predicate::str::contains("(skipped: directory)"));

    // Now apply it and confirm first-match-only on disk.
    bulk_rename(dir.path(), r"2024", "x", &["--force"]).success();
    assert!(dir.path().join("x_2024.log").exists());
    assert!(!dir.path().join("x_x.log").exists());
    // The directory was never renamed.
    assert!(dir.path().join("subdir").exists());
}

/// RENM-01 / WR-05 — under `--recursive`, two SIBLING subdirectories each holding
/// an `a.txt` are renamed to `b.txt` INDEPENDENTLY: the per-directory pre-flight
/// scopes each rename to its own parent, so neither directory's plan bleeds into
/// the other. This locks the per-directory collision scoping the safety argument
/// depends on (D-14) — a same-named target in a sibling dir is NOT a cross-dir
/// collision. Each `b.txt` keeps its own directory's original payload.
#[test]
fn renm_recursive_cross_directory_independent() {
    let dir = assert_fs::TempDir::new().unwrap();
    // Two sibling subdirs, each with an identically-named a.txt but DISTINCT
    // payloads so we can prove no cross-dir mix-up.
    dir.child("one/a.txt").write_str("ONE").unwrap();
    dir.child("two/a.txt").write_str("TWO").unwrap();

    // Rename a.txt -> b.txt everywhere (full base name `a.txt` -> `b.txt`).
    bulk_rename(
        dir.path(),
        r"^a\.txt$",
        "b.txt",
        &["--recursive", "--force"],
    )
    .success();

    let one = dir.path().join("one");
    let two = dir.path().join("two");

    // Each sibling's rename succeeded independently: a.txt is gone, b.txt is
    // present, and each b.txt carries ITS OWN directory's original payload (no
    // cross-directory bleed).
    assert!(
        !one.join("a.txt").exists() && one.join("b.txt").exists(),
        "one/a.txt must have been renamed to one/b.txt"
    );
    assert!(
        !two.join("a.txt").exists() && two.join("b.txt").exists(),
        "two/a.txt must have been renamed to two/b.txt"
    );
    assert_eq!(
        fs::read(one.join("b.txt")).unwrap(),
        b"ONE",
        "one/b.txt must keep one/'s original payload (no sibling bleed)"
    );
    assert_eq!(
        fs::read(two.join("b.txt")).unwrap(),
        b"TWO",
        "two/b.txt must keep two/'s original payload (no sibling bleed)"
    );
}

// --- Scriptable spine (SPINE-02, Wave-7b, D-12/A3) — from tests/uuid.rs:135 -----
//
// `box bulk-rename --json` carries a `dry_run` boolean and a
// `{results:[{src,dst,action,reason}], count, dry_run, to_rename, unchanged,
// skipped}` document. Two behavioral forks:
//   - D-12 override: `--force --json` EMITS the applied rename rows (the human
//     --force path is silent-on-success).
//   - A3 / D-09: a CONFLICTING plan under --json keeps stdout EMPTY, errors to
//     stderr, exits 1 — never an {"error":…} on stdout.

/// Capture `box bulk-rename <dir> <pattern> <replacement> [args]` raw stdout/stderr
/// + exit status, for the JSON assertions (raw bytes, not a trimmed String).
/// Forces `NO_COLOR=1`.
fn bulk_rename_output(
    dir: &Path,
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
    cmd.output().expect("run box bulk-rename")
}

/// SPINE-02 — `box bulk-rename <dir> <pat> <rep> --json` (dry-run default) emits
/// exactly one well-formed JSON document with `.results`, `.count`, `.dry_run`, no
/// ANSI, no BOM. Runnable via `cargo test --test bulk_rename json_purity`.
#[test]
fn json_purity() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let output = bulk_rename_output(dir.path(), r"IMG_(\d+)", "img_$1", &["--json"]);
    assert!(output.status.success(), "bulk-rename --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");

    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 2, "two rename rows planned");
    assert_eq!(v.get("count"), Some(&serde_json::json!(2)));
    assert!(
        v.get("dry_run").and_then(|d| d.as_bool()).is_some(),
        "`.dry_run` must be a boolean"
    );
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

/// SPINE-02 / D-13 — `box bulk-rename --json` (no --force) yields `.dry_run == true`
/// and a `.results` plan, writing nothing. Runnable via
/// `cargo test --test bulk_rename json_dry_run`.
#[test]
fn json_dry_run() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();

    let before = snapshot_names(dir.path());

    let output = bulk_rename_output(dir.path(), r"IMG_(\d+)", "img_$1", &["--json"]);
    assert!(output.status.success(), "dry-run --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(true)),
        "default (no --force) is a dry-run"
    );
    let results = v.get("results").and_then(|r| r.as_array()).unwrap();
    assert_eq!(results.len(), 1, "one rename row");
    assert_eq!(
        results[0].get("action").and_then(|a| a.as_str()),
        Some("rename")
    );

    // Dry-run wrote nothing.
    assert_eq!(before, snapshot_names(dir.path()), "dry-run must not write");
}

/// SPINE-02 / D-12 override — `box bulk-rename --force --json` EMITS the applied
/// rename rows (non-empty `.results`, `.dry_run == false`), even though the human
/// `--force` path is silent-on-success. Runnable via
/// `cargo test --test bulk_rename json_force_emits_rows`.
#[test]
fn json_force_emits_rows() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("IMG_0042.jpg").write_str("a").unwrap();
    dir.child("IMG_0043.jpg").write_str("b").unwrap();

    let output = bulk_rename_output(dir.path(), r"IMG_(\d+)", "img_$1", &["--force", "--json"]);
    assert!(output.status.success(), "--force --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("dry_run"),
        Some(&serde_json::json!(false)),
        "`--force` is a real run"
    );
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert!(
        !results.is_empty(),
        "--force --json must EMIT the applied rename rows (D-12 override), got empty results"
    );
    assert!(
        results
            .iter()
            .any(|r| r.get("action").and_then(|a| a.as_str()) == Some("rename")),
        "the applied renames are present: {v}"
    );

    // The renames really happened on disk.
    let names = listed_names(dir.path());
    assert!(
        names.iter().any(|n| n == "img_0042.jpg"),
        "the rename was applied, got {names:?}"
    );
}

/// SPINE-02 / A3 / D-09 — a CONFLICTING plan under `--json` keeps stdout
/// byte-EMPTY, sends the error to stderr, and exits 1 (no plan-with-conflicts and
/// no {"error":…} on stdout). Runnable via
/// `cargo test --test bulk_rename json_abort_empty_stdout`.
#[test]
fn json_abort_empty_stdout() {
    for force in [false, true] {
        let dir = assert_fs::TempDir::new().unwrap();
        // Both files collapse to `dup.txt` -> a collision aborts the batch.
        dir.child("a1.txt").write_str("aaa").unwrap();
        dir.child("b1.txt").write_str("bbb").unwrap();

        let before = snapshot_names(dir.path());

        let mut args: Vec<&str> = vec!["--json"];
        if force {
            args.push("--force");
        }
        let output = bulk_rename_output(dir.path(), r".+", "dup.txt", &args);

        // Exit 1 (runtime error).
        assert_eq!(
            output.status.code(),
            Some(1),
            "a conflicting plan must exit 1 (force={force})"
        );
        // D-09: stdout must be byte-EMPTY (no plan, no {"error":…}).
        assert!(
            output.stdout.is_empty(),
            "stdout must be byte-empty on a --json abort (force={force}), got: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
        // The error explanation is on stderr.
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Aborted") || stderr.contains("collision"),
            "the conflict must be reported on stderr (force={force}), got: {stderr:?}"
        );

        // Nothing was written.
        assert_eq!(
            before,
            snapshot_names(dir.path()),
            "an aborted --json run must write nothing (force={force})"
        );
    }
}
