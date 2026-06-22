//! Wave-0 integration tests for `box du` (DU-01).
//!
//! Black-box: each test builds a real directory tree in a temp dir with KNOWN,
//! DISTINCT per-immediate-child byte totals (so the `(size desc, name asc)` sort
//! key is a TOTAL order and the rendered output is deterministic), runs the
//! compiled `box du` binary, and asserts on the biggest-first rows, the
//! trailing-`/`-on-dirs convention, the `--top` post-sort truncation, the
//! `--depth` aggregation cap, and the full-scan `{X} of {Y} entries shown.
//! {TOTAL} total.` summary. The three named tests map 1:1 to the VALIDATION
//! DU-01 rows. Every test forces `NO_COLOR=1` so ANSI never leaks into the
//! assertions (RESEARCH determinism rule).
//!
//! Fixture shape (built by [`build_fixture`]) — immediate children of <root>,
//! with DISTINCT recursive totals so the biggest-first order is unambiguous:
//! ```text
//! <root>/
//!   big/                      (directory — recursive total 5000 bytes)
//!     a.bin                   (3000 bytes)
//!     nested/
//!       b.bin                 (2000 bytes)
//!   mid/                      (directory — recursive total 1500 bytes)
//!     c.bin                   (1500 bytes)
//!   small.txt                 (file — 500 bytes)
//! ```
//! Biggest-first immediate-child order: `big/` (5000), `mid/` (1500),
//! `small.txt` (500). Full-scan total = 5000 + 1500 + 500 = 7000 bytes = `6.8 KB`
//! (7000 / 1024 = 6.835… -> "6.8 KB"). 3 immediate children total.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box du <path> [extra args]` with plain output forced and return the
/// assert handle. Mirrors `tests/tree.rs`'s runner shape.
fn du(path: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("du").arg(path);
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state.
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Build the deterministic fixture tree with DISTINCT immediate-child totals so
/// the `(size desc, name asc)` sort is a total order. Returns the temp dir guard
/// (kept alive by the caller) — dropping it deletes the tree.
fn build_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // big/ -> 3000 + 2000 = 5000 bytes recursive total (the biggest child).
    fs::create_dir(root.join("big")).unwrap();
    fs::write(root.join("big").join("a.bin"), vec![b'a'; 3000]).unwrap();
    fs::create_dir(root.join("big").join("nested")).unwrap();
    fs::write(
        root.join("big").join("nested").join("b.bin"),
        vec![b'b'; 2000],
    )
    .unwrap();

    // mid/ -> 1500 bytes recursive total.
    fs::create_dir(root.join("mid")).unwrap();
    fs::write(root.join("mid").join("c.bin"), vec![b'c'; 1500]).unwrap();

    // small.txt -> 500 bytes (a plain file, its own size, the smallest child).
    fs::write(root.join("small.txt"), vec![b's'; 500]).unwrap();

    dir
}

/// Find the byte offset where `needle` first appears in `haystack`, or panic.
fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("expected {needle:?} in output:\n{haystack}"))
}

/// DU-01 — rows are emitted BIGGEST-FIRST (directories show their recursive
/// total, files their own size), directory rows carry a trailing `/` and file
/// rows do not.
#[test]
fn du_biggest_first() {
    let fixture = build_fixture();
    let root = fixture.path();

    let out = du(root, &[]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");

    // Directories carry a trailing `/`; the plain file does not.
    assert!(
        text.contains("big/"),
        "directory rows must carry a trailing `/`:\n{text}"
    );
    assert!(
        text.contains("mid/"),
        "directory rows must carry a trailing `/`:\n{text}"
    );
    assert!(
        text.contains("small.txt"),
        "the file row must be present:\n{text}"
    );
    assert!(
        !text.contains("small.txt/"),
        "a file row must NOT carry a trailing `/`:\n{text}"
    );

    // Biggest-first order: big/ (5000) before mid/ (1500) before small.txt (500).
    let big = index_of(&text, "big/");
    let mid = index_of(&text, "mid/");
    let small = index_of(&text, "small.txt");
    assert!(
        big < mid && mid < small,
        "rows must be biggest-first (big/ < mid/ < small.txt):\n{text}"
    );

    // The directory total is the RECURSIVE sum (big/ = 3000 + 2000 = 5000 ->
    // 4.9 KB), not just its top-level file.
    assert!(
        text.contains("4.9 KB"),
        "big/ must show its recursive total (5000 bytes -> 4.9 KB):\n{text}"
    );
}

/// DU-01 — `--top N` truncates the SHOWN rows to N (post-sort), while the summary
/// `{X} of {Y} entries shown. {TOTAL} total.` reports the FULL-scan total (not
/// just the shown rows).
#[test]
fn du_top_and_total_summary() {
    let fixture = build_fixture();
    let root = fixture.path();

    // Full run (no --top): all 3 children shown; summary reflects the full scan.
    du(root, &[]).success().stdout(
        predicate::str::contains("3 of 3 entries shown.")
            .and(predicate::str::contains("6.8 KB total.")),
    );

    // --top 1 shows ONLY the single biggest child (big/), drops mid/ and
    // small.txt from the rows, but the summary STILL reports the full-scan total
    // (6.8 KB over all 3 children), and "1 of 3 entries shown.".
    let out = du(root, &["--top", "1"])
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    assert!(
        text.contains("big/"),
        "--top 1 must show the biggest child big/:\n{text}"
    );
    assert!(
        !text.contains("mid/") && !text.contains("small.txt"),
        "--top 1 must truncate the smaller rows mid/ and small.txt:\n{text}"
    );
    assert!(
        text.contains("1 of 3 entries shown."),
        "--top 1 must report 1 of 3 entries shown:\n{text}"
    );
    assert!(
        text.contains("6.8 KB total."),
        "the summary total must reflect the FULL scan (6.8 KB), not just the shown row:\n{text}"
    );
}

/// DU-01 — `--depth N` caps how deep totals are rolled up. The immediate-child
/// rows and the full-scan total are unchanged by the depth cap (every byte is
/// still summed into its top-level child); the cap bounds the descent.
#[test]
fn du_depth_cap() {
    let fixture = build_fixture();
    let root = fixture.path();

    // --depth 1: totals roll up no deeper than the immediate children. big/'s
    // recursive total still includes its depth-1 file a.bin (3000) but NOT its
    // depth-2 nested/b.bin (2000) -> big/ shows 3000 bytes = "2.9 KB".
    du(root, &["--depth", "1"]).success().stdout(
        predicate::str::contains("big/")
            .and(predicate::str::contains("2.9 KB"))
            .and(predicate::str::contains("4.9 KB").not()),
    );

    // Without a cap, big/ rolls up the full recursive total (5000 -> 4.9 KB).
    du(root, &[])
        .success()
        .stdout(predicate::str::contains("4.9 KB"));
}
