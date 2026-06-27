//! The `dupes` command: a content-duplicate finder (DUPE-01).
//!
//! Identity model (D-13, DUPE-V2-01) — a THREE-stage size → partial → full
//! cascade, each stage a cheaper pre-filter for the next:
//! 1. Walk the target read-only (hidden pruned via the shared `is_hidden`,
//!    symlinks NOT followed; NO noise list / NO `ignore` crate, D-06/D-07) and
//!    bucket every regular file by `metadata().len()`.
//! 2. Only same-size buckets of `>= 2` files are CANDIDATES — every unique-size
//!    file is skipped and is never hashed (most files are never hashed at all).
//! 3. PARTIAL stage (DUPE-V2-01): BLAKE3 only the first [`PARTIAL_BYTES`] of each
//!    candidate and re-bucket by `(size, partial_hash)`. Same-size files that
//!    differ in their prefix split here after a single small read, so the
//!    expensive full pass only runs on files that agree on size AND prefix.
//! 4. FULL stage: content-hash the surviving `(size, partial)` buckets of `>= 2`
//!    in PARALLEL with rayon, reusing the `hash` slice's BLAKE3 streaming path
//!    (`blake3::Hasher::update_reader`, Plan 03-01). BLAKE3 is chosen for SPEED —
//!    cryptographic-criticality is irrelevant for equality grouping (D-13). The
//!    first hash error short-circuits the `collect` to a clean `anyhow` error →
//!    exit 1, never a panic (T-03-17, FOUND-05). The full hash is the final
//!    arbiter — the partial stage can never change the grouping, only skip work.
//!
//! Hardlink-aware wasted space (DUPE-V2-01, RESEARCH Pitfall 6): content equality
//! is NOT the same as a shared inode. Within each confirmed-duplicate group, paths
//! that share one NTFS file-index `(dwVolumeSerialNumber, nFileIndex)` are a single
//! on-disk file under two names (a hardlink alias) and are COLLAPSED before the
//! wasted-space figure — a hardlink frees nothing if deleted, so it is never
//! counted as wasted: `wasted = (distinct_inodes - 1) * size` per group. The
//! identity is read with the STABLE Win32 `GetFileInformationByHandle`
//! ([`file_identity`]); the nightly-only std `windows_by_handle` handle fields this
//! project's STATE.md once pointed at (issue #63010 OPEN) are deliberately NOT used
//! (RESEARCH Pitfall 1 correction). The human render still LISTS every alias; only
//! the wasted figure collapses them.
//!
//! Determinism (RESEARCH Pitfall 6, T-03-16): rayon completion order is
//! arbitrary, so the `(hash, path)` pairs are `sort()`ed BEFORE grouping/printing
//! — consecutive equal hashes form a group, and only groups of `>= 2` are emitted.
//!
//! Output: each duplicate group (the identical files, one per line) followed by a
//! wasted-space summary = the sum over groups of `(group_len - 1) * file_size`
//! (the bytes occupied by the redundant copies), rendered with the shared
//! `core::output::human_size`. Any styled token is gated on `is_color_on()` so
//! piped output is byte-identical minus ANSI (D-10). Groups go to stdout
//! (FOUND-03).
//!
//! STRICTLY READ-ONLY (T-03-13, locked Out of Scope): there is NO write path here
//! — no `safe_copy`, rename, or delete. `dupes` only reads the filesystem.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::commands::RunCommand;
use crate::core::fs::{is_hidden, normalize_path};
use crate::core::output::{human_size, is_color_on};

