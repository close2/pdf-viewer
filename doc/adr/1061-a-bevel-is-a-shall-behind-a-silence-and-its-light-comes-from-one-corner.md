# ADR 1061 — A bevel is a `shall` behind a silence, and its light comes from one corner

## Status

Accepted, 2026-09-14. Session 1046. Takes the decision ADR 1057 §4 named and deliberately left:
Table 168's `B` and `I` are drawn rather than reported, and the relief that produces the illusion
is chosen here once.

`§N` is ISO 32000-2 and nothing else.

## Context

`appearance.rs` draws a subtype whose clause states a shape and refuses — loudly — one whose clause
"names an appearance without stating what it looks like". Table 168's two relief styles sat on the
refusing side, and until this session a link, a widget or a free text annotation whose `/BS` said
`/S /B` drew a plain solid rectangle and reported "Table 168's beveled and inset borders state no
highlight or shadow colour".

Session 1040 moved Table 169's cloudy `/BE` off that side by reading §12.5.4 whole, and ADR 1057 §4
says outright that `B` and `I` are "the same construction" and that taking them is "a separate
ADR's". This is that ADR, and the reading was made again from the clause rather than from that
sentence.

## Decision, part 1: the styles are owed

§12.5.4 states the obligation to draw a border twice, in sentences that are about every style
Table 168 lists:

> If present, the border shall be drawn completely inside the annotation rectangle.

> If neither the Border nor the BS entry is present, the border shall be drawn as a solid line
> with a width of 1 point.

Table 168's `/S` row is then a statement of *which* border, in the same grammar for all five
values: `S` is "[a] solid rectangle surrounding the annotation", `U` "[a] single line along the
bottom of the annotation rectangle", and

> B (Beveled) A simulated embossed rectangle that appears to be raised above the surface of the
> page. I (Inset) A simulated engraved rectangle that appears to be recessed below the surface of
> the page.

Three of those five were drawn and two were not, on the strength of the table stating no colour for
the relief. That is ADR 0192's shape and ADR 1057's: **a `shall` behind a silence about artwork is
a reason to choose, not to decline.** The contrast that keeps both rules alive is §12.5.6.12's
stamp, where the whole obligation is a recommendation — "should provide predefined icon
appearances" — and there is no `shall` for a choice to discharge. Here the `shall` is the clause's
own and the silence is a table's.

The refusal also cost what the rule exists to avoid. A border that asked to be raised drew a flat
rectangle *and* said something was missing, so the report fired on every `B` and `I` in the world
while the page showed a border the file did not describe.

## Decision, part 2: what is chosen

Every number below is a choice. The standard states none of them, and none is fitted to another
renderer; they are made once here so a later round does not re-litigate them.

- **The relief is two bands inside the line, each as wide as the line.** The line itself is
  unchanged — stroked in the border's own colour, on the rectangle inset by half the width, as
  every other style is. What "simulates" the relief is a lit band and a shaded band just inside it,
  meeting at two opposite corners on the diagonal. The band's width is the one number the
  dictionary states, because inventing a second would be a second choice for no gain.
- **The light falls from the upper left.** A simulation of relief needs a light direction and the
  standard names none; any consistent direction produces the illusion. What is *not* free is that
  `B` and `I` be each other's inverse — that is the whole of the difference between "raised above"
  and "recessed below" — so a bevelled rectangle is lit along its top and left and shaded along its
  bottom and right, and an inset one is the same relief with the two exchanged.
- **The bands are white and mid-grey in `DeviceGray`, not tints of the border's colour.** Table 166
  gives `/C` and Table 192 `/BC`, and each is the colour of the *line*; the relief is two further
  colours the standard does not state. Deriving them from the line's colour would make a border
  whose colour array is empty — Table 166's "0 No colour; transparent" — have no relief at all,
  and the style is a property of the rectangle rather than of the line. One cost, named rather than
  left silent: on a white page the lit band is invisible and only the shadow carries the illusion.
- **The relief is part of the border for anything laid out inside it.** §12.5.4 puts the border
  "completely inside the annotation rectangle", so a widget's icon, caption or §12.7.4.3 text
  starts past twice the width rather than past the width. `Border::thickness` is that number and
  every caller that used `Border::width` for a margin asks it now.
- **Where there is no room, the same rule degenerates rather than switching off.** The bands are
  the rectangle inset by the width and by twice it, each clamped to the rectangle's centre lines —
  the clamp `Border::inset_by` already applies for a cloud's cusps — so a width that leaves the
  relief less than its band still yields two polygons meeting on the diagonal, inside the
  rectangle. Nothing special-cases it, and the only rectangle it is visible on is one whose border
  is a quarter of its own width.

## Consequences

`Style::Simulated` is now `Style::Bevelled` and `Style::Inset`, `Border::simulated` is gone, and
with it the last caller of the report those three subtypes carried. `square_or_circle`'s comment
loses a sentence that had been about a gap rather than about a style: §12.5.4 gives line, square,
circle and ink annotations a `/BS` that supplies "the width and dash pattern" alone, so there is no
style on those four to raise and never was one to report.

**The corpus cannot rank this, counted rather than assumed** (trap 8).
`examples/border_precedence_census` walks every annotation that states no `/AP`, which is every
border this tree constructs: over the curated corpus **0 documents of 1452 state a `B` or an `I`**
among 45 909 constructed borders, and over pdf.js's 963 **0 of 33 781**. The three tests are
therefore hand-built, and the crawl's figures in §12.5.4's ledger row are what says the styles
exist in the world at all.

**Looked at rather than counted** (trap 1). Ten links, `B` and `I` at five widths, rendered at
scale 3: at one and two points the shadow alone carries it, at four and eight the frame reads as
raised or recessed at a glance, and at twenty — a border a quarter of its rectangle's width — the
clamp above is what is on the page.
