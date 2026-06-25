//! The `du` command: a size-sorted (biggest-first) disk-usage view with one row
//! per IMMEDIATE child of the target directory (DU-01).
//!
//! Row model (RESEARCH du Code Example, D-11):
//! - One row per immediate child of the target dir. A FILE child shows its own
//!   `metadata().len()`; a DIRECTORY child shows the RECURSIVE sum of all
//!   non-hidden descendant file sizes (logical size, RESEARCH A4 — not the
//!   allocated/on-disk size; apparent-size is DU-V2).
//! - The recursive descendant sum reuses the shared walk: `WalkDir` with
//!   `follow_links(false)` + `filter_entry(!is_hidden)` so the dotted-root
//!   exemption (walkdir#142, T-03-10) and symlink-loop safety (T-03-09) come for
//!   free — `core::fs::is_hidden` is reused VERBATIM, never re-implemented
//!   (D-06, RESEARCH Pitfall 7).
//!
//! Determinism (RESEARCH Pitfall 6, T-03-12): the rows are `collect`ed and then
//! `sort_by` `(size desc, name asc)` BEFORE printing — NEVER the walk order. The
//! test fixtures use distinct child sizes so the order is a TOTAL order.
//!
//! Flags:
//! - `--depth N` is the AGGREGATION cap: a directory's recursive total rolls up
//!   descendant files no deeper than N levels below that child (the child's own
//!   files are depth 1).
//! - `--top N` is a POST-SORT truncation of the SHOWN list; it does NOT change
//!   the summary total.
//!
//! Output (CONTEXT § specifics, D-11/D-12): the size column is right-aligned to
//! the widest SHOWN `core::output::human_size` value; a trailing `/` (ASCII, so
//! the dir/file distinction survives piping) marks directory rows; only the size
//! VALUE is colored (single `.cyan()` accent) gated on `is_color_on()` — the
//! path/name is uncolored. The summary `{X} of {Y} entries shown. {TOTAL} total.`
//! ALWAYS reflects the FULL scan total (X = rows shown after `--top`, Y = all
//! immediate children, TOTAL = the full-scan sum). Rows + summary go to stdout
//! (FOUND-03).

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;
use owo_colors::OwoColorize;
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::{is_hidden, normalize_path};
use crate::core::output::{human_size, is_color_on};

/// `box du [PATH] [--top N] [--depth N]` — a biggest-first disk-usage view
/// (DU-01).
#[derive(Debug, Args)]
pub struct DuArgs {
    /// Directory to analyze (default: the current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Show only the N biggest entries (post-sort truncation of the shown rows;
    /// the summary total still reflects the full scan). Must be >= 1 (WR-04).
    #[arg(long, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
    pub top: Option<usize>,

    /// Cap how deep a directory's recursive total is rolled up (the directory's
    /// own files are depth 1). Must be >= 1 (WR-04).
    #[arg(long, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
    pub depth: Option<usize>,
}

/// One immediate-child row: its display name, whether it is a directory, and its
/// size (a file's own size, or a directory's recursive descendant sum).
///
/// `#[derive(serde::Serialize)]` for `box du --json` (SPINE-02, D-11): the SAME
/// rows that feed the human render feed the JSON `.results` array (no-drift).
/// `size` is a BARE `u64` (D-3) — the >2^53 precision caveat is documented; PS7
/// (Int64) handles it.
#[derive(serde::Serialize)]
struct Row {
    /// The child's base name (no trailing `/` — that is added at render time).
    name: String,
    is_dir: bool,
    size: u64,
}

/// The `box du --json` document (D-11): the always-wrapped `{results,count}` shape
/// plus the full-scan sibling totals (`total_bytes` = the full-scan sum,
/// `total_children` = the immediate-child count) — both computed BEFORE the
/// `--top` truncation so they reflect the full scan, exactly like the human
/// summary line.
#[derive(serde::Serialize)]
struct DuOutput {
    results: Vec<Row>,
    count: usize,
    total_bytes: u64,
    total_children: usize,
}

