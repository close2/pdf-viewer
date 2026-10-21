# 1655 — A standard font stated by name alone is embedded as the viewer draws it, and a face of the other kind restates its dictionary

Slot 1 of batch one hundred and two, 2026-10-21, an archive round. ADRs 2146, 2147; Q550.

**Built.** (1) ADR 2146: one of the fourteen with no descriptor gets the face the viewer draws it
from, unmodified. Beside it go a descriptor read off the program (Table 120; `/StemV` from `StdVW`
or 0, ascent, descent and cap height measured), and the widths it was laid out by, which Table
109's all-or-none rule obliges. Liberation becomes a `TrueType` dictionary, its encoding restated on
`WinAnsiEncoding` where 6.2.11.6 asks. Dingbats and Symbol get the Annex D names as `/Differences`.
`described::proved` reads the font back and compares outline up to winding, advance and stretch.
(2) ADR 2147: a `TrueType` dictionary given a CFF face becomes `Type1`, and the reverse. Premise
wrong: `/StemV` was never the obstacle (Table 120's 0); Helvetica cannot go under `Type1` at all.
Hypothesis held: 132 pdf.js documents move at 2b.

**Real corpora, `as-if-printed`** (converted / conforming / refused / unreadable; declined 9 and 12
before and after):

| corpus | 2b before | 2b after | 4 before | 4 after |
|---|---|---|---|---|
| pdf.js (976) | 618 / 2 / 336 / 11 | 750 / 2 / 204 / 11 | 570 / 0 / 386 / 11 | 698 / 0 / 258 / 11 |
| `doc/corpora` (503) | 179 / 6 / 290 / 16 | 260 / 6 / 209 / 16 | 157 / 0 / 318 / 16 | 235 / 0 / 240 / 16 |

`as-shown` 2b: 617 → 746, 176 → 259. Font rows at 2b (pdf.js, corpora): no descriptor 192, 168 →
0, 0; Table 124's two sentences 9, 65 → 0, 0. New, by name: font in its resource dictionary 8, 71;
machine face drawn (ADR 0270) 5, 36; drawn differently 0, 9; not one of the fourteen 3, 1.
**Witnesses** (100 dpi, `scratchpad/r1655/w/`): `IdentityToUnicodeMap_charCodeOf.pdf` (Times) byte-identical, `AcroFormForMerge.pdf` 0 px; at 1
level of 255 from Liberation's winding `4PP-Highlighting.pdf` 1, `NegativeFontSize.pdf` 22,
`125619.pdf` 121, `195981.pdf` 5 px.

**Gates.** `cargo fmt --check` and `-D warnings` clippy `--all-targets` on pdf-transform: exit 0.
Nextest pdf-transform: 613 passed, 0 failed. `tools/state.sh --round 1655 archive`: exit 0,
`--remedy-sites` 2b 23 of 68 and 4 17 of 56, validator missed 0 and over 0, `archive_corpus` 2b
525/86 and 4 266/93, `cross_check` 0 disagreements. veraPDF 1.30.3 at 2b over 25 seeded converted
outputs: 22 pass. The 3 failures are producers' embedded fonts and an attachment, none a written
font: `issue6410` 6.2.11.2-6, `wp315` 6.2.11.5, `preEnc_20141025_105451` 6.8.
`t3-foreign_corpus` `--clock --tree 12`: exit 0, 127 s. `cargo test -p conformance`: ok.
**Found.** veraPDF's two font failures are rules this validator passes; the older route restates
Liberation under its reserved name (Q550); the census depends on this machine's fonts (ADR 0270);
`--bin quorra-transform` leaves a stale worker running the old archive code. **Unfinished**: the
machine-face refusal (`doc/todo/72`). 5 000 s.
