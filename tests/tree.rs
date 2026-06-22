//! Wave-0 integration tests for `box tree` (TREE-01).
//!
//! Black-box: each test builds a real source tree in a temp dir with KNOWN,
//! distinct file byte-sizes (so `--sizes` is deterministic), runs the compiled
//! `box tree` binary, and asserts on the rendered tree + the `N directories,
//! M files` summary. The three named tests map 1:1 to the VALIDATION TREE-01
//! rows. Every test forces `NO_COLOR=1` so ANSI never leaks into the assertions
//! (RESEARCH determinism rule).
//!
//! Fixture shape (built by [`build_fixture`]):
//! ```text
//! <root>/
//!   sub/            (directory)
//!     deep.txt      (300 bytes)
//!   alpha.txt       (10 bytes)
//!   beta.txt        (2048 bytes -> "2.0 KB")
//! ```
//! Directories sort before files (D-08), so the rendered order is
//! `sub/`, then `alpha.txt`, then `beta.txt`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box tree <path> [extra args]` with plain output forced and return the
/// assert handle. Mirrors `tests/flatten.rs`'s runner shape.
fn tree(path: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("tree").arg(path);
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Build the deterministic fixture tree (distinct, known file byte-sizes so the
/// `--sizes` column is reproducible across machines). Returns the temp dir guard
/// (kept alive by the caller) — dropping it deletes the tree.
fn build_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub").join("deep.txt"), vec![b'x'; 300]).unwrap();
    fs::write(root.join("alpha.txt"), vec![b'a'; 10]).unwrap();
    fs::write(root.join("beta.txt"), vec![b'b'; 2048]).unwrap();
    dir
}

/// TREE-01 — `--sizes` shows a per-FILE `human_size` column (and a BLANK size for
/// directories, D-10), and `--depth N` caps the DISPLAYED depth.
#[test]
fn tree_sizes_and_depth() {
    let fixture = build_fixture();
    let root = fixture.path();

    // With --sizes, each file shows its human_size; the 2048-byte file is "2.0 KB",
    // the 10-byte file is "10 B".
    tree(root, &["--sizes"])
        .success()
        .stdout(
            predicate::str::contains("alpha.txt")
                .and(predicate::str::contains("beta.txt"))
                .and(predicate::str::contains("deep.txt"))
                .and(predicate::str::contains("2.0 KB"))
                .and(predicate::str::contains("10 B")),
        );

    // --depth 1 caps the tree at the immediate children: `sub` (the directory) is
    // shown, but its child `deep.txt` (depth 2) is NOT.
    tree(root, &["--depth", "1"])
        .success()
        .stdout(predicate::str::contains("alpha.txt").and(predicate::str::contains("deep.txt").not()));

    // Without a depth cap, the nested file IS shown.
    tree(root, &[])
        .success()
        .stdout(predicate::str::contains("deep.txt"));
}

/// TREE-01 — a trailing `N directories, M files` summary line is present (GNU
/// `tree` convention). The fixture has 1 directory (`sub`) and 3 files.
#[test]
fn tree_count_summary() {
    let fixture = build_fixture();
    let root = fixture.path();

    tree(root, &[])
        .success()
        .stdout(predicate::str::contains("1 directories, 3 files"));
}

/// TREE-01 / D-10 — piping `box tree` (assert_cmd captures stdout via a pipe, so
/// it is not a TTY) yields plain text with NO ANSI escape sequence (`\x1b[`).
/// Mirrors `tests/cli.rs::piped_help_has_no_ansi`. Run WITHOUT `NO_COLOR` so the
/// only thing suppressing color is the non-TTY gate, proving the pipe path.
#[test]
fn tree_piped_no_ansi() {
    let fixture = build_fixture();
    let root = fixture.path();

    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("tree")
        .arg(root)
        .arg("--sizes")
        .output()
        .expect("run box tree");
    assert!(out.status.success(), "box tree should exit 0");
    // ESC `[` is the start of every CSI/SGR ANSI sequence.
    assert!(
        !out.stdout.windows(2).any(|w| w == b"\x1b["),
        "piped tree output must contain no ANSI escape (\\x1b[)"
    );
}
