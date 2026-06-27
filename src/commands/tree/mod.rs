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
//!
//! Depth flags (TREE-V2-01, D-20 — all OPT-IN; with none set the render is
//! byte-identical to v1):
//! - `--gitignore` hides entries matched by every `.gitignore` from the tree root
//!   down to the current directory (a NESTED rule overrides an ancestor — the eza
//!   #1086 class). `--ignore '<glob>'` (repeatable) folds into the SAME matcher.
//! - `--dirs-only` drops file children (applied AFTER the ignore filter).
//! - `--sort size` replaces the D-08 dirs-first comparator with size-descending
//!   (files biggest-first, directories — which carry `size: None` — sorted to the
//!   end); without it the D-08 order is unchanged.
//!
//! Matcher-as-filter (D-20): the gitignore/`--ignore`/`--dirs-only`/`--sort` logic
//! lives inside [`read_children`], the SINGLE chokepoint both [`render_dir`] (human)
//! and [`build_node`] (`--json`) call per directory — so the two recursions cannot
//! drift. We deliberately do NOT switch to the recursive walker from the `ignore`
//! crate (which would re-architect both recursions). Nested correctness uses an
//! ancestor-stack (`Vec<Gitignore>`) checked deepest-first so a deeper rule wins.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
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

    /// Respect `.gitignore` files (root + nested; a deeper rule wins). Opt-in —
    /// the default render is unchanged (TREE-V2-01, D-20).
    #[arg(long)]
    pub gitignore: bool,

    /// Hide entries matching this glob (repeatable, e.g. `--ignore '*.log'
    /// --ignore target`). Folds into the same matcher as `--gitignore`.
    #[arg(long, value_name = "GLOB")]
    pub ignore: Vec<String>,

    /// Show directories only (drop all file children, in both the human and JSON
    /// renders).
    #[arg(long = "dirs-only")]
    pub dirs_only: bool,

    /// Sort order. `name` (default) = directories-first then case-insensitive
    /// alphabetical (D-08); `size` = files biggest-first (directories sorted to
    /// the end).
    #[arg(long, value_name = "MODE")]
    pub sort: Option<SortMode>,
}

/// `--sort` mode (TREE-V2-01). `Name` reproduces the default D-08 dirs-first order;
/// `Size` orders files biggest-first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SortMode {
    /// Directories first, then case-insensitive alphabetical (the v1 default).
    Name,
    /// Files biggest-first; directories (no intrinsic size) sorted to the end.
    Size,
}

/// The per-walk depth-flag options threaded through both recursions so the human
/// and JSON renders apply identical filtering/sorting (no-drift).
struct WalkOpts {
    /// Load and honor `.gitignore` files per directory (`--gitignore`).
    gitignore: bool,
    /// Drop file children (`--dirs-only`).
    dirs_only: bool,
    /// The sort comparator selector (`--sort`).
    sort: Option<SortMode>,
}

