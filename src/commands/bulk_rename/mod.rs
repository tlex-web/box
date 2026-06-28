//! The `bulk-rename` command: the phase's single DESTRUCTIVE tool. It builds a
//! regex rename plan over a directory's files, runs an in-memory
//! ABORT-ALL-BEFORE-ANY-RENAME pre-flight (collisions, cycles/swaps, path
//! -separator injection), previews the plan by DEFAULT (dry-run; writes nothing),
//! and executes only with `--force` (D-14..D-19).
//!
//! ⚠️ **The pre-flight check is the ENTIRE safety story.** `std::fs::rename` maps
//! to `MoveFileExW` + `MOVEFILE_REPLACE_EXISTING` on Windows and SILENTLY
//! OVERWRITES its destination — there is NO `create_new` analog for moves the way
//! [`crate::core::fs::safe_copy`] has one for copies. So correctness rests
//! entirely on detecting every clobber/cycle/injection in memory BEFORE the first
//! `rename` and aborting the whole batch if any is found (D-18, RESEARCH Pitfall
//! 4). A missed collision is silent, irreversible data loss.
//!
//! Flow (mirrors flatten's plan→preview→execute split, INVERTED — dry-run is the
//! DEFAULT here, `--force` executes):
//! 1. Compile the user regex (`Regex::new`); a bad pattern is a clean `anyhow`
//!    error → exit 1, never a panic (FOUND-05).
//! 2. Scope the candidate files: top-level files of `dir` by default,
//!    `--recursive` opts into flatten's `WalkDir` walk; dirs/symlinks are `-`
//!    skip rows (D-14/D-15).
//! 3. For each file, `regex.replace(full_base_name, replacement)` — FIRST match
//!    only over the WHOLE base name incl. extension (D-16/D-17). A byte-exact
//!    no-op is an `(unchanged)` `-` row; a case-only change is a REAL rename.
//! 4. Partition the plan by parent directory and run [`preflight`] per directory
//!    (the load-bearing pure detector): any collision, cycle, or separator
//!    -injecting target aborts the whole batch (exit 1, nothing written) in BOTH
//!    dry-run and `--force`.
//! 5. Dry-run prints the plan + the D-19 dry-run summary and returns; `--force`
//!    executes `std::fs::rename` per file (`.context(...)`-wrapped) only AFTER a
//!    clean pre-flight.
//!
//! ## `--backup` undo manifest (RENM-V2-02, D-22)
//!
//! `--backup` writes a JSON undo MANIFEST, NOT file copies. A pure rename
//! (`MoveFileExW`) changes only the NAME, so the entire reversible state is the
//! `{old → new}` map — copying bytes would protect data that was never at risk.
//! The manifest is a zero-drift serde projection of the already-built,
//! pre-flight-cleared [`Plan`] (one `{old, new, applied}` record per renamed file,
//! ABSOLUTE paths via `to_string_lossy`, D-4).
//!
//! - **Location:** `%LOCALAPPDATA%\box\undo\<id>.json` (`LOCALAPPDATA`, NOT
//!   `APPDATA`), `<id>` a sortable `box-undo-<unix_millis>` (A5). It lives OUTSIDE
//!   the renamed tree so `--recursive` never re-walks it and it survives renaming
//!   the target dir; falls back to the target dir only if `LOCALAPPDATA` is unset.
//!   The path is echoed to stderr (stdout stays pure under `--json`).
//! - **Durability ordering (Pitfall 8):** the FULL manifest (every entry
//!   `applied:false`) is written + `File::sync_all()` (fsync)'d BEFORE the first
//!   `fs::rename`; each entry flips `applied:true` (rewrite + fsync) as its rename
//!   returns. A mid-batch I/O error (the existing `?`-propagation → exit 1) leaves
//!   a manifest whose `applied` flags EXACTLY partition done-vs-pending → the
//!   directory is reconcilable by reversing only the applied entries.
//! - **`--force`-only:** `--backup` is orthogonal to and only meaningful with
//!   `--force`; on a dry-run it is a clean no-op. The manifest write is strictly
//!   AFTER a clean pre-flight, so the abort-all-before-any path writes neither the
//!   manifest nor any rename.
//! - **Deferred:** an automated `box bulk-rename --undo` replay subcommand —
//!   RENM-V2-02 needs only the backup written + the dir recoverable, both
//!   satisfied by the manifest plus a one-line manual reverse of the applied
//!   `{old, new}` pairs.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::Args;
use regex::Regex;
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::is_hidden;
use crate::core::output::{format_row, is_color_on, terminal_width, RowStatus};

/// `box bulk-rename <dir> <pattern> <replacement> [--force] [--recursive]` —
/// regex bulk rename with a dry-run-first, abort-on-collision safety model
/// (RENM-01).
///
/// The `pattern` is matched against the **full base name** of each file
/// (including its extension) — extension protection is pattern discipline, not
/// stem-splitting, so `(\d+)` cannot touch `.jpg` but you CAN rewrite an
/// extension on purpose (D-16). Only the **first** match is replaced (D-17).
///
/// Capture groups use the regex-crate `$1` / `${1}` syntax. ⚠️ Foot-gun: an
/// unbraced `$1abc` parses as the group named `1abc` (which does not exist → an
/// empty string); write `${1}abc` when a literal follows a group reference. A
/// reference to a nonexistent group expands to the empty string.
///
/// By DEFAULT this previews the plan and writes NOTHING; pass `--force` to apply
/// it. A collision (two files renaming to one name, or a target clobbering an
/// existing file), a cycle/swap (`a→b, b→a`), or a path-separator-injecting
/// replacement ABORTS the whole batch before any rename — in both modes.
#[derive(Debug, Args)]
pub struct BulkRenameArgs {
    /// Directory whose files to rename (top-level only unless `--recursive`).
    pub dir: PathBuf,
    /// Regex matched against each file's FULL base name (incl. extension).
    pub pattern: String,
    /// Replacement for the FIRST match; `$1` / `${1}` reference capture groups.
    pub replacement: String,
    /// Apply the renames. Without this the command only previews (dry-run).
    #[arg(long)]
    pub force: bool,
    /// Recurse into subdirectories (default: only the target dir's top-level files).
    #[arg(long)]
    pub recursive: bool,
    /// Case-fold the resulting name (`upper`/`lower` fold the whole name; `title`
    /// title-cases the stem only, leaving the extension untouched). Applied AFTER
    /// the regex replacement and `{n}` expansion (D-21).
    #[arg(long, value_name = "MODE")]
    pub case: Option<Case>,
    /// Zero-pad width for the literal `{n}` token (default: auto-width from the file
    /// count). Write `{{n}}` for a literal `{n}`.
    #[arg(long, value_name = "N")]
    pub number_width: Option<usize>,
    /// First value for the `{n}` counter (assigned over the sorted plan order).
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub start: usize,
    /// Increment between consecutive `{n}` values.
    #[arg(long, default_value_t = 1, value_name = "N")]
    pub step: usize,
    /// Write a recoverable JSON undo manifest to `%LOCALAPPDATA%\box\undo\<id>.json`
    /// (outside the renamed tree) before the first rename, fsync'd, flipping each
    /// entry's `applied` flag as its rename returns. Only meaningful with `--force`;
    /// a no-op on a dry-run (RENM-V2-02, D-22).
    #[arg(long)]
    pub backup: bool,
}

