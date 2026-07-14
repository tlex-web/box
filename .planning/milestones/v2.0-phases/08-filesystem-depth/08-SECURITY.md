---
phase: 8
slug: filesystem-depth
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-28
---

# Phase 8 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> **Audit type:** mitigation verification against the PLAN STRIDE registers (08-01..08-06).
> **Verdict:** SECURED — 34/34 threats CLOSED (31 mitigated-in-code + 3 documented accepted risks), 0 open.
> **block_on:** high. Implementation files were read-only during the audit.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI args → filesystem walk | `--extensions`/`--separator` values and multiple path args cross into the walk + collision-encoding | Untrusted user strings |
| CLI globs → matcher | `--ignore`/`--exclude` globs compiled and matched against walked paths | Untrusted glob patterns |
| File contents → hasher | Untrusted file bytes streamed into BLAKE3/RustCrypto (read-only, not a security boundary — equality only) | Untrusted file bytes |
| Filesystem names → renderer | Untrusted file/dir names (incl. non-UTF-8 NTFS) rendered to stdout/JSON | Untrusted OS strings |
| Win32 FFI → du | `GetCompressedFileSizeW` over a user-supplied tree (read-only query) | Wide path strings |
| Win32 FFI → dupes/delete | `GetFileInformationByHandle` over user files for `(volume_serial, file_index)` identity (read-only handle query) | File handles |
| CLI src/out + `--force` → mutation | `flatten --move`: a destructive relocation (copy → verify → delete) | File data, deletions |
| CLI dir + `--force` → deletion | `dupes --delete`: a destructive dedup that removes files | Deletions |
| content-equality → file identity | Equal content is NOT the same inode; only the file-index proves a shared inode | Identity decisions |
| CLI dir/pattern + `--force --backup` → rename | `bulk-rename`: renames whose recoverability depends on a durable manifest | File renames |
| Undo manifest → `%LOCALAPPDATA%` | New persistent OS state written outside the renamed tree | Path names (no secrets) |
| Dependency install → build | Four crates added to the manifest (supply chain) | Third-party code |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation (evidence: file:line / test) | Status |
|-----------|----------|-----------|-------------|------------------------------------------|--------|
| T-8-01 | Tampering | flatten `--separator` | mitigate | `flatten/mod.rs:204-209` rejects `/`/`\` before any I/O; `flatten/rename.rs:43-54` `encode_relative` drops `..`/`.`/empty; test `rename::encode_no_separator` | closed |
| T-8-01-INJ | Tampering | flatten `--extensions` parse | mitigate | `flatten/mod.rs:559-564,621-629` lowercased-set string compare; no glob/regex, no traversal | closed |
| T-8-01-DOS | DoS | non-UTF-8 NTFS path under `--json` | mitigate | `flatten/mod.rs:602,635,255`; `hash/mod.rs:347-349` `to_string_lossy()`; never `to_str().unwrap()` | closed |
| T-8-01-PROG | Info Disclosure | indicatif progress contaminating stdout | mitigate | `flatten/mod.rs:317-321` + `hash/mod.rs:322-324` `ProgressDrawTarget::stderr()` gated `!json`; bar never built under `--json` | closed |
| T-8-SC | Tampering (supply chain) | indicatif/ignore/globset/windows install | mitigate | `Cargo.toml:109,113,114,121` pinned; RESEARCH package audit cleared all four (BurntSushi/console-rs/Microsoft); single `windows` 0.61.3 | closed |
| T-8-02-GLOB | Tampering | du `--exclude` / tree `--ignore` glob scope | mitigate | `du/mod.rs:365-382` clean `anyhow` err + root-relative match; `tree/mod.rs:488-499`; test `malformed_exclude_glob_is_clean_error` | closed |
| T-8-02-GI | Info Disclosure | nested `.gitignore` over/under-match | mitigate | `tree/mod.rs:472-483` deepest-first stack; default output byte-identical; test `gitignore_nested` (3-level) | closed |
| T-8-02-FFI | DoS | unsafe `GetCompressedFileSizeW` | mitigate | `du/mod.rs:390-420` wide NUL path, single localized `unsafe`, `INVALID_FILE_SIZE` disambiguated via `GetLastError`; read-only | closed |
| T-8-02-NAN | Tampering | percentage divide-by-zero | mitigate | `du/mod.rs:326-338` total==0 → `0.0%`; render-only, no `f64` in JSON; test `percent_str_formats_and_guards_nan` | closed |
| T-8-02-ANSI | Info Disclosure | band color / progress contaminating `--json` | mitigate | `du/mod.rs:173` json fork first; `:345` gated on `is_color_on()`; test `json_no_ansi` | closed |
| T-8-02-DOS | DoS | non-UTF-8 NTFS names under `--json` | mitigate | `du/mod.rs:255` + `tree/mod.rs:419,191` `to_string_lossy()` | closed |
| T-8-03-INJ | Tampering | `{n}`/`--case` producing path-sep or `..` | mitigate | `bulk_rename/mod.rs:424` transform BEFORE `:434` preflight; `injects():358-365` refuses `/`,`\`,`..`,`.`,dots/spaces,reserved | closed |
| T-8-03-REPRO | Tampering | non-reproducible `{n}` over walk order | mitigate | `bulk_rename/mod.rs:748-759` sort by `src` BEFORE counter; test `numbering_sorted_reproducible` | closed |
| T-8-03-HL | Spoofing | content-equality mistaken for same-inode | mitigate | `dupes/mod.rs:411-432` `file_identity` `(dwVolumeSerialNumber,fileIndex)`; `:372-398` hardlink-aware; test `hardlink_not_wasted` | closed |
| T-8-03-NIGHTLY | Tampering | nightly-only std `file_index` on stable | mitigate | `dupes/mod.rs:428-431` Win32 only; grep: no `MetadataExt::file_index`/`windows_by_handle` in executable code | closed |
| T-8-03-FFI | DoS | unsafe `GetFileInformationByHandle` | mitigate | `dupes/mod.rs:418-429` single localized wrapped `unsafe`; handle from opened `File`; read-only | closed |
| T-8-03-DOS | DoS | non-UTF-8 NTFS names under `--json` | mitigate | `dupes/mod.rs:242` `to_string_lossy()` | closed |
| T-8-04 | Tampering / data loss | delete-before-confirm in `--move` | mitigate | `flatten/mod.rs:453-554` two-phase: copy(create-new)+verify ALL, then delete ALL; test `move_abort_midbatch_copy_error_snapshot_unchanged` | closed |
| T-8-04-CONTAIN | Tampering | output dir inside source dir under `--move` | mitigate | `flatten/mod.rs:233-242` containment guard (lowercased `starts_with`, NTFS case-fold) bails before `run_move` `:276`; test `move_abort_containment_snapshot_unchanged` | closed |
| T-8-04-TOCTOU | Tampering | TOCTOU between verify and delete | accept | See Accepted Risks Log (AR-1) | closed |
| T-8-04-INJ | Tampering | collision-renamed dest escaping output dir | mitigate | `flatten/mod.rs:643` `encode_relative` + `:496` `safe_copy` create-new; test `rename::encode_no_separator` | closed |
| T-8-04-ANSI | Info Disclosure | `--json` purity under destructive path | mitigate | `flatten/mod.rs:454` json fork first; test `move_json_plan_and_executed` (no `0x1B`, correct `dry_run`) | closed |
| T-8-05 | Tampering / data loss | deleting all copies of a group | mitigate | `dupes/mod.rs:670` keep-first `paths[0]`; `NoSurvivor` preflight `:527,672`; abort-all bail `:597-603`; test `delete_keeps_at_least_one_per_group` | closed |
| T-8-05-HL | Tampering / data loss | deleting a hardlink alias | mitigate | `dupes/mod.rs:694-696` alias of kept identity → spared; test `delete_hardlink_alias_never_deleted` | closed |
| T-8-05-PARTIAL | Tampering / data loss | partial deletion on mid-plan error | mitigate | `dupes/mod.rs:589-603` whole plan + pre-flight before any `remove_file`; loop `:636-642` `?`-propagates first error | closed |
| T-8-05-TOCTOU | Tampering | TOCTOU between identity check and delete | accept | See Accepted Risks Log (AR-2) | closed |
| T-8-05-ANSI | Info Disclosure | `--json` purity / abort-empty-stdout | mitigate | `dupes/mod.rs:597-606` abort prints only `if !is_json_on()` then bails to stderr (empty stdout); test `delete_abort_preflight_snapshot_unchanged` | closed |
| T-8-05-DOS | DoS | non-UTF-8 NTFS names under `--json` | mitigate | `dupes/mod.rs:767-776` `to_string_lossy()` | closed |
| T-8-06 | Tampering / data loss | manifest not durable before renames | mitigate | `bulk_rename/mod.rs:495-525` full `applied:false` manifest + `sync_all()` BEFORE first `fs::rename`; per-rename flip+fsync `:560-563`; test `backup_partition_recoverable` | closed |
| T-8-06-LOC | Tampering / data integrity | manifest written inside renamed tree | mitigate | `bulk_rename/mod.rs:499-502` `%LOCALAPPDATA%\box\undo\<id>.json` (not APPDATA), outside tree; fallback to target dir only if unset | closed |
| T-8-06-ABORT | Tampering / data loss | rename before clean pre-flight | mitigate | `bulk_rename/mod.rs:434,439-454` preflight bail before manifest write `:495` | closed |
| T-8-06-SILENT | Tampering / data loss | `std::fs::rename` silently overwriting on Windows | mitigate | `preflight():259-347` clobber-detection backstop (Rule 2b `:315-325`); `--backup` additive, not a substitute | closed |
| T-8-06-DOS | DoS | non-UTF-8 NTFS names in manifest | mitigate | `bulk_rename/mod.rs:975-976` `old`/`new` via `to_string_lossy()` | closed |
| T-8-06-ACCUM | Info Disclosure | manifests accumulate under `%LOCALAPPDATA%` | accept | See Accepted Risks Log (AR-3) | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-1 | T-8-04-TOCTOU | Single-process, local, single-user CLI. Verify happens immediately before delete (`flatten/mod.rs:501-530`); exposure window identical to the existing copy path. Exploitation needs local write access to the exact file in a sub-second window — capability a local user already has; no privilege boundary crossed. | tlex-web (verified by gsd-security-auditor) | 2026-06-28 |
| AR-2 | T-8-05-TOCTOU | Identity is computed immediately before the plan in one pass (`dupes/mod.rs:532-533,664-734`); deletion runs only after the full pre-flight clears. Same single-process local-user context; no privilege boundary crossed. | tlex-web (verified by gsd-security-auditor) | 2026-06-28 |
| AR-3 | T-8-06-ACCUM | No auto-cleanup this phase (`--undo` replay deferred). Manifests live in the user's private `%LOCALAPPDATA%\box\undo\` and contain only path names (no secrets/credentials/content). Worst case is bounded disk growth in user-private space, recoverable by deleting the folder. Documented in RESEARCH Runtime State Inventory. | tlex-web (verified by gsd-security-auditor) | 2026-06-28 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-28 | 34 | 34 | 0 | gsd-security-auditor (opus) |

---

## Notes on plan-execution deviations affecting the data-loss surface

All three deviations were verified in code and *strengthen* the destructive surface; all three Wave-2 destructive plans recorded their mandatory adversarial code-review gate as approved.

- **08-04 (D-36):** `--move` implemented as two-phase (copy+verify ALL, then delete ALL) rather than the plan's per-item copy→verify→delete loop. A per-item loop would leave items 1..N-1 deleted on a copy failure at item N. Verified `flatten/mod.rs:480-530`.
- **08-05 (D-37):** the pre-flight performs I/O (one `file_identity` read per member during `build_delete_plan`). The abort-all-before-any guarantee is preserved — the whole plan is computed before any `remove_file`. Verified `dupes/mod.rs:589-642`.
- **`write_manifest` (WR-03):** write-temp → fsync → atomic-rename (`bulk_rename/mod.rs:996-1016`), so a failed per-flip rewrite cannot destroy the prior good manifest — strengthens T-8-06 reconcilability.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-28
