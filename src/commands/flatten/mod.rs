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
//!
//! **Destructive `--move` (FLAT-V2-02).** `--move` relocates files instead of
//! copying, reusing the SAME pipeline above (separator validation, containment
//! guard, occupied seed, [`build_plan`]) so every FLAT-V2-01 filter applies
//! identically — only the execution differs ([`run_move`]):
//! - **Dry-run is the DEFAULT** (the D-5 destructive template): `--move` writes
//!   NOTHING and previews the relocation plan unless `--force` is given (the
//!   inverse of copy mode, where `--dry-run` is opt-in). An explicit `--dry-run`
//!   also forces a preview even alongside `--force`.
//! - **`--force` relocates in TWO phases** so a mid-batch failure can never lose
//!   data (Pitfall 5): copy + verify EVERY file (via [`safe_copy`] create-new,
//!   then confirm the destination exists with a matching size) BEFORE deleting any
//!   source; only once the whole batch is copied+verified are the sources deleted.
//!   A failed/short copy therefore never deletes a source, and every abort path
//!   (containment refusal, dry-run, mid-batch copy error) leaves the source tree
//!   byte-for-byte unchanged. Emptied source DIRECTORIES are left in place (only
//!   files relocate).

pub mod rename;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::Args;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::{is_hidden, normalize_path, safe_copy};
use crate::core::output::{
    dry_run_summary, format_row, human_size, real_run_summary, terminal_width, RowStatus,
};

/// "Large input" cutoff for the FLAT-V2-01 real-run stderr progress bar (Claude's
/// Discretion): a file-count bar is drawn only when the plan has MORE than this
/// many items. Below it (small flattens, including every existing test fixture) no
/// bar appears. Always stderr-only, never constructed under `--json` (Pitfall 2).
const PROGRESS_ITEM_THRESHOLD: usize = 16;

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
    /// Only copy files whose final extension matches one of these (comma-separated,
    /// case-insensitive, e.g. `jpg,png`). Non-matching files are simply absent from
    /// the plan/output (they never inflate the counts).
    #[arg(long)]
    pub extensions: Option<String>,
    /// Join character for collision-encoded names (default `_`); e.g. `--separator
    /// -` turns `docs\sub\a.txt` into `docs-sub-a.txt`. Must not contain a path
    /// separator (`/` or `\`).
    #[arg(long)]
    pub separator: Option<String>,
    /// Include hidden files and directories the default walk prunes (names starting
    /// with `.` and, on Windows, the FILE_ATTRIBUTE_HIDDEN flag).
    #[arg(long)]
    pub include_hidden: bool,
    /// DESTRUCTIVE: relocate files instead of copying — each file is copied,
    /// verified (destination exists with a matching size), then the source is
    /// deleted. Dry-run by DEFAULT: previews the plan and writes nothing unless
    /// `--force` is also given. Every abort path leaves the source tree unchanged;
    /// empty source directories are left in place. (`move` is a Rust keyword, so the
    /// field is `move_`; the CLI flag stays `--move`.)
    #[arg(long = "move")]
    pub move_: bool,
    /// Actually perform a `--move` relocation. Without it, `--move` only previews
    /// (dry-run). Ignored when `--move` is absent.
    #[arg(long)]
    pub force: bool,
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

/// The serde projection of one [`PlanItem`] for `box flatten --json` (SPINE-02,
/// D-13): `{src, dst, action, reason}`. `src` is the source label (lossy string,
/// D-4), `dst` the flat destination name (`None` for skips), `action` the
/// lowercased [`RowStatus`] (`"copy"`/`"rename"`/`"skip"`), `reason` the same
/// inline reason the human row shows. The raw fields are serialized — NEVER the
/// aligned `format_row` output (that is human layout).
#[derive(serde::Serialize)]
struct FlattenRow {
    src: String,
    dst: Option<String>,
    action: &'static str,
    reason: Option<String>,
}

/// The `box flatten --json` document (D-12/D-13): the always-wrapped
/// `{results,count}` shape plus a `dry_run` boolean (so a script can tell a preview
/// from an applied run) and the locked sibling summary counts. On a dry-run,
/// `copied`/`total_bytes` are 0 and the counts come from the plan; on a real run
/// they reflect the actual copy.
#[derive(serde::Serialize)]
struct FlattenOutput {
    results: Vec<FlattenRow>,
    count: usize,
    dry_run: bool,
    copied: usize,
    renamed: usize,
    skipped: usize,
    total_bytes: u64,
}

