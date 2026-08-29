# 0756 — A quantity the clause hands over is not one two processors can be compared on

Status: accepted.
Context: `crates/pdf-model/tests/text_extraction.rs`'s word-box geometry instrument (ADR 0323's
instrument 1, geometry half; ADR 0421's ratchet; ADR 0726's reading of its tail), and ISO 32000-2
§12.7.4.3.

## The instrument, and what it is a comparison of

`the_word_boxes_we_place_agree_with_the_references` matches this tree's word boxes against
`pdftotext -bbox -cropbox`'s by unique text and holds each matched pair to two bounds. The tight
one — half a point on each edge along the reading axis — is not a taste. It was derived by running
two positional extractors that share no code against each other over the same corpus, and it is
half a point because that is what *they* agree to. The quantity behind that agreement is §9.4.4's
displacement arithmetic: a content stream states a `Tm`, a `Td`, a `Tc`, a `Tw` and a set of
widths, and two correct readers of those numbers land on the same point.

So the bound is a statement about a *file's* arithmetic, and it is only meaningful where a file did
the arithmetic.

## Where no file did

§12.7.4.3 describes the other case in its own opening:

> In such cases, the PDF document cannot provide a statically defined appearance stream for
> displaying the field. Instead, the PDF processor shall construct an appearance stream
> dynamically at rendering time.

It then divides the construction into a part it fixes and a part it hands over. Fixed: the
resources come from `/DR`, and the bounding box's "lower-left corner … is set to coordinates (0, 0)
in the form coordinate system. The box's top and right coordinates are taken from the dimensions of
the annotation rectangle". Handed over — and this is the whole of the clause's instruction about
where the glyphs go:

> the interactive PDF processor shall replace the horizontal and vertical translation components
> with positioning values it determines to be appropriate, based on the field value, the quadding
> ( Q ) attribute, and any layout rules it employs

The clause states no margin inside the box, no baseline rule and no inset. "Any layout rules it
employs" is where a margin comes from, and a processor that employs one is doing what the sentence
says.

Two processors will therefore disagree, legitimately. This tree insets the layout box by §12.5.4's
border width — one point where nothing states another — so a value clears the border the same
appearance draws and nothing else. `pdftotext`'s rule, read off the corpus rather than off
poppler's source, is the `/BS` width plus two points: its word `xMin` sits 2.000 pt inside the
`/Rect` on the four witnesses with no `/BS` and 3.000 pt inside on `issue12750.pdf`, whose `/BS`
states `/W 1`. The difference is exactly one point, and it appears on *every* word of those pages,
on both edges, with the vertical centre agreeing to 0.41 of the word's height.

## What was wrong, and it was not the reading

The seven-hundred-and-ninety-first session read all of that and wrote it into the list's own note
under the heading *§12.7.4.3's layout hand-off* (ADR 0726). Every sentence of that diagnosis is
correct. What it did not do is tell the instrument, and the consequences are the ones a diagnosis
kept as prose always has:

- the six documents stayed in `SELECTION_BELOW_FLOOR`, which is checked for **equality** in both
  directions — so they are pinned there, and a round that made them agree would have had to argue
  its way past a ratchet;
- the tail's own printed summary counted them under *past the horizontal edges alone*, which reads
  as "this tree places six documents' words wrongly";
- and the verdict's denominator carried words whose position no bound in this file can be about.

The third is the sharpest, because it is what makes this a defect in the instrument rather than an
untidiness. `HORIZONTAL_BOUND` is a tolerance on a shared quantity. Applying it to an unshared one
does not measure a looser version of the same thing; it measures nothing at all, and reports a
number.

## The decision

A word laid out into an appearance §12.7.4.3 had *this processor* build is set aside: counted,
named in the printed refusal table, and not judged.

Which words those are is answered from the **file**, not from the delta. An annotation's appearance
is the file's unless the file says otherwise, and the file has two ways of saying otherwise —
Table 224's `/NeedAppearances`, "a flag specifying whether to construct appearance streams and
appearance dictionaries for all widget annotations in the document", and an annotation that states
no `/AP` at all, leaving §12.5.5's stream to be built (§12.5.6.19 for a widget, §12.5.6.6 for free
text). A matched word whose centre lies inside such an annotation's `/Rect` is one of these.

Three details are decisions rather than mechanics:

