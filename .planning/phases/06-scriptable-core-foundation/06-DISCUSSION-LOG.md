# Phase 6: Scriptable-Core Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-25
**Phase:** 6-Scriptable-Core Foundation
**Areas discussed:** JSON shape + field names, hash --verify 64-hex tie, --clip multi-value rule, error behavior under --json
**Mode:** advisor (full_maturity calibration — 4 parallel `gsd-advisor-researcher` agents, opus)

---

## ① JSON document shape + field names

| Option | Description | Selected |
|--------|-------------|----------|
| Always-object root | scalar=flat obj; multi-capable=`{results:[…],count:N}` (N=1 incl.); tree=recursive `{children}`. Extensible, never collapses, hash stable into Phase 8. Fields uuid/version, path/algo/digest. | ✓ |
| Bare array (gh-style) | top-level array always; single=1-element array. PS7 auto-enumerates but root can't carry metadata + single-element collapse footgun. | |
| Hybrid by arity | scalar=object, multi=bare array, tree=object. Each shape idiomatic but "depends on the command" is the ambiguity that caused the contradiction. | |

**User's choice:** Always-object root.
**Notes:** Resolves the SUMMARY.md (bare-array) vs ARCHITECTURE.md (object-wrapper) contradiction in favor of the extensible object root. Decisive grounding: PS7 `ConvertFrom-Json` auto-enumerates bare arrays (collapses single-element), so a bare-array root can never grow a sibling key — wrong for a schema frozen across 23 commands. `box hash file` ships as a 1-element `results` array so Phase-8 multi-file is shape-identical. Field-name verdicts: `algo` (matches `--algo` flag) over `algorithm`; `digest` over `hash`; `path` over `file` (generalizes to all fs commands).

---

## ② hash --verify with a 64-hex digest (SHA-256 vs BLAKE3 tie)

| Option | Description | Selected |
|--------|-------------|----------|
| SHA-256 + blake3 probe | bare `--verify <64hex>` stays SHA-256 (no baseline breaks); on mismatch, also test BLAKE3 and print a decisive hint if that matches. Amends HASH-V2-01 + SUMMARY (they say BLAKE3). | ✓ |
| SHA-256 + static hint | same SHA-256 resolution, one-line static hint, no second hash computed. | |
| Flip verify to BLAKE3 | 64-hex verifies as BLAKE3 (consistent w/ new compute default) but silently breaks every stored-SHA-256 verify script — the #1 v2 risk. | |
| Require explicit --algo | bare 64-hex `--verify` → exit 2; removes ambiguity but hard-breaks all legacy verify invocations. | |

**User's choice:** SHA-256 + blake3 probe.
**Notes:** Resolves the REQUIREMENTS/SUMMARY (verify→BLAKE3) vs ARCHITECTURE/STATE (verify stays SHA-256) contradiction in favor of staying SHA-256. Compute default still flips to BLAKE3 — compute and verify are decoupled. Grounding: every checksum tool (b3sum, sha256sum, coreutils, openssl) pins algorithm by tool/flag/tag, never guesses by length; changing the verify default is backward-destructive to stored baselines. Existing `algo_from_len` (hash/mod.rs:78-85) unchanged; only the compute `unwrap_or` (line 162) flips; the probe attaches to the mismatch `bail!` (lines 154-156). **Triggers a required doc amendment (D-06):** HASH-V2-01 + SUMMARY Pitfall 6 must drop the "verify maps to BLAKE3" wording.

---

## ③ --clip with multi-value output (e.g. box uuid -n 5 --clip)

| Option | Description | Selected |
|--------|-------------|----------|
| Copy all, newline-joined | clipboard = everything printed (uuid -n 5 → all 5), single trim_end. IS the locked tee; zero special-casing; matches clip.exe/Set-Clipboard/pbcopy; `--json --clip` = whole doc free. | ✓ |
| Copy first/primary only | clipboard = first line; needs a per-command "primary" definition (drift risk), contradicts the tee primitive, surprising for -n 5. | |
| Add --clip-first flag | copy-all default + opt-in flag; extra frozen surface with no demand; defer to v3. | |

**User's choice:** Copy all, newline-joined.
**Notes:** "The primary result" (SPINE-03) resolves uniformly to "the command's full stdout payload" — mechanical, no per-command branching, the only rule a frozen template can carry. `-n 5` is an explicit request for 5; copying 1 would violate least-surprise. Locked siblings unchanged: copy-and-print, stderr confirmation suppressed when not a TTY, `--clip` forces no-ANSI, flush once on the main thread after successful dispatch.

---

## ④ Error behavior under --json

| Option | Description | Selected |
|--------|-------------|----------|
| stderr + exit code | failure → stdout EMPTY, `error:…` to stderr, exit 1 (runtime)/2 (usage). Strongest SPINE-01 purity, zero new surface, matches gh/kubectl/cargo/jq/ripgrep. Malformed config = exit 2. | ✓ |
| JSON envelope on stderr | stdout empty; `{"error":…}` JSON on stderr for machine-parsing; keeps stdout pure but adds a --json-conditional stderr shape to freeze across 21 cmds. | |
| JSON envelope on stdout | `{"error":…}` on stdout, one capture point; violates SPINE-01/FOUND-03 — data channel carries non-data (aws-cli style). | |

**User's choice:** stderr + exit code (no envelope).
**Notes:** Preserves the v1 0/1/2 contract and the stdout-purity invariant that makes `--json | jq`/`ConvertFrom-Json` reliable. Linked sub-decision — **malformed config = exit 2** (via `BoxError::Config`, same family as clap usage errors; matches BSD sysexits `EX_CONFIG` + aws-cli's distinct config code; reuses the existing main() downcast). Architecture had called 1-vs-2 "a minor call"; research made exit 2 definitive. Missing config → silent defaults stays locked.

## Claude's Discretion

- Env-var tier spelling (SPINE-05 mandates the tier; no scheme locked) — suggested `BOX_<SECTION>_<KEY>` uppercase.
- Phase-6 `Config` struct field scope — lean to only `default_hash_algo` now; grow per-command.
- Exact wording of D-05 mismatch hints and the "Copied to clipboard" confirmation.

## Deferred Ideas

None — discussion stayed within Phase 6 scope. (A future opt-in `hash.strict_verify` config key — making the 64-hex tie an error — was noted by research as out of scope for v2.)