/// What the plan will do with one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    /// A real rename (`~`): the new name differs from the old (incl. case-only).
    Rename,
    /// Skipped (`-`): a directory, a symlink, or an unchanged no-op.
    Skip,
}

impl ItemKind {
    fn status(self) -> RowStatus {
        match self {
            ItemKind::Rename => RowStatus::Rename,
            ItemKind::Skip => RowStatus::Skip,
        }
    }
}

/// The optional name-case transform applied AFTER `re.replace` (RENM-V2-01, D-21).
/// `Upper`/`Lower` fold the WHOLE resulting name; `Title` capitalizes the stem only
/// and leaves the final extension untouched. The `regex` crate has no `\U`/`\L`
/// case-fold escapes, so case is a separate post-pass — the regex semantics stay
/// 100% untouched (D-21 rationale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Case {
    Upper,
    Lower,
    Title,
}

/// One planned action. Built once and consumed by both the preview and the
/// executor so the dry-run can never diverge from the real run.
#[derive(Debug)]
struct PlanItem {
    /// Absolute source path of the file.
    src: PathBuf,
    /// Parent directory (collision scope is per-directory, D-14).
    parent: PathBuf,
    /// Source base name (the on-disk name being renamed away).
    old_name: String,
    /// Label shown to the user (source-relative for `--recursive`, else the name).
    src_label: String,
    /// New base name (the rename target); `None` for skips.
    new_name: Option<String>,
    kind: ItemKind,
    /// Trailing reason shown inline, e.g. `(unchanged)`, `(skipped: directory)`.
    reason: Option<String>,
}

/// A detected pre-flight conflict (the load-bearing safety output). Each variant
/// carries enough context for the locked abort message.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Conflict {
    /// Two or more sources rename to the same target, or a target clobbers a
    /// pre-existing on-disk name not renamed away.
    Collision {
        /// The contested target name.
        target: String,
        /// The sources that all want it (or the existing occupant).
        sources: Vec<String>,
    },
    /// A target equals another item's SOURCE — a cycle/swap (no two-phase pass).
    Cycle {
        /// The renaming source.
        source: String,
        /// Its target, which is some other item's source.
        target: String,
    },
    /// A target contains a path separator (`/` or `\`) — refused (V5).
    Separator {
        /// The renaming source.
        source: String,
        /// The injecting target.
        target: String,
    },
}

/// One rename a directory wants to perform: its on-disk source name and the new
/// name the regex produced. Both are EXACT (non-folded) — case-folding is applied
/// only when building the collision KEY, so a case-only change is not a false
/// self-collision (Pitfall 5).
#[derive(Debug, Clone)]
struct Rename {
    /// The exact on-disk source name being renamed away.
    old: String,
    /// The exact new name the regex produced.
    new: String,
}

/// The pure, I/O-free pre-flight detector for ONE directory (D-18). Given the
/// renames a directory wants and the names already present on disk, returns every
/// conflict that must abort the batch. An EMPTY result means the directory's plan
/// is safe to execute.
///
/// The four D-18 rules, all here so the safety logic is unit-testable without a
/// terminal or any disk:
///
/// 1. **Occupied set (case-folded, full-Unicode `to_lowercase` per WR-01),
///    seeded from pre-existing on-disk names NOT being renamed away.** A target
///    that lands on one of those clobbers it.
/// 2. **Every target checked vs other planned targets AND the occupied set.** Two
///    sources wanting one target, or a target hitting a still-present file, is a
///    [`Conflict::Collision`].
/// 3. **Cycles/swaps:** any target equal to ANOTHER item's source (compared
///    case-folded, since the move would land on that still-present file) is a
///    [`Conflict::Cycle`] — v1 detects and aborts (no temp-name pass).
/// 4. **Path-separator / traversal refusal:** any target containing `/` or `\`,
///    OR a target that escapes its directory (`..`, `.`, or any name that is
///    purely dots/spaces — which Windows trims to a degenerate target) is a
///    [`Conflict::Separator`] (mirrors flatten's `encode_no_separator`). Such a
///    name cannot be a base name inside the parent directory, so it can never be
///    "safe": `PathBuf::join("..")` resolves to the GRANDPARENT and `join(".")`
///    to the parent itself, escaping the intended target dir (CR-01).
///
/// Note: no-op renames (`new == old` byte-exact) are filtered out by the caller
/// BEFORE this runs (they are `(unchanged)` skips), so a name folding to its own
/// key is, by construction, a real case-only rename — never a self-collision.
fn preflight(renames: &[Rename], existing: &[String]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    // Rule 4 first: a separator-injecting or directory-escaping target can never
    // be safe, regardless of collisions. Refuse it outright.
    for r in renames {
        if injects(&r.new) {
            conflicts.push(Conflict::Separator {
                source: r.old.clone(),
                target: r.new.clone(),
            });
        }
    }
    // The set of renames that are NOT separator-injecting/escaping; only these
    // participate in the collision/cycle analysis (a refused one already aborts).
    let safe: Vec<&Rename> = renames.iter().filter(|r| !injects(&r.new)).collect();

    // Rule 1: seed the occupied set from on-disk names NOT being renamed away.
    // A name that some item renames AWAY frees its slot; everything else still
    // occupies its slot and a target landing on it is a clobber.
    let renamed_away: HashSet<String> = safe.iter().map(|r| fold(&r.old)).collect();
    let mut occupied: HashSet<String> = HashSet::new();
    for name in existing {
        let key = fold(name);
        if !renamed_away.contains(&key) {
            occupied.insert(key);
        }
    }

    // Rule 2a: two or more sources wanting the SAME target (case-folded). Group
    // sources by their target key; any group of >= 2 is a collision.
    let mut by_target: HashMap<String, Vec<String>> = HashMap::new();
    for r in &safe {
        by_target
            .entry(fold(&r.new))
            .or_default()
            .push(r.old.clone());
    }
    // Deterministic order: sort the contested keys so the abort message is stable.
    let mut contested: Vec<(&String, &Vec<String>)> =
        by_target.iter().filter(|(_, v)| v.len() >= 2).collect();
    contested.sort_by(|a, b| a.0.cmp(b.0));
    for (target_key, sources) in contested {
        // Recover an exact target name for the message (any of the colliding
        // renames produced it; they all fold to the same key).
        let exact_target = safe
            .iter()
            .find(|r| &fold(&r.new) == target_key)
            .map(|r| r.new.clone())
            .unwrap_or_else(|| target_key.clone());
        let mut srcs = sources.clone();
        srcs.sort();
        conflicts.push(Conflict::Collision {
            target: exact_target,
            sources: srcs,
        });
    }

    // Rule 2b: a target landing on a pre-existing on-disk name not renamed away
    // (the occupied set), even if only ONE source wants it.
    for r in &safe {
        let key = fold(&r.new);
        if occupied.contains(&key) {
            conflicts.push(Conflict::Collision {
                target: r.new.clone(),
                sources: vec![r.old.clone()],
            });
        }
    }

    // Rule 3: cycles/swaps — a target equal (case-folded) to ANOTHER item's
    // source. The other source is still present at plan time, so the move would
    // clobber it unless that item moves first — which a single-pass rename cannot
    // guarantee. v1 aborts. A target equal to its OWN source is a case-only
    // rename (the no-op filter already removed byte-exact ones), NOT a cycle.
    let sources_by_key: HashSet<String> = safe.iter().map(|r| fold(&r.old)).collect();
    for r in &safe {
        let target_key = fold(&r.new);
        if fold(&r.old) == target_key {
            continue; // case-only rename onto itself — not a cycle
        }
        if sources_by_key.contains(&target_key) {
            conflicts.push(Conflict::Cycle {
                source: r.old.clone(),
                target: r.new.clone(),
            });
        }
    }

    conflicts
}