/// The immutable walk configuration carried (by `&`) through both recursions, so
/// the per-call argument count stays small (clippy `too_many_arguments`). The
/// mutable threading state (`Counts`, the gitignore `stack`) is passed separately.
struct WalkCtx<'a> {
    /// The displayed-depth cap (`--depth N`); `None` = unbounded.
    max_depth: Option<usize>,
    /// Show the per-file size column (`--sizes`) — consulted by `render_dir` only.
    sizes: bool,
    /// The depth-flag options (`--gitignore`/`--dirs-only`/`--sort`).
    opts: &'a WalkOpts,
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

        // The depth-flag options threaded through BOTH recursions (no-drift).
        let opts = WalkOpts {
            gitignore: self.gitignore,
            dirs_only: self.dirs_only,
            sort: self.sort,
        };

        // Build the initial matcher stack (deepest-first checked at match time):
        // the `--ignore` globs form the SHALLOWEST entry, then the tree root's own
        // `.gitignore` (when `--gitignore`). Children of deeper dirs push their own
        // `.gitignore` as the recursion descends. An empty stack = no filtering, so
        // the default render is byte-identical to v1.
        let mut stack: Vec<Gitignore> = Vec::new();
        if let Some(ig) = build_ignore_matcher(&root, &self.ignore)? {
            stack.push(ig);
        }
        if opts.gitignore {
            if let Some(gi) = load_dir_gitignore(&root)? {
                stack.push(gi);
            }
        }

        // Fork on `is_json_on()` FIRST (Pitfall 1): tree has FOUR human stdout
        // writes (root label + tree + blank + summary), ALL of which must live
        // behind the `else`. Under --json the ONLY stdout write is emit_json, of a
        // REAL recursive node tree (A4) — NOT the {results,count} shape (D-17
        // root-rule exception). The root node is the target dir; its children
        // recurse with the SAME read_children/sort_children the printer uses, so
        // JSON order matches human order (no-drift).
        let ctx = WalkCtx {
            max_depth: self.depth,
            sizes: self.sizes,
            opts: &opts,
        };

        if crate::core::output::is_json_on() {
            let root_node = build_node(&ctx, &root, root_label, true, None, 1, &mut stack)?;
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
        render_dir(&ctx, &root, "", 1, &mut counts, &mut stack)?;

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
    ctx: &WalkCtx,
    dir: &Path,
    prefix: &str,
    depth: usize,
    counts: &mut Counts,
    stack: &mut Vec<Gitignore>,
) -> anyhow::Result<()> {
    // Stop once we'd exceed the displayed-depth cap (root is depth 0).
    if let Some(max) = ctx.max_depth {
        if depth > max {
            return Ok(());
        }
    }

    // `stack` already includes `dir`'s own `.gitignore` (pushed by the caller).
    let children = read_children(dir, ctx.opts, stack)?;
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
        let size_col = if ctx.sizes {
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
            // Push this child dir's own `.gitignore` (deepest-first wins), recurse,
            // then pop so siblings don't inherit it.
            let pushed = push_dir_gitignore(stack, &child.path, ctx.opts)?;
            render_dir(ctx, &child.path, &child_prefix, depth + 1, counts, stack)?;
            if pushed {
                stack.pop();
            }
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
    ctx: &WalkCtx,
    dir: &Path,
    name: String,
    is_dir: bool,
    size: Option<u64>,
    depth: usize,
    stack: &mut Vec<Gitignore>,
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
    let descend = match ctx.max_depth {
        Some(max) => depth <= max,
        None => true,
    };

    let mut children = Vec::new();
    if descend {
        // `stack` already includes `dir`'s own `.gitignore` (pushed by the caller).
        for child in read_children(dir, ctx.opts, stack)? {
            // Push the child dir's `.gitignore` before recursing (deepest-first).
            let pushed = if child.is_dir {
                push_dir_gitignore(stack, &child.path, ctx.opts)?
            } else {
                false
            };
            children.push(build_node(
                ctx,
                &child.path,
                child.name,
                child.is_dir,
                child.size,
                depth + 1,
                stack,
            )?);
            if pushed {
                stack.pop();
            }
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
/// symlinks not followed), applying — in order — the gitignore/`--ignore` filter
/// (via the ancestor `stack`, deepest-first), then `--dirs-only`, then sorting per
/// `opts.sort`. `stack` MUST already include `dir`'s own `.gitignore` (the caller
/// pushes it). With an empty stack, no `--dirs-only`, and no `--sort`, the result
/// is byte-identical to v1 (directories-first then case-insensitive alphabetical,
/// D-08).
fn read_children(dir: &Path, opts: &WalkOpts, stack: &[Gitignore]) -> anyhow::Result<Vec<Child>> {
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
        let is_dir = entry.file_type().is_dir();
        // Gitignore/`--ignore` filter FIRST (layered after the hidden prune): check
        // the ancestor stack deepest-first so a nested rule wins (Pitfall 4).
        if !stack.is_empty() && is_ignored(stack, entry.path(), is_dir) {
            continue;
        }
        // `--dirs-only` AFTER the ignore filter: drop file children identically in
        // both renders.
        if opts.dirs_only && !is_dir {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
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

    sort_children(&mut children, opts.sort);
    Ok(children)
}

/// Sort children per `sort`. `None`/`Some(Name)` keep the v1 D-08 order
/// (directories-first then case-insensitive alphabetical). `Some(Size)` orders
/// files biggest-first (ties broken case-insensitive); directories carry no
/// intrinsic size (`size: None`) so they sort to a defined end, alphabetically.
fn sort_children(children: &mut [Child], sort: Option<SortMode>) {
    match sort {
        Some(SortMode::Size) => {
            children.sort_by(|a, b| match (a.size, b.size) {
                // Both files: biggest-first, tie-break case-insensitive name.
                (Some(sa), Some(sb)) => sb
                    .cmp(&sa)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                // A file sorts before a directory (files carry sizes).
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                // Both directories: case-insensitive alphabetical.
                (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
        }
        // Default (and explicit `--sort name`): the v1 D-08 comparator.
        _ => {
            children.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }
    }
}

/// Check the ancestor gitignore `stack` deepest-first; the FIRST decisive match
/// wins (a nested rule overrides an ancestor — Pitfall 4). Returns `true` when
/// `path` should be hidden. A whitelist (`!pattern`) in a deeper file re-shows a
/// path an ancestor ignored.
fn is_ignored(stack: &[Gitignore], path: &Path, is_dir: bool) -> bool {
    for gi in stack.iter().rev() {
        let m = gi.matched(path, is_dir);
        if m.is_ignore() {
            return true;
        }
        if m.is_whitelist() {
            return false;
        }
    }
    false
}

/// Build a single [`Gitignore`] from the `--ignore` globs (rooted at the tree
/// target so patterns match relative paths). Returns `None` when no globs were
/// given. A malformed glob is a clean error (exit 1), never a panic (T-8-02-GLOB).
fn build_ignore_matcher(root: &Path, globs: &[String]) -> anyhow::Result<Option<Gitignore>> {
    if globs.is_empty() {
        return Ok(None);
    }
    let mut builder = GitignoreBuilder::new(root);
    for g in globs {
        builder
            .add_line(None, g)
            .with_context(|| format!("invalid --ignore glob: {g}"))?;
    }
    Ok(Some(builder.build().context("building --ignore matcher")?))
}

/// Load `dir`'s own `.gitignore` (rooted at `dir`) when present, returning `None`
/// when the file is absent. A partial parse error (`add` returns the error while
/// retaining the valid lines) is non-fatal — git is similarly lenient about an
/// individual bad pattern.
fn load_dir_gitignore(dir: &Path) -> anyhow::Result<Option<Gitignore>> {
    let gi_path = dir.join(".gitignore");
    if !gi_path.is_file() {
        return Ok(None);
    }
    let mut builder = GitignoreBuilder::new(dir);
    let _ = builder.add(&gi_path);
    let gi = builder
        .build()
        .with_context(|| format!("parsing {}", gi_path.display()))?;
    Ok(Some(gi))
}

/// Push `dir`'s own `.gitignore` onto the matcher `stack` when `--gitignore` is set
/// and the file exists, returning whether a matcher was pushed (so the caller knows
/// to pop after recursing). A no-op (returns `false`) when `--gitignore` is off.
fn push_dir_gitignore(
    stack: &mut Vec<Gitignore>,
    dir: &Path,
    opts: &WalkOpts,
) -> anyhow::Result<bool> {
    if !opts.gitignore {
        return Ok(false);
    }
    match load_dir_gitignore(dir)? {
        Some(gi) => {
            stack.push(gi);
            Ok(true)
        }
        None => Ok(false),
    }
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

    fn file_sized(name: &str, size: u64) -> Child {
        Child {
            name: name.to_string(),
            is_dir: false,
            size: Some(size),
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
        // Default order (no --sort) is the D-08 dirs-first comparator.
        sort_children(&mut v, None);
        let order: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        // Directories first (alpha, Beta — case-insensitive), then files
        // (apple.rs, README.md, Zebra.txt — case-insensitive).
        assert_eq!(
            order,
            vec!["alpha", "Beta", "apple.rs", "README.md", "Zebra.txt"]
        );
    }

    #[test]
    fn sort_name_matches_default() {
        // `--sort name` is explicitly the same as the default D-08 order.
        let mut v = vec![
            child("Zebra.txt", false),
            child("alpha", true),
            child("Beta", true),
        ];
        sort_children(&mut v, Some(SortMode::Name));
        let order: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(order, vec!["alpha", "Beta", "Zebra.txt"]);
    }

    #[test]
    fn sort_size_is_files_biggest_first_dirs_last() {
        // Files carry sizes; directories (size None) sort to the end alphabetically.
        let mut v = vec![
            file_sized("small.txt", 100),
            child("zdir", true),
            file_sized("big.txt", 3000),
            file_sized("mid.txt", 500),
            child("Adir", true),
        ];
        sort_children(&mut v, Some(SortMode::Size));
        let order: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        // Files biggest-first (big, mid, small), then dirs alpha (Adir, zdir).
        assert_eq!(
            order,
            vec!["big.txt", "mid.txt", "small.txt", "Adir", "zdir"]
        );
    }

    #[test]
    fn sort_size_ties_break_alpha() {
        // Equal-size files break the tie by case-insensitive name.
        let mut v = vec![
            file_sized("zeta.txt", 200),
            file_sized("Alpha.txt", 200),
        ];
        sort_children(&mut v, Some(SortMode::Size));
        let order: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(order, vec!["Alpha.txt", "zeta.txt"]);
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
