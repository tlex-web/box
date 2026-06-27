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
//! Output (CONTEXT § specifics, D-11/D-12, DU-V2-01): each row prints a
//! right-aligned `NN.N%` column (the size's share of the full-scan total) BEFORE
//! the size value; the size value is colored by percentage band — `>50%` red,
//! `10–50%` yellow, else default (REPLACING the v1 lone `.cyan()` accent) — gated
//! on `is_color_on()` so piped/`--json` output is byte-identical minus ANSI; a
//! trailing `/` (ASCII) marks directory rows; the name is uncolored. The summary
//! `{X} of {Y} entries shown. {TOTAL} total.` ALWAYS reflects the FULL scan total
//! (X = rows shown after `--top`, Y = all immediate children, TOTAL = the
//! full-scan sum). Rows + summary go to stdout (FOUND-03).
//!
//! Depth flags (DU-V2-01 / DU-V2-02):
//! - `--exclude '<glob>'` (`globset`, repeatable) drops a matching IMMEDIATE child
//!   (no row) AND keeps a matching DESCENDANT out of every directory total —
//!   matched on the path relative to the target root (T-8-02-GLOB).
//! - `--on-disk` reports each file's allocated/compressed NTFS size via Win32
//!   `GetCompressedFileSizeW`; a directory row sums its descendants' on-disk sizes
//!   and the percentage basis switches to the on-disk total. JSON `size` carries
//!   the on-disk bytes and the document gains a top-level `on_disk: true` marker
//!   (A2 — no raw `f64` percent field, so JSON never carries `NaN`, Pitfall 3).

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;
use globset::{Glob, GlobSet, GlobSetBuilder};
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

    /// Exclude paths matching this glob (repeatable, e.g. `--exclude '*.log'
    /// --exclude node_modules`). Matched on the path relative to the target root;
    /// drops matching immediate children AND excludes matching descendants from
    /// directory totals (DU-V2-01).
    #[arg(long, value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Report allocated/compressed on-disk (NTFS) size via Win32
    /// `GetCompressedFileSizeW` instead of the logical file size (DU-V2-02).
    #[arg(long = "on-disk")]
    pub on_disk: bool,
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
    /// `true` when sizes are the allocated/compressed on-disk bytes (`--on-disk`),
    /// `false` for logical apparent sizes (DU-V2-02 / A2). A top-level marker so
    /// the consumer can tell which basis `size`/`total_bytes` carry, WITHOUT a raw
    /// `f64` percent field (which would risk `NaN`-in-JSON, Pitfall 3).
    on_disk: bool,
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

        // Compile the --exclude globs ONCE (a malformed glob is a clean error,
        // exit 1, never a panic — T-8-02-GLOB). An empty set matches nothing.
        let exclude = build_exclude(&self.exclude)?;

        // Build one row per immediate child (file = own size, dir = recursive
        // descendant sum capped by --depth). When --on-disk is set every size is
        // the allocated/compressed Win32 size instead of the logical length.
        let mut rows = collect_rows(&root, self.depth, &exclude, self.on_disk)?;

        // The full-scan total is the sum over ALL immediate children — captured
        // BEFORE any --top truncation so the summary always reflects the full
        // scan (D-11). With --on-disk it is the on-disk total, so the percentage
        // basis stays internally consistent (D-23).
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
                on_disk: self.on_disk,
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
        // Right-align BOTH the percentage column and the size column to their
        // widest SHOWN values (D-12). The percentage (a render-only value from the
        // full-scan `total`, A2) sits BEFORE the size.
        let pct_strings: Vec<String> = rows.iter().map(|r| percent_str(r.size, total)).collect();
        let pct_width = pct_strings.iter().map(|s| s.len()).max().unwrap_or(0);
        let size_strings: Vec<String> = rows.iter().map(|r| human_size(r.size)).collect();
        let size_width = size_strings.iter().map(|s| s.len()).max().unwrap_or(0);

        for ((row, size_str), pct_str) in rows.iter().zip(size_strings.iter()).zip(pct_strings.iter())
        {
            // Right-pad the percentage column.
            let pct_col = format!("{pct_str:>pct_width$}");
            // Right-pad the size, then color ONLY the size value by its percentage
            // band (gated so piped output is byte-identical minus ANSI, D-11).
            let padded = format!("{size_str:>size_width$}");
            let size_col = band_color(&padded, row.size, total);
            // Trailing `/` (ASCII) marks directories so the distinction survives
            // piping (D-11).
            let slash = if row.is_dir { "/" } else { "" };
            println!("{pct_col}  {size_col}  {}{slash}", row.name);
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
/// non-hidden descendant files, rolled up no deeper than `depth` (when set). An
/// immediate child whose path (relative to `root`) matches `exclude` is dropped
/// entirely (no row). When `on_disk` is set every size is the allocated/compressed
/// Win32 size instead of the logical length.
fn collect_rows(
    root: &Path,
    depth: Option<usize>,
    exclude: &GlobSet,
    on_disk: bool,
) -> anyhow::Result<Vec<Row>> {
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
        // --exclude: drop a matching immediate child (no row), matched on the path
        // relative to the target root (T-8-02-GLOB).
        if is_excluded(exclude, root, entry.path()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_dir();
        let size = if is_dir {
            dir_total(root, entry.path(), depth, exclude, on_disk)?
        } else if on_disk {
            compressed_size(entry.path())?
        } else {
            // A file's own logical size (RESEARCH A4 — metadata().len()).
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };
        rows.push(Row { name, is_dir, size });
    }
    Ok(rows)
}

/// The recursive sum of non-hidden descendant FILE sizes under `dir`, rolled up no
/// deeper than `depth` levels (the directory's own files are depth 1). A descendant
/// whose path (relative to the target `root`) matches `exclude` never counts toward
/// the total. With `on_disk` each file contributes its `GetCompressedFileSizeW`
/// allocated size instead of `metadata().len()`. Symlinks are never followed
/// (T-03-09) and hidden entries are pruned (D-06) — the same shared walk.
fn dir_total(
    root: &Path,
    dir: &Path,
    depth: Option<usize>,
    exclude: &GlobSet,
    on_disk: bool,
) -> anyhow::Result<u64> {
    // `dir` itself is walkdir depth 0; its immediate files are depth 1. Cap the
    // descent at `depth` when set so --depth N bounds the rolled-up total.
    let mut walker = WalkDir::new(dir).min_depth(1).follow_links(false);
    if let Some(max) = depth {
        walker = walker.max_depth(max);
    }

    let mut total: u64 = 0;
    for entry in walker.into_iter().filter_entry(|e| !is_hidden(e)) {
        let entry = entry.with_context(|| format!("scanning {}", dir.display()))?;
        // --exclude: a matching descendant never counts toward the total, matched
        // on the path relative to the target root (T-8-02-GLOB).
        if is_excluded(exclude, root, entry.path()) {
            continue;
        }
        // Sum only regular files; directories contribute via their children,
        // symlinks are skipped (never followed).
        if entry.file_type().is_file() {
            total += if on_disk {
                compressed_size(entry.path())?
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            };
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

/// Render `size`'s share of the full-scan `total` as a right-alignable token
/// (DU-V2-01). `total == 0` (or a genuine zero-byte row) → `0.0%`; a tiny-but-
/// nonzero share → `<0.1%`; otherwise `{:.1}%`. This is a RENDER-only value, never
/// a JSON float, so the divide-by-zero guard keeps `NaN` out of every channel
/// (Pitfall 3 / A2).
fn percent_str(size: u64, total: u64) -> String {
    if total == 0 {
        return "0.0%".to_string();
    }
    let pct = size as f64 / total as f64 * 100.0;
    if pct == 0.0 {
        "0.0%".to_string()
    } else if pct < 0.1 {
        "<0.1%".to_string()
    } else {
        format!("{pct:.1}%")
    }
}

/// Color the (already right-aligned) size token by its percentage band — `>50%`
/// red, `10–50%` yellow, else plain — when color is on, else return it plain. This
/// REPLACES the v1 lone `.cyan()` accent; it is the single styled token in du,
/// gated so piped/`--json` output is byte-identical minus ANSI (D-11 / D-23).
fn band_color(size_token: &str, size: u64, total: u64) -> String {
    if !is_color_on() {
        return size_token.to_string();
    }
    let pct = if total == 0 {
        0.0
    } else {
        size as f64 / total as f64 * 100.0
    };
    if pct > 50.0 {
        size_token.red().to_string()
    } else if pct >= 10.0 {
        size_token.yellow().to_string()
    } else {
        size_token.to_string()
    }
}

/// Compile the `--exclude` globs into a single [`GlobSet`] (DU-V2-01). An empty
/// list yields an empty set (matches nothing). A malformed glob is a clean
/// `anyhow` error (exit 1), never a panic (T-8-02-GLOB).
fn build_exclude(globs: &[String]) -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let glob = Glob::new(g).with_context(|| format!("invalid --exclude glob: {g}"))?;
        builder.add(glob);
    }
    builder.build().context("building --exclude glob set")
}

/// Whether `path` (made relative to the target `root`) matches the `exclude` set.
/// A no-op (always `false`) for an empty set, so the default walk is unchanged.
fn is_excluded(exclude: &GlobSet, root: &Path, path: &Path) -> bool {
    if exclude.is_empty() {
        return false;
    }
    let rel = path.strip_prefix(root).unwrap_or(path);
    exclude.is_match(rel)
}

/// The allocated/compressed on-disk size of one file via Win32
/// `GetCompressedFileSizeW` (DU-V2-02, RESEARCH Pattern 3). NTFS sparse/compressed
/// files report fewer bytes than their logical length; a normal file reports its
/// cluster-rounded allocation. `INVALID_FILE_SIZE` (`0xFFFFFFFF`) is ALSO a legal
/// low dword, so it is only an error when `GetLastError` is not `NO_ERROR`.
#[cfg(windows)]
fn compressed_size(path: &Path) -> anyhow::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, NO_ERROR};
    use windows::Win32::Storage::FileSystem::GetCompressedFileSizeW;

    const INVALID_FILE_SIZE: u32 = u32::MAX;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut high: u32 = 0;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path; `&mut high` is a valid
    // out-param. The call is a read-only metadata query (no handle retained, no OS
    // state registered) — T-8-02-FFI.
    let low = unsafe { GetCompressedFileSizeW(PCWSTR(wide.as_ptr()), Some(&mut high)) };
    if low == INVALID_FILE_SIZE {
        // SAFETY: reads the calling thread's last-error code.
        let err = unsafe { GetLastError() };
        if err != NO_ERROR {
            anyhow::bail!(
                "GetCompressedFileSizeW failed for {}: {:?}",
                path.display(),
                err
            );
        }
    }
    Ok(((high as u64) << 32) | (low as u64))
}

