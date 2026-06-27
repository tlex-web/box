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

/// DU-01 — `--depth N` caps how deep a directory's total is rolled up. With a
/// cap, big/'s ROW shows a smaller total; without one it shows the full recursive
/// sum. The cap bounds the descent — it changes the rolled-up totals as specified.
#[test]
fn du_depth_cap() {
    let fixture = build_fixture();
    let root = fixture.path();

    // --depth 1: big/'s recursive total rolls up only its depth-1 file a.bin
    // (3000) and EXCLUDES its depth-2 nested/b.bin (2000), so big/'s ROW shows
    // "2.9 KB  big/" (not "4.9 KB  big/"). We assert against the big/ ROW LINE so
    // the summary total (which legitimately sums the capped rows) doesn't muddy
    // the assertion.
    let out = du(root, &["--depth", "1"])
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    let big_row = text
        .lines()
        .find(|l| l.contains("big/"))
        .expect("a big/ row");
    assert!(
        big_row.contains("2.9 KB"),
        "--depth 1 must roll big/ up to only its depth-1 file (3000 -> 2.9 KB), got row: {big_row:?}"
    );
    assert!(
        !big_row.contains("4.9 KB"),
        "--depth 1 must EXCLUDE the depth-2 descendant from big/'s row, got row: {big_row:?}"
    );

    // Without a cap, big/'s row rolls up the full recursive total (5000 -> 4.9 KB).
    let out = du(root, &[]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    let big_row = text
        .lines()
        .find(|l| l.contains("big/"))
        .expect("a big/ row");
    assert!(
        big_row.contains("4.9 KB"),
        "uncapped, big/'s row must show the full recursive total (5000 -> 4.9 KB), got row: {big_row:?}"
    );
}

/// DU-01 / WR-02 — pointing `box du` at a FILE (not a directory) is a clear error,
/// not silent empty output. Previously a file argument printed `0 of 0 entries
/// shown. 0 B total.` with exit 0.
#[test]
fn du_file_argument_errors() {
    let fixture = build_fixture();
    let file = fixture.path().join("small.txt");

    du(&file, &[])
        .failure()
        .stderr(predicate::str::contains("is not a directory"));
}

/// DU-01 / WR-03 — a non-existent path yields a clear "no such directory: X"
/// message naming the path, not dunce's raw `(os error 3)`.
#[test]
fn du_missing_path_friendly_error() {
    let fixture = build_fixture();
    let missing = fixture.path().join("does-not-exist");

    du(&missing, &[])
        .failure()
        .stderr(predicate::str::contains("no such directory"))
        .stderr(predicate::str::contains("does-not-exist"));
}

/// DU-01 / WR-04 — the degenerate `--depth 0` and `--top 0` inputs are rejected at
/// parse time (exit 2, a clap usage error) rather than silently producing
/// confusing empty/degenerate output. A valid `--depth 1` / `--top 1` still works.
#[test]
fn du_zero_depth_and_top_rejected() {
    let fixture = build_fixture();
    let root = fixture.path();

    // `--depth 0` is rejected by clap's range(1..) parser → exit 2.
    du(root, &["--depth", "0"])
        .failure()
        .code(2)
        .stderr(predicate::str::contains("0").and(predicate::str::contains("not in")));
    // `--top 0` is likewise rejected → exit 2.
    du(root, &["--top", "0"]).failure().code(2);

    // The boundary value 1 is accepted (proving we only reject 0, not all values).
    du(root, &["--depth", "1"]).success();
    du(root, &["--top", "1"]).success();
}

// --- Scriptable spine (SPINE-02, Wave-7b) — copied from tests/uuid.rs:135 -------
//
// `box du <dir> --json` emits ONE `{results,count,total_bytes,total_children}`
// document (D-11): each `.results` row is `{name, is_dir, size}` with a BARE u64
// `size` (D-3), all human chrome (the blank line + summary) suppressed. Purity:
// one JSON value, no 0x1B ANSI, no UTF-8 BOM.

/// Capture `box du <path> [args]` raw stdout bytes + exit status, for the
/// JSON-purity assertions (raw bytes, not a trimmed String). Forces `NO_COLOR=1`.
fn du_output(path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("du").arg(path);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box du")
}

/// SPINE-02 — `box du <dir> --json` emits exactly one well-formed JSON document
/// `{"results":[{"name":…,"is_dir":…,"size":N}],"count":N,"total_bytes":N,
/// "total_children":N}`: `.results` is an array, `.count` matches, every row
/// carries a numeric `size`, with no ANSI and no BOM. Runnable via
/// `cargo test --test du json_purity`.
#[test]
fn json_purity() {
    let fixture = build_fixture();
    let out = du_output(fixture.path(), &["--json"]);
    assert!(out.status.success(), "box du --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The D-11 shape: an object wrapping a `results` array + a `count`.
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    // The fixture has 3 immediate children (big/, mid/, small.txt).
    assert_eq!(results.len(), 3, "fixture has 3 immediate children");
    assert_eq!(
        v.get("count"),
        Some(&serde_json::json!(3)),
        "`.count` must match the results length"
    );

    // 3. Every row carries a NUMERIC `size` (bare u64, D-3) + a string `name`.
    for row in results {
        assert!(
            row.get("size").and_then(|s| s.as_u64()).is_some(),
            "every row carries a numeric `size`: {row}"
        );
        assert!(
            row.get("name").and_then(|n| n.as_str()).is_some(),
            "every row carries a string `name`: {row}"
        );
        assert!(
            row.get("is_dir").and_then(|d| d.as_bool()).is_some(),
            "every row carries a boolean `is_dir`: {row}"
        );
    }
    // The full-scan sibling totals are present and numeric.
    assert!(
        v.get("total_bytes").and_then(|t| t.as_u64()).is_some(),
        "`.total_bytes` must be a number"
    );
    assert_eq!(
        v.get("total_children"),
        Some(&serde_json::json!(3)),
        "`.total_children` must be the full child count"
    );

    // 4. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 5. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

// --- Wave-0 (DU-V2-01 / DU-V2-02) depth flags: percentage column + band color,
// --exclude <glob>, and --on-disk (Win32 GetCompressedFileSizeW). RED until plan
// 08-02 Task 3 lands the implementation. -------------------------------------

/// DU-V2-01 — every shown row carries a right-aligned `NN.N%` column BEFORE the
/// size; the basis is the FULL-SCAN total (so percentages stay honest after
/// `--top`); an empty/zero-byte row renders `0.0%`, never `NaN`.
#[test]
fn percentage_column() {
    let fixture = build_fixture();
    let root = fixture.path();

    // big/ = 5000/7000 = 71.4%, mid/ = 1500/7000 = 21.4%, small.txt = 500/7000 = 7.1%.
    du(root, &[]).success().stdout(
        predicate::str::contains("71.4%")
            .and(predicate::str::contains("21.4%"))
            .and(predicate::str::contains("7.1%")),
    );

    // --top 1 shows only big/, but the percentage basis is STILL the full-scan
    // total (7000) — so big/ reads 71.4%, NOT 100.0% (which would mean the basis
    // wrongly switched to the shown rows).
    let out = du(root, &["--top", "1"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    assert!(
        text.contains("71.4%"),
        "the percentage basis must stay the FULL-scan total after --top:\n{text}"
    );
    assert!(
        !text.contains("100.0%"),
        "--top must NOT rebase the percentage onto the shown rows:\n{text}"
    );

    // NaN guard: a directory whose only child is a 0-byte file → total == 0 → the
    // row renders 0.0%, never NaN.
    let zero = tempfile::tempdir().unwrap();
    fs::write(zero.path().join("zero.txt"), b"").unwrap();
    let out = du(zero.path(), &[]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    assert!(text.contains("0.0%"), "a zero-byte total must render 0.0%:\n{text}");
    assert!(!text.contains("NaN"), "a zero-byte total must NOT render NaN:\n{text}");
}

/// Build a fixture for `--exclude`: an immediate `*.log` child AND a descendant
/// `*.log` inside a subdirectory, so the exclude must drop the row AND keep the
/// descendant out of the dir total.
///
/// ```text
/// <root>/
///   keep.txt   (1000 bytes)
///   drop.log   (2000 bytes — immediate child, excluded → no row)
///   data/
///     a.bin    (500 bytes)
///     b.log    (3000 bytes — descendant, excluded → not in data/ total)
/// ```
fn build_exclude_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("keep.txt"), vec![b'k'; 1000]).unwrap();
    fs::write(root.join("drop.log"), vec![b'd'; 2000]).unwrap();
    fs::create_dir(root.join("data")).unwrap();
    fs::write(root.join("data").join("a.bin"), vec![b'a'; 500]).unwrap();
    fs::write(root.join("data").join("b.log"), vec![b'b'; 3000]).unwrap();
    dir
}

/// DU-V2-01 — `--exclude '<glob>'` drops a matching IMMEDIATE child (no row) AND
/// excludes a matching DESCENDANT from a directory's rolled-up total.
#[test]
fn exclude_glob() {
    let fixture = build_exclude_fixture();
    let root = fixture.path();

    // WITHOUT exclude: drop.log is a row and data/ = a.bin (500) + b.log (3000) = 3500 -> 3.4 KB.
    let out = du(root, &[]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    assert!(text.contains("drop.log"), "baseline must show drop.log:\n{text}");
    let data_row = text.lines().find(|l| l.contains("data")).expect("a data/ row");
    assert!(
        data_row.contains("3.4 KB"),
        "baseline data/ total must include b.log (3500 -> 3.4 KB), got: {data_row:?}"
    );

    // WITH --exclude '*.log': drop.log row gone; data/ total drops b.log -> 500 B.
    let out = du(root, &["--exclude", "*.log"]).success().get_output().stdout.clone();
    let text = String::from_utf8(out).expect("du output is utf-8");
    assert!(!text.contains("drop.log"), "--exclude must drop the drop.log row:\n{text}");
    assert!(text.contains("keep.txt"), "--exclude must keep non-matching children:\n{text}");
    let data_row = text.lines().find(|l| l.contains("data")).expect("a data/ row");
    assert!(
        data_row.contains("500 B"),
        "--exclude must keep b.log out of data/'s total (500 B), got: {data_row:?}"
    );
    assert!(
        !data_row.contains("3.4 KB"),
        "--exclude must NOT count the excluded descendant in data/'s total, got: {data_row:?}"
    );
}

/// DU-V2-01 / DU-V2-02 — `box du --json` and `box du --on-disk --json` each emit
/// one parseable value with no ANSI; the `on_disk` marker is `false` by default
/// and `true` under `--on-disk`.
#[test]
fn json_no_ansi() {
    let fixture = build_fixture();
    let root = fixture.path();

    // Plain --json: parseable, no ANSI, on_disk == false.
    let out = du_output(root, &["--json"]);
    assert!(out.status.success(), "box du --json should exit 0");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --json stdout");
    assert_eq!(
        v.get("on_disk"),
        Some(&serde_json::json!(false)),
        "plain --json must carry on_disk:false"
    );

    // --on-disk --json: parseable, no ANSI, on_disk == true.
    let out = du_output(root, &["--on-disk", "--json"]);
    assert!(out.status.success(), "box du --on-disk --json should exit 0");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI escape in --on-disk --json stdout");
    assert_eq!(
        v.get("on_disk"),
        Some(&serde_json::json!(true)),
        "--on-disk --json must carry on_disk:true"
    );
}

/// DU-V2-02 — `box du --on-disk` reports the allocated/compressed NTFS size via
/// Win32 `GetCompressedFileSizeW`, with the `on_disk:true` marker in JSON. Uses a
/// 5000-byte (non-MFT-resident) file so the allocated size is a positive number.
#[test]
fn on_disk_size() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // 5000 bytes is large enough to be non-resident (a small <~700-byte file can be
    // stored in the MFT record, where GetCompressedFileSizeW reports 0).
    fs::write(root.join("big.bin"), vec![b'z'; 5000]).unwrap();

    // Apparent size from plain --json.
    let out = du_output(root, &["--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let apparent = row_size(&v, "big.bin").expect("big.bin apparent row");
    assert_eq!(apparent, 5000, "apparent size is the logical 5000 bytes");

    // On-disk size + marker from --on-disk --json.
    let out = du_output(root, &["--on-disk", "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v.get("on_disk"),
        Some(&serde_json::json!(true)),
        "--on-disk --json must carry on_disk:true"
    );
    let on_disk = row_size(&v, "big.bin").expect("big.bin on-disk row");
    assert!(
        on_disk > 0,
        "GetCompressedFileSizeW must report a positive allocated size for a 5000-byte file, got {on_disk}"
    );
}

/// Find a `box du --json` row's `size` by its `name`.
fn row_size(value: &serde_json::Value, name: &str) -> Option<u64> {
    value
        .get("results")?
        .as_array()?
        .iter()
        .find(|r| r.get("name").and_then(|n| n.as_str()) == Some(name))
        .and_then(|r| r.get("size"))
        .and_then(|s| s.as_u64())
}