/// Whether a rename target is a path-separator injection OR a directory-escaping
/// / degenerate name that can never be a safe base name inside the parent dir
/// (CR-01). Refused in pre-flight as a [`Conflict::Separator`], mirroring
/// `flatten::rename`'s `..`/`.`/empty handling:
/// - contains `/` or `\` — a path separator that would write outside the dir;
/// - is exactly `..` (`join` -> the GRANDPARENT) or `.` (`join` -> the parent);
/// - is purely dots and/or spaces (e.g. `...`, `  `) — Windows trims trailing
///   dots/spaces, so such a name collapses to ``/`.`/`..`, a degenerate target.
fn injects(name: &str) -> bool {
    name.contains('/')
        || name.contains('\\')
        || name == ".."
        || name == "."
        || name.trim_matches(['.', ' ']).is_empty()
}

/// Full-Unicode case fold for collision keys (WR-01) — matches
/// `flatten::rename::dedupe`, so non-ASCII case pairs (`RÉSUMÉ` vs `résumé`) also
/// collide on the case-insensitive NTFS filesystem.
fn fold(name: &str) -> String {
    name.to_lowercase()
}

impl RunCommand for BulkRenameArgs {
    fn run(self) -> anyhow::Result<()> {
        // (1) Compile the regex; a bad pattern is a clean anyhow error (exit 1),
        //     never a panic (FOUND-05).
        let re = Regex::new(&self.pattern)
            .with_context(|| format!("compiling regex pattern {:?}", self.pattern))?;

        // (2)/(3) Walk the scope and build the plan.
        let mut plan = build_plan(&self.dir, &re, &self.replacement, self.recursive)?;

        // (3b) RENM-V2-01: expand the literal `{n}` counter and fold `--case` over
        //      the deterministic SORTED plan order (D-21 apply order: re.replace →
        //      {n} → --case), BEFORE the unchanged pre-flight — so every generated
        //      name still flows through the load-bearing collision/cycle/separator
        //      detector exactly as today (the safety logic is untouched).
        apply_number_and_case_to_plan(
            &mut plan,
            self.case,
            self.number_width,
            self.start,
            self.step,
        );

        // (4) Per-directory pre-flight (D-18 ABORT-ALL). Partition the real
        //     renames by parent dir; collision scope is per-directory (D-14).
        let conflicts = preflight_plan(&plan)?;

        let arrow_col = arrow_col(&plan);
        let width = terminal_width();

        if !conflicts.is_empty() {
            // A3 / D-09: under --json the abort path must keep stdout EMPTY — the
            // plan-with-conflicts is human chrome that would corrupt the machine
            // channel, and there is NO {"error":…} envelope (D-09). Guard the
            // stdout print behind `if !is_json_on()`; the `bail!` error always goes
            // to stderr (main.rs maps it to exit 1), so under --json the conflict
            // explanation still reaches the user via stderr.
            if !crate::core::output::is_json_on() {
                // Print the plan with `[collision]` inline reasons on offending
                // rows so the user sees exactly what clashed, then the abort
                // summary.
                print_plan_with_conflicts(&plan, &conflicts, arrow_col, width);
                println!();
            }
            bail!("{}", abort_summary(&conflicts));
        }

        // (5) Dry-run is the DEFAULT: preview + summary, write nothing.
        let (to_rename, unchanged, skipped) = tally(&plan);
        if !self.force {
            // Fork on `is_json_on()` FIRST (Pitfall 1): under --json emit the PLAN
            // with `dry_run: true` (D-12 — --json is orthogonal to --force); all
            // human chrome (rows + summary) is suppressed.
            if crate::core::output::is_json_on() {
                let doc = RenameOutput {
                    count: plan.items.len(),
                    results: rename_rows(&plan),
                    dry_run: true,
                    to_rename,
                    unchanged,
                    skipped,
                };
                crate::core::output::emit_json(&doc)?;
                return Ok(());
            }
            print_plan(&plan, arrow_col, width);
            println!();
            println!(
                "Dry run: {to_rename} to rename, {unchanged} unchanged, {skipped} skipped. \
                 Re-run with --force to apply."
            );
            return Ok(());
        }

        // --force: execute only AFTER a clean pre-flight. A predictable collision
        // already aborted above; here we stop on the first UNEXPECTED I/O error.
        // Whether the rows are PRINTED depends on the fork: D-12 override — under
        // --json the applied rename rows are EMITTED as one document, while the
        // human --force path stays silent-on-success (only a "Done:" summary).
        let json = crate::core::output::is_json_on();

        // RENM-V2-02 / D-22: when `--backup` is set (only meaningful with
        // `--force`), write the FULL undo manifest (every entry `applied:false`) and
        // `File::sync_all()` it BEFORE the first `std::fs::rename`, then flip each
        // entry as its rename returns. This runs strictly AFTER the clean pre-flight
        // above, so a colliding plan never reaches here — abort writes no manifest.
        let mut backup: Option<(PathBuf, BackupManifest)> = if self.backup {
            let entries = build_manifest(&plan);
            // `%LOCALAPPDATA%\box\undo\` — OUTSIDE the renamed tree (Pitfall 8);
            // LOCALAPPDATA, not APPDATA. Fall back to the target dir only if unset.
            let manifest_dir = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|p| p.join("box").join("undo"))
                .unwrap_or_else(|| self.dir.clone());
            std::fs::create_dir_all(&manifest_dir).with_context(|| {
                format!("creating undo manifest dir {}", manifest_dir.display())
            })?;
            // `<id>` = a sortable timestamp `box-undo-<unix_millis>` (A5).
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let id = format!("box-undo-{millis}");
            let manifest_path = manifest_dir.join(format!("{id}.json"));
            let manifest = BackupManifest {
                id,
                dir: self.dir.to_string_lossy().into_owned(),
                entries,
            };
            // Write + fsync the all-`applied:false` manifest BEFORE the first rename.
            write_manifest(&manifest_path, &manifest)?;
            // Echo the manifest path to stderr (stdout stays pure under --json).
            eprintln!("Backup manifest: {}", manifest_path.display());
            Some((manifest_path, manifest))
        } else {
            None
        };