/// `box dupes [PATH]` — find duplicate files by content (DUPE-01).
#[derive(Debug, Args)]
pub struct DupesArgs {
    /// Directory to scan for content duplicates (default: the current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// A confirmed duplicate group: the shared content hash, the common file size,
/// and the (sorted) paths of the identical files.
struct DupeGroup {
    size: u64,
    paths: Vec<PathBuf>,
}

/// The serde projection of one [`DupeGroup`] for `box dupes --json` (SPINE-02,
/// D-17). `paths` are `to_string_lossy()` STRINGS (D-4) — a non-UTF-8 NTFS path
/// never panics and never reaches `to_str().unwrap()`, matching the human
/// `path.display()` render (no-drift).
///
/// The content `hash` is INTENTIONALLY omitted (WR-05): the human render omits it
/// too (no drift), and emitting it would change the locked D-17 group schema that
/// `tests/dupes.rs::json_purity` pins. A consumer that needs to confirm byte
/// identity can re-hash the listed paths; surfacing the hash is a deliberate v1
/// non-goal, noted here so a future reviewer does not re-flag the omission.
///
/// Ordering guarantee: `paths` within a group are SORTED ascending (the
/// `(hash, path)` sort in `run()` before grouping, RESEARCH Pitfall 6), so a
/// consumer may rely on a deterministic intra-group path order — asserted by
/// `tests/dupes.rs::json_paths_sorted_within_group`.
#[derive(serde::Serialize)]
struct DupeRow {
    size: u64,
    paths: Vec<String>,
}

/// The `box dupes --json` document (D-17): `{results, count, wasted_bytes}` where
/// `count` is the number of duplicate groups and `wasted_bytes` is the redundant-
/// copy total (the SAME `wasted_space` figure the human summary prints).
#[derive(serde::Serialize)]
struct DupesOutput {
    results: Vec<DupeRow>,
    count: usize,
    wasted_bytes: u64,
}

impl RunCommand for DupesArgs {
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

        // `dupes` scans a directory tree: a FILE argument has nothing to walk, so
        // it would silently print `No duplicate files found.`. Refuse it with a
        // clear error instead (WR-02).
        if !root.is_dir() {
            anyhow::bail!("{} is not a directory", self.path.display());
        }

        // Bucket every non-hidden regular file by size (the cheap pre-filter).
        let by_size = collect_by_size(&root)?;

        // Candidates = the flattened union of same-size buckets with >= 2 files.
        // Unique-size files are never hashed (most files are never hashed at all).
        let candidates: Vec<(u64, PathBuf)> = by_size
            .into_iter()
            .filter(|(_, paths)| paths.len() >= 2)
            .flat_map(|(size, paths)| paths.into_iter().map(move |p| (size, p)))
            .collect();

