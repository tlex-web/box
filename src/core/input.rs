//! Shared stdin/arg input reader for `box` commands (D-04/D-05/D-06).
//!
//! Every stdin-consuming Phase-2 command (base64, cowsay, epoch, color) and the
//! later file commands (hash, json, clip, lolcat) acquire their input through
//! one of two readers here, so the arg-vs-stdin-vs-TTY precedence and the
//! no-block guard live in exactly one place.
//!
//! Precedence (D-04), evaluated by [`resolve`] / [`resolve_bytes`]:
//! 1. positional `arg` is `Some(s)` and `s != "-"` → use it directly; stdin is
//!    never touched.
//! 2. `arg` is `None` or `Some("-")` **and** stdin is **not** an interactive TTY
//!    (i.e. piped/redirected) → read stdin to EOF.
//! 3. `arg` is `None`/`"-"` **and** stdin **is** an interactive TTY → return
//!    [`BoxError::MissingInput`] (→ exit 2 in `main()`) instead of blocking on a
//!    read that would hang the terminal.
//!
//! [`read_input`] returns a UTF-8 `String` (cowsay/epoch/color). [`read_input_bytes`]
//! returns a `Vec<u8>` via `read_to_end` with **no** UTF-8 validation, so binary
//! input (base64) is byte-exact even for non-UTF-8 bytes (D-05/D-06).
//!
//! The TTY decision is threaded through the inner `resolve*` helpers as an
//! `is_tty` bool and the stdin source as a `Read`, so branch 3 (and the
//! piped-bytes path) are unit-testable without a real terminal (RESEARCH:540-543).
//! A future `--file PATH` layer (D-06, Phase 3) slots in ahead of the stdin
//! branch inside these helpers without reshaping the public signatures.

use std::fs::File;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::core::errors::BoxError;

/// Read UTF-8 text input for a command, following the D-04 precedence.
///
/// Use for text-oriented commands (cowsay, color). Reads piped stdin as a UTF-8
/// `String`; an interactive TTY with no argument yields
/// [`BoxError::MissingInput`] (exit 2) rather than blocking.
//
// Live as of Plan 02-03: `color` is the first caller of the String reader, so
// the forward-compat `#[allow(dead_code)]` has been removed here (and on the
// inner `resolve`), restoring the strict dead-code gate on the String path —
// mirroring the byte-path removal at 02-02 and the [01-03] allow-then-remove
// precedent. (`epoch` reads input itself because no-arg means "print now", not
// the exit-2 missing-input case, so it deliberately does not call this.)
pub fn read_input(arg: Option<String>) -> anyhow::Result<String> {
    let stdin = std::io::stdin();
    resolve(arg, stdin.is_terminal(), stdin.lock())
}

/// Read binary-exact input for a command, following the D-04 precedence.
///
/// Use for byte-oriented commands (base64). Reads piped stdin via `read_to_end`
/// into a `Vec<u8>` with no UTF-8 validation, so arbitrary bytes round-trip
/// unchanged (D-05/D-06). An interactive TTY with no argument yields
/// [`BoxError::MissingInput`] (exit 2) rather than blocking.
//
// Live as of Plan 02-02: `base64` is the first caller, so the Phase-1-style
// forward-compat `#[allow(dead_code)]` has been removed here (and on
// `resolve_bytes` + `BoxError::MissingInput`), restoring the strict dead-code
// gate on the byte path (mirrors STATE.md [01-03] allow-then-remove).
pub fn read_input_bytes(arg: Option<String>) -> anyhow::Result<Vec<u8>> {
    let stdin = std::io::stdin();
    resolve_bytes(arg, stdin.is_terminal(), stdin.lock())
}

/// Inner resolver for [`read_input`] — `is_tty` and the reader are injected so the
/// three branches are unit-testable without a real terminal.
// Live via `read_input` (color, Plan 02-03); the scoped allow has been removed
// alongside its public caller, restoring the strict dead-code gate.
fn resolve<R: Read>(arg: Option<String>, is_tty: bool, mut reader: R) -> anyhow::Result<String> {
    match arg.as_deref() {
        // Branch 1: an explicit argument that is not the stdin sentinel "-".
        Some(s) if s != "-" => Ok(s.to_string()),
        // Branch 3: no usable arg AND interactive TTY → do not block (D-04).
        // Returned as the typed variant via `.into()` (not a plain anyhow macro),
        // so `main()` can downcast to BoxError::MissingInput and map it to exit 2
        // (RESEARCH Pitfall 2). A type-erased anyhow error would lose the variant.
        _ if is_tty => Err(BoxError::MissingInput.into()),
        // Branch 2: no usable arg AND piped → read stdin to EOF as UTF-8.
        _ => {
            let mut buf = String::new();
            reader
                .read_to_string(&mut buf)
                .context("failed to read input from stdin")?;
            Ok(buf)
        }
    }
}

