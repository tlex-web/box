//! Integration tests for `box qr` (QR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. These tests pin the
//! QR-01 byte-level contract (D-01/D-02/D-03) — the camera-readability of the
//! rendered QR is a physical/perceptual property covered by the human-verify
//! checkpoint (phone scan), not here. What IS auto-testable:
//!   - input as a positional arg renders a half-block QR (exit 0, non-empty);
//!   - the SAME input via piped stdin renders identically (the `-`/no-arg path);
//!   - the output carries NO ANSI even WITHOUT `NO_COLOR` — `qr` must NEVER emit
//!     ANSI (the whole reason qr2term was rejected, D-01). Mirrors the run-
//!     WITHOUT-`NO_COLOR` pipe-path proof from `tests/json.rs::json_piped_no_ansi`.

use assert_cmd::Command;
use assert_fs::prelude::*;

/// True if `s` contains at least one Unicode half-block glyph (`▀`/`▄`/`█`) —
/// the proof that the Dense1x2 renderer produced real QR output, not an empty or
/// ANSI-only stream.
fn has_half_block(s: &str) -> bool {
    s.contains('▀') || s.contains('▄') || s.contains('█')
}

/// (1) D-01/D-02 — input as a positional arg renders a half-block QR: `box qr
/// "https://example.com"` exits 0 with non-empty stdout containing a half-block
/// glyph and no ANSI.
#[test]
fn arg_renders_half_block_qr() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr <arg> should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(!stdout.is_empty(), "rendered QR must be non-empty");
    assert!(
        has_half_block(&stdout),
        "output must contain a half-block glyph (▀/▄/█), got: {stdout:?}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "output must contain no ANSI escape: {stdout:?}"
    );
}

/// (2) D-03 — the SAME input via piped stdin (no arg) also renders a half-block
/// QR. Proves the `core::input` arg → stdin precedence is wired for `qr`.
#[test]
fn piped_stdin_renders_half_block_qr() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .env("NO_COLOR", "1")
        .write_stdin("https://example.com")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr (piped) should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        has_half_block(&stdout),
        "piped output must contain a half-block glyph (▀/▄/█), got: {stdout:?}"
    );
}

/// (3) D-01 — run WITHOUT `NO_COLOR` but with piped stdout (assert_cmd captures
/// stdout via a pipe, so it is not a TTY): the output must contain no ANSI escape
/// (`\x1b[`). `qr` must NEVER emit ANSI — the exact reason qr2term was rejected.
/// Mirrors `json_piped_no_ansi` (json.rs:109-123).
#[test]
fn qr_piped_no_ansi() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .write_stdin("https://example.com")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr should exit 0");
    // ESC `[` is the start of every CSI/SGR ANSI sequence.
    assert!(
        !out.stdout.windows(2).any(|w| w == b"\x1b["),
        "piped qr output must contain no ANSI escape (\\x1b[)"
    );
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — D-14 metadata + D-15 clip-text ---
//
// qr is a Wave-7c odd-fit: under --json it emits ENCODE METADATA, not the glyph
// block (D-14: `{text, error_correction}`); under --clip it copies the SOURCE
// TEXT, not the rendered half-blocks (D-15 — the one deliberate break from the
// copy-all rule, since pasting ▀▄ glyphs as clipboard text is garbage).
//   - `json_metadata_not_glyphs` — --json yields {text, error_correction:"M"},
//                                  NO glyphs in the document (D-14).
//   - `json_purity`              — one JSON value, the QrOutput shape, no 0x1B/BOM.
//   - `clip_copies_source_text`  — #[ignore]d: pasted == INPUT text, not glyphs (D-15).

/// Capture `box qr <args>` raw stdout bytes + exit status for the JSON-purity
/// assertions (which inspect raw bytes for ANSI/BOM). Forces `NO_COLOR=1`.
fn qr_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("qr");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box qr")
}