/// The lowercased `action` string for a plan item (D-13) — the lowercased
/// [`RowStatus`] spelling, reusing the same `status()` source of truth the human
/// glyph derives from (no-drift).
fn action_str(kind: ItemKind) -> &'static str {
    match kind.status() {
        RowStatus::Copy => "copy",
        RowStatus::Rename => "rename",
        RowStatus::Skip => "skip",
    }
}

/// Project the plan's items into the JSON `.results` rows (raw fields, not
/// `format_row` layout, D-13).
fn flatten_rows(plan: &Plan) -> Vec<FlattenRow> {
    plan.items
        .iter()
        .map(|item| FlattenRow {
            src: item.src_label.clone(),
            dst: item.dst_name.clone(),
            action: action_str(item.kind),
            reason: item.reason.clone(),
        })
        .collect()
}

impl RunCommand for FlattenArgs {
    fn run(self) -> anyhow::Result<()> {
        // (0) Validate --separator FIRST, before any I/O (V5 input validation): it
        //     must not carry a path separator, which would let a collision-encoded
        //     name escape the output dir (T-8-01). A clean anyhow error → exit 1,
        //     never a panic. Default join char is `_` (v1 parity).
        let separator = self.separator.as_deref().unwrap_or("_");
        if separator.contains('/') || separator.contains('\\') {
            bail!(
                "invalid --separator {separator:?}: must not contain a path separator (/ or \\)"
            );
        }
        // Parse --extensions once into a lowercased set (leading dots + surrounding
        // whitespace tolerated). `None` means no extension filter (default).
        let ext_set: Option<HashSet<String>> = self.extensions.as_deref().map(parse_extensions);

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

        // (4) Walk + build the single plan, threading the FLAT-V2-01 filters into
        //     the one source-of-truth walk (so human render and --json cannot drift).
        let plan = build_plan(
            &src_root,
            &mut occupied,
            self.include_hidden,
            ext_set.as_ref(),
            separator,
        )?;

        // FLAT-V2-02 — destructive `--move`: relocate (copy → verify → delete)
        // instead of copy. The shared setup above (separator validation, the
        // containment guard, the occupied seed, and `build_plan`) has already run,
        // so every FLAT-V2-01 filter applies identically — only the execution
        // differs. Dry-run is the DEFAULT here (the destructive template); an
        // explicit `--dry-run` also forces a preview even alongside `--force`.
        if self.move_ {
            let execute = self.force && !self.dry_run;
            return run_move(&plan, &out_root, execute);
        }

        // (5) Dry-run prints and writes nothing; real run copies.
        if self.dry_run {
            // Fork on `is_json_on()` FIRST (Pitfall 1): under --json emit the PLAN
            // (D-12 — --json is orthogonal to --force; dry-run+json = the plan),
            // with `dry_run: true` and zeroed real-run counts. All human chrome
            // (rows + blank + summary) is suppressed.
            if crate::core::output::is_json_on() {
                let doc = FlattenOutput {
                    count: plan.items.len(),
                    results: flatten_rows(&plan),
                    dry_run: true,
                    // A dry-run copies nothing; report the plan's intent counts.
                    copied: 0,
                    renamed: plan.renamed,
                    skipped: plan.skipped,
                    total_bytes: 0,
                };
                crate::core::output::emit_json(&doc)?;
                return Ok(());
            }
            print_plan(&plan);
            println!();
            println!(
                "{}",
                dry_run_summary(plan.to_copy, plan.renamed, plan.skipped)
            );
            return Ok(());
        }

        // The real run always performs the copies; whether it PRINTS the rows
        // depends on the fork (--json suppresses all human chrome and emits one
        // document at the end instead — D-12 real+json = the executed result).
        let json = crate::core::output::is_json_on();
        let arrow_col = arrow_col(&plan);
        let width = terminal_width();
        // stderr-only copy progress (Pitfall 2): shown only for a plan above the
        // cutoff and only when --json is off; never constructed under --json and
        // never drawn to stdout.
        let progress = if !json && plan.items.len() > PROGRESS_ITEM_THRESHOLD {
            let pb = ProgressBar::with_draw_target(
                Some(plan.items.len() as u64),
                ProgressDrawTarget::stderr(),
            );
            pb.set_style(
                ProgressStyle::with_template("{bar:30} {pos}/{len} files")
                    .unwrap_or_else(|_| ProgressStyle::default_bar()),
            );
            Some(pb)
        } else {
            None
        };
        let mut copied = 0usize;
        let mut bytes_written: u64 = 0;
        for item in &plan.items {
            match item.kind {
                ItemKind::Skip => {
                    if !json {
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
                    if !json {
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
            if let Some(pb) = &progress {
                pb.inc(1);
            }
        }
        if let Some(pb) = progress {
            pb.finish_and_clear();
        }

        // WR-06: `count`/`results` are derived from the PLAN, not the executed
        // loop. That is sound here because reaching this point means the loop ran
        // to completion (any `safe_copy` error is `?`-propagated above → exit 1
        // with empty stdout, no JSON emitted), so plan == outcome: every non-skip
        // item was copied (`copied == to_copy + renamed`) and the three tallies
        // partition the plan. These debug assertions pin that coupling so a future
        // executor/planner divergence trips in test/dev builds instead of silently
        // misreporting the JSON.
        debug_assert_eq!(
            copied,
            plan.to_copy + plan.renamed,
            "executed copies must equal the planned copy+rename count on a successful run"
        );
        debug_assert_eq!(
            plan.to_copy + plan.renamed + plan.skipped,
            plan.items.len(),
            "the plan tallies must partition plan.items (count is plan.items.len())"
        );

        // Under --json, the ONLY stdout write is the single emit_json carrying the
        // EXECUTED result (real copied / total_bytes captured above, `dry_run:
        // false`). Otherwise the human blank line + real-run summary.
        if json {
            let doc = FlattenOutput {
                count: plan.items.len(),
                results: flatten_rows(&plan),
                dry_run: false,
                copied,
                renamed: plan.renamed,
                skipped: plan.skipped,
                total_bytes: bytes_written,
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
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

/// Execute (or preview) the destructive FLAT-V2-02 `--move` relocation.
///
/// **Dry-run-DEFAULT (the destructive template):** when `execute` is false this
/// writes NOTHING and emits the relocation plan — the [`FlattenOutput`] plan
/// projection with `dry_run: true` under `--json`, otherwise the human preview
/// rows + the dry-run summary. This inverts copy mode's opt-in `--dry-run` so a
/// destructive run can never be accidental.
///
/// **`execute` (from `--force`):** relocate each planned file in TWO phases so a
/// mid-batch failure can never lose data (Pitfall 5, threat T-8-04):
/// 1. **Copy + verify EVERY file** — [`safe_copy`] (create-new, never clobbers)
///    then confirm the destination exists and its byte length equals the source's.
///    Any error `?`-propagates HERE, with some destinations possibly written but
///    ZERO sources deleted — so the source tree stays byte-for-byte unchanged.
/// 2. **Delete EVERY source** — reached ONLY when the whole batch copied+verified,
///    so a failed/short copy can never orphan (delete) a source. Emptied source
///    DIRECTORIES are deliberately left in place (locked: only files relocate).
///
/// The `--json` document reuses [`FlattenOutput`]/[`flatten_rows`] (no-drift):
/// `dry_run` flips with `execute`, and `copied`/`total_bytes` carry the real
/// relocation counts.
fn run_move(plan: &Plan, out_root: &Path, execute: bool) -> anyhow::Result<()> {
    let json = crate::core::output::is_json_on();

    // Dry-run-DEFAULT: preview only, write nothing, unless --force (execute).
    if !execute {
        if json {
            let doc = FlattenOutput {
                count: plan.items.len(),
                results: flatten_rows(plan),
                dry_run: true,
                copied: 0,
                renamed: plan.renamed,
                skipped: plan.skipped,
                total_bytes: 0,
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }
        print_plan(plan);
        println!();
        println!(
            "{}",
            dry_run_summary(plan.to_copy, plan.renamed, plan.skipped)
        );
        return Ok(());
    }

    // Phase 1 — copy + verify every file (NO deletes yet). On any error we
    // `?`-propagate with zero sources deleted, so the source tree is unchanged.
    let mut moved = 0usize;
    let mut bytes_written: u64 = 0;
    for item in &plan.items {
        if item.kind == ItemKind::Skip {
            continue;
        }
        let dst_name = item
            .dst_name
            .as_deref()
            .expect("copy/rename items always have a destination");
        let dst = out_root.join(dst_name);
        // create-new copy: never clobbers an existing destination (planned
        // collisions were already renamed via the occupied seed; this is the WR-02
        // defense-in-depth backstop).
        let n = safe_copy(&item.src, &dst)
            .with_context(|| format!("moving {} (copy step)", item.src.display()))?;
        // Verify BEFORE any delete (Pitfall 5): the destination must exist and its
        // byte length must equal the source's. A short/failed copy must NEVER reach
        // the delete phase.
        let dst_len = std::fs::metadata(&dst)
            .with_context(|| format!("verifying moved file {}", dst.display()))?
            .len();
        let src_len = std::fs::metadata(&item.src)
            .with_context(|| format!("reading source size for {}", item.src.display()))?
            .len();
        if dst_len != src_len {
            bail!(
                "move verification failed for {src}: destination {dst} is {dst_len} bytes \
                 but the source is {src_len} bytes — refusing to delete the source on a \
                 short copy",
                src = item.src.display(),
                dst = dst.display(),
            );
        }
        bytes_written += n;
        moved += 1;
    }

    // Phase 2 — every copy verified, so NOW delete each source. Reached only after
    // the whole batch copied, so no copy error can ever delete a source. Emptied
    // source DIRECTORIES are left in place (locked: only files relocate).
    for item in &plan.items {
        if item.kind == ItemKind::Skip {
            continue;
        }
        std::fs::remove_file(&item.src).with_context(|| {
            format!("removing source {} after a verified copy", item.src.display())
        })?;
    }

    // Output: --json emits the EXECUTED result (dry_run:false, real counts);
    // otherwise the human rows + the locked D-11 real-run summary.
    if json {
        let doc = FlattenOutput {
            count: plan.items.len(),
            results: flatten_rows(plan),
            dry_run: false,
            copied: moved,
            renamed: plan.renamed,
            skipped: plan.skipped,
            total_bytes: bytes_written,
        };
        crate::core::output::emit_json(&doc)?;
        return Ok(());
    }
    print_plan(plan);
    println!();
    println!(
        "{}",
        real_run_summary(moved, plan.renamed, plan.skipped, &human_size(bytes_written))
    );
    Ok(())
}

/// Parse a `--extensions` list into a lowercased set of bare extensions
/// (FLAT-V2-01): comma-separated, surrounding whitespace and a leading `.`
/// tolerated, empties dropped (`"jpg, .PNG ,"` → `{"jpg","png"}`).
fn parse_extensions(list: &str) -> HashSet<String> {
    list.split(',')
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

/// Walk `src_root` and build the plan, resolving each non-skipped file's
/// destination name against `occupied` and inserting the chosen name so within-run
/// collisions also dedupe. The FLAT-V2-01 filters fold into this single
/// source-of-truth walk so the human render and `--json` can never diverge:
/// `include_hidden` bypasses the hidden prune; `extensions` (when `Some`) keeps
/// only files whose final extension is in the set; `separator` is the
/// collision-encoding join char passed to [`rename::encode_relative`].
fn build_plan(
    src_root: &Path,
    occupied: &mut HashSet<String>,
    include_hidden: bool,
    extensions: Option<&HashSet<String>>,
    separator: &str,
) -> anyhow::Result<Plan> {
    let mut plan = Plan::default();

    let walker = WalkDir::new(src_root)
        .follow_links(false)
        .into_iter()
        // --include-hidden drops the hidden prune entirely; unset keeps the D-06
        // `is_hidden` filter exactly as v1 (a hidden directory still prunes its
        // whole subtree cheaply).
        .filter_entry(move |e| include_hidden || !is_hidden(e));

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

        // FLAT-V2-01 extension filter: when --extensions is set, only files whose
        // final extension (case-insensitive) is in the set become plan items.
        // Non-matching files are simply skipped BEFORE becoming plan items, so they
        // never inflate `count` and are not emitted as skip rows.
        if let Some(exts) = extensions {
            let matches = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_ascii_lowercase()))
                .unwrap_or(false);
            if !matches {
                continue;
            }
        }

        // Base name for a plain copy.
        let base = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let base_safe = rename::sanitize_reserved(&base);
        let base_key = base_safe.to_lowercase();

        if occupied.contains(&base_key) {
            // Collision: encode the source-relative path (joined with the chosen
            // separator), then numeric-dedupe.
            let encoded = rename::encode_relative(rel, separator);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_reason_distinguishes_single_and_multi() {
        assert_eq!(collision_reason("a_b.txt", "a_b.txt"), "[collision]");
        // `a_b.txt` -> `a_b_1.txt` is the second instance of that name.
        assert_eq!(collision_reason("a_b.txt", "a_b_1.txt"), "[collision x2]");
        assert_eq!(collision_reason("a_b.txt", "a_b_2.txt"), "[collision x3]");
    }
}
