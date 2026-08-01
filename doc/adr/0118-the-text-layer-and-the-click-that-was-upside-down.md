# ADR 0118 — The text layer, and the click that was upside down

Status: accepted, 2026-08-01.

## The defect, first, because it is the important half

**Every click has been mapped to the wrong half of the page since the sixty-first session.**

`pdf_model::content::user_space_at` maps a point back to default user space, and its doc comment
opened with: "Maps a point in the *page's* space — the display list's, and **the raster's** —
back to default user space."

Those are not one space. PDF's y axis points up from the bottom of the page; a raster's points
down from its top row; and the flip between them belongs to `TargetSpec::for_page`, whose own
comment has said so since the sixty-first session — "the page's top edge is raster row zero", ADR
0064. The page's transform never had it. So a caller holding a *pixel* position and handing it
straight to `user_space_at` was asking about the mirror of the point it meant.

The caller was the click. `pdf-viewer.rs` did exactly this from the fifty-seventh session, when
links were first followed, and the flip that made it wrong arrived four sessions later in code
that had nothing to do with clicking. Session 132 moved the same two lines into `viewer-core`
without noticing.

### Why nothing caught it

- **No gate clicks.** The oracle renders and compares pixels; the corpus interprets; the text
  gate extracts. None of them has a pointer.
- **The tests written for it four hours earlier passed.** They took the point from a grid scan of
  `Query::LinkAt` — the same broken mapping — and asked whether *a* link was there. On
  `basicapi.pdf` the mirror of a link is another link, because the page's four links sit in two
  bands and the mirror of one band is the other. **A test that finds its own input by asking the
  thing it is testing cannot fail.**
- The arithmetic is invisible in the small: on a page 841.89 units tall, a link at y = 703 and
  its mirror at y = 139 are both plausible-looking numbers.

### What found it

Writing the text layer. The layer's quadrilaterals come out in the display list's space, and
asking "so which space is that, and is it the one the click uses?" is a question that had never
been asked out loud. Then one `println!` of `basicapi.pdf`'s link rectangles against the point
the tests were clicking: the links are at device y ≈ 139–172 of a 1000-pixel window and the tests
were clicking y = 830.

### The fix, and the test that pins it

`Viewer::user_space` flips about the **page's** height rather than the raster's, because that is
what the forward transform translates by — a raster is rounded up to contain the page and the
leftover fraction of a row is at the bottom.

The tests now compute the device point *from the link's own `/Rect`* through the geometry the
viewer reports, and assert that the point mirrored about the viewport's middle is **not** a link.
Both fail against the old mapping; the mirror assertion is what makes the pair discriminating
rather than merely passing.

`user_space_at`'s doc comment now says which space it means and which it does not.

## The text layer

`Interpretation::text_layer` is one `Placed` per character code the page shows: the range of the
readback that code produced, and the quadrilateral its glyph occupies in the display list's
coordinates. Selection needs point → position, range → quads and a stable order; search needs the
second; §14.9's accessibility consumer needs the third. **Build it once and three consumers
appear**, which is why it is one artefact rather than three.

Two decisions inside it:

**The box is the advance by Table 122's `/Ascent` and `/Descent`.** Nothing in ISO 32000-2 asks
where a selection highlight goes — selecting text is not a thing the standard describes — but it
does say where a glyph is drawn (§9.4.4's text rendering matrix) and how far a font's glyphs
reach above and below the baseline, and those two answer the question without inventing a
constant. A font that states neither entry gets the **em box**, 1 up and 0 down: a defined
quantity rather than a guess, and the one place a fallback here could have invented a number.

**It is built for text nothing draws.** Rendering modes 3 and 7 mark no pixels, and an OCR layer
under a scanned page is nothing but mode 3 — which is the text a person most wants to select. A
layer built from what was *painted* would be empty on exactly those pages.
`invisible_text_is_placed_too` pins it on `issue1155.pdf`: 0 glyphs marked, a readback, and a
full layer.

## The cost, measured

A/B by callgrind on `examples/callgrind_interpret`, both builds in one sitting:

| | instructions |
|---|---|
| without the layer | 2 114 818 575 |
| with it | 2 150 654 805 |
| | **+35.8 M, +1.69%** |

Kept unconditional, with the cost written down. The alternative was a flag on `interpret`, and it
buys only the gates: a viewer wants the layer on every page it shows, and two behaviours behind
one function is how a caller ends up holding a layer that was never built.

**One plausible optimisation was refused by counting.** Reserving `bytes.len()` entries once per
show operation — an upper bound on the codes in it — *costs* 10 M rather than saving anything:
2 160 662 744, +0.47%. `Vec`'s growth is already amortised and the reserve is a capacity check per
show operation on a vector that is usually large enough. Three sessions of this project have now
refused an optimisation by counting it first.

## Consequences

Tests 912 → 916, and the four are `text_layer.rs`'s invariants — each fails against a different
mistake, and one was checked by making the mistake: giving every box the em width instead of the
glyph's advance takes abutting pairs from 1485 of 5898 to **0 of 5898**. The mirror assertion
went into the click test that was already there, which is where it belongs: that test was the one
that could not fail.

`pdf_font::LoadedFont::extent` is new, and reads Table 122's two entries.

The four gates are unmoved: the layer is output nothing else reads, and the y flip lives in a
crate no gate exercises.

## The lesson

**A doc comment that names two things as one is a defect with a five-year fuse.** "The display
list's, and the raster's" was written when they *were* the same, and the session that made them
different fixed the rendering, wrote an ADR about the flip, and left the sentence standing. The
handover's own rule — *a retired claim is a string, and strings are greppable* — was written for
ledger notes; it applies at least as hard to the sentence above a function that converts between
two coordinate spaces.

And: **when a test needs a point, take it from the document, not from the code under test.**