        let mut renamed = 0usize;
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
                ItemKind::Rename => {
                    let new_name = item
                        .new_name
                        .as_deref()
                        .expect("rename items always have a new name");
                    let dst = item.parent.join(new_name);
                    std::fs::rename(&item.src, &dst).with_context(|| {
                        format!("renaming {} -> {}", item.src.display(), dst.display())
                    })?;
                    // D-22: this rename returned Ok — flip its manifest entry's
                    // `applied` flag and persist (rewrite + fsync) so the on-disk
                    // flags always partition done-vs-pending. The manifest entries
                    // are built from `ItemKind::Rename` items in plan order, so
                    // `renamed` (incremented BELOW) indexes the next entry; use it
                    // as the current entry index here, before the increment.
                    if let Some((path, manifest)) = backup.as_mut() {
                        manifest.entries[renamed].applied = true;
                        write_manifest(path, manifest)?;
                    }
                    renamed += 1;
                    if !json {
                        println!(
                            "{}",
                            format_row(
                                item.kind.status(),
                                &item.src_label,
                                Some(new_name),
                                item.reason.as_deref(),
                                arrow_col,
                                width,
                            )
                        );
                    }
                }
            }
        }

        // WR-06: `count`/`results` are derived from the PLAN, not the executed
        // loop. That is sound here because reaching this point means the loop ran
        // to completion (any `std::fs::rename` error is `?`-propagated above →
        // exit 1 with empty stdout, no JSON emitted), so plan == outcome: every
        // `Rename` item was applied (`renamed == to_rename`) and the three tallies
        // partition the plan. These debug assertions pin that coupling so a future
        // executor/planner divergence trips in test/dev builds instead of silently
        // misreporting the JSON.
        debug_assert_eq!(
            renamed, to_rename,
            "executed renames must equal the planned rename count on a successful run"
        );
        debug_assert_eq!(
            to_rename + unchanged + skipped,
            plan.items.len(),
            "the plan tallies must partition plan.items (count is plan.items.len())"
        );

        // D-12 override: under --json emit the applied rename rows (the ONLY stdout
        // write), `dry_run: false`. The human `--force` path stays silent-on-
        // success with just the "Done:" summary.
        if json {
            let doc = RenameOutput {
                count: plan.items.len(),
                results: rename_rows(&plan),
                dry_run: false,
                to_rename,
                unchanged,
                skipped,
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        println!();
        println!("Done: renamed {renamed} files, {unchanged} unchanged, {skipped} skipped.");
        let _ = is_color_on(); // color is applied inside format_row's glyph wrap
        Ok(())
    }
}

/// Walk the scope (top-level files, or the full `--recursive` walk) and build the
/// plan: a `~` rename for each file whose regex-replaced name differs, a `-` skip
/// for directories, symlinks, and byte-exact no-ops.
fn build_plan(dir: &Path, re: &Regex, replacement: &str, recursive: bool) -> anyhow::Result<Plan> {
    let mut plan = Plan::default();

    // The walk: `--recursive` reuses flatten's hidden-pruned, symlink-no-follow
    // walk; the default is the same walk capped at depth 1 (top-level only, D-14).
    let mut walker = WalkDir::new(dir).follow_links(false).min_depth(1);
    if !recursive {
        walker = walker.max_depth(1);
    }

    for entry in walker.into_iter().filter_entry(|e| !is_hidden(e)) {
        let entry = entry.with_context(|| format!("walking {}", dir.display()))?;
        let path = entry.path();

        let rel = path.strip_prefix(dir).unwrap_or(path);
        let src_label = rel.to_string_lossy().to_string();
        let parent = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| dir.to_path_buf());

        // Directories and symlinks/junctions are `-` skip rows (D-15) — never
        // renamed (renaming a dir mid-walk under --recursive is hazardous).
        let is_symlink = entry.path_is_symlink();
        if is_symlink {
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                parent,
                old_name: name_of(path),
                src_label,
                new_name: None,
                kind: ItemKind::Skip,
                reason: Some("(skipped: symlink)".to_string()),
            });
            plan.skipped += 1;
            continue;
        }
        if entry.file_type().is_dir() {
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                parent,
                old_name: name_of(path),
                src_label,
                new_name: None,
                kind: ItemKind::Skip,
                reason: Some("(skipped: directory)".to_string()),
            });
            plan.skipped += 1;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        let old_name = name_of(path);
        // Match against the FULL base name incl. extension (D-16); FIRST match
        // only (`Regex::replace`, D-17).
        let new_name = re.replace(&old_name, replacement).into_owned();

        if new_name == old_name {
            // Byte-exact no-op → `(unchanged)` skip (D-18.4). A case-only change is
            // byte-DIFFERENT and falls through to a real rename below.
            plan.items.push(PlanItem {
                src: path.to_path_buf(),
                parent,
                old_name,
                src_label,
                new_name: None,
                kind: ItemKind::Skip,
                reason: Some("(unchanged)".to_string()),
            });
            plan.unchanged += 1;
            continue;
        }

        plan.items.push(PlanItem {
            src: path.to_path_buf(),
            parent,
            old_name,
            src_label,
            new_name: Some(new_name),
            kind: ItemKind::Rename,
            reason: None,
        });
        plan.to_rename += 1;
    }

    Ok(plan)
}