        // PARTIAL stage (DUPE-V2-01): BLAKE3 the first PARTIAL_BYTES of each
        // candidate IN PARALLEL and re-bucket by (size, partial_hash). The first
        // partial-hash error short-circuits to a clean anyhow error (exit 1, no
        // panic, T-03-17). Same-size files with a different prefix split here after
        // a single small read, so the expensive full pass only runs on files that
        // already agree on BOTH size and prefix.
        let partial_hashed: Vec<(u64, String, PathBuf)> = candidates
            .par_iter()
            .map(|(size, path)| {
                let ph = partial_hash(path)
                    .with_context(|| format!("partial-hashing {}", path.display()))?;
                Ok((*size, ph, path.clone()))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut by_partial: HashMap<(u64, String), Vec<PathBuf>> = HashMap::new();
        for (size, ph, path) in partial_hashed {
            by_partial.entry((size, ph)).or_default().push(path);
        }

        // Only (size, partial) buckets with >= 2 files can hold a true duplicate;
        // everything else is provably unique after one prefix read.
        let full_candidates: Vec<(u64, PathBuf)> = by_partial
            .into_iter()
            .filter(|(_, paths)| paths.len() >= 2)
            .flat_map(|((size, _partial), paths)| paths.into_iter().map(move |p| (size, p)))
            .collect();

        // FULL stage: content-hash the surviving candidates IN PARALLEL (rayon).
        // The first hash error short-circuits the collect to a clean anyhow error
        // (exit 1, no panic, T-03-17). Each tuple is (hash, size, path). The full
        // hash is the final arbiter — it can never disagree with the partial stage's
        // pre-filter, only confirm or split within a (size, partial) bucket.
        let mut hashed: Vec<(String, u64, PathBuf)> = full_candidates
            .par_iter()
            .map(|(size, path)| {
                let hash = hash_file_blake3(path)
                    .with_context(|| format!("hashing {}", path.display()))?;
                Ok((hash, *size, path.clone()))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Determinism (RESEARCH Pitfall 6): sort by (hash, path) BEFORE grouping —
        // rayon order is arbitrary. Distinct-content groups make the order total.
        hashed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2)));

        // Group consecutive equal hashes; keep only groups of >= 2 (the dupes).
        let groups = group_duplicates(hashed);

        // Fork on `is_json_on()` FIRST (Pitfall 1): `render` has the empty-case
        // human line + the per-group lines + the wasted summary — ALL human chrome
        // that must NOT reach stdout under --json. The empty case maps to
        // `{results:[], count:0, wasted_bytes:0}`, never the "No duplicate files
        // found." line.
        if crate::core::output::is_json_on() {
            let doc = DupesOutput {
                count: groups.len(),
                wasted_bytes: wasted_space(&groups),
                // Project each group, serializing paths via `to_string_lossy`
                // (D-4) so non-UTF-8 NTFS paths never panic.
                results: groups
                    .iter()
                    .map(|g| DupeRow {
                        size: g.size,
                        paths: g
                            .paths
                            .iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect(),
                    })
                    .collect(),
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        // INVARIANT (WR-04): `render` (and every `println!` it makes) is reachable
        // ONLY here, AFTER the `is_json_on()` fork above `return`ed under `--json`.
        // Its raw prints intentionally bypass `out_line` (dupes is NOT a SPINE-04
        // `--clip` command, so its human render must not tee to the clipboard).
        // Never hoist a human write above the fork or it would contaminate the
        // JSON channel.
        render(&groups);
        Ok(())
    }
}

/// Walk `root` read-only and bucket every non-hidden regular file by its logical
/// `metadata().len()`. Reuses the shared walk skeleton VERBATIM — hidden pruned
/// via `is_hidden`, symlinks not followed (`follow_links(false)`), NO noise list /
/// NO `ignore` crate (D-06/D-07).
fn collect_by_size(root: &Path) -> anyhow::Result<HashMap<u64, Vec<PathBuf>>> {
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry.with_context(|| format!("scanning {}", root.display()))?;
        // Only regular files are duplicate candidates; dirs/symlinks are skipped
        // (symlinks are never followed, T-03-14).
        if !entry.file_type().is_file() {
            continue;
        }
        let size = entry
            .metadata()
            .with_context(|| format!("reading metadata for {}", entry.path().display()))?
            .len();
        by_size
            .entry(size)
            .or_default()
            .push(entry.path().to_path_buf());
    }
    Ok(by_size)
}

/// Stream `path` through the native stable `blake3::Hasher` and return the
/// lowercase 64-hex digest — the SAME content-equality path the `hash` slice uses
/// (`update_reader`, Plan 03-01, D-13). Streams with a SIMD-sized internal buffer;
/// never buffers the whole file (T-03-03).
fn hash_file_blake3(path: &Path) -> anyhow::Result<String> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    hash_reader_blake3(file)
}

/// The BLAKE3 streaming core (reader-generic so it is unit-testable without a real
/// file). Mirrors `hash::hash_blake3` from Plan 03-01 — the few-line native path is
/// lifted rather than widening the `hash` module's surface.
fn hash_reader_blake3<R: Read>(reader: R) -> anyhow::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(reader)
        .context("failed to read input while hashing")?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// How many leading bytes the PARTIAL stage hashes (DUPE-V2-01). 16 KiB is enough
/// to split most same-size files cheaply while reading at most one disk block run.
const PARTIAL_BYTES: u64 = 16 * 1024;

/// BLAKE3 the first [`PARTIAL_BYTES`] of `path` — the cheap re-bucketing stage
/// between size-bucketing and the full content hash. Reuses the reader-generic
/// [`hash_reader_blake3`] core over a `Read::take(PARTIAL_BYTES)`. For a file
/// `<= PARTIAL_BYTES` the partial covers the WHOLE file, so its partial hash already
/// proves byte-identity; the subsequent full pass over such a file is redundant but
/// harmless (correctness is unaffected, and the code stays a single uniform path).
fn partial_hash(path: &Path) -> anyhow::Result<String> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    hash_reader_blake3(file.take(PARTIAL_BYTES))
}

/// Fold a `(hash, size, path)` list — already sorted by `(hash, path)` — into the
/// duplicate groups: runs of consecutive equal hashes with `>= 2` members. The
/// resulting groups (and their paths) are deterministically ordered because the
/// input was sorted before grouping (RESEARCH Pitfall 6).
fn group_duplicates(hashed: Vec<(String, u64, PathBuf)>) -> Vec<DupeGroup> {
    let mut groups: Vec<DupeGroup> = Vec::new();
    let mut iter = hashed.into_iter();
    let Some((mut cur_hash, mut cur_size, first_path)) = iter.next() else {
        return groups;
    };
    let mut cur_paths = vec![first_path];
    for (hash, size, path) in iter {
        if hash == cur_hash {
            cur_paths.push(path);
        } else {
            if cur_paths.len() >= 2 {
                groups.push(DupeGroup {
                    size: cur_size,
                    paths: std::mem::take(&mut cur_paths),
                });
            }
            cur_hash = hash;
            cur_size = size;
            cur_paths = vec![path];
        }
    }
    if cur_paths.len() >= 2 {
        groups.push(DupeGroup {
            size: cur_size,
            paths: cur_paths,
        });
    }
    groups
}

