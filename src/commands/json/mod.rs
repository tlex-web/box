//! The `json` command: validate and pretty-print JSON (JSON-01).
//!
//! Flow (Pattern 1 — thin orchestrator over a pure colorizer):
//! `run()` acquires the input text via [`crate::core::input::read_input`]
//! (arg → piped stdin → no-arg interactive TTY → exit 2), parses it to a
//! `serde_json::Value`, and then emits one of three forms:
//!   - `--compact` → [`serde_json::to_string`] (minified, single line);
//!   - colored TTY → the hand-rolled [`colorize`] walker (2-space pretty + ANSI);
//!   - plain/piped → [`serde_json::to_string_pretty`] (2-space pretty, no ANSI).
//!
//! Decisions encoded here:
//! - **D-04** — `serde_json` is built with `preserve_order` (see Cargo.toml), so
//!   the parsed `Value`'s object map is an insertion-ordered `IndexMap`:
//!   `{"b":1,"a":2}` keeps `b` before `a`, it is never alphabetized.
//! - **D-05** — coloring is a hand-rolled colorizer over `Value` tokens using
//!   owo-colors, gated SOLELY on [`is_color_on`] (no `colored_json` crate, no
//!   second color stack, no `set_override` toggle). The non-color and `--compact`
//!   paths delegate to serde_json's own serializers, so piped output is
//!   byte-identical to the colored output minus the ANSI escapes.
//! - **D-06** — invalid JSON `bail!`s with the 1-based line and column from
//!   [`serde_json::Error`] (→ exit 1 via `main()`); valid input exits 0; pretty
//!   default is exactly 2-space indent; `--compact` minifies.

use clap::Args;
use owo_colors::OwoColorize;
use serde_json::Value;

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// `box json [INPUT] [--compact]` — validate and pretty-print JSON (JSON-01).
///
/// `INPUT` is the JSON text; omit it to read from piped stdin. The default is a
/// 2-space pretty print (syntax-colored in a TTY, plain when piped). `--compact`
/// minifies instead. Input key order is preserved. Invalid JSON prints a
/// 1-based line/column error to stderr and exits 1.
#[derive(Debug, Args)]
pub struct JsonArgs {
    /// JSON text to format; omit to read from piped stdin.
    pub input: Option<String>,
    /// Minify instead of pretty-printing.
    #[arg(long)]
    pub compact: bool,
}

impl RunCommand for JsonArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        let text = crate::core::input::read_input(self.input)?;

        match serde_json::from_str::<Value>(&text) {
            // D-06 (WR-01): a parse error surfaces the 1-based line/column on
            // stderr and exits 1 (main() adds the `error:` prefix). Exit 1 — not
            // exit 2 — is DELIBERATE: malformed JSON is bad *data* the command
            // processed and rejected (a runtime/data error), NOT a *usage* error
            // in how `box json` was invoked. Exit 2 is reserved for usage errors
            // (missing input, bad flags, unsupported `--verify` length); see the
            // exit-code policy in `main.rs`. No panic on bad input — `from_str`
            // returns a `Result` (T-04J-02). Pinned by `tests/json.rs`.
            Err(e) => anyhow::bail!("at line {} column {}: {e}", e.line(), e.column()),
            Ok(value) => {
                // D-16 identity passthrough (the ONE sanctioned direct-serde
                // command, a root-rule EXCEPTION alongside tree): under --json emit
                // the parsed `Value` VERBATIM via `emit_json` — pure (no BOM,
                // trailing \n), NOT wrapped in {results,count}, and it tees the
                // whole document to the clipboard under `--json --clip`. The fork
                // is FIRST (Pitfall 1) and wins over `--compact` (the machine
                // document is always the pretty serde form).
                if crate::core::output::is_json_on() {
                    return crate::core::output::emit_json(&value);
                }

                if self.compact {
                    // Minified single line — delegate to serde_json (D-06). Route
                    // through out_line so `--compact --clip` tees the compact form
                    // (SPINE-04).
                    crate::core::output::out_line(&serde_json::to_string(&value)?);
                } else if is_color_on() {
                    // The ONE color path: a hand-rolled walker, gated on the
                    // single color decision (D-05). Piped/NO_COLOR never reaches
                    // here, so the plain branch is byte-identical minus ANSI. Under
                    // --clip, init_output forces COLOR_ON=false so this branch is
                    // never taken (the plain branch below tees instead).
                    print!("{}", colorize(&value, 0));
                } else {
                    // Plain 2-space pretty — delegate so the bytes match the
                    // colorized layout exactly minus the escapes (D-05/D-06). Route
                    // through out_line so `--clip` tees the pretty form (SPINE-04).
                    crate::core::output::out_line(&serde_json::to_string_pretty(&value)?);
                }
                Ok(())
            }
        }
    }
}