/// Apply the `{n}` counter and `--case` fold to a built [`Plan`] over the
/// deterministic SORTED source order (RENM-V2-01, D-21), BEFORE the unchanged
/// pre-flight. The counter is assigned in sorted-by-source order so it is
/// reproducible across runs/machines (RESEARCH Pitfall 7) — never walk order.
///
/// Operates on every regular FILE — both real renames AND no-op `(unchanged)` skips
/// — so `--case`/`{n}` apply even when the regex replacement was a byte-exact no-op
/// (e.g. `box bulk-rename . "(.*)" "$1" --case upper` uppercases everything).
/// Directory and symlink skips are excluded (only files are numbered). After the
/// transform the byte-exact no-op check is RE-RUN per item: a transformed name equal
/// to its source becomes an `(unchanged)` skip; a former no-op the transform changed
/// is promoted to a real rename. The tallies stay in sync so the summary is correct.
fn apply_number_and_case_to_plan(
    plan: &mut Plan,
    case: Option<Case>,
    number_width: Option<usize>,
    start: usize,
    step: usize,
) {
    // Eligible for numbering/case: real renames and no-op (unchanged) skips.
    // Directory/symlink skips carry a different reason and are left untouched.
    let mut idx: Vec<usize> = plan
        .items
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            it.kind == ItemKind::Rename || it.reason.as_deref() == Some("(unchanged)")
        })
        .map(|(i, _)| i)
        .collect();

    // Reproducible counter: sort by absolute source path, then the display label
    // (Pitfall 7 — never the arbitrary walk order).
    idx.sort_by(|&a, &b| {
        plan.items[a]
            .src
            .cmp(&plan.items[b].src)
            .then_with(|| plan.items[a].src_label.cmp(&plan.items[b].src_label))
    });

    // Auto width = digits needed for the file count unless --number-width pins it.
    let width = number_width.unwrap_or_else(|| digit_count(idx.len()));

    let mut n = start;
    for &i in &idx {
        // Base name = the post-re.replace name for a rename, or the source name for
        // a no-op skip (whose re.replace produced no change).
        let base = plan.items[i]
            .new_name
            .clone()
            .unwrap_or_else(|| plan.items[i].old_name.clone());
        let transformed = apply_number_and_case(&base, n, width, case);
        // Every eligible file consumes a counter value (whether or not it uses
        // `{n}`), so numbering stays stable regardless of no-op collapses.
        n = n.saturating_add(step);

        let was_rename = plan.items[i].kind == ItemKind::Rename;
        let is_noop = transformed == plan.items[i].old_name;
        match (was_rename, is_noop) {
            (true, true) => {
                // A rename the transform collapsed to a byte-exact no-op.
                plan.items[i].kind = ItemKind::Skip;
                plan.items[i].new_name = None;
                plan.items[i].reason = Some("(unchanged)".to_string());
                plan.to_rename -= 1;
                plan.unchanged += 1;
            }
            (true, false) => {
                plan.items[i].new_name = Some(transformed);
            }
            (false, true) => { /* still a no-op — leave the (unchanged) skip as-is */ }
            (false, false) => {
                // A former no-op the transform turned into a real rename.
                plan.items[i].kind = ItemKind::Rename;
                plan.items[i].new_name = Some(transformed);
                plan.items[i].reason = None;
                plan.unchanged -= 1;
                plan.to_rename += 1;
            }
        }
    }
}

/// Expand the literal `{n}` token then fold case (RENM-V2-01, D-21 apply order:
/// `{n}` THEN `--case`). `{n}` is the LITERAL token (≠ the regex crate's `${n}`
/// group syntax); write `{{n}}` for a literal `{n}` (A4 brace escape). `title`
/// operates on the stem only (extension preserved).
fn apply_number_and_case(name: &str, n: usize, width: usize, case: Option<Case>) -> String {
    let numbered = expand_number(name, n, width);
    match case {
        Some(Case::Upper) => numbered.to_uppercase(),
        Some(Case::Lower) => numbered.to_lowercase(),
        Some(Case::Title) => title_case_stem(&numbered),
        None => numbered,
    }
}

/// Replace the literal `{n}` token with the zero-padded counter, honoring the
/// `{{n}}` escape for a literal `{n}` (A4). The escape is protected with a NUL
/// sentinel (a byte that can never appear in a file name) so a real `{n}` nested
/// inside the escape is not expanded.
fn expand_number(name: &str, n: usize, width: usize) -> String {
    const SENTINEL: &str = "\u{0}box-literal-n\u{0}";
    let num = format!("{n:0width$}");
    name.replace("{{n}}", SENTINEL)
        .replace("{n}", &num)
        .replace(SENTINEL, "{n}")
}

/// Title-case the STEM of `name`, leaving the final extension untouched (D-21). The
/// extension is everything after the LAST `.` (a leading-dot name like `.gitignore`
/// is treated as all-stem, no extension). Within the stem, the first alphanumeric of
/// each word (word = a run separated by non-alphanumerics, e.g. `_`/`-`/space) is
/// uppercased and the rest lowercased.
fn title_case_stem(name: &str) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, Some(e)),
        _ => (name, None),
    };
    let titled = title_case_words(stem);
    match ext {
        Some(e) => format!("{titled}.{e}"),
        None => titled,
    }
}

/// Title-case a bare string: uppercase the first alphanumeric of each word,
/// lowercase the rest; non-alphanumerics are copied verbatim and start a new word.
fn title_case_words(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            if at_word_start {
                out.extend(ch.to_uppercase());
            } else {
                out.extend(ch.to_lowercase());
            }
            at_word_start = false;
        } else {
            out.push(ch);
            at_word_start = true;
        }
    }
    out
}

/// Digits needed to print `count` (at least 1) — the auto `--number-width` when the
/// flag is not given.
fn digit_count(count: usize) -> usize {
    let mut digits = 1;
    let mut v = count;
    while v >= 10 {
        v /= 10;
        digits += 1;
    }
    digits
}

/// The full plan plus pre-tallied counts for the dry-run summary.
#[derive(Debug, Default)]
struct Plan {
    items: Vec<PlanItem>,
    to_rename: usize,
    unchanged: usize,
    skipped: usize,
}

/// The serde projection of one [`PlanItem`] for `box bulk-rename --json` (SPINE-02,
/// D-13): `{src, dst, action, reason}`. `src` is the source label (lossy string,
/// D-4), `dst` the new base name (`None` for skips), `action` the lowercased
/// [`RowStatus`] (`"rename"`/`"skip"`), `reason` the same inline reason the human
/// row shows. The raw fields are serialized — NEVER the aligned `format_row`
/// output.
#[derive(serde::Serialize)]
struct RenameRow {
    src: String,
    dst: Option<String>,
    action: &'static str,
    reason: Option<String>,
}