- **The centre, not the whole box.** A field's value is routinely drawn taller than the rectangle
  it is drawn in, because §12.7.4.3 clips the value rather than shrinking it, and the readback's
  quad is the font's ascent-to-descent band whatever the clip then does to the ink:
  `issue12750.pdf`'s quad clears its `/Rect` by 0.53 pt and `issue19389.pdf`'s by 3.4 pt.
  Whole-box containment therefore answers *does this value fit its field*, which is a different
  question and one no bound here asks — measured both ways, and it left two of the six documents
  in the list under a heading that had already explained them.
- **The condition is the instrument's and is deliberately wider than the interpreter's.**
  `pdf_model::appearance::regenerates` is narrower: it excludes the three field types §12.7.4.3
  lays out no text for. It is `pub(crate)` and unreachable from a test, and it is also not what
  should be duplicated here — a wider set-aside can only take matched words *out* of the verdict,
  never put agreement into it, so wide is the safe direction and the printed count is what keeps it
  honest (trap 11).
- **The refusal for a page with nothing left.** Five documents' every unique match was a field
  value, so scoring them 0 of 0 would have put them in the out-of-bounds list on an empty
  population. They become a named refusal — `every unique match is a value §12.7.4.3 placed` —
  counted beside the other nine reasons, and `JUDGED_FLOOR` falls with the argument written beside
  it. A ratchet lowered by argument is a different act from a ratchet lowered by attrition, and the
  difference is whether the reason is in the file.

## What was refused

Moving this tree's inset to two points, which would take all six documents inside the bound. That
is curve-fitting to another renderer on a question the standard answers with a delegation, and
`CLAUDE.md` forbids it outright. There is no clause to appeal to: the inset is ours to choose, ours
is derived from §12.5.4's border, and poppler's extra two points are derived from nothing this
project can read.

## What it found that the note could not have

`issue16021.pdf` was filed one paragraph further down, under *No stated pair at all* — a
disagreement about a font's ascent and descent, on the evidence that its `/Rect`-relative
horizontal deltas are 0.00 pt and its vertical centre is 0.51 of the word's height. It is a
`/FreeText` annotation with no `/AP` on a page whose `/Resources` are empty, so its "Hello World"
is the whole of the page's text and §12.7.4.3 places every glyph of it. The vertical centre is the
hand-off's *other* half, which the clause delegates in the same sentence.

The general shape is worth more than the instance: **a class read off the measure cannot see who
placed the word.** Both documents show a small vertical-centre disagreement with exact horizontal
edges, and one is a font-metric convention while the other is a layout rule the clause hands over.
Only the file distinguishes them, which is why the set-aside is derived from the file.

## What it costs, printed

Five documents leave the judged set and 23 more matched words leave four judged documents, of which
20 were inside the bounds. The verdict moves from 487 of 508 documents fully in bounds to 489 of
503, and from 98.28% of matched words to 98.38%. Every one of those figures is the gate's own
output; none of them is written into an instruction file.

## Calibration

Trap 13, both ways, above the commit that made the change:

- with `placed_by_this_processor` returning nothing, the gate fails naming all seven documents that
  come back — `7 document(s) newly carrying a word out of bounds`;
- with the `/NeedAppearances`-or-no-`/AP` condition removed so that every widget and free text
  rectangle sets its words aside, the judged set falls to 480 and the gate fails on `JUDGED_FLOOR`.

Both were reverted.

## What is still owed

Nine of the fourteen documents left in the list fail the horizontal bound and four the vertical
centre alone, and ADR 0726's remaining classes still name each. Two of them are worth a round on
their own terms and neither is this one's:

- **`issue6127.pdf` is still undiagnosed**, and it is the one where both references agree against
  this tree. ADR 0726 already says so; nothing here changes it.
- **The vertical-centre bound divides by the mean of two box heights, one of which this instrument
  has already declared to be each extractor's own convention** (ADR 0323's Finding 3). Where the
  reference's box collapses — `issue1350.pdf`'s is 0.146 pt tall and `bug868745.pdf`'s 0.200 pt,
  against ten- and twelve-point words — the denominator halves and the numerator becomes half our
  own height, so the measure reads ≈ 1.0 by construction and says nothing about where the word sits.
  That is a second instrument question of the same family as this one, and it was left alone here
  because every obvious repair lands those two documents within a thousandth of the bound, which is
  a tuned constant wearing a fix.