/// The two-space indent unit used at every nesting level (D-06).
const INDENT: &str = "  ";

/// Pretty-print a [`Value`] to a syntax-colored 2-space-indented `String`.
///
/// Pure + crate-only-on-owo-colors, so it is unit-testable without a terminal.
/// The CALLER has already checked [`is_color_on`]; this function always emits the
/// ANSI tokens (the non-color path never calls it — D-05). The layout is
/// byte-for-byte the same shape `serde_json::to_string_pretty` produces (2-space
/// indent, `": "` after keys, `,`-then-newline between members) so piped (plain)
/// and TTY (colored) output differ only by the ANSI escapes.
///
/// `indent` is the current nesting depth (number of [`INDENT`] units), not a
/// column count. Token colors (RESEARCH OQ-3 discretion): key = blue,
/// string = green, number = yellow, bool/null = magenta, punctuation = plain.
fn colorize(value: &Value, indent: usize) -> String {
    let mut out = String::new();
    write_value(&mut out, value, indent);
    out
}

/// Append the colored rendering of `value` at nesting `depth` to `out`. Walks all
/// six `Value` variants (Null / Bool / Number / String / Array / Object).
fn write_value(out: &mut String, value: &Value, depth: usize) {
    match value {
        Value::Null => out.push_str(&"null".magenta().to_string()),
        Value::Bool(b) => out.push_str(&b.magenta().to_string()),
        Value::Number(n) => out.push_str(&n.to_string().yellow().to_string()),
        Value::String(s) => out.push_str(&color_json_string(s).green().to_string()),
        Value::Array(items) => write_array(out, items, depth),
        Value::Object(map) => write_object(out, map, depth),
    }
}

/// Render a JSON array. Empty → `[]`; otherwise each element on its own line,
/// indented one level deeper, with the closing `]` aligned to the opening depth.
fn write_array(out: &mut String, items: &[Value], depth: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push('[');
    out.push('\n');
    let inner = depth + 1;
    for (i, item) in items.iter().enumerate() {
        push_indent(out, inner);
        write_value(out, item, inner);
        if i + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    push_indent(out, depth);
    out.push(']');
}

/// Render a JSON object. Empty → `{}`; otherwise each `"key": value` pair on its
/// own line, indented one level deeper, iterated in INSERTION order (D-04 —
/// `preserve_order` makes the map an `IndexMap`), with the closing `}` aligned to
/// the opening depth.
fn write_object(out: &mut String, map: &serde_json::Map<String, Value>, depth: usize) {
    if map.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push('{');
    out.push('\n');
    let inner = depth + 1;
    let last = map.len() - 1;
    for (i, (key, val)) in map.iter().enumerate() {
        push_indent(out, inner);
        // Key in blue, with the JSON string quoting/escaping applied.
        out.push_str(&color_json_string(key).blue().to_string());
        out.push_str(": ");
        write_value(out, val, inner);
        if i != last {
            out.push(',');
        }
        out.push('\n');
    }
    push_indent(out, depth);
    out.push('}');
}

/// Push `depth` copies of the 2-space [`INDENT`] unit onto `out`.
fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}

