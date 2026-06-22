//! The `tree` command: print a directory tree with box-drawing glyphs, colored
//! directory names, an optional per-file size column, and a `--depth N` cap
//! (TREE-01).
//!
//! Render model (RESEARCH Pattern 3, D-08/D-09/D-10):
//! - Walk each directory's immediate children via `WalkDir` so the shared
//!   `core::fs::is_hidden` filter is reused **verbatim** (D-06 — never
//!   re-implemented; it exempts the root so a dotted target isn't pruned to
//!   zero, walkdir#142) and symlinks are never followed (`follow_links(false)`,
//!   T-03-05).
//! - Children are partitioned with the D-08 comparator: directories first, then
//!   files, each case-insensitive alphabetical — so the render is deterministic
//!   regardless of `read_dir` order.
//! - Box-drawing prefixes are computed from "is this the last child at this
//!   level": `└── ` (last) vs `├── ` (non-last); the continuation from each
//!   ancestor is `│   ` where that ancestor was non-last and `    ` (gap) where
//!   it was last. These four constants are STRUCTURE (Unicode is correct here),
//!   distinct from flatten's ASCII status glyphs (D-09).
//! - Only **directory names** are colored (`.blue().bold()`), gated on
//!   `is_color_on()` so piped output is byte-identical minus ANSI (D-10). File
//!   names and the branch glyphs are default-colored.
//! - `--sizes` shows `core::output::human_size` per FILE only; directories show a
//!   blank size column (recursive dir totals are du's job, D-10).
//! - `--depth N` caps the DISPLAYED depth. The trailing `N directories, M files`
//!   summary (GNU `tree` convention) counts every shown dir/file, printed to
//!   stdout after the tree (FOUND-03 — rows + summary to stdout, errors to
//!   stderr).

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;
use owo_colors::OwoColorize;
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::{is_hidden, normalize_path};
use crate::core::output::{human_size, is_color_on};

/// Box-drawing glyphs (RESEARCH Pattern 3 — STRUCTURE, so Unicode is correct,
/// distinct from flatten's ASCII status glyphs, D-09).
const TEE: &str = "├── ";
const ELL: &str = "└── ";
const PIPE: &str = "│   ";
const GAP: &str = "    ";

/// `box tree [PATH] [--sizes] [--depth N]` — print a directory tree (TREE-01).
#[derive(Debug, Args)]
pub struct TreeArgs {
    /// Directory to render (default: the current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Show a per-file size column (directories show a blank size).
    #[arg(long)]
    pub sizes: bool,

    /// Limit the displayed tree to this depth (the root is depth 0).
    #[arg(long)]
    pub depth: Option<usize>,
}

/// One child entry to render: its display name, whether it is a directory, and
/// its byte size (files only; `None` for directories).
struct Child {
    name: String,
    is_dir: bool,
    size: Option<u64>,
    path: PathBuf,
}

/// Running totals for the trailing summary line.
#[derive(Default)]
struct Counts {
    dirs: usize,
    files: usize,
}

impl RunCommand for TreeArgs {
    fn run(self) -> anyhow::Result<()> {
        // Normalize via dunce so we never leak a `\\?\` UNC prefix (FOUND-06,
        // T-03-07).
        let root = normalize_path(&self.path)
            .with_context(|| format!("resolving {}", self.path.display()))?;

        // Print the root label as the path the user passed (not the canonical
        // absolute path) so the render reads naturally, matching GNU `tree`.
        let root_label = self.path.to_string_lossy().to_string();
        println!("{}", color_dir(&root_label));

        let mut counts = Counts::default();
        render_dir(&root, "", self.depth, 1, self.sizes, &mut counts)?;

        println!();
        println!("{} directories, {} files", counts.dirs, counts.files);
        Ok(())
    }
}