/// Non-Windows fallback (the project targets Windows, but this keeps the module
/// portable for `cargo check`/tests on other hosts): the logical length stands in
/// for the on-disk size.
#[cfg(not(windows))]
fn compressed_size(path: &Path) -> anyhow::Result<u64> {
    Ok(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))
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
    fn band_color_is_plain_when_color_off() {
        // init_color defaults COLOR_ON to false; the size token must carry no ANSI
        // in the plain path (byte-identical minus ANSI, D-11) regardless of band.
        let s = band_color("4.9 KB", 9000, 10000); // >50% -> would be red if color on
        assert_eq!(s, "4.9 KB");
        assert!(!s.contains('\x1b'), "plain size token must contain no ANSI");
    }

    #[test]
    fn percent_str_formats_and_guards_nan() {
        // Divide-by-zero guard: total == 0 -> 0.0% (never NaN), Pitfall 3.
        assert_eq!(percent_str(0, 0), "0.0%");
        assert_eq!(percent_str(123, 0), "0.0%");
        // Genuine zero-byte row -> 0.0%.
        assert_eq!(percent_str(0, 1000), "0.0%");
        // Tiny-but-nonzero -> <0.1% (1 of 10000 = 0.01%).
        assert_eq!(percent_str(1, 10000), "<0.1%");
        // Ordinary shares -> one decimal place.
        assert_eq!(percent_str(5000, 10000), "50.0%");
        assert_eq!(percent_str(1000, 7000), "14.3%");
        assert_eq!(percent_str(10000, 10000), "100.0%");
    }

    #[test]
    fn empty_exclude_matches_nothing() {
        let set = build_exclude(&[]).unwrap();
        let root = Path::new("C:/proj");
        assert!(!is_excluded(&set, root, Path::new("C:/proj/anything.log")));
    }

    #[test]
    fn exclude_matches_relative_path() {
        let set = build_exclude(&["*.log".to_string()]).unwrap();
        let root = Path::new("C:/proj");
        // A descendant *.log matches on its path relative to the root.
        assert!(is_excluded(&set, root, Path::new("C:/proj/sub/app.log")));
        // A non-matching extension does not.
        assert!(!is_excluded(&set, root, Path::new("C:/proj/sub/app.txt")));
    }

    #[test]
    fn malformed_exclude_glob_is_clean_error() {
        // An invalid glob is a clean Err (exit 1), never a panic (T-8-02-GLOB).
        let err = build_exclude(&["a[".to_string()]).expect_err("invalid glob must error");
        let msg = format!("{err:#}");
        assert!(msg.contains("--exclude"), "error should name the bad flag: {msg}");
    }

    #[test]
    fn dir_total_sums_recursive_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // root/a.bin (3000) + root/nested/b.bin (2000) = 5000 recursive.
        std::fs::write(root.join("a.bin"), vec![b'a'; 3000]).unwrap();
        std::fs::create_dir(root.join("nested")).unwrap();
        std::fs::write(root.join("nested").join("b.bin"), vec![b'b'; 2000]).unwrap();

        let none = GlobSet::empty();
        // No depth cap: full recursive sum.
        assert_eq!(dir_total(root, root, None, &none, false).unwrap(), 5000);
        // --depth 1: only the immediate files (a.bin, depth 1); nested/b.bin
        // (depth 2) is excluded -> 3000.
        assert_eq!(dir_total(root, root, Some(1), &none, false).unwrap(), 3000);
        // --depth 2: includes the depth-2 file -> full 5000.
        assert_eq!(dir_total(root, root, Some(2), &none, false).unwrap(), 5000);
    }

    #[test]
    fn dir_total_excludes_matching_descendant() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.bin"), vec![b'a'; 500]).unwrap();
        std::fs::write(root.join("b.log"), vec![b'b'; 3000]).unwrap();
        // --exclude '*.log' keeps b.log out of the total -> 500 (only a.bin).
        let set = build_exclude(&["*.log".to_string()]).unwrap();
        assert_eq!(dir_total(root, root, None, &set, false).unwrap(), 500);
    }

    #[test]
    fn dir_total_skips_hidden_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("visible.bin"), vec![b'v'; 1000]).unwrap();
        // A dot-prefixed file is hidden (D-06) and must NOT count toward the total.
        std::fs::write(root.join(".hidden.bin"), vec![b'h'; 9999]).unwrap();
        assert_eq!(
            dir_total(root, root, None, &GlobSet::empty(), false).unwrap(),
            1000
        );
    }
}