/// D-14 — `box qr "https://example.com" --json` emits encode METADATA, NOT the
/// glyph block: `{text, error_correction:"M"}`. The half-block glyphs (▀/▄/█) are
/// a visual, never a scriptable value, so the JSON document must carry NONE of
/// them. Runnable via `cargo test --test qr json_metadata_not_glyphs`.
#[test]
fn json_metadata_not_glyphs() {
    let out = qr_output(&["https://example.com", "--json"]);
    assert!(out.status.success(), "box qr --json should exit 0");

    // stdout is exactly one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // D-14 metadata: text == the input, error_correction == "M".
    assert_eq!(
        v.get("text"),
        Some(&serde_json::json!("https://example.com")),
        "`.text` must echo the encoded input"
    );
    assert_eq!(
        v.get("error_correction"),
        Some(&serde_json::json!("M")),
        "`.error_correction` must be the fixed v1 level \"M\""
    );

    // The document carries NO half-block glyph anywhere (metadata, not the render).
    let stdout = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    assert!(
        !has_half_block(&stdout),
        "the --json document must NOT contain glyphs (▀/▄/█): {stdout:?}"
    );
}

/// SPINE-02 — `box qr <text> --json` emits EXACTLY one well-formed JSON document
/// `{"text":…,"error_correction":"M"}`: no UTF-8 BOM, no ANSI escape, no human
/// chrome. The QrOutput-shaped variant of the frozen `json_purity` template.
/// Runnable via `cargo test --test qr json_purity`.
#[test]
fn json_purity() {
    let out = qr_output(&["https://example.com", "--json"]);
    assert!(out.status.success(), "box qr --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The flat QrOutput shape (scalar → flat object, D-01): a `text` string +
    //    an `error_correction` string. No `results`/`count` wrapper (qr is scalar).
    assert!(
        v.get("text").and_then(|t| t.as_str()).is_some(),
        "`.text` must be a string"
    );
    assert!(
        v.get("error_correction")
            .and_then(|e| e.as_str())
            .is_some(),
        "`.error_correction` must be a string"
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

// --- QR-V2-01 — --save PNG/SVG + --error-correction (Wave-2 depth) -----------
//
// The file write is the one new filesystem surface this phase. `assert_fs` gives a
// scratch TempDir so no test touches a real path. Coverage:
//   - `save_png_writes_magic`   — --save f.png writes a non-empty PNG (magic bytes),
//                                 stdout suppresses the glyph block.
//   - `save_svg_writes_marker`  — --save f.svg writes a non-empty SVG (<svg marker).
//   - `save_bad_ext_exits_1`    — an unknown extension exits 1 with a .png/.svg hint,
//                                 no file written, no panic.
//   - `save_json_orthogonal`    — --save f.png --json writes the file AND emits one
//                                 metadata doc carrying saved_path (Open-Q1).
//   - `ec_level_reflected_in_json` — --error-correction H --json → error_correction "H".

/// QR-V2-01 — `box qr <text> --save out.png` writes a non-empty PNG (first bytes
/// are the PNG magic) and SUPPRESSES the terminal glyph block on stdout (Pitfall 3).
#[test]
fn save_png_writes_magic_and_suppresses_glyphs() {
    let tmp = assert_fs::TempDir::new().expect("tempdir");
    let png = tmp.child("out.png");

    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .arg("--save")
        .arg(png.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr --save");
    assert!(out.status.success(), "box qr --save out.png should exit 0");

    // The file exists, is non-empty, and starts with the PNG magic bytes.
    png.assert(predicates::path::exists());
    let bytes = std::fs::read(png.path()).expect("read saved png");
    assert!(!bytes.is_empty(), "saved PNG must be non-empty");
    assert_eq!(
        &bytes[..4],
        &[0x89, b'P', b'N', b'G'],
        "saved file must be a PNG (magic bytes)"
    );

    // The glyph block is suppressed under --save (stdout carries no half-block).
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        !has_half_block(&stdout),
        "--save must suppress the terminal glyph block, got: {stdout:?}"
    );
    // The stderr confirmation names the written path.
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Saved QR to"),
        "--save must confirm the write on stderr, got: {stderr:?}"
    );

    tmp.close().unwrap();
}

/// QR-V2-01 — `box qr <text> --save out.svg` writes a non-empty SVG document.
#[test]
fn save_svg_writes_marker() {
    let tmp = assert_fs::TempDir::new().expect("tempdir");
    let svg = tmp.child("out.svg");

    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .arg("--save")
        .arg(svg.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr --save svg");
    assert!(out.status.success(), "box qr --save out.svg should exit 0");

    svg.assert(predicates::path::exists());
    let text = std::fs::read_to_string(svg.path()).expect("read saved svg");
    assert!(!text.is_empty(), "saved SVG must be non-empty");
    let head = text.trim_start();
    assert!(
        head.starts_with("<svg") || head.starts_with("<?xml"),
        "saved file must be an SVG document: {:?}",
        &head[..head.len().min(32)]
    );

    tmp.close().unwrap();
}

/// T-09-02-SAVE — an unsupported `--save` extension exits 1 with a `.png`/`.svg`
/// hint, writes no file, and never panics.
#[test]
fn save_bad_extension_exits_1() {
    let tmp = assert_fs::TempDir::new().expect("tempdir");
    let gif = tmp.child("out.gif");

    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .arg("--save")
        .arg(gif.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr --save gif");

    assert_eq!(
        out.status.code(),
        Some(1),
        "an unsupported --save extension must exit 1"
    );
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains(".png") && stderr.contains(".svg"),
        "the error must hint .png/.svg, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("panicked"),
        "a bad --save extension must not panic: {stderr:?}"
    );
    gif.assert(predicates::path::missing());

    tmp.close().unwrap();
}

/// QR-V2-01 / Open-Q1 — `--save` and `--json` are ORTHOGONAL: `box qr <text>
/// --save out.png --json` WRITES the file AND emits exactly one metadata document
/// carrying `saved_path` (plus the chosen `error_correction`).
#[test]
fn save_and_json_are_orthogonal() {
    let tmp = assert_fs::TempDir::new().expect("tempdir");
    let png = tmp.child("both.png");

    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .arg("--save")
        .arg(png.path())
        .arg("--json")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr --save --json");
    assert!(out.status.success(), "box qr --save --json should exit 0");

    // The file was written (the action happened despite --json being the mode).
    png.assert(predicates::path::exists());
    let bytes = std::fs::read(png.path()).expect("read saved png");
    assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G'], "the PNG must still be written");

    // stdout is exactly one JSON document carrying saved_path.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    let saved = v
        .get("saved_path")
        .and_then(|s| s.as_str())
        .expect("`.saved_path` must be a string under --save --json");
    assert!(
        saved.contains("both.png"),
        "`.saved_path` must name the written file, got {saved:?}"
    );
    // No half-block glyphs leak into the metadata document.
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        !has_half_block(&stdout),
        "the --json document must not contain glyphs: {stdout:?}"
    );

    tmp.close().unwrap();
}

/// QR-V2-01 — `--error-correction` is reflected in the JSON metadata for BOTH the
/// document (and, by construction, the render): `--error-correction H --json` →
/// `error_correction == "H"` (the v1 default of `"M"` is verified in json_purity).
#[test]
fn ec_level_reflected_in_json() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .arg("--error-correction")
        .arg("H")
        .arg("--json")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr --error-correction H --json");
    assert!(out.status.success(), "box qr --error-correction H --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("error_correction"),
        Some(&serde_json::json!("H")),
        "`.error_correction` must reflect the chosen level"
    );
    // saved_path is ABSENT without --save (skip_serializing_if).
    assert!(
        v.get("saved_path").is_none(),
        "`.saved_path` must be absent without --save"
    );
}

/// SPINE-04 / D-15 — live Windows-clipboard round-trip with the deliberate qr
/// exception: `box qr <text> --clip` prints the glyph block AND copies the SOURCE
/// TEXT (not the glyphs); reading the clipboard back equals the INPUT string.
/// `#[ignore]`d (touches shared OS clipboard). Run locally with:
///   cargo test --test qr -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_copies_source_text() {
    const INPUT: &str = "https://example.com";

    // `box qr <text> --clip` prints the glyphs AND tees the SOURCE TEXT to clip.
    let out = qr_output(&[INPUT, "--clip"]);
    assert!(out.status.success(), "box qr --clip should exit 0");
    let printed = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    // The human stdout is still the glyph block (display unchanged, D-15).
    assert!(
        has_half_block(&printed),
        "the human --clip path still prints the glyph block"
    );

    // Read the clipboard back — it must be the INPUT text, NOT the glyph block.
    let pasted = {
        let out = Command::cargo_bin("box")
            .unwrap()
            .args(["clip", "--paste"])
            .output()
            .expect("run box clip --paste");
        assert!(out.status.success(), "box clip --paste should exit 0");
        String::from_utf8(out.stdout)
            .expect("clipboard text is UTF-8")
            .trim()
            .to_string()
    };

    assert_eq!(
        pasted, INPUT,
        "--clip must copy the SOURCE TEXT (D-15), not the rendered glyph block"
    );
    assert!(
        !has_half_block(&pasted),
        "the clipboard must NOT contain glyphs (D-15)"
    );
}