/// Inner resolver for [`read_input_bytes`] — mirrors [`resolve`] but reads bytes
/// via `read_to_end` (no UTF-8 validation) so binary input is byte-exact.
// Live via `read_input_bytes` (base64, Plan 02-02); allow removed.
fn resolve_bytes<R: Read>(
    arg: Option<String>,
    is_tty: bool,
    mut reader: R,
) -> anyhow::Result<Vec<u8>> {
    match arg.as_deref() {
        Some(s) if s != "-" => Ok(s.as_bytes().to_vec()),
        _ if is_tty => Err(BoxError::MissingInput.into()),
        _ => {
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .context("failed to read input from stdin")?;
            Ok(buf)
        }
    }
}

/// A streaming input source plus the label to display for it.
///
/// `hash` (the first consumer, Plan 03-01) must **stream** its payload into the
/// hasher — never buffer a whole (potentially multi-GB) file into a `Vec` first
/// (RESEARCH anti-pattern / T-03-03). So unlike [`read_input_bytes`] this carries
/// an open `impl Read` handle, not the bytes, plus the coreutils-style `label`
/// printed in the `<hash>  <label>` row (D-05): the file path for a `--file`/
/// positional path, or `-` for piped stdin.
pub struct ResolvedInput {
    /// The streaming byte source (an open `File`, piped stdin, or — in tests — an
    /// injected reader). Boxed because the concrete type varies by branch.
    pub reader: Box<dyn Read>,
    /// The display label for the output row: the path, or `-` for stdin (D-05).
    pub label: String,
}

// `Box<dyn Read>` is not `Debug`, so derive won't work; a manual impl prints only
// the label (enough for `Result::unwrap_err` assertions and any error diagnostics
// without exposing the reader's bytes).
impl std::fmt::Debug for ResolvedInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedInput")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

/// Resolve a **streaming** input source for a path-or-stdin command (`hash`),
/// honoring the deferred `--file PATH` layer (D-05/D-06) AHEAD of the stdin branch.
///
/// Precedence (extends the [`resolve_bytes`] precedence with a leading file branch):
/// 1. `path` is `Some(p)` and `p != "-"` → open the file and stream it; the label
///    is the path. An unreadable/missing path surfaces a `.context(...)`-wrapped
///    `anyhow` error (→ exit 1), never a panic (FOUND-05 / T-03-02).
/// 2. `path` is `None`/`Some("-")` **and** stdin is **not** an interactive TTY →
///    stream piped stdin; the label is `-`.
/// 3. `path` is `None`/`"-"` **and** stdin **is** an interactive TTY → return
///    [`BoxError::MissingInput`] (→ exit 2), inheriting the no-block guard.
///
/// The `-` sentinel and the `MissingInput` → exit-2 semantics are inherited from
/// the existing byte/string resolvers; only the file branch is new.
pub fn read_file_or_stdin(path: Option<String>) -> anyhow::Result<ResolvedInput> {
    let stdin = std::io::stdin();
    let is_tty = stdin.is_terminal();
    resolve_reader(path.as_deref(), is_tty, || Box::new(stdin.lock()))
}

/// Inner resolver for [`read_file_or_stdin`]: `is_tty` and the stdin factory are
/// injected so all three branches are unit-testable without a real terminal or
/// process stdin (mirrors the `is_tty`/`Read` injection of [`resolve`]).
///
/// `make_stdin` is a closure (not an eager `Read`) so the file branch never
/// touches stdin — matching the "stdin is never read when an arg is present"
/// guarantee of the existing resolvers.
fn resolve_reader(
    path: Option<&str>,
    is_tty: bool,
    make_stdin: impl FnOnce() -> Box<dyn Read>,
) -> anyhow::Result<ResolvedInput> {
    match path {
        // Branch 1 (NEW — the deferred `--file PATH` layer, ahead of stdin):
        // an explicit path that is not the stdin sentinel "-". Stream the file
        // (no whole-file read_to_end — T-03-03). A missing/unreadable path is a
        // clean anyhow error, not a panic (FOUND-05 / T-03-02).
        Some(p) if p != "-" => {
            let file = open_input_file(Path::new(p))?;
            Ok(ResolvedInput {
                reader: Box::new(file),
                label: p.to_string(),
            })
        }
        // Branch 3: no usable path AND interactive TTY → do not block (D-04),
        // inheriting the typed exit-2 variant.
        _ if is_tty => Err(BoxError::MissingInput.into()),
        // Branch 2: no usable path AND piped → stream stdin, labelled `-` (D-05).
        _ => Ok(ResolvedInput {
            reader: make_stdin(),
            label: "-".to_string(),
        }),
    }
}

