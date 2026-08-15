# ADR 0216 — A band a descriptor cannot lie outside

Status: accepted, 2026-08-07 (session 378).

## Context

The project owner reported a failure they had seen in other viewers: over a scanned page's OCR
layer, no blue highlight appeared, and the cursor had to be aimed half a line below the text to
select anything. `doc/todo/14`, which this decision closes and deletes, took that apart into four
risks and found two of them already answered here — a wrong `/Widths` moves the glyphs and their
boxes together, because `select.rs` takes the box from the interpreter's own positioning and there
is no second calculation to get out of step; and §9.3.6's invisible rendering modes accumulate
readback and boxes like any other mode.
One was real and one was unverified.

**The real one.** A text object states a baseline, a size, a matrix, `Tz`, `Ts` and `TL`, and it
never states a line box. So the height of a selection rectangle is not in the file, and a viewer
invents it. `pdf_font::vertical_extent` invents it from §9.8.1's Table 120 — and its guard on the
two entries was `ascent > descent`, which is an ordering rather than a plausibility. It accepts
`/Ascent 0 /Descent -205`, a box entirely below the baseline; `/Ascent 8 /Descent -2`, a sliver;
and `/Ascent 4000 /Descent -1140`, a box five ems tall.

**And no instrument in this tree could see any of it.** `tests/text_extraction.rs` compares the
*characters* a page reads back as and never asks where they are. The oracle compares pixels, and
mode 3 marks none. Selection geometry over an OCR layer is a blind spot *between* the two gates,
which is the strongest argument in the todo file for spending a round here at all: a
wrong-height highlight ships without a single number moving.

## The census, first

`doc/todo/13`'s rule, and what made §10.5 a small change rather than a brave one: measure the
population before deciding what to build. `crates/pdf-model/examples/font_metric_census.rs` walks
every page's `/Font` resources and every form `XObject`'s, of all 974 corpus documents.

**964 documents open, 1629 distinct font dictionaries on their pages:**

| | fonts | documents |
|---|---|---|
| state a pair the band accepts | 1320 | |
| state neither entry — the em box already answered | 269 | |
| **state a pair `ascent > descent` accepted and the band rejects** | **22** | **15** |
| state a pair `ascent > descent` already rejected | 18 | 15 |
| **state a `/Descent` without its sign, read as the depth it states** | **42** | **23** |
| state a `/Widths` that is present, non-empty and all zeroes | 4 | 1 |

So it is a population rather than a file: **53 of the 974 documents** state something a
measurement of a face could not be. The corpus even holds a `zero_descent.pdf` and a
`font_ascent_descent.pdf`, which is somebody else having met this.

The distribution is more interesting than the total, and it changed what got built. The largest
group is not a lie at all — it is `/Ascent 905 /Descent 211`, which is Arial's real metrics with
the negative sign dropped, and `/Ascent 891 /Descent 216`, which is Times New Roman's. The next
largest is a *unit* error: `/Ascent 1808 /Descent -1048` and `/Ascent 3116 /Descent -2463` are
faces measured in a glyph space that is not §9.2.4's, and `/Ascent 8 /Descent -2` and
`/Ascent 9.464 /Descent -2.73` are the same mistake in the other direction.

## Decision

**Believe a stated pair only where it could be a measurement of a face, on a band derived from
what the standard prints; answer everything else with the em box.** One function,
`pdf_font::measured_extent`, called from `vertical_extent` and from nowhere else in the drawing
path.

### Where each number comes from

`CLAUDE.md` forbids tuning constants until a corpus matches, so the provenance of each number is
the point of this section. The band was fixed before the census ran and the census did not move it.

**The unit.** §9.8.1 puts every dimension of Table 120 in glyph space — "[a]ll dimensional values
shall be units in glyph space" — and §9.2.4 fixes what one of those is: "[f]or all font types
except Type 3, the units of glyph space are one-thousandth of a unit of text space". A text space
unit is the font size. So these entries are not free numbers; they are multiples of a fixed
quantity, and that is what makes a band on them derivable at all.

**The two sign conditions come from Table 120's own definitions.** `/Ascent` is "[t]he maximum
height above the baseline reached by glyphs in this font" — a height *above* the baseline, so a
value at or below zero describes a font whose glyphs reach nothing above it, which is not a font
anybody sets text in. `/Descent` is "[t]he maximum depth below the baseline reached by glyphs in
this font. The value shall be a negative number."

**The anchor for the line comes from §14.8.5.4.4's Table 380**, which is the one place the
standard says what a line of text is worth in font sizes. Its `/LineHeight` entry: "[t]he meaning
of the term 'reasonable value' is left to the PDF processor to determine. It should be
approximately 1.2 times the font size". The same entry's NOTE 1 states the arithmetic this crate
was already doing, which is why the citation is not a coincidence: "a reasonable method of
calculating the line height … is to find the difference between the associated font's Ascent and
Descent values (see 9.8, 'Font descriptors'), map it from glyph space to default user space".