/// The `box bulk-rename --json` document (D-12/D-13): the always-wrapped
/// `{results,count}` shape plus a `dry_run` boolean and the locked sibling summary
/// counts (`to_rename`/`unchanged`/`skipped`).
#[derive(serde::Serialize)]
struct RenameOutput {
    results: Vec<RenameRow>,
    count: usize,
    dry_run: bool,
    to_rename: usize,
    unchanged: usize,
    skipped: usize,
}

/// The lowercased `action` string for a plan item (D-13) — the lowercased
/// [`RowStatus`] spelling, reusing the same `status()` source of truth the human
/// glyph derives from (no-drift). bulk-rename only ever produces `rename`/`skip`.
fn action_str(kind: ItemKind) -> &'static str {
    match kind.status() {
        RowStatus::Copy => "copy",
        RowStatus::Rename => "rename",
        RowStatus::Skip => "skip",
    }
}

/// Project the plan's items into the JSON `.results` rows (raw fields, not
/// `format_row` layout, D-13).
fn rename_rows(plan: &Plan) -> Vec<RenameRow> {
    plan.items
        .iter()
        .map(|item| RenameRow {
            src: item.src_label.clone(),
            dst: item.new_name.clone(),
            action: action_str(item.kind),
            reason: item.reason.clone(),
        })
        .collect()
}

/// One `{old, new, applied}` record in the `--backup` undo manifest (RENM-V2-02,
/// D-22): the ABSOLUTE source path, the ABSOLUTE destination path, and whether the
/// rename has been applied yet. Both paths are `to_string_lossy` (D-4 — never
/// `to_str().unwrap()`, which panics on non-UTF-8 NTFS names).
#[derive(serde::Serialize)]
struct BackupEntry {
    old: String,
    new: String,
    applied: bool,
}

/// The `--backup` undo manifest (RENM-V2-02, D-22): a sortable `id`, the target
/// `dir`, and one [`BackupEntry`] per renamed file. A zero-drift serde projection
/// of the pre-flight-cleared [`Plan`] — write + fsync the FULL manifest (all
/// `applied:false`) BEFORE the first rename, then flip each entry as its rename
/// returns so the on-disk flags always partition done-vs-pending.
#[derive(serde::Serialize)]
struct BackupManifest {
    id: String,
    dir: String,
    entries: Vec<BackupEntry>,
}

/// Build the undo-manifest entries from the pre-flight-cleared plan: one entry per
/// `ItemKind::Rename` item, in plan order (so the executor's per-rename `applied`
/// flip lines up index-for-index). `old` is the absolute source path (`item.src`);
/// `new` is `item.parent.join(new_name)`. Both are absolute when the target dir is
/// absolute. A zero-drift projection of the SAME items the executor consumes.
fn build_manifest(plan: &Plan) -> Vec<BackupEntry> {
    plan.items
        .iter()
        .filter(|item| item.kind == ItemKind::Rename)
        .map(|item| {
            let new_name = item
                .new_name
                .as_deref()
                .expect("rename items always have a new name");
            BackupEntry {
                old: item.src.to_string_lossy().into_owned(),
                new: item.parent.join(new_name).to_string_lossy().into_owned(),
                applied: false,
            }
        })
        .collect()
}