/// Open a `--file`/positional path for streaming, wrapping any I/O error with the
/// offending path so the user gets a clear message (FOUND-05) instead of a bare
/// `os error 2` or a panic. Returns `PathBuf` in the context for a non-UTF-8-safe
/// display.
fn open_input_file(path: &Path) -> anyhow::Result<File> {
    File::open(path).with_context(|| format!("failed to open {}", PathBuf::from(path).display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Branch 1: an explicit arg is returned verbatim and stdin is never read
    /// (we pass an empty reader; if it were touched the result would be empty).
    #[test]
    fn arg_branch_returns_arg_verbatim() {
        let out = resolve(Some("hello".to_string()), false, &b""[..]).unwrap();
        assert_eq!(out, "hello");
    }

    /// Branch 3: no arg on an interactive TTY returns an error that downcasts to
    /// `BoxError::MissingInput` — proving the exit-2 path in `main()` is reachable
    /// and that we returned the typed variant (a type-erased anyhow error created
    /// by a plain macro would not downcast).
    #[test]
    fn missing_input_on_tty_downcasts_to_box_error() {
        let err = resolve(None, true, &b""[..]).unwrap_err();
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::MissingInput)),
            "expected BoxError::MissingInput, got: {err:?}"
        );
    }

    /// Branch 2 (bytes): piped input is read byte-exactly via `read_to_end`,
    /// including a non-UTF-8 byte (0xFF) — proving we use `read_to_end`, not
    /// `read_to_string` (which would reject/mangle the byte).
    #[test]
    fn piped_bytes_are_byte_exact_including_non_utf8() {
        let input: &[u8] = &[0x00, 0xFF, b'a', 0x80, b'\n'];
        let out = resolve_bytes(None, false, input).unwrap();
        assert_eq!(out, input);
    }

    /// The "-" sentinel with no real arg behaves like branch 2 (read stdin) when
    /// piped — confirming "-" is treated as "read from stdin", not as literal text.
    #[test]
    fn dash_sentinel_reads_piped_stdin() {
        let out = resolve(Some("-".to_string()), false, &b"piped"[..]).unwrap();
        assert_eq!(out, "piped");
    }

    /// Helper: drain a `ResolvedInput` reader to a `Vec<u8>` for assertions.
    fn drain(mut r: ResolvedInput) -> (Vec<u8>, String) {
        let mut buf = Vec::new();
        r.reader.read_to_end(&mut buf).unwrap();
        (buf, r.label)
    }

    /// `--file` branch (NEW): a real on-disk path is streamed AHEAD of stdin —
    /// the injected stdin factory must never run (if it did, the bytes would be
    /// `"STDIN"`, not the file contents), and the label is the path, not `-`.
    #[test]
    fn file_branch_reads_named_file_ahead_of_stdin() {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("box_input_test_{}.bin", std::process::id()));
        std::fs::write(&tmp, b"file-bytes\x00\xff").unwrap();
        let path = tmp.to_string_lossy().to_string();

        let resolved = resolve_reader(Some(&path), false, || {
            panic!("stdin factory must NOT be called when --file is present")
        })
        .unwrap();
        let (bytes, label) = drain(resolved);

        assert_eq!(bytes, b"file-bytes\x00\xff");
        assert_eq!(label, path);
        let _ = std::fs::remove_file(&tmp);
    }

    /// A missing/unreadable `--file` path surfaces a clean `anyhow` error
    /// (FOUND-05 / T-03-02), NOT a panic, and the message names the path.
    #[test]
    fn file_branch_missing_path_is_clean_error_not_panic() {
        let missing = "this_path_should_not_exist_box_42.bin";
        let err = resolve_reader(Some(missing), false, || unreachable!()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("failed to open") && msg.contains(missing),
            "expected a path-naming open error, got: {msg}"
        );
    }

    /// `--file` Branch 3: no path on an interactive TTY yields the typed
    /// `MissingInput` variant (→ exit 2), inheriting the no-block guard.
    #[test]
    fn file_branch_missing_input_on_tty_downcasts() {
        let err = resolve_reader(None, true, || unreachable!()).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<BoxError>(),
            Some(BoxError::MissingInput)
        ));
    }

    /// `--file` Branch 2: no path / `-` when piped streams stdin, labelled `-`.
    #[test]
    fn file_branch_dash_streams_stdin_labelled_dash() {
        let resolved =
            resolve_reader(Some("-"), false, || Box::new(&b"piped-stdin"[..])).unwrap();
        let (bytes, label) = drain(resolved);
        assert_eq!(bytes, b"piped-stdin");
        assert_eq!(label, "-");
    }
}
