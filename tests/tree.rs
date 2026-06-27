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

// --- Wave-0 (TREE-V2-01) depth flags: --gitignore / --ignore / --dirs-only /
// --sort size. These are RED until plan 08-02 Task 2 lands the matcher-as-filter
// inside read_children. -------------------------------------------------------

/// Build a 3-LEVEL-deep fixture with a ROOT `.gitignore` (ignoring `target/` and
/// `*.log`) AND a nested `src/.gitignore` (ignoring `*.tmp` but WHITELISTING
/// `keep.log`). This exercises the eza #1086 nested-gitignore class: a deeper rule
/// must win over an ancestor (the `!keep.log` whitelist re-shows a file the root
/// `*.log` would hide; the nested `*.tmp` hides files at and below `src/`).
///
/// ```text
/// <root>/
///   .gitignore          "target/\n*.log\n"
///   app.log             (hidden by root *.log)
///   target/             (hidden by root target/)
///     junk.o
///   src/
///     .gitignore        "*.tmp\n!keep.log\n"
///     main.rs           (shown)
///     scratch.tmp       (hidden by nested *.tmp)
///     keep.log          (SHOWN — nested !keep.log whitelists over root *.log)
///     deep/
///       mod.rs          (shown)
///       cache.tmp       (hidden by ancestor src *.tmp at depth 2)
/// ```
fn build_gitignore_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::write(root.join(".gitignore"), "target/\n*.log\n").unwrap();
    fs::write(root.join("app.log"), b"log").unwrap();

    fs::create_dir(root.join("target")).unwrap();
    fs::write(root.join("target").join("junk.o"), b"obj").unwrap();

    let src = root.join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join(".gitignore"), "*.tmp\n!keep.log\n").unwrap();
    fs::write(src.join("main.rs"), b"fn main() {}").unwrap();
    fs::write(src.join("scratch.tmp"), b"tmp").unwrap();
    fs::write(src.join("keep.log"), b"keep").unwrap();

    let deep = src.join("deep");
    fs::create_dir(&deep).unwrap();
    fs::write(deep.join("mod.rs"), b"mod x;").unwrap();
    fs::write(deep.join("cache.tmp"), b"cache").unwrap();

    dir
}

/// TREE-V2-01 — `--gitignore` honors the ROOT `.gitignore` AND every NESTED
/// `.gitignore` from the tree root down (a deeper rule wins). Default (no flag)
/// output is unchanged, proving the feature is opt-in.
#[test]
fn gitignore_nested() {
    let fixture = build_gitignore_fixture();
    let root = fixture.path();

    // Opt-in proof: WITHOUT --gitignore, the ignored entries are still shown.
    let out = tree(root, &[]).success().get_output().stdout.clone();
    let plain = String::from_utf8(out).expect("tree output is utf-8");
    assert!(
        plain.contains("app.log") && plain.contains("target") && plain.contains("scratch.tmp"),
        "default (no --gitignore) output must be UNCHANGED — ignored entries still shown:\n{plain}"
    );

    // WITH --gitignore, root + nested rules both apply, deeper wins.
    let out = tree(root, &["--gitignore"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("tree output is utf-8");

    // Hidden by the ROOT .gitignore.
    assert!(!text.contains("target"), "root `target/` must be hidden:\n{text}");
    assert!(!text.contains("junk.o"), "target/junk.o must be hidden with its parent:\n{text}");
    assert!(!text.contains("app.log"), "root `*.log` must hide app.log:\n{text}");
    // Hidden by the NESTED src/.gitignore (`*.tmp`), at depth 1 and depth 2.
    assert!(!text.contains("scratch.tmp"), "nested `*.tmp` must hide src/scratch.tmp:\n{text}");
    assert!(!text.contains("cache.tmp"), "nested `*.tmp` must hide src/deep/cache.tmp (depth 2):\n{text}");
    // Shown — not ignored.
    assert!(text.contains("main.rs"), "src/main.rs must be shown:\n{text}");
    assert!(text.contains("mod.rs"), "src/deep/mod.rs must be shown:\n{text}");
    assert!(text.contains("src"), "the src directory must be shown:\n{text}");
    // The DEEPER-WINS case: nested `!keep.log` whitelists over the root `*.log`.
    assert!(
        text.contains("keep.log"),
        "src/keep.log must be SHOWN — a nested `!keep.log` whitelist overrides the root `*.log` (eza #1086 class):\n{text}"
    );
}

/// TREE-V2-01 — `--gitignore --json` produces the SAME omissions as the human
/// render (no-drift): the recursive node tree must not contain any ignored name,
/// must keep every shown name, and stays one parseable document with no ANSI.
#[test]
fn gitignore_json_nodrift() {
    let fixture = build_gitignore_fixture();
    let out = tree_output(fixture.path(), &["--gitignore", "--json"]);
    assert!(out.status.success(), "box tree --gitignore --json should exit 0");

    // Exactly one JSON value, no ANSI.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --json stdout");

    // Collect every node name recursively.
    let mut names = Vec::new();
    collect_names(&v, &mut names);

    // The ignored names must be ABSENT from the JSON node tree (no-drift).
    for hidden in ["target", "junk.o", "app.log", "scratch.tmp", "cache.tmp"] {
        assert!(
            !names.iter().any(|n| n == hidden),
            "ignored `{hidden}` must NOT appear in --gitignore --json node tree; names={names:?}"
        );
    }
    // The shown names must be PRESENT (incl. the deeper-wins whitelist).
    for shown in ["main.rs", "mod.rs", "keep.log"] {
        assert!(
            names.iter().any(|n| n == shown),
            "shown `{shown}` must appear in --gitignore --json node tree; names={names:?}"
        );
    }
}

/// Recursively push every `name` field of a `box tree --json` node tree into `out`.
fn collect_names(value: &serde_json::Value, out: &mut Vec<String>) {
    if let Some(name) = value.get("name").and_then(|n| n.as_str()) {
        out.push(name.to_string());
    }
    if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
        for child in children {
            collect_names(child, out);
        }
    }
}

/// Build a flat fixture for the `--dirs-only` / `--ignore` / `--sort size` checks:
/// two directories and three files with DISTINCT sizes.
///
/// ```text
/// <root>/
///   adir/
///   bdir/
///   big.txt    (3000 bytes)
///   app.log    (500 bytes)
///   small.txt  (100 bytes)
/// ```
fn build_flag_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("adir")).unwrap();
    fs::create_dir(root.join("bdir")).unwrap();
    fs::write(root.join("big.txt"), vec![b'x'; 3000]).unwrap();
    fs::write(root.join("app.log"), vec![b'l'; 500]).unwrap();
    fs::write(root.join("small.txt"), vec![b's'; 100]).unwrap();
    dir
}

