//! The `clip` command: pipe stdin → the Windows clipboard, or read the clipboard
//! → stdout with `--paste` (CLIP-01). The command that bypasses `clip.exe`'s
//! Unicode mangling and its lack of a paste path.
//!
//! Mode (D-04): copy-by-default — no flag reads stdin → clipboard; `box clip
//! --paste` reads the clipboard → stdout. ⚠️ `clip` deliberately does NOT route
//! through [`crate::core::input::read_input`]: that would inherit the
//! no-arg-interactive-TTY → `BoxError::MissingInput` (exit 2) contract, which is
//! wrong for a clipboard copy. The copy path reads RAW stdin bytes directly
//! (`std::io::stdin().read_to_end`) so it owns its own UTF-8 validation and
//! newline policy.
//!
//! Trailing-newline policy (D-05): on COPY, strip AT MOST ONE trailing terminator
//! (`\r\n`, then a lone `\n`) — never interior or multiple newlines; on `--paste`,
//! emit the clipboard text BYTE-EXACT (no added/stripped newline). PowerShell 7
//! appends an implicit CRLF when piping a string to a native command
//! (`"x" | box clip` sends `x\r\n`), so a byte-verbatim copy would land a spurious
//! trailing newline on the clipboard — the exact `clip.exe` friction this command
//! exists to fix. Mirrors `pbcopy`/`xclip`. The single helper
//! [`trim_one_trailing_newline`] is the pure unit-test seam.
//!
//! Crate = `arboard` 3.6.1 (D-06). Windows persistence is SAFE: `set_text`
//! performs the synchronous `OpenClipboard → EmptyClipboard →
//! SetClipboardData(CF_UNICODETEXT) → CloseClipboard` inside one call, and per
//! Microsoft the system owns the handle after `SetClipboardData`, so the copied
//! text survives process exit (no keep-alive pump — that drop-on-exit problem is
//! X11/Wayland-specific and does NOT apply on Windows). The STATE.md "arboard
//! main-thread only" pitfall is satisfied by this single-shot synchronous flow:
//! create the `Clipboard` at the point of use, do one op, return; never spawn the
//! arboard call onto a worker thread (CLIP-2). Both `set_text` and `get_text` take
//! `&mut self`, so the binding MUST be `let mut cb`.
//!
//! Errors → clean exit 1, never a panic (D-04/FOUND-05): non-UTF-8 stdin on copy
//! is rejected by `String::from_utf8(...).context(...)?` and any `arboard::Error`
//! is mapped via `.context(...)?`. `clip` introduces NO exit-2 path (it reads raw
//! stdin, not `read_input`, so there is no `MissingInput`).

// # Spine omission (SC4)
// `clip` is a DISPLAY-ONLY / clipboard-I/O command: it INTENTIONALLY does not honor
// the global `--json`/`--clip` flags (roadmap SC4). The flags parse (global on `Cli`)
// but `run()` never calls `is_json_on()` / `emit_json` — `box clip --json` and
// `box clip --paste --json` perform the normal clipboard copy/paste and emit NO JSON
// document to stdout (`--paste` emits the clipboard bytes verbatim; wrapping them in a
// JSON envelope is meaningless, and `--clip` on the clipboard command is a no-op).
// Asserted by `tests/cli.rs::display_only_omit_json`.

use std::io::{Read, Write};

use anyhow::Context;
use clap::Args;

use crate::commands::RunCommand;

/// `box clip [--paste]` — read from or write to the Windows clipboard (CLIP-01).
///
/// With no flag, `clip` reads piped stdin and copies it to the clipboard
/// (`"text" | box clip`). With `--paste`, it reads the clipboard and writes it to
/// stdout (`box clip --paste`).
///
/// On copy, a single trailing newline is stripped (one `\r\n` or one `\n`) so the
/// implicit CRLF PowerShell appends when piping a string does not land a spurious
/// blank line on the clipboard — interior and multiple newlines are preserved. On
/// paste, the clipboard text is emitted byte-exact with no newline added or
/// removed.
#[derive(Debug, Args)]
pub struct ClipArgs {
    /// Read the clipboard to stdout instead of copying stdin to the clipboard.
    #[arg(long)]
    pub paste: bool,
}

