# 561 — A standard set of fourteen may not disagree with itself

**Finding.** `doc/todo/21` §6's defect is a property of `data/standard-fonts/` rather than of any
page: ten of the compiled-in standard-14 faces are Foxit's bare CFF and four are Liberation Sans
`sfnt`s, and the two formats wind their contours **opposite** ways — measured this round at −0.186
against +0.165 signed em² for a capital `B`, a split that is exactly the format split and admits no
exception. §9.3.6 combines a text object's glyph outlines into one path under the non-zero winding
number rule and its NOTE 2 says the direction "can cause different output for overlapping glyphs",
so a document drawing two of §9.6.2.2's fourteen into one clip lost its overlap. The clause that
turns this from a permitted difference into a defect is §9.6.2.2's own first sentence — the
fourteen are **one set of Type 1 fonts**, and one set does not disagree with itself — while §9.5's
NOTE 5 is what makes the direction ours to fix. Every outline of a face this program *chose* is now
wound counter-clockwise, measured per glyph rather than inferred from the format; an embedded
program is never touched, because there the direction is the producer's statement.

**Date.** 2026-08-17.
**ADR.** [0396](../adr/0396-a-standard-set-of-fourteen-may-not-disagree-with-itself.md).
**Touched.** `crates/pdf-render/src/geom.rs` (`Path::signed_area`, `Path::reversed`,
`reverse_subpath`, five unit tests), `crates/pdf-font/src/substituted.rs`
(`wound_counter_clockwise`), `crates/pdf-font/src/loading.rs` (`build_outline` calls it under
`self.substituted`), `crates/pdf-font/src/standard.rs`
(`every_compiled_in_face_winds_its_contours_the_same_way`),
`crates/pdf-model/tests/glyph_clip_direction.rs` (new — the three constructions),
`doc/conformance/ledger.toml` (§8.5.3.3.2, §9.3.6, §9.5, §9.6.2.2),
`doc/todo/21-font-substitution.md` (§6 closed), `doc/todo/03-more-corpora.md` (§14's third
finding), `doc/adr/0396-*` (new), this file.

## What was verified by construction, and what by looking

**The three pairs session 558 built by hand are now three assertions**, each rasterised: each
glyph's clip alone, the pair's clip together, and the union of the two masks compared pixel by
pixel. Deleting the fix was run, and that is what makes the middle column evidence.

| pair | overlap | union pixels lost, before | after |
|---|---|---|---|
| Helvetica + Helvetica-Bold | 1851 | 0 | 0 |
| Times-Bold + Times-BoldItalic | 1816 | 0 | 0 |
| Helvetica + Times-Bold | 1642 | **1760** | 0 |

The two same-family rows pass on both sides and are kept deliberately: three cases that all failed
would not have said that each family was consistent while the set was not.

**And the page was opened** (trap 1), before and after, beside the corpus's own
`overlapping-glyph-clip-correct.png`. Before: white where the `8` crosses the `B`, and the right
pair's upper bowl gone. After: the union solid, white only where **both** glyphs have a counter,
which is what the non-zero rule says. Its ink, at 72 dpi on the crop box against three references
that agree to 1.2:

| | ink of 255 |
|---|---|
| `poppler` 79.962, `mupdf` 79.801, `ghostscript` 80.995 | |
| ours before | 70.812, **−8.989** from the lightest |
| ours after | 78.685, **−1.116** |

The residue is the faces' own shapes, which §9.5's NOTE 5 leaves ours, and it is below that
corpus's previous next-worst gap of −1.237.

## Every gate

- `cargo fmt --all --check`: silent. `cargo clippy --workspace --all-targets`: silent — one
  `clippy::arithmetic_side_effects` of my own in a new test was fixed before the run below, and the
  `viewer-qt@` lines are gcc's on a cold build as `doc/todo/02` §2 describes.
- `cargo nextest run --workspace`: 2069 → **2077**, all passing, 15 skipped. The eight are five
  geometry unit tests, the set's orientation assertion, and the two constructions.
- **Trap 1's two load-bearing font tests were run by name and pass unweakened**:
  `the_pdf_widths_agree_with_the_font_programs_own_advances` and
  `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`.
- `cargo test --workspace --doc`, `cargo test -p conformance`: pass (141 unit + 5 checks).
- **corpus**: `974 documents in 6.7s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 65
  incomplete, 0 slow`, silence lines 5 over 2, 57 over 9, 1226 over 41. Unchanged in every field.
- **oracle**: `1794 pages` — `agrees` 906/862, `contradicted` 67/66, `ambiguous` 786/753, our
  geometry 1/0, reference geometry 2/2, `not comparable` 13/7, `no render` 19/0. **No verdict
  moves, and the run was taken on both arms**: all 888 per-page metric lines are byte-identical.
- **text_extraction** (two gates): PDFBox `40 documents … overall 99.8% (14257/14281 words)` both
  orders, 4 below 90%; pdf.js `974 documents … overall 99.2% (22836/23015 words)`, 22 below 90%.
  Placement `10969/11163 (98.26%)`, 486 of 508 fully in bounds. All unmoved.
- **dates**, **xmp**, **jpeg2000**: pass.
- **render-quorra corpus**, default lane: `956 pages compared in 50.8s: 931 agree, 23 differ, 2
  refused, 18 not comparable`. **gpu lane at 4×**: `951 pages compared in 300.4s: 937 agree, 10
  differ, 4 refused, 23 not comparable`, ratchets off as that lane always reports. Both unchanged.
- **`display_list_digest` over all 974 first pages**, both arms with the same worker on disk:
  **162 lines differ**, every one with the same command count and the same `Debug` byte length and
  a different hash — the same commands with their points in a different order, which is exactly
  what a reversal is.
- **`doc/todo/00` step 7's ink sweep**, both arms, over every one of the gate's own 786 `ambiguous`
  lines: **byte-identical**, 786 measured and 0 skipped. Twenty at or past −1, sixteen of them
  documents this tree calls incomplete, and the four complete ones are `issue16038.pdf` −5.433,
  `issue12295.pdf` −2.828, `issue14297.pdf` −1.092 and `issue7821.pdf` −1.069 — all four diagnosed,
  the alarm holding.

**A sixth of the corpus changed geometry and not one pixel of it moved**, which is the claim this
round most wanted to be able to make and the reason both arms of three instruments were run rather
than argued about. It follows from the arithmetic — reversing every subpath negates every winding
number and both of §8.5.3.3's rules test a magnitude — but the arithmetic is what a round writes
down when it has not run the gate.

## One note on the instruments

The ink sweep's absolute values are the instrument's, not the page's: this run uses
`(1 − mean) × 255` over a luma greyscale, and session 558's head figures sit a little lower or
higher than the `-colorspace Gray` runs recorded elsewhere in `doc/todo/00`. What is compared across
a round is the same instrument before and after, which is what was done here. Two pages session 558
skipped for having no reference with ink are measured here, because a reference whose `.png` is not
an image at all is now dropped like one that drew nothing rather than taking its page off the sweep.