**The standard states the same quantity a second time, and at a different number**, which is why
what follows is a band rather than a tolerance. §9.2.2: "[a] font defines the glyphs at one
standard size. This standard is arranged so that the nominal height of tightly spaced lines of
text is 1 unit." That is one em — the same height *tightly* spaced, where Table 380 is a line with
room around it. It is also, exactly, the em box this function has always fallen back to, arrived
at on other grounds two hundred and sixty sessions ago.

**The factor of two is the one number that is not printed anywhere, and it is a choice.** It
accepts every line from 0.6 em to 2.4 em, which contains both of the standard's own readings with
room either side. Its size is decided by the asymmetry of the two mistakes, not by any file:
disbelieving a true pair costs a highlight bounded by the em box, which a person can still see and
still click; believing a false one costs the highlight altogether, or puts it on a line the person
did not select. The cost of the width, written down rather than discovered later: **a pair that is
wrong by less than a factor of two is still believed.**

### The one repair, which the census asked for and the standard permits

**A positive `/Descent` is read as the depth it states.** This is a choice and not a clause, and
the argument is that Table 120's entry is two sentences rather than one. "The maximum depth below
the baseline reached by glyphs in this font" defines a *depth*, which is a magnitude; "[t]he value
shall be a negative number" is the convention for writing it down. A file that writes 905 and 211
for a face whose metrics are 905 and −211 has broken the convention and stated the measurement.

Answering those 42 fonts with the em box instead would be defensible and worse: their highlight
would stop at the baseline and miss every descender's tail, on 23 documents, where the file states
the number. Nothing this function feeds can move a glyph — it is read for a highlight and for
nothing else — so a mistaken repair costs a box and never a page.

**One file pays for it.** `issue19802.pdf` states `/Ascent 1056.15 /Descent 1148.93`; repaired,
that is a 2.2 em line, inside the band, where the old guard rejected it outright and the em box
answered. A 2.2 em box is what the band already tolerates for anybody, so this is the width of the
band showing up rather than a new kind of harm.

### What was considered and declined

- **A fourth condition from `/CapHeight`.** Table 120 defines it as "[t]he vertical coordinate of
  the top of flat capital letters, measured from the baseline", and a capital letter is a glyph, so
  `/Ascent` ≥ `/CapHeight` follows from two printed definitions with no constant at all — the most
  purely derivable rule available here. **The census killed it**: 109 font dictionaries state an
  ascent below their own cap height, and 102 of those are pairs the band otherwise accepts. A rule
  that throws away a hundred descriptors over a self-contradiction of a few glyph-space units is a
  rule whose cost exceeds what it buys. It is counted by the census and left unimplemented, which
  is a decision rather than an oversight.
- **`/FontBBox`.** The todo file ruled it out and the reason holds: Table 120 states it "expressed
  in the glyph coordinate system", which for a Type 3 font is the `/FontMatrix`'s and a different
  scale per file, and §9.6.4's Table 110 lets all four elements be zero — "a PDF processor shall
  make no assumptions about glyph sizes based on the font bounding box".
- **The zero `/Widths`.** §9.6.2 states no lower bound on a width, so a font whose every width is 0
  places every glyph at one point and every quadrilateral is a zero-width sliver — which
  `quads_for`'s run merge then joins into a run of no width at all. Acting on it would be a repair
  of a *conforming* file, and the corpus's only witness is 4 font dictionaries in one fuzzed
  document. Recorded as a choice in §9.6.2.1's ledger row, with the cost, and left alone.
- **Moving the band into `select.rs`.** The todo file's first "what not to do", and it is right:
  the extent is a property of the font, read once per show operation, and a consumer-side
  correction would be the second calculation the horizontal case is safe from precisely because it
  does not have one.
- **Applying the band in `variable_text::Metrics::read`**, which reads the same two entries to
  decide where a form field's baseline sits. Its guard is already stricter than the old one here
  (`ascent > 0 && descent < 0`), it answers a different question, and it is the one of the two that
  *draws*. Sharing the band would move pixels, so it is left for a round that measures them; the
  divergence is named here rather than left to be found.

## The gate, and that it would have failed before

`crates/pdf-model/tests/selection_geometry.rs`, six tests, built on a fixture that states a
descriptor the test chooses. Three of them are controls that must pass in both directions, and
three would have failed against the old guard — which was checked by putting `ascent > descent`
back and running them:

| test | before | after |
|---|---|---|
| `a_descriptor_that_cannot_be_a_measurement_gets_the_em_box` | **fails**: `/Ascent 0 /Descent -205` reaches 100 above a baseline at 100 | passes |
| `a_positive_descent_is_the_depth_it_states` | **fails**: the box's bottom edge is at 104.22, above the baseline it is meant to sit under | passes |
| `a_measured_descriptor_is_believed_to_the_number` | passes | passes |
| `invisible_modes_place_every_glyph_they_do_not_draw` | passes | passes |
| `consecutive_boxes_meet_at_the_advance` | passes | passes |
| `a_type_3_fonts_box_is_the_em_box_in_text_space` | passes | passes |

