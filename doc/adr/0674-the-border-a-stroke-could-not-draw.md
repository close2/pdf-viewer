# 0674 — The border a stroke could not draw

**Status.** Accepted. Session 756.

§12.5.4's border is drawn by stroking the annotation rectangle inset by half the border's width.
That construction is exact until the width reaches one of the rectangle's dimensions, and past that
point it does not draw a border at all — it draws a band in the middle of the rectangle, or, past
*both* dimensions, nothing. The comment above it said it "fills the rectangle solid", the oracle's
note about the corpus page that exercises it said "[o]urs fills it", and the raster said neither.

## Context

The round was sent to settle one pixel on `bug766086.pdf`: whether §12.5.4's

> If present, the border shall be drawn completely inside the annotation rectangle.

puts a one-unit border on the rectangle's own edge or inset by half its width. It does the latter,
this tree does the latter, and ADR 0675 has that half of the round. What follows here was found on
the way, by rendering the neighbouring page the same clause's other note is written about.

`bug1552113.pdf` is 250 × 50 with one link: `/Border [0 0 112]`, `/C [0 0 1]`, `/Rect [5 25 155 45]`
— a 112-unit border on a 150 × 20 rectangle, and a content stream whose second line reads "Bug
1552113 - this text should be visible." `AMBIGUOUS_OVERSIZED_BORDER` in `tests/oracle.rs` reads the
clause correctly and then says what it expects to see:

> A border that wide, drawn inside that rectangle, *is* the rectangle. Ours fills it.

It did not. Ours drew a **38 × 20** block in the middle of a 150 × 20 rectangle, which is the shape
the arithmetic produces and nothing anybody would call a border. Two synthetic widths on a 100 × 60
rectangle show the whole picture:

| `/Border` | the region the clause states | what this tree drew |
|---|---|---|
| 10 | the 2800-pixel frame inside `/Rect` | 2800 pixels, exactly the frame |
| 80 (past the height) | the rectangle, 6000 pixels | a 20 × 60 band, 1200 pixels |
| 300 (past both) | the rectangle, 6000 pixels | **nothing at all** |

## Why the stroke stops working, which is the whole of the argument

Table 168 calls `/W` "[t]he border width in points" and the sentence above puts the ink inside
`/Rect`, so a border of width *w* is the part of the rectangle within *w* of its boundary — the
rectangle minus the rectangle inset by *w*. Stroking the rectangle inset by *w*/2 with a pen *w*
wide lays down exactly that region, and does so for every width where the inset path is a rectangle:
`w < rect_width` and `w < rect_height`.

At the boundary the inset path degenerates. A "rectangle" with a zero-length pair of sides is a
segment, and a stroke of a segment covers a band *along* it — the two sides that vanished covered
the rest of the region, and they now cover nothing. `Border::inset` clamped the inset to the centre
line to keep the path from inverting, which it does; what the clamp cannot do is put back the two
sides. Past both dimensions the path is a *point*, and a butt-capped stroke of a point is empty.

**The condition at which the frame closes and the condition at which the stroke fails are the same
condition**, which is what makes this a clean two-case construction rather than a special case bolted
on: both are `w >= rect_width || w >= rect_height`. Below it, stroke the inset path; at or above it,
the region is the rectangle, so fill the rectangle.

## The decision

`Border::fills` states that condition and `Border::draw` acts on it, and every caller of the border
— §12.5.6.5's link, §12.5.6.19's widget, §12.5.6.6's free text — goes through `draw` rather than
composing `apply`, `outline` and `paint` for itself. §12.5.6.8's square and circle take the same
condition inline, because Table 180's `/BS` is the width of "the lines drawn by" the annotation
rather than a border style, so its shape and its paint are its own; the geometry underneath is
identical and the ellipse's inscribed band closes at exactly the same width.

Three details are decisions rather than consequences:

- **The fill is in the border's colour, not `/IC`'s.** A square whose line covers it has its
  interior colour underneath the line, whole. Painting `/IC` and stroking nothing would show the
  wrong one of the two.
- **`draw` wraps the marks in `q`/`Q`.** The fill colour a covering border sets would otherwise
  outlive the border, and so would the line width and dash `apply` writes — which they already did,
  harmlessly, and there is no reason to keep a leak while adding a second.
- **Table 168's `U` underline is left alone.** It spans the rectangle's full width with butt caps,
  so its own stroke already covers the whole rectangle once the width reaches the height. It needs
  no second shape and gets none.

## What this is evidence of, beyond the clause

**A comment that says what the code achieves is a claim, and this one outlived its truth by eight
hundred sessions.** Nothing in the tree could see it: the arithmetic is right in the case every test
covered, the corpus's one witness is a page whose whole content is a link, and the oracle's verdict
on it was *ambiguous* — a page where three references draw no link border at all and the fourth
draws a wrong one is a page nothing can rank us on. The instrument that found it was
`doc/traps/pixels-and-rasterisers.md` trap 1, unassisted: render the page and look at it.

The two new tests in `tests/annotations.rs` fail against the tree before this change and pass after
it, which is trap 13 and was run rather than assumed.
