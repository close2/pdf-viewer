# 0675 — The rectangle a reference draws outside, measured over a population

**Status.** Accepted. Session 756.

`poppler` puts an annotation border's path **on** `/Rect`'s boundary, so half the stroke falls
outside the rectangle §12.5.4 requires it to be completely inside. This tree insets the path by half
the width and is right. The finding is a reference's, it is systematic rather than a page's, and
`crates/pdf-model/examples/border_overhang_census.rs` is the instrument that says so over a
population instead of a witness.

## Context

Session 751 found this on one page and did not act on it. `AMBIGUOUS_LINK_BORDER`'s note had
recorded that ours and `poppler`'s borders on `bug766086.pdf` "both draw it" and priced the ink —
20.61 against 20.73 — which is right and hid a pixel, because the measure that page's distance is
taken on is the structural similarity rather than the mean. Read off the two rasters at the page's
own scale, ours strokes device columns 5 and 189 and rows 10 and 39; `poppler` strokes 5 and **190**
and 10 and **40**. This round re-measured both and reproduces them exactly.

## What the clause requires, which is the only question that decides it

§12.5.4, of any annotation's border:

> If present, the border shall be drawn completely inside the annotation rectangle.

and Table 168 of the entry that sizes it:

> W — number — (Optional) The border width in points. If this value is 0, no border shall be drawn.
> Default value: 1.

That is the whole of what the subclause says about placement, and there is **no width-1 case in
it**: the only sentence naming 1 gives it as the default width where neither `/Border` nor `/BS` is
present, which is a statement about how wide the border is and not about where it goes. A stroke
straddles its path, so a border of width *w* whose ink is entirely inside the rectangle has its path
inset by *w*/2, at every width. Ours is that. ADR 0674 is what happens at the widths where a stroke
can no longer state the region at all.

## What each renderer is doing, at two widths and on a synthetic page

The one-pixel disagreement is the hardest place to read a mechanism off, so the mechanism was read
where it is unambiguous. A link with `/Border [0 0 10]` on `/Rect [20 20 120 80]`, 72 dpi, one
device pixel per unit:

- **ours** — blue at device columns 20…119 and rows 20…79, 2800 pixels: the frame ten units wide
  measured inward from `/Rect`, and not one pixel outside it.
- **`poppler`** — 15…124 and 15…84: **five units beyond `/Rect` on all four sides**, which is *w*/2
  exactly, and 3200 pixels because the frame it draws is centred on the boundary.

`bug1552113.pdf`'s `/Border [0 0 112]` is the same behaviour with nothing to hide behind: its blue
covers the page and the document's own content stream says "this text should be visible" underneath
it. At width 1 `poppler` snaps a thin line to the pixel grid, so which sides show the overhang
depends on where `/Rect`'s edges fall — two of four on `bug766086.pdf`, whose edges are integers,
and none on `issue12750.pdf`, whose `/Rect` is `[178.019 654.247 265.051 668.194]` and whose border
lands on the same columns as ours. **That is rounding on top of the placement, not a second
placement.**

## The population, and the instrument that reads it

A single page is a witness. `border_overhang_census` takes both renders of every page whose
annotation states a border this tree strokes and no `/AP`, maps `/Rect` into each raster through the
crop box and that raster's own size, and asks how far outside the rectangle ink of the border's
**stated colour** reaches. Over `doc/pdf.js` and `doc/corpora` together its comparison set — the
annotations where both renderers put that exact colour round the rectangle — divides in two, and the
division is the design rather than a presentation:

- **In a colour of the page's own**: `poppler` reaches further outside than this tree on three
  quarters of them and this tree reaches further on **none**.
- **In §8.4.1's black**: the figures are noise. A `/C [0 0 0]` border sits in the one colour a page
  of text is already full of, so black glyphs near the rectangle raise *both* renders' numbers and
  neither is the border. One corpus document supplies almost all of them.

The counts are the census's to print and are not written here (ADR 0281). Two properties of the
instrument are, because a reader of its output needs them:

- **The level is contaminated and the difference is not.** The page under the two renders is the
  same page, so content in the border's colour cancels in the comparison and only a band one
  renderer reaches is the border. The first version of this test asked whether a pixel was *nearer*
  the border's colour than the paper, and on `issue17056.pdf` — 31 links whose `/C` is `[0 0 0.5]`
  over black text — it called every black glyph within two pixels of a rectangle that annotation's
  border and reported **this tree** two pixels outside `/Rect` on all 31. Equality, not nearness,
  and the split above for what equality still cannot separate.
- **The two renderers do not scan-convert a thin line alike, so their populations differ.** Counting
  only pixels covered *whole* means a stroke narrow or fractionally placed enough that no pixel is
  ever the stated colour is drawn by nobody here. `poppler` paints such a line solid where this tree
  blends it — on `issue18030.pdf` its render holds 256 pixels of the border's `[0 0 .8]` and ours
  holds none, the nearest being `(159, 159, 236)` — so the reference's population is the larger one.
  That is §10.7.4 and this clause has nothing to do with it.

## The decision

Nothing in this tree changes on account of the reference. The exemption on `bug766086.pdf` and
`bug1552113.pdf` stays, and both notes now say what it is an exemption *for*: **a documented
departure of `poppler`'s**, priced in pixels against the clause, rather than a corner of the
specification nobody can settle. That is principle 5 run in the direction it is written — the
disagreement was taken to the clause, the clause answered, and the answer is not a target to move
toward.

The instrument is the part worth keeping. `border_precedence_census` has stated §12.5.4's placement
question since it was written and could never answer it, because where ink lands is a fact about a
raster and that census reads dictionaries. Its module comment now says which census answers it.
