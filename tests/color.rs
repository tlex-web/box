//! Integration tests for `box color` (COLR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. The exact aligned
//! Hex/RGB/Tuple/HSL block (plus the plain `██████████` swatch under NO_COLOR)
//! is locked by the trycmd snapshot (`tests/cmd/color.trycmd`); these tests
//! cover the cross-cutting CLI contract: bidirectional input parses to the same
//! block, and malformed input exits 1 cleanly with no panic (T-02-06).

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box color <input>` with `NO_COLOR=1`, returning the assert handle.
fn color(input: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input);
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Capture `box color <input>` stdout (exit 0, stderr empty), NO_COLOR.
fn color_stdout(input: &str) -> String {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input).env("NO_COLOR", "1");
    let out = cmd.output().expect("run box color");
    assert!(out.status.success(), "box color {input:?} should exit 0");
    assert!(
        out.stderr.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// Hex input yields the locked block with the uppercase hex, CSS rgb(), tuple,
/// and HSL rows, and a plain `██████████` swatch under NO_COLOR.
#[test]
fn hex_input_prints_full_block() {
    let out = color_stdout("#3b82f6");
    assert!(out.contains("#3B82F6"), "hex row uppercase: {out:?}");
    assert!(out.contains("rgb(59, 130, 246)"), "rgb row: {out:?}");
    assert!(out.contains("59 130 246"), "tuple row: {out:?}");
    assert!(out.contains("hsl(217, 91%, 60%)"), "hsl row: {out:?}");
    assert!(out.contains("██████████"), "plain swatch present: {out:?}");
    // Under NO_COLOR the whole output is byte-identical minus ANSI: no escapes.
    assert!(!out.contains('\u{1b}'), "no ANSI under NO_COLOR: {out:?}");
}

/// RGB input (both comma- and space-separated) yields the identical block.
#[test]
fn rgb_inputs_match_hex_block() {
    let from_hex = color_stdout("#3b82f6");
    let from_comma = color_stdout("59,130,246");
    let from_space = color_stdout("59 130 246");
    assert_eq!(from_comma, from_hex, "comma RGB should match hex block");
    assert_eq!(from_space, from_hex, "space RGB should match hex block");
}

/// Short hex `#abc` expands to `(170,187,204)` → `#AABBCC` (Pitfall 8 / D-13).
#[test]
fn short_hex_expands() {
    let out = color_stdout("#abc");
    assert!(out.contains("#AABBCC"), "short-hex expand: {out:?}");
    assert!(out.contains("rgb(170, 187, 204)"), "short-hex rgb: {out:?}");
}