/// Total wasted space = the sum over groups of `(distinct_inodes - 1) * file_size`
/// (the bytes the redundant copies occupy — one copy of each group is "kept").
///
/// Hardlink-aware (DUPE-V2-01, RESEARCH Pitfall 6): paths within a group that share
/// one NTFS file-index are a single on-disk file under several names, so they are
/// collapsed to ONE inode before the `(len - 1)` redundancy count — a hardlink alias
/// frees nothing if deleted and is never reported as wasted. [`distinct_inodes`]
/// counts a path whose identity cannot be read as its OWN inode (conservative — we
/// never UNDER-report wasted space on a transient `file_identity` error; this also
/// means the unit tests, whose synthetic paths do not exist on disk, see each path
/// as a distinct inode and so match the plain `(len - 1) * size` arithmetic).
fn wasted_space(groups: &[DupeGroup]) -> u64 {
    groups
        .iter()
        .map(|g| {
            let distinct = distinct_inodes(&g.paths) as u64;
            distinct.saturating_sub(1) * g.size
        })
        .sum()
}

/// Count the distinct inodes among a group's paths via [`file_identity`]: paths
/// sharing one `(volume_serial, file_index)` (hardlink aliases) count ONCE. A path
/// whose identity cannot be read is conservatively counted as its own distinct
/// inode (so wasted space is never under-reported).
fn distinct_inodes(paths: &[PathBuf]) -> usize {
    let mut ids: HashSet<(u32, u64)> = HashSet::new();
    let mut unknown = 0usize;
    for p in paths {
        match file_identity(p) {
            Ok(id) => {
                ids.insert(id);
            }
            Err(_) => unknown += 1,
        }
    }
    ids.len() + unknown
}

/// The stable filesystem identity of `path` as `(volume_serial, file_index)` — two
/// paths sharing one inode (a hardlink alias) return the SAME pair (DUPE-V2-01).
///
/// Windows: read `(dwVolumeSerialNumber, nFileIndex)` off an open handle via the
/// STABLE Win32 `GetFileInformationByHandle`. The std handle fields behind
/// `windows_by_handle` (issue #63010 OPEN) are NIGHTLY-only, so they are
/// deliberately NOT used here (RESEARCH Pitfall 1, correcting STATE.md:113 for this
/// stable-MSVC build). This is
/// the localized-FFI pattern (matching `du`'s `compressed_size`): one tiny wrapped
/// `unsafe`, a read-only metadata query that registers no OS state (T-8-03-FFI).
#[cfg(windows)]
fn file_identity(path: &Path) -> anyhow::Result<(u32, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = std::fs::File::open(path)
        .with_context(|| format!("opening {} for file identity", path.display()))?;
    // `file` is kept alive for the whole call, so the borrowed handle stays valid.
    // `RawHandle` is `*mut c_void`, matching `HANDLE`'s field — no cast needed.
    let handle = HANDLE(file.as_raw_handle());
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `handle` is a live, valid file handle owned by `file` (alive for the
    // duration of the call); `&mut info` is a valid writable out-param. The call is
    // a read-only metadata query that retains no handle and registers no OS state
    // (T-8-03-FFI). Errors surface as a clean `anyhow` context, never a panic.
    unsafe { GetFileInformationByHandle(handle, &mut info) }
        .with_context(|| format!("GetFileInformationByHandle failed for {}", path.display()))?;
    let file_index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
    Ok((info.dwVolumeSerialNumber, file_index))
}

/// Non-Windows Unix fallback: `(st_dev, st_ino)` from `MetadataExt` is the
/// equivalent stable identity, so hardlink collapse works on Unix hosts too (keeps
/// `cargo test` meaningful off Windows). The project targets Windows.
#[cfg(all(not(windows), unix))]
fn file_identity(path: &Path) -> anyhow::Result<(u32, u64)> {
    use std::os::unix::fs::MetadataExt;
    let m =
        std::fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;
    Ok((m.dev() as u32, m.ino()))
}

/// Other-host fallback (neither Windows nor Unix): no stable file-index API in std,
/// so hash the path — each path becomes its own identity and hardlink collapse is a
/// no-op. Only keeps `cargo check` green on exotic hosts; the project targets
/// Windows.
#[cfg(all(not(windows), not(unix)))]
fn file_identity(path: &Path) -> anyhow::Result<(u32, u64)> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    Ok((0, h.finish()))
}

