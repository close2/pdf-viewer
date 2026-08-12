# 458 — A border that shall be drawn inside, and an entry that shall be ignored

**Finding.** §12.5.4 is one of the three `partial` rows `git blame` still put above the fold after
the four-hundred-and-forty-second read thirty-two off the same list, and reading it against
`appearance.rs`'s `Border` found two silent departures from two sentences. §12.5.4 says "[i]f
present, the border shall be drawn completely inside the annotation rectangle", which
`Border::inset` executes for the four rectangular styles in Table 168 and did **not** for `U` — the
underline's path sat *on* the bottom edge, so half a `/W`-wide line fell outside `/Rect`, where
`Constructed::bounded`'s clip cut it away. The reader therefore saw an underline half the width the
document asked for, which is not a report, a refusal or a distance from a reference. And Table
166's "[i]f an annotation dictionary includes the BS entry, then the Border entry is ignored" was
obeyed for the width, the style and the dash but not for `/Border`'s **corner radii** — the one
thing the array states that Table 168 has no entry for, which is what made reading them beside a
`/BS` look like completeness rather than a square border drawn round.

**Date.** 2026-08-13.
**ADR.** [0293](../adr/0293-a-border-that-shall-be-inside-and-an-entry-that-shall-be-ignored.md).
**Touched.** `crates/pdf-model/src/appearance.rs` (`Border::read`, `Border::outline`),
`crates/pdf-model/tests/annotations.rs` (two tests),
`crates/pdf-model/examples/border_precedence_census.rs` (new),
`doc/conformance/ledger.toml` (§12.5.4, §12.5.2), `doc/todo/01-ledger-partial-rows.md`,
`doc/adr/0293-*`, this file.

## What the sweeps found

Run over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and `doc/adr/`.

- **Expired blockers (1)**: 5 over the ledger. Three are the quoted retired wording inside a
  correction (§11.3.7.2, §11.6.4.3, §11.7.4.4, all saying "this row used to say"); §12.10.2's wait
  on §12.10.3 and §12.5.6.22's on printing are real.
- **Entries claimed unread (2)**: 11, and every one is the known one-short-key-three-clauses
  population or a correction quoting the sentence it retired — §7.6.6's `/EFF` reads "read by
  nothing **until this session**", written by the four-hundred-and-forty-second.
- **Capability reasons (3)**: 16 over the ledger and 136 over the source roots, every one a true
  statement about a boundary a crate keeps. `render-quorra`'s "quorra strokes but does not dash"
  was re-read against the release round 456 took and it is still a *decision* rather than a gap —
  `RENDER_LIBRARY.md` §4.5 settles dashing on this side so both backends share `kurbo::dash`.
- **Retired claim (4)**, over this round's own nouns — `corner radius`, `rounded`, `Underline`,
  `/Border`: clean once the two fixes landed. `annotation.rs`'s "[t]he border's width, from `/BS`
  `/W` or Table 166's `/Border`" is the precedence stated correctly one crate over.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, both of which `doc/todo/01` records as read and kept.
- **Citations (8)**: the known false positives only — §8.9.6.1's `doc/todo/20`, §12.7's and
  `viewer-gtk/src/controls.rs`'s `doc/todo/37`, each a correction quoting the pointer it retired.
- **Parent's stated count (10)**: six raw hits, all of them prose counting *sibling* subclauses
  (§11.7.4.1, §11.7.5.1, §O.1) or the family's descendants minus its own `General` row (§14.8.2,
  §14.12, §F), which is the convention the four-hundred-and-thirty-seventh recorded.
- **The fourteenth**, a `partial` row whose note names nothing owed: 42 hits, all of them the
  sweep's own vocabulary problem — notes owing something in words it does not hold.
- **The blame list** is what produced the round. Of 607 commits, three `partial` rows still have
  notes last written before commit 110 and are not among the seventeen the four-hundred-and-forty-
  second read and kept: §12.5.4, §12.5.6 and §12.5.6.8. This round took the first.

## What the round verified rather than assumed

- **Both tests fail without their fix**, checked by restoring each departure separately:
  `an_underline_border_is_drawn_inside_the_rectangle_rather_than_across_its_edge` comes back
  `(20, 20, 79, 21)` against `(20, 20, 79, 23)` — the clipped half — and
  `a_border_style_dictionary_ignores_the_border_arrays_corner_radii` fails on the square corner.
- **The condition was counted before it was believed.** `border_precedence_census` over the 964
  openable documents: 33 781 annotations state no `/AP`, of which **one** states a `U` border
  (`annotation-border-styles.pdf` object 29, a `/Subtype /Link`), one a `D`, none a `B` or an `I`;
  192 state both `/Border` and `/BS` and 6 of those a non-zero radius, all six on `/Subtype /Ink`
  annotations, whose mark is `/InkList` and which never reach `outline`. So the second departure
  has no corpus witness at all and its fixture is a pair differing only in the `/BS`.
- **`doc/todo/00` step 7's ink sweep**, over all 786 ambiguous pages: twenty at or past −1 of 255,
  sixteen of them documents this tree calls incomplete, and the four complete ones the four
  diagnosed names — `issue16038.pdf` −5.734, `issue12295.pdf` −2.823, `issue14297.pdf` −1.145,
  `issue7821.pdf` −1.000. **It cannot see this round** and says so: the one page whose pixels move
  is not in the ambiguous bucket.

## Gates

`fmt`, `clippy --workspace` (silent of lints; the `viewer-qt@0.1.0:` lines are gcc's on a cold
build), `nextest --workspace` (1636 passed, 11 skipped), doctests, `pdf-sandbox`'s gates binaries,
the pdf-model corpus gate, the oracle, both text gates, dates, xmp, jpeg2000, the quorra corpus
gate, and `conformance`. Every ratchet held.

## What the next round should know

- **The blame list has two rows left above commit 110**: §12.5.6 and §12.5.6.8, both written in the
  same sitting as §12.5.4 and both about the same family. A wrong sentence arrives as a block, which
  is the ninth sweep's signature and is worth assuming here.
- **Ask what a construction's clip absorbs.** `Constructed::bounded` exists so that a link's border
  cannot escape its rectangle, and it turned "half the mark is in the wrong place" into "the mark is
  thinner" — a symptom no instrument in this tree ranks. The four subtypes that opt *out* of the
  clip (§12.5.6.7's `/L`, §12.5.6.9's `/Vertices`, §12.5.6.10's `/QuadPoints`, §12.5.6.13's
  `/InkList`) are the ones where the same mistake would have been visible.