/// Atomically-as-possible persist the manifest: (re)create the file, write it
/// pretty-printed, and `File::sync_all()` (fsync) so the bytes are durable on disk
/// before the next rename runs. Called once for the all-`applied:false` manifest
/// before the first rename, then once per rename to flip an `applied` flag —
/// keeping the on-disk `applied` flags a faithful partition of done-vs-pending even
/// across a crash or a mid-batch I/O error (Pitfall 8).
fn write_manifest(path: &Path, manifest: &BackupManifest) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("creating undo manifest {}", path.display()))?;
    serde_json::to_writer_pretty(&file, manifest)
        .with_context(|| format!("writing undo manifest {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("fsync'ing undo manifest {}", path.display()))?;
    Ok(())
}

/// The base name of `path` as an owned `String`.
fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Run the pure [`preflight`] detector per parent directory (collision scope is
/// per-directory, D-14) and collect every conflict across the plan. Reads the
/// CURRENT on-disk names per directory to seed each occupied set.
fn preflight_plan(plan: &Plan) -> anyhow::Result<Vec<Conflict>> {
    // Group the real renames by parent directory.
    let mut by_dir: HashMap<PathBuf, Vec<Rename>> = HashMap::new();
    for item in &plan.items {
        if item.kind != ItemKind::Rename {
            continue;
        }
        let new = item
            .new_name
            .clone()
            .expect("rename items always have a new name");
        by_dir.entry(item.parent.clone()).or_default().push(Rename {
            old: item.old_name.clone(),
            new,
        });
    }

    let mut conflicts = Vec::new();
    // Deterministic directory order for a stable abort message.
    let mut dirs: Vec<&PathBuf> = by_dir.keys().collect();
    dirs.sort();
    for dir in dirs {
        let renames = &by_dir[dir];
        // Seed the occupied set from the CURRENT on-disk names in this directory.
        let existing =
            read_dir_names(dir).with_context(|| format!("reading directory {}", dir.display()))?;
        conflicts.extend(preflight(renames, &existing));
    }
    Ok(conflicts)
}

/// The base names of every entry (file, dir, symlink) directly inside `dir`.
fn read_dir_names(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("reading an entry of {}", dir.display()))?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    Ok(names)
}

/// Tally `(to_rename, unchanged, skipped)`. `skipped` excludes `unchanged` so the
/// summary distinguishes "would-be-no-op" from "not-a-file".
fn tally(plan: &Plan) -> (usize, usize, usize) {
    (plan.to_rename, plan.unchanged, plan.skipped)
}

/// The alignment column for the `->` arrow: the widest source label across all
/// rename rows (so destinations line up).
fn arrow_col(plan: &Plan) -> usize {
    plan.items
        .iter()
        .filter(|i| i.new_name.is_some())
        .map(|i| i.src_label.chars().count())
        .max()
        .unwrap_or(0)
}

/// Print every plan row (used by the dry-run preview and the `--force` path's
/// skip rows). Real renames are printed as they execute.
fn print_plan(plan: &Plan, arrow_col: usize, width: usize) {
    for item in &plan.items {
        println!(
            "{}",
            format_row(
                item.kind.status(),
                &item.src_label,
                item.new_name.as_deref(),
                item.reason.as_deref(),
                arrow_col,
                width,
            )
        );
    }
}

/// Print the plan with `[collision]` / `[cycle]` / `[separator]` inline reasons on
/// the rows whose target appears in a conflict, so the abort output shows exactly
/// what clashed (CONTEXT.md § specifics).
fn print_plan_with_conflicts(plan: &Plan, conflicts: &[Conflict], arrow_col: usize, width: usize) {
    for item in &plan.items {
        let reason = conflict_reason(item, conflicts).or_else(|| item.reason.clone());
        println!(
            "{}",
            format_row(
                item.kind.status(),
                &item.src_label,
                item.new_name.as_deref(),
                reason.as_deref(),
                arrow_col,
                width,
            )
        );
    }
}

/// The inline reason for a row if its (old, new) pair participates in a conflict.
fn conflict_reason(item: &PlanItem, conflicts: &[Conflict]) -> Option<String> {
    let new = item.new_name.as_deref()?;
    for c in conflicts {
        match c {
            Conflict::Collision { target, sources } => {
                if fold(target) == fold(new) && sources.iter().any(|s| s == &item.old_name) {
                    return Some("[collision]".to_string());
                }
            }
            Conflict::Cycle { source, target } => {
                if source == &item.old_name && fold(target) == fold(new) {
                    return Some("[cycle]".to_string());
                }
            }
            Conflict::Separator { source, target } => {
                if source == &item.old_name && target == new {
                    return Some("[separator]".to_string());
                }
            }
        }
    }
    None
}

/// The locked abort summary (CONTEXT.md § specifics wording): a leading count of
/// conflicts, a one-line explanation per conflict, and the `No files were
/// renamed.` guarantee.
fn abort_summary(conflicts: &[Conflict]) -> String {
    let n = conflicts.len();
    let noun = if n == 1 { "conflict" } else { "conflicts" };
    let mut out = format!("Aborted: {n} {noun} detected.");
    for c in conflicts {
        out.push(' ');
        match c {
            Conflict::Collision { target, sources } => {
                if sources.len() >= 2 {
                    out.push_str(&format!("{} both rename to {target}.", join_and(sources)));
                } else {
                    let src = sources.first().cloned().unwrap_or_default();
                    out.push_str(&format!("{src} renames to {target}, which already exists."));
                }
            }
            Conflict::Cycle { source, target } => {
                out.push_str(&format!(
                    "{source} renames to {target}, but {target} is itself being renamed (a cycle)."
                ));
            }
            Conflict::Separator { source, target } => {
                out.push_str(&format!(
                    "{source} renames to {target}, which contains a path separator (refused)."
                ));
            }
        }
    }
    out.push_str(" No files were renamed.");
    out
}

/// Join names as `a, b and c` for the abort message.
fn join_and(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} and {}", names[0], names[1]),
        _ => {
            let (last, head) = names.split_last().unwrap();
            format!("{} and {}", head.join(", "), last)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rn(old: &str, new: &str) -> Rename {
        Rename {
            old: old.to_string(),
            new: new.to_string(),
        }
    }

    /// Rule 2a: two sources renaming to ONE target is a collision.
    #[test]
    fn detects_two_sources_one_target() {
        let renames = vec![rn("a.txt", "dup.txt"), rn("b.txt", "dup.txt")];
        let conflicts = preflight(&renames, &["a.txt".into(), "b.txt".into()]);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            &conflicts[0],
            Conflict::Collision { target, sources }
                if target == "dup.txt" && sources.len() == 2
        ));
    }

    /// Rule 2b: a target landing on a pre-existing on-disk name not renamed away
    /// is a collision even with a single source.
    #[test]
    fn detects_clobber_of_existing_file() {
        // `a.txt` -> `keep.txt`, but `keep.txt` already exists and is NOT renamed.
        let renames = vec![rn("a.txt", "keep.txt")];
        let conflicts = preflight(&renames, &["a.txt".into(), "keep.txt".into()]);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            &conflicts[0],
            Conflict::Collision { target, .. } if target == "keep.txt"
        ));
    }

    /// Rule 2b negative: a target landing on a name that IS being renamed away is
    /// NOT a collision (the slot is freed).
    #[test]
    fn no_clobber_when_existing_is_renamed_away() {
        // `a.txt` -> `b.txt` and `b.txt` -> `c.txt`: b.txt's slot is freed, so
        // a.txt landing on b.txt is... a CYCLE (b is still a source), but NOT a
        // plain clobber-collision. Test the simpler chain that frees cleanly:
        // `a.txt` -> `final.txt` while `old.txt` -> `a.txt` (a.txt freed).
        let renames = vec![rn("a.txt", "final.txt"), rn("old.txt", "a.txt")];
        let conflicts = preflight(
            &renames,
            &["a.txt".into(), "old.txt".into(), "final.txt".into()],
        );
        // `final.txt` pre-exists and is NOT renamed away -> a.txt clobbers it.
        // That's the only conflict; `old.txt`->`a.txt` is fine because a.txt is
        // freed (renamed away to final.txt) — proving the renamed-away exclusion.
        assert!(
            conflicts
                .iter()
                .any(|c| matches!(c, Conflict::Collision { target, .. } if target == "final.txt")),
            "final.txt clobber must be detected: {conflicts:?}"
        );
        // a.txt is renamed away, so old.txt -> a.txt is NOT a clobber-collision.
        assert!(
            !conflicts.iter().any(|c| matches!(
                c,
                Conflict::Collision { target, sources }
                    if target == "a.txt" && sources == &vec!["old.txt".to_string()]
            )),
            "old.txt -> a.txt must not be a clobber (a.txt's slot is freed): {conflicts:?}"
        );
    }

    /// Rule 3: a swap `ab.txt <-> ba.txt` (each target equals the other's source)
    /// is a cycle.
    #[test]
    fn detects_swap_cycle() {
        let renames = vec![rn("ab.txt", "ba.txt"), rn("ba.txt", "ab.txt")];
        let conflicts = preflight(&renames, &["ab.txt".into(), "ba.txt".into()]);
        // Both directions are cycles (each target is the other's source).
        assert!(
            conflicts
                .iter()
                .filter(|c| matches!(c, Conflict::Cycle { .. }))
                .count()
                >= 1,
            "a swap must be detected as a cycle: {conflicts:?}"
        );
    }

    /// Rule 4: a case-only rename (`foo.txt` -> `Foo.txt`) is NOT a self-collision
    /// and NOT a cycle — it is a real, conflict-free rename (Pitfall 5).
    #[test]
    fn case_only_is_not_a_conflict() {
        let renames = vec![rn("foo.txt", "Foo.txt")];
        // The on-disk name is `foo.txt`; it is renamed away, so it does not occupy
        // its own slot, and the target folds to the same key as its own (renamed
        // -away) source — which must NOT be a clobber.
        let conflicts = preflight(&renames, &["foo.txt".into()]);
        assert!(
            conflicts.is_empty(),
            "a case-only rename must be conflict-free: {conflicts:?}"
        );
    }

    /// Rule 1: a path-separator-injecting target is refused (both `/` and `\`).
    #[test]
    fn refuses_path_separators() {
        let fwd = preflight(&[rn("a.txt", "sub/evil.txt")], &["a.txt".into()]);
        assert!(
            fwd.iter().any(|c| matches!(c, Conflict::Separator { .. })),
            "forward slash must be refused: {fwd:?}"
        );
        let back = preflight(&[rn("a.txt", "sub\\evil.txt")], &["a.txt".into()]);
        assert!(
            back.iter().any(|c| matches!(c, Conflict::Separator { .. })),
            "backslash must be refused: {back:?}"
        );
    }

    /// Rule 4 (CR-01): a target of exactly `..` or `.` — or any purely
    /// dots/spaces name — escapes the parent directory (`join("..")` -> the
    /// grandparent, `join(".")` -> the parent) and must be refused as a
    /// [`Conflict::Separator`] in the same pure pre-flight pass, just like a path
    /// separator. It must never reach the executor's `rename` call.
    #[test]
    fn refuses_dot_and_dotdot_targets() {
        for target in ["..", ".", "...", "  ", " . "] {
            let conflicts = preflight(&[rn("a.txt", target)], &["a.txt".into()]);
            assert!(
                conflicts
                    .iter()
                    .any(|c| matches!(c, Conflict::Separator { target: t, .. } if t == target)),
                "a directory-escaping/degenerate target {target:?} must be refused: {conflicts:?}"
            );
        }
    }

    /// `injects` recognizes separators and directory-escaping/degenerate names but
    /// passes ordinary base names (incl. ones that merely CONTAIN dots).
    #[test]
    fn injects_classifies_unsafe_targets() {
        // Unsafe: separators, exact dot/dot-dot, purely dots/spaces.
        for bad in ["a/b", "a\\b", "..", ".", "...", "  ", " .. "] {
            assert!(injects(bad), "{bad:?} must be classified as unsafe");
        }
        // Safe: ordinary names, including dotfiles and names with extensions.
        for ok in ["a.txt", ".gitignore", "..a", "a..b", "report.final.txt"] {
            assert!(!injects(ok), "{ok:?} must be classified as safe");
        }
    }

    /// A clean plan (distinct targets, no clobbers, no cycles, no separators)
    /// yields ZERO conflicts — the executable path.
    #[test]
    fn clean_plan_has_no_conflicts() {
        let renames = vec![
            rn("IMG_0042.jpg", "img_0042.jpg"),
            rn("IMG_0043.jpg", "img_0043.jpg"),
        ];
        let conflicts = preflight(&renames, &["IMG_0042.jpg".into(), "IMG_0043.jpg".into()]);
        assert!(
            conflicts.is_empty(),
            "clean plan must be conflict-free: {conflicts:?}"
        );
    }

    /// Full-Unicode case fold (WR-01): `RÉSUMÉ.txt` and `résumé.txt` collide.
    #[test]
    fn collision_key_is_full_unicode_folded() {
        // Two sources whose targets fold to the same key under full Unicode.
        let renames = vec![rn("a", "RÉSUMÉ.txt"), rn("b", "résumé.txt")];
        let conflicts = preflight(&renames, &["a".into(), "b".into()]);
        assert!(
            conflicts
                .iter()
                .any(|c| matches!(c, Conflict::Collision { .. })),
            "non-ASCII case pair must collide (WR-01): {conflicts:?}"
        );
    }

    /// The abort summary carries the locked phrasing: a conflict count and the
    /// `No files were renamed.` guarantee.
    #[test]
    fn abort_summary_wording() {
        let conflicts = vec![Conflict::Collision {
            target: "dup.txt".into(),
            sources: vec!["a.txt".into(), "b.txt".into()],
        }];
        let s = abort_summary(&conflicts);
        assert!(s.starts_with("Aborted: 1 conflict detected."), "got: {s}");
        assert!(
            s.contains("a.txt and b.txt both rename to dup.txt."),
            "got: {s}"
        );
        assert!(s.ends_with("No files were renamed."), "got: {s}");
    }

    // --- RENM-V2-01: {n} expansion + --case fold (pure post-passes) -------------

    /// `apply_number_and_case` expands `{n}` with zero-pad width, then folds case:
    /// `Upper`/`Lower` over the WHOLE name, `Title` over the stem only (D-21).
    #[test]
    fn apply_number_and_case_expands_and_folds() {
        // {n} expands with zero-pad width; case None leaves the rest as-is.
        assert_eq!(
            apply_number_and_case("img_{n}.JPG", 5, 3, None),
            "img_005.JPG"
        );
        // Upper/lower fold the WHOLE name (extension included).
        assert_eq!(
            apply_number_and_case("img_{n}.jpg", 5, 3, Some(Case::Upper)),
            "IMG_005.JPG"
        );
        assert_eq!(
            apply_number_and_case("IMG_{n}.JPG", 5, 3, Some(Case::Lower)),
            "img_005.jpg"
        );
        // Title folds the stem only; the extension stays as-is.
        assert_eq!(
            apply_number_and_case("hello_world.txt", 1, 1, Some(Case::Title)),
            "Hello_World.txt"
        );
    }

    /// `{{n}}` is the literal-brace escape → a literal `{n}`, never expanded (A4).
    #[test]
    fn apply_number_and_case_literal_brace_escape() {
        assert_eq!(
            apply_number_and_case("keep_{{n}}.txt", 7, 3, None),
            "keep_{n}.txt"
        );
        // A real {n} alongside an escaped one: only the real token expands.
        assert_eq!(
            apply_number_and_case("{n}_{{n}}.txt", 7, 2, None),
            "07_{n}.txt"
        );
    }

    /// `title_case_stem` capitalizes each word of the stem and preserves the final
    /// extension verbatim (D-21).
    #[test]
    fn title_case_stem_preserves_extension() {
        assert_eq!(title_case_stem("hello_world.txt"), "Hello_World.txt");
        // Mixed-case input is normalized per word; extension untouched.
        assert_eq!(title_case_stem("hELLO worLD.md"), "Hello World.md");
        // A leading-dot name has no extension → all stem (leading dot preserved).
        assert_eq!(title_case_stem(".gitignore"), ".Gitignore");
        // No extension at all.
        assert_eq!(title_case_stem("readme"), "Readme");
    }

    /// `digit_count` is the auto `--number-width`: digits needed for the file count.
    #[test]
    fn digit_count_auto_width() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(100), 3);
    }
}