/// Print each duplicate group (one file per line, a blank line between groups)
/// followed by the wasted-space summary. When there are no duplicates, print a
/// single "No duplicate files found." line. Only the size accent is colored, gated
/// on `is_color_on()` so piped output is byte-identical minus ANSI (D-10).
fn render(groups: &[DupeGroup]) {
    if groups.is_empty() {
        println!("No duplicate files found.");
        return;
    }
    for group in groups {
        let header = format!(
            "{} each, {} copies",
            human_size(group.size),
            group.paths.len()
        );
        println!("{}", accent(&header));
        for path in &group.paths {
            println!("  {}", path.display());
        }
        println!();
    }
    let wasted = wasted_space(groups);
    println!(
        "{} wasted in {} duplicate group(s).",
        accent(&human_size(wasted)),
        groups.len()
    );
}

/// Color a token `.yellow()` when color is on, else return it plain — the single
/// styled accent in dupes, gated so piped output is byte-identical minus ANSI
/// (D-10).
fn accent(s: &str) -> String {
    if is_color_on() {
        s.yellow().to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(size: u64, n: usize) -> DupeGroup {
        DupeGroup {
            size,
            paths: (0..n).map(|i| PathBuf::from(format!("f{i}"))).collect(),
        }
    }

    #[test]
    fn wasted_space_sums_redundant_copies() {
        // (3-1)*1024 + (2-1)*2048 = 2048 + 2048 = 4096.
        let groups = vec![g(1024, 3), g(2048, 2)];
        assert_eq!(wasted_space(&groups), 4096);
    }

    #[test]
    fn wasted_space_zero_when_no_groups() {
        assert_eq!(wasted_space(&[]), 0);
    }

    #[test]
    fn group_duplicates_keeps_only_runs_of_two_or_more() {
        // Sorted by (hash, path): "aaa" x3, "bbb" x1 (unique), "ccc" x2.
        let input = vec![
            ("aaa".to_string(), 10, PathBuf::from("a1")),
            ("aaa".to_string(), 10, PathBuf::from("a2")),
            ("aaa".to_string(), 10, PathBuf::from("a3")),
            ("bbb".to_string(), 20, PathBuf::from("b1")),
            ("ccc".to_string(), 30, PathBuf::from("c1")),
            ("ccc".to_string(), 30, PathBuf::from("c2")),
        ];
        let groups = group_duplicates(input);
        // The "bbb" singleton is dropped; two groups survive.
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].paths.len(), 3);
        assert_eq!(groups[0].size, 10);
        assert_eq!(groups[1].paths.len(), 2);
        assert_eq!(groups[1].size, 30);
        // Wasted = (3-1)*10 + (2-1)*30 = 20 + 30 = 50.
        assert_eq!(wasted_space(&groups), 50);
    }

    #[test]
    fn group_duplicates_empty_input() {
        assert!(group_duplicates(Vec::new()).is_empty());
    }

    #[test]
    fn hash_reader_blake3_matches_known_vector() {
        // b"box" -> the same BLAKE3 known-answer the hash slice locked (Plan 03-01).
        const BOX_BLAKE3: &str = "095dfefdedb7f0870e801730da35823caaa8e969078e53b6e262c66f1a5b1c1e";
        assert_eq!(hash_reader_blake3(&b"box"[..]).unwrap(), BOX_BLAKE3);
    }

    #[test]
    fn hash_reader_blake3_distinguishes_same_size_different_content() {
        // Same length, different content -> different hashes (the property the
        // size-pre-filter-then-content-hash identity relies on, D-13).
        let a = hash_reader_blake3(&b"AAAA"[..]).unwrap();
        let b = hash_reader_blake3(&b"BBBB"[..]).unwrap();
        assert_ne!(a, b, "distinct content must hash differently");
    }
}