/// Render a Rust string as a JSON string literal — `"…"` with the standard JSON
/// escapes applied — so the colored output is valid JSON (and matches what
/// serde_json would emit for the plain path). Uses `serde_json::to_string` on a
/// `Value::String`, which is infallible for any `&str`, falling back to a simple
/// quoted form only if that (cannot-happen) path errors.
fn color_json_string(s: &str) -> String {
    serde_json::to_string(&Value::String(s.to_string())).unwrap_or_else(|_| format!("{s:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-04 — key order is preserved (NOT alphabetized): parsing `{"b":1,"a":2}`
    /// with `preserve_order` keeps `b` before `a`, both in the plain
    /// `to_string_pretty` form and in the hand-rolled `colorize` walker. If
    /// `preserve_order` were off, serde would sort the keys and `a` would lead.
    #[test]
    fn colorize_preserves_key_order() {
        let value: Value = serde_json::from_str("{\"b\":1,\"a\":2}").unwrap();

        let pretty = serde_json::to_string_pretty(&value).unwrap();
        assert!(
            pretty.find("\"b\"").unwrap() < pretty.find("\"a\"").unwrap(),
            "to_string_pretty must keep b before a: {pretty:?}"
        );

        let colored = colorize(&value, 0);
        // Strip ANSI by scanning for the raw key bytes — the `"b"`/`"a"` literals
        // survive coloring (color wraps, it does not rewrite the token text).
        assert!(
            colored.find("\"b\"").unwrap() < colored.find("\"a\"").unwrap(),
            "colorize must keep b before a: {colored:?}"
        );
    }

    /// `colorize` emits a 2-space-indented structure for a nested value:
    /// `{"a":[1,true,null]}` renders the array members one-per-line, the inner
    /// elements indented two levels (4 spaces), and the tokens carry ANSI (the
    /// caller has already gated on `is_color_on`).
    #[test]
    fn colorize_nested_shape_and_indent() {
        let value: Value = serde_json::from_str("{\"a\":[1,true,null]}").unwrap();
        let out = colorize(&value, 0);

        // The object opens and its single key sits at one indent level (2 spaces).
        assert!(
            out.starts_with("{\n"),
            "object opens with brace+newline: {out:?}"
        );
        assert!(
            out.contains("\n  \u{1b}"),
            "key line is indented two spaces then a colored token: {out:?}"
        );
        // The array elements are indented two levels deep (4 spaces) before a
        // colored number/bool/null token.
        assert!(
            out.contains("\n    \u{1b}"),
            "array elements indented four spaces: {out:?}"
        );
        // Colored output carries ANSI (the gate is the caller's job, not ours).
        assert!(
            out.contains('\u{1b}'),
            "colorize emits ANSI tokens: {out:?}"
        );
        // The three JSON value keywords/numbers are all present in token text.
        assert!(out.contains('1'), "number 1 present: {out:?}");
        assert!(out.contains("true"), "bool true present: {out:?}");
        assert!(out.contains("null"), "null present: {out:?}");
    }

    /// An empty object and empty array collapse to `{}` / `[]` (no inner newline),
    /// matching serde_json's pretty output for empties.
    #[test]
    fn colorize_empty_containers_collapse() {
        let obj: Value = serde_json::from_str("{}").unwrap();
        let arr: Value = serde_json::from_str("[]").unwrap();
        assert_eq!(colorize(&obj, 0), "{}");
        assert_eq!(colorize(&arr, 0), "[]");
    }

    /// A bare string value is JSON-quoted (with escaping) and green-wrapped — the
    /// raw `"hi"` token text survives inside the ANSI wrapping.
    #[test]
    fn colorize_string_is_quoted() {
        let value: Value = serde_json::from_str("\"hi\"").unwrap();
        let out = colorize(&value, 0);
        assert!(out.contains("\"hi\""), "string is JSON-quoted: {out:?}");
        assert!(out.contains('\u{1b}'), "string token is colored: {out:?}");
    }

    /// WR-05 / D-05 — the ACTUAL invariant: stripping ANSI from `colorize(&v, 0)`
    /// yields EXACTLY `serde_json::to_string_pretty(&v)`, so the colored TTY path
    /// and the plain piped path differ ONLY by the ANSI escapes (byte-identical
    /// minus color). The earlier tests only assert substring presence (`"1"`,
    /// `"true"`); this pins byte-equality across a battery of values — floats,
    /// large ints, negatives, escaped strings, nested containers — which is where
    /// a hand-formatted scalar (e.g. `n.to_string()` vs serde's Display) would
    /// diverge if it ever did.
    #[test]
    fn colorize_stripped_equals_pretty() {
        let cases = [
            "null",
            "true",
            "false",
            "0",
            "-1",
            "42",
            "3.14",
            "-2.5e10",
            "123456789012345",
            "\"hi\"",
            "\"tab\\tnewline\\nquote\\\"\"",
            "[]",
            "{}",
            "[1,2,3]",
            "{\"b\":1,\"a\":2}",
            "{\"a\":[1,true,null],\"nested\":{\"x\":-3.5,\"y\":\"z\"}}",
            "[[1,[2,[3]]],{\"deep\":{\"k\":false}}]",
        ];
        for src in cases {
            let value: Value = serde_json::from_str(src).expect("fixture parses");
            let colored = colorize(&value, 0);
            // Strip the ANSI escapes the colorizer wrapped each token in.
            let stripped = strip_ansi_escapes::strip_str(&colored);
            let pretty = serde_json::to_string_pretty(&value).expect("pretty serializes");
            assert_eq!(
                stripped, pretty,
                "colorize stripped of ANSI must equal to_string_pretty for {src:?}"
            );
        }
    }
}
