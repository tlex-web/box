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
    tree(root, &["--sizes"]).success().stdout(
        predicate::str::contains("alpha.txt")
            .and(predicate::str::contains("beta.txt"))
            .and(predicate::str::contains("deep.txt"))
            .and(predicate::str::contains("2.0 KB"))
            .and(predicate::str::contains("10 B")),
    );

    // --depth 1 caps the tree at the immediate children: `sub` (the directory) is
    // shown, but its child `deep.txt` (depth 2) is NOT.
    tree(root, &["--depth", "1"]).success().stdout(
        predicate::str::contains("alpha.txt").and(predicate::str::contains("deep.txt").not()),
    );

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

/// TREE-01 / WR-02 — pointing `box tree` at a FILE (not a directory) is a clear
/// error, not silent empty output. Previously a file argument walked to zero
/// children and printed `0 directories, 0 files` with exit 0.
#[test]
fn tree_file_argument_errors() {
    let fixture = build_fixture();
    let file = fixture.path().join("alpha.txt");

    tree(&file, &[])
        .failure()
        .stderr(predicate::str::contains("is not a directory"));
}

/// TREE-01 / WR-03 — a non-existent path yields a clear "no such directory: X"
/// message naming the path, not dunce's raw `(os error 3)`.
#[test]
fn tree_missing_path_friendly_error() {
    let fixture = build_fixture();
    let missing = fixture.path().join("does-not-exist");

    tree(&missing, &[])
        .failure()
        .stderr(predicate::str::contains("no such directory"))
        .stderr(predicate::str::contains("does-not-exist"));
}

/// TREE-01 / WR-04 — the degenerate `--depth 0` input is rejected at parse time
/// (exit 2, a clap usage error) rather than silently rendering only the root. A
/// valid `--depth 1` still works.
#[test]
fn tree_zero_depth_rejected() {
    let fixture = build_fixture();
    let root = fixture.path();

    tree(root, &["--depth", "0"])
        .failure()
        .code(2)
        .stderr(predicate::str::contains("0").and(predicate::str::contains("not in")));

    // The boundary value 1 is accepted (proving we only reject 0).
    tree(root, &["--depth", "1"]).success();
}

// --- Scriptable spine (SPINE-02, Wave-7b, A4) — copied from tests/uuid.rs:135 ---
//
// `box tree <dir> --json` is the ROOT-RULE EXCEPTION (D-17): NOT `{results,count}`
// but a recursive node `{name, type:"dir"|"file", size?, children:[]}`. A real
// `build_node` recursion (the A4 surprise) reuses the same read_children /
// sort_children helpers as the printer so JSON order matches human order.

/// Capture `box tree <path> [args]` raw stdout bytes + exit status, for the
/// JSON-purity assertions (raw bytes, not a trimmed String). Forces `NO_COLOR=1`.
fn tree_output(path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("tree").arg(path);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box tree")
}

/// SPINE-02 — `box tree <dir> --json` emits exactly one well-formed JSON document
/// that is a recursive object (NOT `{results,count}`): the root has
/// `.type == "dir"` and a `.children` array, with no ANSI and no BOM. Runnable via
/// `cargo test --test tree json_purity`.
#[test]
fn json_purity() {
    let fixture = build_fixture();
    let out = tree_output(fixture.path(), &["--json"]);
    assert!(out.status.success(), "box tree --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The D-17 recursive shape: the root is a `dir` node with `children`.
    assert_eq!(
        v.get("type"),
        Some(&serde_json::json!("dir")),
        "the root node `.type` must be \"dir\""
    );
    assert!(
        v.get("children").and_then(|c| c.as_array()).is_some(),
        "the root node must carry a `.children` array"
    );
    assert!(
        v.get("name").and_then(|n| n.as_str()).is_some(),
        "the root node carries a string `name`"
    );

    // 3. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 4. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-02 / D-17 — `box tree <nested-dir> --json` is a real recursive node tree:
/// the root is a `dir`, a nested FILE node has `.type == "file"` and a numeric
/// `.size`, and a directory node OMITS `.size`. Runnable via
/// `cargo test --test tree json_recursive_shape`.
#[test]
fn json_recursive_shape() {
    let fixture = build_fixture();
    let out = tree_output(fixture.path(), &["--json"]);
    assert!(out.status.success(), "box tree --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // Root is a directory and OMITS size (D-17: size for files only).
    assert_eq!(v.get("type"), Some(&serde_json::json!("dir")));
    assert!(
        v.get("size").is_none(),
        "a directory node must omit `.size` (D-17), got: {v}"
    );

    let children = v
        .get("children")
        .and_then(|c| c.as_array())
        .expect("root `.children` must be an array");

    // Find the `sub` directory child — it must be a `dir`, omit `.size`, and have
    // its own `.children` (containing deep.txt).
    let sub = children
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("sub"))
        .expect("a `sub` directory child");
    assert_eq!(
        sub.get("type"),
        Some(&serde_json::json!("dir")),
        "`sub` is a dir node"
    );
    assert!(
        sub.get("size").is_none(),
        "`sub` (a directory) must omit `.size`"
    );
    let sub_children = sub
        .get("children")
        .and_then(|c| c.as_array())
        .expect("`sub` must carry its own `.children`");

    // The nested file node `deep.txt` has type "file" and a numeric size (300).
    let deep = sub_children
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("deep.txt"))
        .expect("`deep.txt` nested file node");
    assert_eq!(
        deep.get("type"),
        Some(&serde_json::json!("file")),
        "`deep.txt` is a file node"
    );
    assert_eq!(
        deep.get("size").and_then(|s| s.as_u64()),
        Some(300),
        "`deep.txt` carries its numeric byte size (300)"
    );
    assert!(
        deep.get("children").and_then(|c| c.as_array()).map(|a| a.is_empty()) != Some(false),
        "a file node has no (or an empty) `.children`"
    );
}