/// Malformed hex → exit 1, error on stderr, empty stdout, no panic (T-02-06).
#[test]
fn malformed_hex_exits_1_clean() {
    color("#zz")
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

/// Out-of-range RGB component → exit 1, clean stderr, no panic (T-02-06).
#[test]
fn out_of_range_rgb_exits_1_clean() {
    color("300,0,0")
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

// --- COLR-V2-01: CSS named colors (both directions) + HSL input ----------------

/// Capture `box color <input> --json` as a parsed JSON value (exit 0, NO_COLOR).
fn color_json(input: &str) -> serde_json::Value {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input).arg("--json").env("NO_COLOR", "1");
    let out = cmd.output().expect("run box color --json");
    assert!(out.status.success(), "box color {input:?} --json should exit 0");
    serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value")
}

/// CSS name → RGB: the four anchors resolve to their known hex via the CLI.
#[test]
fn css_name_resolves_to_rgb() {
    assert_eq!(
        color_json("black").get("hex").and_then(|h| h.as_str()),
        Some("#000000")
    );
    assert_eq!(
        color_json("white").get("hex").and_then(|h| h.as_str()),
        Some("#ffffff")
    );
    assert_eq!(
        color_json("rebeccapurple").get("hex").and_then(|h| h.as_str()),
        Some("#663399")
    );
    assert_eq!(
        color_json("cornflowerblue").get("hex").and_then(|h| h.as_str()),
        Some("#6495ed")
    );
}

/// `box color "hsl(210, 100%, 50%)"` parses to the expected RGB block (and the
/// space form agrees). The human block shows the resolved hex/rgb.
#[test]
fn hsl_input_parses_to_block() {
    let comma = color_stdout("hsl(210, 100%, 50%)");
    assert!(comma.contains("#0080FF"), "hsl → #0080FF: {comma:?}");
    assert!(comma.contains("rgb(0, 128, 255)"), "hsl → rgb: {comma:?}");
    // The modern space form resolves to the same block.
    let space = color_stdout("hsl(210 100% 50%)");
    assert_eq!(space, comma, "space-form HSL must match comma-form");
}

/// RGB→name (both directions): for an EXACT keyword the JSON `name` == `nearest`;
/// for an arbitrary color `name` is null and `nearest` is non-empty. Both fields
/// are ALWAYS present.
#[test]
fn json_name_and_nearest_always_present() {
    // Exact keyword (#ff0000 == red): name == nearest == "red".
    let red = color_json("#ff0000");
    assert_eq!(red.get("name").and_then(|n| n.as_str()), Some("red"));
    assert_eq!(red.get("nearest").and_then(|n| n.as_str()), Some("red"));

    // Arbitrary color (#3b82f6): name is null, nearest is a non-empty keyword.
    let arb = color_json("#3b82f6");
    assert!(
        arb.get("name").is_some() && arb.get("name").unwrap().is_null(),
        "arbitrary color → name: null (present, null): {arb}"
    );
    let nearest = arb
        .get("nearest")
        .and_then(|n| n.as_str())
        .expect("nearest must always be a string");
    assert!(!nearest.is_empty(), "nearest must be non-empty: {arb}");
}

/// Malformed `hsl(...)` → exit 1, empty stdout, non-empty stderr, no panic (V5).
#[test]
fn malformed_hsl_exits_1_clean() {
    color("hsl(400, 100%, 50%)")
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — copied from tests/uuid.rs ----------
//
// color is a SCALAR command with the D-17 LOCKED NESTED shape:
// `{hex, rgb:{r,g,b}, hsl:{h,s,l}}`. The JSON `hex` is LOWERCASE `#rrggbb` (locked
// for a deterministic test, even though the human render uses uppercase). SPINE-04.

/// Capture `box color <input>` raw stdout bytes + exit status for purity assertions.
fn color_output(input: &str, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input).args(extra);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box color")
}

/// SPINE-02 / D-17 — `box color "#ff0000" --json` emits the NESTED
/// `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` shape: `.rgb.r == 255`, `.hex == "#ff0000"`
/// (lowercase), nested sub-objects resolve; no ANSI, no UTF-8 BOM.
#[test]
fn json_purity() {
    let out = color_output("#ff0000", &["--json"]);
    assert!(out.status.success(), "box color #ff0000 --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // hex is LOWERCASE `#rrggbb` (locked).
    assert_eq!(
        v.get("hex").and_then(|h| h.as_str()),
        Some("#ff0000"),
        "`.hex` must be lowercase #ff0000"
    );
    // Nested rgb sub-object: `.rgb.r == 255`, `.rgb.g == 0`, `.rgb.b == 0`.
    let rgb = v.get("rgb").expect("`.rgb` must be present");
    assert_eq!(
        rgb.get("r").and_then(|x| x.as_u64()),
        Some(255),
        "`.rgb.r` must be 255"
    );
    assert_eq!(rgb.get("g").and_then(|x| x.as_u64()), Some(0), "`.rgb.g` == 0");
    assert_eq!(rgb.get("b").and_then(|x| x.as_u64()), Some(0), "`.rgb.b` == 0");
    // Nested hsl sub-object: `.hsl.h` resolves (pure red → hue 0).
    let hsl = v.get("hsl").expect("`.hsl` must be present");
    assert_eq!(
        hsl.get("h").and_then(|x| x.as_u64()),
        Some(0),
        "`.hsl.h` for pure red is 0"
    );
    assert!(
        hsl.get("s").is_some() && hsl.get("l").is_some(),
        "`.hsl` carries s and l"
    );
    assert!(
        v.get("results").is_none(),
        "color is scalar — no `results` wrapper"
    );

    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}

/// SPINE-04 / D-07 — live Windows-clipboard round-trip for `box color --clip`.
/// The human render is multi-line; `--clip` copies the whole printed block
/// (newline-joined, trimmed once). `#[ignore]`d. Run locally with:
///   cargo test --test color -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    let printed = {
        let out = color_output("#ff0000", &["--clip"]);
        assert!(out.status.success(), "box color #ff0000 --clip should exit 0");
        String::from_utf8(out.stdout)
            .expect("stdout is UTF-8")
            .trim_end()
            .to_string()
    };
    let pasted = {
        let out = Command::cargo_bin("box")
            .unwrap()
            .args(["clip", "--paste"])
            .output()
            .expect("run box clip --paste");
        assert!(out.status.success(), "box clip --paste should exit 0");
        String::from_utf8(out.stdout)
            .expect("clipboard text is UTF-8")
            .trim_end()
            .to_string()
    };
    assert_eq!(
        pasted, printed,
        "--clip must copy exactly the printed color block to the clipboard"
    );
}