impl RunCommand for DuArgs {
    fn run(self) -> anyhow::Result<()> {
        // Pre-check the common typo path: a non-existent target gives a clear
        // "no such directory: X" instead of dunce's raw `(os error 3)` (WR-03).
        if !self.path.exists() {
            anyhow::bail!("no such directory: {}", self.path.display());
        }

        // Normalize via dunce so we never leak a `\\?\` UNC prefix (FOUND-06,
        // T-03-11).
        let root = normalize_path(&self.path)
            .with_context(|| format!("resolving {}", self.path.display()))?;

        // `du` reports one row per immediate child: a FILE argument has none, so it
        // would silently print `0 of 0 entries shown. 0 B total.`. Refuse it with a
        // clear error instead (WR-02).
        if !root.is_dir() {
            anyhow::bail!("{} is not a directory", self.path.display());
        }

        // Build one row per immediate child (file = own size, dir = recursive
        // descendant sum capped by --depth).
        let mut rows = collect_rows(&root, self.depth)?;

        // The full-scan total is the sum over ALL immediate children — captured
        // BEFORE any --top truncation so the summary always reflects the full
        // scan (D-11).
        let total: u64 = rows.iter().map(|r| r.size).sum();
        let total_children = rows.len();

        // Determinism (RESEARCH Pitfall 6): sort by (size desc, name asc) BEFORE
        // printing — never the walk order.
        sort_rows(&mut rows);

        // --top N: post-sort truncation of the SHOWN list (does NOT change the
        // summary total). Applies to BOTH paths — the JSON `.results` honor `--top`
        // exactly like the printed rows, while the sibling totals stay full-scan.
        if let Some(top) = self.top {
            rows.truncate(top);
        }
        let shown = rows.len();

        // Fork on `is_json_on()` FIRST (Pitfall 1): du has THREE human stdout
        // writes (rows + blank line + summary), ALL of which must live behind the
        // `else`. Under --json the ONLY stdout write is the single emit_json.
        if crate::core::output::is_json_on() {
            let doc = DuOutput {
                count: shown,
                results: rows,
                total_bytes: total,
                total_children,
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        // INVARIANT (WR-04): every `println!` below is reachable ONLY when
        // `!is_json_on()` — the `is_json_on()` fork above already `return`ed under
        // `--json`. These raw prints intentionally bypass `out_line` (du is NOT a
        // SPINE-04 `--clip` command, so its human render must not tee to the
        // clipboard). If any human write is ever moved ABOVE the fork, it would
        // leak chrome into the JSON channel — keep them strictly below it.
        //
        // Right-align the size column to the widest SHOWN human_size value (D-12).
        let size_strings: Vec<String> = rows.iter().map(|r| human_size(r.size)).collect();
        let size_width = size_strings.iter().map(|s| s.len()).max().unwrap_or(0);

        for (row, size_str) in rows.iter().zip(size_strings.iter()) {
            // Right-pad to the column width, then color ONLY the size value
            // (single accent, gated so piped output is byte-identical minus ANSI).
            let padded = format!("{size_str:>size_width$}");
            let size_col = color_size(&padded);
            // Trailing `/` (ASCII) marks directories so the distinction survives
            // piping (D-11).
            let slash = if row.is_dir { "/" } else { "" };
            println!("{size_col}  {}{slash}", row.name);
        }

        // The summary ALWAYS reflects the FULL scan total, not just the shown
        // rows (D-11).
        println!();
        println!(
            "{shown} of {total_children} entries shown. {} total.",
            human_size(total)
        );
        Ok(())
    }
}

/// Enumerate the immediate children of `root` (hidden pruned via the shared
/// `is_hidden`, symlinks not followed) and compute one [`Row`] per child: a file
/// row carries its own size; a directory row carries the recursive sum of its
/// non-hidden descendant files, rolled up no deeper than `depth` (when set).
fn collect_rows(root: &Path, depth: Option<usize>) -> anyhow::Result<Vec<Row>> {
    // WalkDir at exactly depth 1 gives the immediate children as
    // `walkdir::DirEntry`s, so `core::fs::is_hidden` is reused VERBATIM (D-06)
    // and `follow_links(false)` keeps symlinks undescended (T-03-09).
    let mut rows = Vec::new();
    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry.with_context(|| format!("reading {}", root.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_dir();
        let size = if is_dir {
            dir_total(entry.path(), depth)?
        } else {
            // A file's own logical size (RESEARCH A4 — metadata().len()).
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };
        rows.push(Row { name, is_dir, size });
    }
    Ok(rows)
}

/// The recursive sum of all non-hidden descendant FILE sizes under `dir`, rolled
/// up no deeper than `depth` levels (the directory's own files are depth 1).
/// Symlinks are never followed (T-03-09) and hidden entries are pruned (D-06) —
/// the same shared walk the whole toolbox uses.
fn dir_total(dir: &Path, depth: Option<usize>) -> anyhow::Result<u64> {
    // `dir` itself is walkdir depth 0; its immediate files are depth 1. Cap the
    // descent at `depth` when set so --depth N bounds the rolled-up total.
    let mut walker = WalkDir::new(dir).min_depth(1).follow_links(false);
    if let Some(max) = depth {
        walker = walker.max_depth(max);
    }

    let mut total: u64 = 0;
    for entry in walker.into_iter().filter_entry(|e| !is_hidden(e)) {
        let entry = entry.with_context(|| format!("scanning {}", dir.display()))?;
        // Sum only regular files' logical sizes; directories contribute via their
        // children, symlinks are skipped (never followed).
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}

/// Sort rows by `(size desc, name asc)` — biggest-first, ties broken by
/// case-insensitive name so the order is deterministic (RESEARCH Pitfall 6).
fn sort_rows(rows: &mut [Row]) {
    rows.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Color the (already right-aligned) size token `.cyan()` when color is on, else
/// return it plain — the single styled token in du, gated so piped output is
/// byte-identical minus ANSI (D-11).
fn color_size(size: &str) -> String {
    if is_color_on() {
        size.cyan().to_string()
    } else {
        size.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, is_dir: bool, size: u64) -> Row {
        Row {
            name: name.to_string(),
            is_dir,
            size,
        }
    }

    #[test]
    fn sort_is_biggest_first_then_name() {
        // Mixed sizes incl. a tie (200 == 200) broken by case-insensitive name.
        let mut v = vec![
            row("small", false, 100),
            row("Big", true, 5000),
            row("zeta", false, 200),
            row("Alpha", false, 200),
            row("mid", true, 1500),
        ];
        sort_rows(&mut v);
        let order: Vec<&str> = v.iter().map(|r| r.name.as_str()).collect();
        // 5000, 1500, then the 200-tie (Alpha < zeta, case-insensitive), then 100.
        assert_eq!(order, vec!["Big", "mid", "Alpha", "zeta", "small"]);
    }

    #[test]
    fn color_size_is_plain_when_color_off() {
        // init_color defaults COLOR_ON to false; the size token must carry no ANSI
        // in the plain path (byte-identical minus ANSI, D-11).
        let s = color_size("4.9 KB");
        assert_eq!(s, "4.9 KB");
        assert!(!s.contains('\x1b'), "plain size token must contain no ANSI");
    }

    #[test]
    fn dir_total_sums_recursive_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // root/a.bin (3000) + root/nested/b.bin (2000) = 5000 recursive.
        std::fs::write(root.join("a.bin"), vec![b'a'; 3000]).unwrap();
        std::fs::create_dir(root.join("nested")).unwrap();
        std::fs::write(root.join("nested").join("b.bin"), vec![b'b'; 2000]).unwrap();

        // No depth cap: full recursive sum.
        assert_eq!(dir_total(root, None).unwrap(), 5000);
        // --depth 1: only the immediate files (a.bin, depth 1); nested/b.bin
        // (depth 2) is excluded -> 3000.
        assert_eq!(dir_total(root, Some(1)).unwrap(), 3000);
        // --depth 2: includes the depth-2 file -> full 5000.
        assert_eq!(dir_total(root, Some(2)).unwrap(), 5000);
    }

    #[test]
    fn dir_total_skips_hidden_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("visible.bin"), vec![b'v'; 1000]).unwrap();
        // A dot-prefixed file is hidden (D-06) and must NOT count toward the total.
        std::fs::write(root.join(".hidden.bin"), vec![b'h'; 9999]).unwrap();
        assert_eq!(dir_total(root, None).unwrap(), 1000);
    }
}
