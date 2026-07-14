//! Best-effort on-disk response cache for `box` (D-11), the foundation the weather
//! depth work (10-05) reads to serve repeated calls without re-hitting the network.
//!
//! This module is the strictly-best-effort sibling of [`crate::core::config`]: it
//! reuses config's missing-file tolerance and `%…%`-env-var-FIRST path resolution,
//! but — unlike config, whose malformed file is a loud `exit 2` — the cache NEVER
//! errors the caller. Every read failure (absent, stale, truncated, garbage,
//! permission-denied) degrades to a **MISS** ([`get`] returns `None`, the caller
//! fetches fresh); every write failure is a silent no-op ([`put`] swallows all
//! errors). A cache problem must never turn a working command into a failing one.
//!
//! ## On-disk shape
//! - **Location:** `%LOCALAPPDATA%\box\cache\` ([`cache_dir`] — `LOCALAPPDATA` env
//!   var first, [`dirs::cache_dir`] fallback; the env-var-first order is load-bearing
//!   for per-process test isolation, exactly as [`crate::core::config`] documents).
//! - **Filename:** `blake3-hex(key).json` ([`entry_path`]). The logical key (which,
//!   for weather, derives from a user-controlled location string) is HASHED into a
//!   fixed 64-char hex name — the raw key is NEVER interpolated into the path, so a
//!   hostile key containing `..\`/`/` cannot escape the cache dir (T-10-04-TRAVERSAL).
//! - **Body:** a JSON [`Envelope`] `{ fetched_at: u64 (unix secs), payload: String }`
//!   via `serde_json` + `std::fs` (both already deps — no new crate).
//!
//! ## TTL
//! An entry older than [`TTL_SECS`] (~10 min) is a MISS. Staleness is decided by the
//! pure [`is_fresh`] helper so the boundary is unit-testable without touching the clock.

/// The freshness window: an entry whose age (`now - fetched_at`) is `>= TTL_SECS`
/// is stale and treated as a MISS (~10 minutes, D-11).
const TTL_SECS: u64 = 600;

/// The on-disk cache entry: when it was written (unix seconds) plus the cached
/// payload. Serialized as compact JSON; a body that fails to parse into this shape
/// is a MISS (never a trust of unvalidated JSON — T-10-04-CACHE-PARSE).
#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope {
    /// Unix timestamp (seconds) at which the payload was cached.
    fetched_at: u64,
    /// The cached response text (opaque to the cache — weather stores its doc here).
    payload: String,
}

/// Return the fresh (`< TTL_SECS`) cached payload for `key`, else `None`.
///
/// Every failure mode — absent file, unreadable file, malformed JSON, or an entry
/// older than the TTL — returns `None` (a MISS). NEVER returns an `Err`, NEVER
/// panics: a broken cache degrades to a fresh fetch, not a command failure.
pub fn get(_key: &str) -> Option<String> {
    todo!("10-04 Task 2 GREEN: tolerant read + TTL check")
}

/// Best-effort write of `payload` under `key`. Creates the cache dir and writes the
/// JSON envelope; swallows ALL errors (a read-only or missing cache dir is a silent
/// no-op). NEVER panics, NEVER propagates — a failed write must not fail the caller.
pub fn put(_key: &str, _payload: &str) {
    todo!("10-04 Task 2 GREEN: best-effort envelope write")
}

/// Pure staleness predicate: `true` iff the entry's age (`now - fetched_at`) is
/// STRICTLY LESS than `ttl`. An age of exactly `ttl` is stale. A `fetched_at` in the
/// future (clock skew) saturates to age 0 → fresh. Pure so the boundary is testable
/// without the clock.
fn is_fresh(_fetched_at: u64, _now: u64, _ttl: u64) -> bool {
    todo!("10-04 Task 2 GREEN: now.saturating_sub(fetched_at) < ttl")
}

/// Current unix time in whole seconds; `0` if the system clock predates the epoch
/// (the cache tolerates it — a bogus clock just makes entries look stale).
fn now_unix() -> u64 {
    todo!("10-04 Task 2 GREEN: SystemTime since UNIX_EPOCH")
}

/// The cache directory: `%LOCALAPPDATA%\box\cache\` when `LOCALAPPDATA` is set (the
/// env-var-FIRST branch — load-bearing for per-process test isolation, mirroring
/// [`crate::core::config`]'s `config_path`), else `dirs::cache_dir()/box/cache`.
fn cache_dir() -> Option<std::path::PathBuf> {
    todo!("10-04 Task 2 GREEN: LOCALAPPDATA-first cache dir")
}

