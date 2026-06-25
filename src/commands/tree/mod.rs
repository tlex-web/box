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

    /// Limit the displayed tree to this depth (the root is depth 0). Must be >= 1
    /// (WR-04) — `--depth 0` would show only the root, almost certainly a typo.
    #[arg(long, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
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

/// One node of the `box tree --json` recursive document (SPINE-02, A4 / D-17 —
/// the ROOT-RULE EXCEPTION: a recursive object, NOT `{results,count}`). `kind`
/// serializes to the string `"dir"` or `"file"` under the JSON key `type`; `size`
/// is `Some` for FILES only and OMITTED for directories (`skip_serializing_if`).
/// `name` is a lossy string (D-4). This is a REAL node tree (the A4 surprise) — the
/// current human printer never builds one, so [`build_node`] is new work that
/// shares `read_children`/`sort_children` with the printer so the two cannot drift.
#[derive(serde::Serialize)]
struct Node {
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    children: Vec<Node>,
}

/// Running totals for the trailing summary line.
#[derive(Default)]
struct Counts {
    dirs: usize,
    files: usize,
}

impl RunCommand for TreeArgs {
    fn run(self) -> anyhow::Result<()> {
        // Pre-check the common typo path: a non-existent target gives a clear
        // "no such directory: X" instead of dunce's raw `(os error 3)` (WR-03).
        if !self.path.exists() {
            anyhow::bail!("no such directory: {}", self.path.display());
        }

        // Normalize via dunce so we never leak a `\\?\` UNC prefix (FOUND-06,
        // T-03-07).
        let root = normalize_path(&self.path)
            .with_context(|| format!("resolving {}", self.path.display()))?;

        // `tree` is a directory-analysis tool: a FILE argument would walk to zero
        // children and silently print an empty tree with a `0 directories, 0
        // files` summary. Refuse it with a clear error instead (WR-02).
        if !root.is_dir() {
            anyhow::bail!("{} is not a directory", self.path.display());
        }

        // The root label is the path the user passed (not the canonical absolute
        // path) so the render reads naturally, matching GNU `tree`.
        let root_label = self.path.to_string_lossy().to_string();

        // Fork on `is_json_on()` FIRST (Pitfall 1): tree has FOUR human stdout
        // writes (root label + tree + blank + summary), ALL of which must live
        // behind the `else`. Under --json the ONLY stdout write is emit_json, of a
        // REAL recursive node tree (A4) — NOT the {results,count} shape (D-17
        // root-rule exception). The root node is the target dir; its children
        // recurse with the SAME read_children/sort_children the printer uses, so
        // JSON order matches human order (no-drift).
        if crate::core::output::is_json_on() {
            let root_node = build_node(&root, root_label, true, None, self.depth, 1)?;
            crate::core::output::emit_json(&root_node)?;
            return Ok(());
        }

        // INVARIANT (WR-04): every `println!` from here on (root label, the
        // `render_dir` tree, the blank line, and the summary) is reachable ONLY
        // when `!is_json_on()` — the `is_json_on()` fork above already `return`ed
        // under `--json`. These raw prints intentionally bypass `out_line` (tree
        // is NOT a SPINE-04 `--clip` command, so its human render must not tee to
        // the clipboard). Keep all human writes strictly below the fork so none
        // ever leak into the JSON channel.
        //
        // Print the root label, then render the tree below it.
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

/// Build the recursive `box tree --json` [`Node`] for `dir` (A4 / D-17). `name` is
/// the node's display label, `is_dir` whether this node is a directory, `size` its
/// byte size (files only). For a directory, recurse into its children using the
/// SAME `read_children` + `sort_children` helpers the human printer uses (so the
/// JSON child order matches the rendered order — no-drift), honoring the `--depth`
/// cap exactly like `render_dir`: a directory AT the cap depth still appears as a
/// node but its children (one level deeper) are not descended.
fn build_node(
    dir: &Path,
    name: String,
    is_dir: bool,
    size: Option<u64>,
    max_depth: Option<usize>,
    depth: usize,
) -> anyhow::Result<Node> {
    // A file is a leaf — it carries its size and no children.
    if !is_dir {
        return Ok(Node {
            name,
            kind: "file",
            size,
            children: Vec::new(),
        });
    }

    // A directory: descend unless this child's subtree would exceed the displayed-
    // depth cap (same boundary as render_dir, which stops once `depth > max`).
    let descend = match max_depth {
        Some(max) => depth <= max,
        None => true,
    };

    let mut children = Vec::new();
    if descend {
        for child in read_children(dir)? {
            children.push(build_node(
                &child.path,
                child.name,
                child.is_dir,
                child.size,
                max_depth,
                depth + 1,
            )?);
        }
    }

    Ok(Node {
        name,
        kind: "dir",
        // Directories OMIT `.size` (D-17) — `None` is skipped on serialize.
        size: None,
        children,
    })
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