/// Find the byte offset where `needle` first appears in `haystack`, or panic.
fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("expected {needle:?} in output:\n{haystack}"))
}

/// TREE-V2-01 — `--dirs-only` drops file children; `--ignore '<glob>'` folds into
/// the same matcher; `--sort size` orders files biggest-first.
#[test]
fn dirs_only_ignore_sort() {
    let fixture = build_flag_fixture();
    let root = fixture.path();

    // --dirs-only: only the directories are shown, no files.
    let out = tree(root, &["--dirs-only"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("tree output is utf-8");
    assert!(text.contains("adir") && text.contains("bdir"), "dirs must be shown:\n{text}");
    assert!(
        !text.contains("big.txt") && !text.contains("small.txt") && !text.contains("app.log"),
        "--dirs-only must drop ALL file children:\n{text}"
    );

    // --ignore '*.log': app.log is hidden; other files + dirs remain.
    let out = tree(root, &["--ignore", "*.log"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("tree output is utf-8");
    assert!(!text.contains("app.log"), "--ignore '*.log' must hide app.log:\n{text}");
    assert!(
        text.contains("big.txt") && text.contains("small.txt") && text.contains("adir"),
        "--ignore must leave non-matching entries:\n{text}"
    );

    // --sort size: files ordered biggest-first (big.txt 3000 > app.log 500 > small.txt 100).
    let out = tree(root, &["--sort", "size"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("tree output is utf-8");
    let big = index_of(&text, "big.txt");
    let app = index_of(&text, "app.log");
    let small = index_of(&text, "small.txt");
    assert!(
        big < app && app < small,
        "--sort size must order files biggest-first (big.txt < app.log < small.txt):\n{text}"
    );
}

/// TREE-V2-01 — with NO new flags, the default `box tree` render is byte-identical
/// to v1 (the trycmd `tests/cmd/tree.trycmd` pin guards the canonical shape; this
/// guards the glyphs over the standard fixture). Proves the depth flags are opt-in.
#[test]
fn default_output_unchanged() {
    let fixture = build_fixture();
    let root = fixture.path();

    let out = tree(root, &[]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("tree output is utf-8");

    // The exact v1 box-drawing shape (dirs-first then alpha): sub/ then its child,
    // then the two top-level files, then the summary.
    assert!(text.contains("├── sub"), "default render unchanged (sub branch):\n{text}");
    assert!(text.contains("│   └── deep.txt"), "default render unchanged (nested file):\n{text}");
    assert!(text.contains("├── alpha.txt"), "default render unchanged (alpha):\n{text}");
    assert!(text.contains("└── beta.txt"), "default render unchanged (beta, last):\n{text}");
    assert!(text.contains("1 directories, 3 files"), "default summary unchanged:\n{text}");
}
