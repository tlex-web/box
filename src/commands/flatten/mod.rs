//! The `flatten` anchor command: recursively copy every file from a source tree
//! into one flat output directory, originals untouched, collisions renamed by
//! encoding the source path — with a `--dry-run` preview that writes nothing.
//!
//! Flow (RESEARCH Architecture diagram, Pattern 6 planner/executor split):
//! 1. `create_dir_all(out)` (D-13) then [`normalize_path`] BOTH roots (dunce).
//! 2. **Containment guard** — abort if the canonical output dir is inside the
//!    canonical source dir, lowercased (NTFS is case-insensitive; Pitfall 4),
//!    BEFORE any copy, so `flatten ./p ./p/flat` can never loop.
//! 3. Seed an `occupied` name-set from `read_dir(out)` lowercased (D-14) so a
//!    pre-existing output file is never silently clobbered.
//! 4. Walk `src` with `follow_links(false)` + `filter_entry(!is_hidden)` (D-12),
//!    skipping symlinks (Pitfall 8); for each file decide Copy / Rename / Skip and
//!    build one [`Plan`] — the single source of truth for dry-run and real run.
//! 5. Dry-run prints the plan + the locked D-11 dry-run summary, writing nothing;
//!    otherwise execute via [`safe_copy`] (timestamps preserved) + D-11 real-run
//!    summary. Copy I/O is `.context(...)`-wrapped so deep-path (>260) failures
//!    surface loudly, never silently dropped (FOUND-06).

pub mod rename;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::Args;
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::{is_hidden, normalize_path, safe_copy};
use crate::core::output::{
    dry_run_summary, format_row, real_run_summary, terminal_width, RowStatus,
};

/// `box flatten <src> <out> [--dry-run]` — flatten a folder tree into one
/// directory (FLAT-01..04).
#[derive(Debug, Args)]
pub struct FlattenArgs {
    /// Source directory tree to flatten.
    pub src: PathBuf,
    /// Output directory to copy every file into (created if missing).
    pub out: PathBuf,
    /// Preview the plan without writing anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// What the plan will do with one source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    /// A plain copy under the file's own base name (no collision).
    Copy,
    /// A copy under a collision-renamed name.
    Rename,
    /// Skipped (symlink); never copied.
    Skip,
}

impl ItemKind {
    fn status(self) -> RowStatus {
        match self {
            ItemKind::Copy => RowStatus::Copy,
            ItemKind::Rename => RowStatus::Rename,
            ItemKind::Skip => RowStatus::Skip,
        }
    }
}

/// One planned action. The plan is built once and consumed by both the dry-run
/// printer and the executor so the preview can never diverge from the real run.
#[derive(Debug)]
struct PlanItem {
    /// Absolute source path.
    src: PathBuf,
    /// Source label shown to the user (path relative to the source root).
    src_label: String,
    /// Destination base name (the flat name in the output dir); `None` for skips.
    dst_name: Option<String>,
    kind: ItemKind,
    /// Trailing reason shown inline, e.g. `[collision]`, `(skipped: symlink)`.
    reason: Option<String>,
}

/// The full plan plus pre-tallied counts for the locked summaries (D-11).
#[derive(Debug, Default)]
struct Plan {
    items: Vec<PlanItem>,
    to_copy: usize,
    renamed: usize,
    skipped: usize,
}

impl RunCommand for FlattenArgs {
    fn run(self) -> anyhow::Result<()> {
        // (1) Auto-create the output dir first (D-13), then canonicalize BOTH
        //     roots via dunce so the containment guard and collision-encoding work
        //     on real, UNC-free paths (FOUND-06, Pitfall 1).
        std::fs::create_dir_all(&self.out)
            .with_context(|| format!("creating output dir {}", self.out.display()))?;
        let src_root = normalize_path(&self.src)
            .with_context(|| format!("resolving source dir {}", self.src.display()))?;
        let out_root = normalize_path(&self.out)
            .with_context(|| format!("resolving output dir {}", self.out.display()))?;

        // (2) Containment guard — abort BEFORE any I/O if the output dir is inside
        //     the source dir, or the disk fills as the walker re-visits its own
        //     output (Pitfall 4). `Path::starts_with` is case-sensitive but NTFS
        //     is not, so compare lowercased.
        let src_low = src_root.to_string_lossy().to_ascii_lowercase();
        let out_low = out_root.to_string_lossy().to_ascii_lowercase();
        if Path::new(&out_low).starts_with(Path::new(&src_low)) {
            bail!(
                "refusing to flatten: output dir {} is inside source dir {} \
                 (this would copy files into themselves)",
                out_root.display(),
                src_root.display()
            );
        }

        // (3) Seed the occupied-name set from the existing output dir (case-folded
        //     with `to_lowercase`, matching `rename::dedupe` — full Unicode, not
        //     ASCII-only, so non-ASCII case pairs also collide; WR-01) so an
        //     incoming name that already exists is renamed, not clobbered
        //     (D-14, T-03-overwrite).
        let mut occupied: HashSet<String> = HashSet::new();
        for entry in std::fs::read_dir(&out_root)
            .with_context(|| format!("reading output dir {}", out_root.display()))?
        {
            let entry =
                entry.with_context(|| format!("reading an entry of {}", out_root.display()))?;
            occupied.insert(entry.file_name().to_string_lossy().to_lowercase());
        }

        // (4) Walk + build the single plan.
        let plan = build_plan(&src_root, &mut occupied)?;

        // (5) Dry-run prints and writes nothing; real run copies.
        if self.dry_run {
            print_plan(&plan);
            println!();
            println!(
                "{}",
                dry_run_summary(plan.to_copy, plan.renamed, plan.skipped)
            );
            return Ok(());
        }

        let arrow_col = arrow_col(&plan);
        let width = terminal_width();
        let mut copied = 0usize;
        let mut bytes_written: u64 = 0;
        for item in &plan.items {
            match item.kind {
                ItemKind::Skip => {
                    println!(
                        "{}",
                        format_row(
                            item.kind.status(),
                            &item.src_label,
                            None,
                            item.reason.as_deref(),
                            arrow_col,
                            width,
                        )
                    );
                }
                ItemKind::Copy | ItemKind::Rename => {
                    let dst_name = item
                        .dst_name
                        .as_deref()
                        .expect("copy/rename items always have a destination");
                    let dst = out_root.join(dst_name);
                    let n = safe_copy(&item.src, &dst)
                        .with_context(|| format!("flattening {}", item.src.display()))?;
                    bytes_written += n;
                    copied += 1;
                    println!(
                        "{}",
                        format_row(
                            item.kind.status(),
                            &item.src_label,
                            Some(dst_name),
                            item.reason.as_deref(),
                            arrow_col,
                            width,
                        )
                    );
                }
            }
        }

        println!();
        println!(
            "{}",
            real_run_summary(
                copied,
                plan.renamed,
                plan.skipped,
                &human_size(bytes_written)
            )
        );
        Ok(())
    }
}