impl RunCommand for ClipArgs {
    fn run(self) -> anyhow::Result<()> {
        if self.paste {
            // PASTE: clipboard → stdout, byte-exact (D-05 — no newline policy). The
            // single-shot main-thread arboard flow (create → one op → return)
            // satisfies the STATE.md "arboard main-thread only" pitfall (CLIP-2).
            let mut cb = arboard::Clipboard::new().context("open clipboard")?;
            // Distinguish the common "nothing to paste" case from a genuine Win32
            // read failure (WR-02). `arboard` returns `ContentNotAvailable` when the
            // clipboard is empty or holds non-text content (an image, a file list) —
            // that is not a read FAILURE, so a generic "read clipboard: …" misleads.
            // Both branches stay exit 1 (the error path is unchanged); only the
            // diagnostic message differs. The success path and the byte-exact write
            // below are untouched (D-05).
            let text = match cb.get_text() {
                Ok(t) => t,
                Err(arboard::Error::ContentNotAvailable) => {
                    anyhow::bail!("clipboard is empty or contains no text")
                }
                Err(e) => return Err(e).context("read clipboard"),
            };
            // Write the raw bytes; do NOT add or strip a trailing newline (D-05).
            std::io::stdout().write_all(text.as_bytes())?;
        } else {
            // COPY: raw stdin → clipboard. Read RAW bytes directly, NOT via
            // `core::input::read_input` (D-04) — that would inherit the
            // no-arg-TTY → exit-2 contract, wrong for a clipboard copy.
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            // Strip at most one trailing terminator so the implicit CRLF PowerShell
            // appends when piping a string does not land a spurious blank line on
            // the clipboard (D-05 / Pitfall CLIP-1).
            let buf = trim_one_trailing_newline(buf);
            // Validate UTF-8 BEFORE touching the clipboard: a non-UTF-8 stream is a
            // clean exit-1 error here (main() prints `error: …`), never a panic
            // (D-04 / FOUND-05 / T-05-CLIP-DoS). `arboard::Clipboard::new()` is not
            // even reached on bad input, so the failure is deterministic and
            // clipboard-independent.
            let text = String::from_utf8(buf).context("clipboard input must be UTF-8")?;
            // Single-shot main-thread set: arboard's `set_text` performs the full
            // OpenClipboard → SetClipboardData(CF_UNICODETEXT) → CloseClipboard
            // synchronously; the OS owns the handle after, so the copy persists past
            // process exit (D-06 — no keep-alive pump needed on Windows). `set_text`
            // takes `&mut self`, hence `let mut cb` (Pitfall CLIP-2). The clipboard
            // contents are never echoed to stdout/stderr (T-05-CLIP-INFO).
            let mut cb = arboard::Clipboard::new().context("open clipboard")?;
            cb.set_text(text).context("write clipboard")?;
        }
        Ok(())
    }
}

/// Strip AT MOST ONE trailing line terminator from `s`: a lone `\n`, or a `\r\n`
/// pair (D-05). Interior newlines and any second/preceding terminator are left
/// untouched — `b"x\n\n"` loses exactly one `\n` to become `b"x\n"`.
///
/// Pure + crate-free, so it is the deterministic unit-test seam for the copy-path
/// newline policy (the PowerShell-implicit-CRLF fix, Pitfall CLIP-1).
fn trim_one_trailing_newline(mut s: Vec<u8>) -> Vec<u8> {
    if s.last() == Some(&b'\n') {
        s.pop();
        if s.last() == Some(&b'\r') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-05 — a CRLF terminator (`x\r\n`, the PowerShell-pipe case) is stripped as
    /// one unit down to `x`.
    #[test]
    fn trims_one_crlf() {
        assert_eq!(trim_one_trailing_newline(b"x\r\n".to_vec()), b"x");
    }

    /// D-05 — a lone LF terminator (`x\n`) is stripped down to `x`.
    #[test]
    fn trims_one_lf() {
        assert_eq!(trim_one_trailing_newline(b"x\n".to_vec()), b"x");
    }

    /// D-05 — no trailing terminator is a no-op (`x` stays `x`).
    #[test]
    fn no_terminator_is_noop() {
        assert_eq!(trim_one_trailing_newline(b"x".to_vec()), b"x");
    }

    /// D-05 — only ONE terminator is stripped: `x\n\n` → `x\n` (the second newline
    /// is preserved, never collapsed).
    #[test]
    fn strips_only_one_of_two_lf() {
        assert_eq!(trim_one_trailing_newline(b"x\n\n".to_vec()), b"x\n");
    }

    /// D-05 — empty input is a no-op (no panic on an empty `Vec`).
    #[test]
    fn empty_is_noop() {
        assert_eq!(trim_one_trailing_newline(b"".to_vec()), b"");
    }

    /// D-05 — interior newlines are NEVER stripped: `a\nb` (no trailing terminator)
    /// is returned unchanged.
    #[test]
    fn interior_newline_preserved() {
        assert_eq!(trim_one_trailing_newline(b"a\nb".to_vec()), b"a\nb");
    }
}
