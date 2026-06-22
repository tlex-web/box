# Third-Party Licenses & Attributions

This file records bundled third-party data and the attribution its license requires.

## EFF Long Wordlist (passphrase word source for `box passgen --words`)

- **File:** `src/data/eff_large_wordlist.txt` (embedded at compile time via `include_str!`)
- **Source:** EFF Large (Diceware) Wordlist — <https://www.eff.org/dice>
- **Author / Copyright:** © Electronic Frontier Foundation
- **License:** Creative Commons Attribution 3.0 United States (CC-BY 3.0 US) — <https://creativecommons.org/licenses/by/3.0/us/>

**Required attribution:**

> Passphrase wordlist: EFF Long Wordlist, © Electronic Frontier Foundation, CC-BY 3.0 US.

The canonical EFF file ships as `<5-digit-dice-code>\t<word>` lines; this project stores
the 7776 words only (dice codes stripped). No words were added, removed, or altered, so the
list remains the authentic EFF Large wordlist.