/// Walk `src_root` (hidden pruned, symlinks skipped) and build the plan, resolving
/// each non-skipped file's destination name against `occupied` and inserting the
/// chosen name so within-run collisions also dedupe.
fn build_plan(src_root: &Path, occupied: &mut HashSet<String>) -> anyhow::Result<Plan> {
    let mut plan = Plan::default();

    let walker = WalkDir::new(src_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e));

    for entry in walker {
        let entry = entry.with_context(|| format!("walking {}", src_root.display()))?;

        // Directories are structure only — flatten drops them; we only act on
        // files and on symlinks (which we explicitly skip).
        let is_symlink = entry.path_is_symlink();
        if !is_symlink && !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel = path.strip_prefix(src_root).unwrap_or(path);
        let src_label = rel.to_string_lossy().to_string();

        // Skip symlinks/junctions safely — never followed, no loop (Pitfall 8).
        if is_symlink {
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                src_label,
                dst_name: None,
                kind: ItemKind::Skip,
                reason: Some("(skipped: symlink)".to_string()),
            });
            plan.skipped += 1;
            continue;
        }

        // Base name for a plain copy.
        let base = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let base_safe = rename::sanitize_reserved(&base);
        let base_key = base_safe.to_lowercase();

        if occupied.contains(&base_key) {
            // Collision: encode the source-relative path, then numeric-dedupe.
            let encoded = rename::encode_relative(rel);
            let chosen = rename::dedupe(&encoded, occupied);
            occupied.insert(chosen.to_lowercase());
            // `[collision]` vs `[collision xN]` when the chosen name itself needed
            // a numeric suffix (a double collision).
            let reason = collision_reason(&encoded, &chosen);
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                src_label,
                dst_name: Some(chosen),
                kind: ItemKind::Rename,
                reason: Some(reason),
            });
            plan.renamed += 1;
        } else {
            occupied.insert(base_key);
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                src_label,
                dst_name: Some(base_safe),
                kind: ItemKind::Copy,
                reason: None,
            });
            plan.to_copy += 1;
        }
    }

    Ok(plan)
}

/// `[collision]` for a single collision, `[collision xN]` when the encoded name
/// also collided and a numeric `_N` suffix was appended (N = the suffix number).
fn collision_reason(encoded: &str, chosen: &str) -> String {
    if encoded == chosen {
        return "[collision]".to_string();
    }
    // chosen looks like `{stem}_{n}{.ext}`; recover N for the message.
    let stem_part = chosen.rsplit_once('.').map(|(s, _)| s).unwrap_or(chosen);
    let n = stem_part
        .rsplit_once('_')
        .and_then(|(_, num)| num.parse::<usize>().ok())
        .map(|n| n + 1)
        .unwrap_or(2);
    format!("[collision x{n}]")
}

/// The alignment column for the `->` arrow: the widest source label across all
/// non-skip rows (so destinations line up), with a sane lower bound.
fn arrow_col(plan: &Plan) -> usize {
    plan.items
        .iter()
        .filter(|i| i.dst_name.is_some())
        .map(|i| i.src_label.chars().count())
        .max()
        .unwrap_or(0)
}

/// Print every plan row (used by dry-run). Real run prints rows as it copies.
fn print_plan(plan: &Plan) {
    let col = arrow_col(plan);
    let width = terminal_width();
    for item in &plan.items {
        let dst = item.dst_name.as_deref();
        println!(
            "{}",
            format_row(
                item.kind.status(),
                &item.src_label,
                dst,
                item.reason.as_deref(),
                col,
                width,
            )
        );
    }
}

/// Human-readable byte size for the real-run summary (`1.2 MB`, `512 B`).
fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn collision_reason_distinguishes_single_and_multi() {
        assert_eq!(collision_reason("a_b.txt", "a_b.txt"), "[collision]");
        // `a_b.txt` -> `a_b_1.txt` is the second instance of that name.
        assert_eq!(collision_reason("a_b.txt", "a_b_1.txt"), "[collision x2]");
        assert_eq!(collision_reason("a_b.txt", "a_b_2.txt"), "[collision x3]");
    }
}
