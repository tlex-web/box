# Phase 2: Pure Transform Utilities - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-22
**Phase:** 2-pure-transform-utilities
**Areas discussed:** Dependency strategy, Input source convention, Bundled data & randomness, color & cowsay output design (advisor mode — research-backed comparison tables)
**Mode:** Advisor (full_maturity calibration — vendor_philosophy = thorough-evaluator; NON_TECHNICAL_OWNER = false). 4 parallel `gsd-advisor-researcher` agents (opus).

---

## Dependency strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Lean bundle | uuid + base64 + chrono crates; hand-roll passgen (rand 0.9 OsRng), color, cowsay, whimsy | ✓ |
| passwords crate for passgen | Built-in strength scoring; accepts rand 0.8/0.9 dup + extra deps + unused bcrypt/md5 | |
| Also hand-roll uuid | Zero non-essential deps; accept RFC-4122 bit-masking risk | |

**User's choice:** Lean bundle
**Notes:** passgen contested call resolved to hand-roll on `rand` 0.9 (D-02) — both paths are ChaCha12 CSPRNG-grade so security is a wash; lean binary + single rand version + free passphrase mode decided it. ⚠️ Captured constraint: sample with `choose`/`gen_range`, never `% len`.

---

## Input source convention

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-detect, layered | arg → piped-stdin → interactive-TTY-error; `-` honored; `--file` later | ✓ |
| Explicit `-` only | No implicit stdin; breaks the `… \| box base64` pipe success criterion | |
| Implicit, no TTY guard | Simplest; interactive-hang footgun | |

**User's choice:** Auto-detect, layered
**Notes:** Reusable `core::input::read_input` (+ `read_input_bytes`) mirroring the existing `IsTerminal` gate; satisfies both Phase-2 criteria unchanged. PS7 UTF-16 pipe re-encoding caveat captured (D-06) — text as UTF-8 String, base64 as Vec<u8>; `--file` extension point deferred to Phase 3.

---

## Bundled data & randomness

| Option | Description | Selected |
|--------|-------------|----------|
| EFF-large + split RNG | include_str! files (EFF 7776/fortune/roast), const 20 for 8ball; OsRng passgen + ThreadRng whimsy | ✓ |
| EFF short wordlist | Smaller binary, fewer bits/word | |
| Unified OsRng everywhere | One CSPRNG path for all commands | |

**User's choice:** EFF-large + split RNG
**Notes:** EFF Large 7776-word Diceware list (CC-BY 3.0 attribution required); canonical 20 8ball answers; ~50–150 fortunes / ~30–80 roasts, original/PD/CC0. Fresh OS entropy each process → varying output; unbiased `choose`/`random_range`; no manual seed.

---

## color & cowsay output design

| Option | Description | Selected |
|--------|-------------|----------|
| HSL block + fg swatch; classic cowsay | color: Hex/RGB/Tuple/HSL + fg ██ truecolor swatch; cowsay: classic, fixed 40-col + --width | ✓ |
| color: drop HSL | Hex+rgb()+tuple only, thinner swatch | |
| cowsay: wrap to terminal width | Fuller bubbles, but non-reproducible pipe-vs-TTY output | |
| color: add HSL + 0-1 float | Plus normalized float row | |

**User's choice:** HSL block + fg swatch; classic cowsay
**Notes:** Foreground `██` swatch is the only option that degrades correctly under the locked COLOR_ON gating (background-ANSI swatch → blank line when piped + banned parallel color path). cowsay fixed 40-col chosen for reproducible pipe-vs-TTY output. Reuses `is_color_on`/`terminal_width`; new `hsl()` helper.

---

## Follow-up details (cross-cutting, resolved)

| Question | Options | Selected |
|----------|---------|----------|
| epoch date formats | ISO 8601 + common forms / RFC 3339 only / liberal multi-format | ISO 8601 + `YYYY-MM-DD` + `YYYY-MM-DD HH:MM:SS` (local) |
| color input | hex+RGB auto-detect / hex-only MVP / hex + CSS rgb() | hex + RGB auto-detected (bidirectional) |
| passgen charset | all four curated symbols / all four full symbols / alphanumeric | all four, curated paste-safe symbol set |

---

## Claude's Discretion

- `core::input` error wording; bytes/string helper sharing.
- Module layout (`8ball` → `eight_ball`); HSL helper placement.
- Exact curated symbol set + passgen restriction-flag surface.
- cowsay cow art bytes; `--width` clamp behavior.
- fortune/roast exact wording + final counts; data-file directory name.
- `uuid` flag spelling beyond `-n N` / `--upper`.

## Deferred Ideas

- `--file PATH` input flag (PS7 UTF-16 pipe workaround) — Phase 3.
- cowsay alternate cow packs + `--think` — follow-up.
- color `--float` (0-1) row — future flag.
- passgen strength-scoring output — only if asked.
- epoch timezone-by-name (`chrono-tz`) — out of scope.