/// The on-disk path for `key`: `cache_dir()/{blake3-hex(key)}.json`. The key is
/// HASHED into the filename (the T-10-04-TRAVERSAL mitigation) — the raw key is
/// never part of the path, so it cannot contain separators that escape the dir.
fn entry_path(_key: &str) -> Option<std::path::PathBuf> {
    todo!("10-04 Task 2 GREEN: cache_dir + blake3-hex(key).json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The cache dir is resolved from the process-global `LOCALAPPDATA` env var, so
    /// the env-mutating tests must not run concurrently (each points the var at its
    /// own TempDir). This lock serializes them; poison is tolerated (a panicking
    /// test must not wedge the rest).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Lock the env, point `LOCALAPPDATA` at a fresh TempDir, and return both guards
    /// (the caller must keep them alive for the test body — dropping the TempDir
    /// deletes the cache dir).
    fn isolate() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("LOCALAPPDATA", tmp.path());
        (lock, tmp)
    }

    /// put → get within the TTL returns the exact payload; an unwritten key is a MISS.
    #[test]
    fn put_then_get_round_trips_within_ttl() {
        let (_lock, _tmp) = isolate();

        put("London|metric|current", "the-forecast-json");
        assert_eq!(
            get("London|metric|current").as_deref(),
            Some("the-forecast-json"),
            "a fresh entry must round-trip its payload"
        );
        assert_eq!(
            get("Paris|metric|current"),
            None,
            "an unwritten key is a MISS"
        );
    }

    /// An entry stamped older than the TTL is a MISS (stale = fetch fresh).
    #[test]
    fn stale_entry_is_a_miss() {
        let (_lock, _tmp) = isolate();

        let key = "stale-key";
        let path = entry_path(key).expect("entry path resolves");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = now_unix().saturating_sub(TTL_SECS + 60);
        let env = Envelope { fetched_at: old, payload: "old".into() };
        std::fs::write(&path, serde_json::to_string(&env).unwrap()).unwrap();

        assert_eq!(get(key), None, "an entry older than TTL must be a MISS");
    }

    /// A malformed/garbage cache file is a MISS — never a panic, never an error.
    #[test]
    fn malformed_entry_is_a_miss() {
        let (_lock, _tmp) = isolate();

        let key = "garbage-key";
        let path = entry_path(key).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"}{ not valid json at all").unwrap();

        assert_eq!(
            get(key),
            None,
            "a malformed cache file must be a MISS, never a panic"
        );
    }

    /// No cache file at all is a MISS.
    #[test]
    fn absent_entry_is_a_miss() {
        let (_lock, _tmp) = isolate();
        assert_eq!(get("never-written"), None);
    }

    /// The pure TTL boundary: age `< ttl` is fresh, age `== ttl` (and beyond) is
    /// stale, and a future `fetched_at` (clock skew) saturates to fresh.
    #[test]
    fn is_fresh_boundary() {
        assert!(is_fresh(1000, 1000, 600), "age 0 is fresh");
        assert!(is_fresh(1000, 1599, 600), "age just under ttl is fresh");
        assert!(!is_fresh(1000, 1600, 600), "age == ttl is stale");
        assert!(!is_fresh(1000, 5000, 600), "age past ttl is stale");
        assert!(is_fresh(2000, 1000, 600), "future fetched_at saturates to fresh");
    }

    /// T-10-04-TRAVERSAL — a key containing path separators / `..\` CANNOT escape
    /// the cache dir: the entry stays a direct child of the cache root and its
    /// filename is fixed-charset blake3 hex + `.json`, and put/get still round-trip.
    #[test]
    fn hostile_key_cannot_escape_cache_dir() {
        let (_lock, _tmp) = isolate();

        let cache_root = cache_dir().expect("cache dir resolves");
        let hostile = r"..\..\..\Windows\System32\evil";
        let path = entry_path(hostile).expect("entry path resolves");

        assert_eq!(
            path.parent(),
            Some(cache_root.as_path()),
            "the entry must stay a direct child of the cache dir"
        );
        let fname = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(fname.ends_with(".json"), "filename must be .json: {fname}");
        let stem = fname.trim_end_matches(".json");
        assert_eq!(stem.len(), 64, "blake3 hex is 64 chars: {stem}");
        assert!(
            stem.chars().all(|c| c.is_ascii_hexdigit()),
            "filename stem must be pure hex (no raw-key separators): {stem}"
        );

        // And a hostile key still round-trips WITHOUT writing outside the dir.
        put(hostile, "safe");
        assert_eq!(get(hostile).as_deref(), Some("safe"));
    }
}