The three controls are what makes the other three mean something: a band that threw away true
statements would fail the first of them, and 1320 of the corpus's font dictionaries are in exactly
that case.

**The invisible-modes test was owed rather than new.** §9.3.6 requires that "[t]he e and f
components of Tm shall be updated for each glyph drawn when using text rendering mode 3 or 7 in
exactly the same way as would be done for other text rendering modes", and this tree has done it
since the thirteenth session — but nothing asserted it, and it is what makes every scanned
document selectable at all. A text layer built from what was *drawn* would be empty for all of
them. The test compares modes 3 and 7 against mode 0 over one string: the same readback, the same
number of placements, every quadrilateral identical, and no glyph in the display list.

**Three more state the band as arithmetic**, in `pdf-font` beside the function: the corpus's own
rejected pairs with the document each comes from, the two edges of the band, and the sign repair
with the sliver it still refuses. A rule that is arithmetic on two numbers should be checkable
without building a PDF, and those three survive every fixture above being deleted.

An eleventh test is one crate over, in `viewer_core::select`: a line of OCR boxes merges into one
shape, and a line of *zero-height* ones does not. That is the consumer-side consequence of the
band, and it was not obvious — `joins` measures the gap it will step over against the line's own
height, so a layer with no height comes back as one shape per glyph, with a seam at every edge.

## The Type 3 question, verified

The todo file marked it unverified: `interpret` gives a Type 3 font the em box outright, and
whether that box then goes through the `/FontMatrix` on its way to the quadrilateral was unknown.

**It does not, and that is correct.** The em box is stated in *text* space, where §9.4.4 makes one
unit the font size; the font matrix maps *glyph* space to text space (Table 110), so putting the
box through it would be converting a quantity that has already arrived. What does go through it is
the advance, because Table 110 says so of a Type 3 `/Widths` — "[t]hese widths shall be interpreted
in glyph space as specified by `FontMatrix` (unlike the widths of a Type 1 font, which are in
thousandths of a unit of text space)" — and `Type3Font::advance` applies the matrix's `a`
coefficient.

Reading it was not enough to leave it at, because the common file cannot tell the two readings
apart: Table 110's NOTE calls `[0.001 0 0 0.001 0 0]` "[a] common practice", and against a matrix
nobody varies, a box put through it once too often is wrong by a factor no fixture would notice.
`a_type_3_fonts_box_is_the_em_box_in_text_space` states `[0.01 0 0 0.01 0 0]`, where the two
readings differ by a hundred, and asserts the advance beside the box — because a test that checked
only the height would pass on an implementation that put *neither* through the matrix.

## And a family of citations that named the wrong table

`/Ascent` and `/Descent` are in **Table 120**, "Entries common to all font descriptors". Table 122
is "Additional font descriptor entries for CIDFonts", two subclauses away. Thirteen comments,
doc comments and ledger notes across four crates cited Table 122 for them — including
`vertical_extent`'s own, `select.rs`'s module comment and `oracle.rs`'s. Three more cited Table 122
for `/FontName` and for `/DW2`, which are Table 120's and Table 115's, and one ledger note cited
Table 127 — "PDF halftone types" — for the embedded font organisation, which is Table 124.

Every one of them is ISO 32000-1's number for a table ISO 32000-2 renumbered, which is the same
species the eighty-second session found three of in the ledger. The conformance gate cannot catch
it: as `tools/conformance/tests/conformance.rs` says in as many words, a wrong clause number names
nothing while a wrong table number names *another table*, so the assertion is the weaker true one
and the titles are printed for a person to read. All seventeen are corrected, and the four
remaining `Table 122` citations in the tree are about a CIDFont's `/Style` and `/Panose`, which is
what that table is.

## Consequences

- A selection highlight over a badly built OCR font contains the text it highlights, on the 53
  corpus documents whose descriptors say otherwise, and is unchanged on the 1320 font dictionaries
  that state a measurement.
- The blind spot between the text gate and the oracle has a gate in it. It is a *geometry* gate
  rather than a pixel one, which is the only kind that can see text nobody draws.
- Nothing a gate draws can move, and the reason is worth stating rather than assuming: the extent
  is read for `Placed::quad` and for nothing else. `glyph_quad`'s output reaches no display list
  command, so the corpus, the oracle, quorra and the text gate cannot see this change — and the
  numbers below say they did not.
- `pdf_font::measured_extent` is public, so the census measures the program's own rule rather than
  a copy of it that could drift.

*(**And the sweep that found this left six documents standing**, which the five-hundred-and-forty-fifth
corrected: ADRs 0032, 0045, 0118, 0211, 0323 and `doc/ui-boundary.md` went on attributing `/Ascent`,
`/Descent` and — in ADR 0045 — `/DW2` to Table 122. This ADR corrected the *tree* and said so; a
document is where a number a round retired goes on living, and the ninth sweep reads them now.)*