/// Recursively render the children of `dir`. `prefix` is the accumulated
/// continuation string from the ancestors (`│   ` / `    ` segments); `max_depth`
/// is the optional displayed-depth cap; `depth` is the current depth (root is 0,
/// so its immediate children are depth 1). Tallies every shown dir/file into
/// `counts`.
fn render_dir(
    dir: &Path,
    prefix: &str,
    max_depth: Option<usize>,
    depth: usize,
    sizes: bool,
    counts: &mut Counts,
) -> anyhow::Result<()> {
    // Stop once we'd exceed the displayed-depth cap (root is depth 0).
    if let Some(max) = max_depth {
        if depth > max {
            return Ok(());
        }
    }

    let children = read_children(dir)?;
    let last_idx = children.len().saturating_sub(1);

    for (i, child) in children.iter().enumerate() {
        let is_last = i == last_idx;
        let branch = if is_last { ELL } else { TEE };

        // The name, colored only for directories (D-10).
        let name = if child.is_dir {
            color_dir(&child.name)
        } else {
            child.name.clone()
        };

        // `--sizes`: per-file human_size, blank for directories (D-10).
        let size_col = if sizes {
            match child.size {
                Some(bytes) => format!("  {}", human_size(bytes)),
                None => String::new(),
            }
        } else {
            String::new()
        };

        println!("{prefix}{branch}{name}{size_col}");

        if child.is_dir {
            counts.dirs += 1;
            // The continuation for this child's subtree: `│   ` if more siblings
            // follow, else `    ` (gap).
            let child_prefix = format!("{prefix}{}", if is_last { GAP } else { PIPE });
            render_dir(
                &child.path,
                &child_prefix,
                max_depth,
                depth + 1,
                sizes,
                counts,
            )?;
        } else {
            counts.files += 1;
        }
    }

    Ok(())
}

/// Read `dir`'s immediate children (hidden pruned via the shared `is_hidden`,
/// symlinks not followed), returning them sorted directories-first then
/// case-insensitive alphabetical (D-08).
fn read_children(dir: &Path) -> anyhow::Result<Vec<Child>> {
    // WalkDir at exactly depth 1 gives the immediate children as
    // `walkdir::DirEntry`s, so `core::fs::is_hidden` is reused VERBATIM (D-06)
    // and `follow_links(false)` keeps symlinks undescended (T-03-05).
    let mut children = Vec::new();
    for entry in WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_dir();
        // File size for the `--sizes` column; directories carry no size.
        let size = if is_dir {
            None
        } else {
            entry.metadata().ok().map(|m| m.len())
        };
        children.push(Child {
            name,
            is_dir,
            size,
            path: entry.into_path(),
        });
    }

    sort_children(&mut children);
    Ok(children)
}

/// Sort children directories-first, then case-insensitive alphabetical (D-08).
fn sort_children(children: &mut [Child]) {
    children.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Color a directory name `.blue().bold()` when color is on, else return it plain
/// — the single styled token in tree, gated so piped output is byte-identical
/// minus ANSI (D-10).
fn color_dir(name: &str) -> String {
    if is_color_on() {
        name.blue().bold().to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(name: &str, is_dir: bool) -> Child {
        Child {
            name: name.to_string(),
            is_dir,
            size: None,
            path: PathBuf::from(name),
        }
    }

    #[test]
    fn dirs_sort_before_files_then_alpha() {
        // Mixed dirs/files in scrambled, mixed-case order.
        let mut v = vec![
            child("Zebra.txt", false),
            child("alpha", true),
            child("README.md", false),
            child("Beta", true),
            child("apple.rs", false),
        ];
        sort_children(&mut v);
        let order: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        // Directories first (alpha, Beta — case-insensitive), then files
        // (apple.rs, README.md, Zebra.txt — case-insensitive).
        assert_eq!(
            order,
            vec!["alpha", "Beta", "apple.rs", "README.md", "Zebra.txt"]
        );
    }

    #[test]
    fn color_dir_is_plain_when_color_off() {
        // init_color defaults COLOR_ON to false; a styled dir name must carry no
        // ANSI in the plain path (byte-identical minus ANSI, D-10).
        let s = color_dir("src");
        assert_eq!(s, "src");
        assert!(!s.contains('\x1b'), "plain dir name must contain no ANSI");
    }

    #[test]
    fn glyph_constants_are_box_drawing() {
        // The four STRUCTURE glyphs are Unicode box-drawing (D-09), distinct from
        // flatten's ASCII +/~/- status glyphs.
        assert_eq!(TEE, "├── ");
        assert_eq!(ELL, "└── ");
        assert_eq!(PIPE, "│   ");
        assert_eq!(GAP, "    ");
    }
}
