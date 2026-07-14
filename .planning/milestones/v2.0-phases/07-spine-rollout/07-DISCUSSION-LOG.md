# Phase 7: Spine Rollout - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-25
**Phase:** 7-Spine Rollout
**Areas discussed:** Filesystem doc key conflict, Action commands under --json, qr & json commands under the spine, Field schemas

---

## Filesystem multi-row document key (du/dupes/flatten/bulk-rename)

Resolves a direct conflict: roadmap Phase-7 SC1 says `box du --json` yields an `entries` array; Phase-6 D-01 froze multi-row commands to `{results,count}`.

| Option | Description | Selected |
|--------|-------------|----------|
| Frozen results/count | `{results:[…],count:N}` on all four filesystem commands, identical to uuid/hash pilots; amend roadmap SC1's `entries` wording | ✓ |
| Domain keys | du→entries, dupes→groups, flatten/bulk-rename→operations (each + count) | |

**User's choice:** Frozen results/count (recommended).
**Notes:** D-01 is the higher authority — the frozen template the whole milestone is built around; SC1 wording predates it. Uniform `.results` for scripting wins. → D-11 + D-11a doc-amendment action item.

---

## flatten / bulk-rename under --json

| Option | Description | Selected |
|--------|-------------|----------|
| Format-only + dry_run flag | `--json` never changes behavior; follows existing `--force` gate; root carries `dry_run` boolean; per-op `{src,dst,action,reason}` + summary counts; bulk-rename --force --json emits applied renames | ✓ |
| Format-only, no dry_run flag | Same orthogonal behavior but omit the dry_run boolean | |
| --json forces dry-run | Refuse to mutate under --json (safety) | |

**User's choice:** Format-only + dry_run flag (recommended).
**Notes:** `--json` is an output format, orthogonal to mutation. → D-12 + D-13.

---

## qr & json commands under the spine

qr is a renderer; `json` is itself json-formatting — both are odd fits. qr `--json` = `{text,error_correction}` metadata and `json --json` = passthrough were taken as recommended without a separate vote; the live question was qr's `--clip` payload.

| Option | Description | Selected |
|--------|-------------|----------|
| Source text | qr --clip copies the input string the QR encodes (SPINE-04 "text payload") | ✓ |
| Rendered glyph block | qr --clip copies the printed ▀▄ half-block QR (D-07 copy-all) | |
| Drop qr from --clip | Treat qr as display-only for clipboard | |

**User's choice:** Source text (recommended).
**Notes:** Pasting glyphs as clipboard text is garbage in most apps; the encoded text is the useful payload — the one deliberate exception to D-07 copy-all. → D-14 (qr --json), D-15 (qr --clip), D-16 (json passthrough).

---

## Field schemas for structured value commands (color/epoch/tree/weather)

| Option | Description | Selected |
|--------|-------------|----------|
| Nested + unified | color `{hex,rgb:{r,g,b},hsl:{h,s,l}}`; epoch unified `{epoch,utc,local}` for all modes; tree `{name,type,size?,children}`; weather current-only | ✓ |
| Flat fields | color `{hex,r,g,b,h,s,l}` flat; epoch mode-dependent | |
| Defer to planner | Lock only root rules; planner picks per-command fields | |

**User's choice:** Nested + unified (recommended).
**Notes:** Sub-objects script cleanly via `.rgb.r` in PS7; one stable epoch shape beats branching on input direction. → D-17.

---

## Claude's Discretion

- Exact field names beyond those pinned (base64, passgen rows, fun-trio single-field objects, weather sub-fields).
- `action` enum string spelling for action-command rows (match lowercased `RowStatus`).
- Wrap behavior for soft-wrapped fun commands under `--json` (emit unwrapped string).
- Scalar-vs-multi classification per D-01 (passgen --count is multi-capable).

## Deferred Ideas

None — discussion stayed within Phase 7 scope. All per-command depth ideas are already roadmapped to Phases 8–10.
