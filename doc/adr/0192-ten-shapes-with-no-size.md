# ADR 0192 — Ten shapes with no size

Status: accepted, 2026-08-05 (session 314).

## Context

The corpus's incomplete list held this sentence, on `issue13447.pdf`:

> Line: no appearance stream, and Table 179's line endings state no size, so the ends are drawn
> plain

Table 178 makes `/LE` "[a]n array of two names specifying the line ending styles **that shall be
used in drawing the line**", and Table 179 gives ten of them. What the table gives with each name is
a *description* and no dimension at all:

| | |
|---|---|
| Square, Circle, Diamond | "A square / a circle / a diamond shape filled with the annotation's interior colour, if any" |
| `OpenArrow` | "Two short lines meeting in an acute angle to form an open arrowhead" |
| `ClosedArrow` | the same "connected by a third line to form a triangular closed arrowhead filled with the annotation's interior colour, if any" |
| `ROpenArrow`, `RClosedArrow` | "in the reverse direction from" the two above |
| Butt | "A short line at the endpoint perpendicular to the line itself" |
| Slash | "A short line at the endpoint approximately 30 degrees clockwise from perpendicular to the line itself" |
| None | "No line ending" |

So this is **a `shall` behind a silence about artwork** — ADR 0109's shape, where §12.5.6.4 requires
"predefined icon appearances" for seven names and draws none of them. It had been refused since the
eighty-fifth session on the true observation that the table states no dimension, which is a reason
to *choose* one rather than a reason to decline: the same move ADR 0043 made for §12.5.6.10's text
markup, whose thickness the standard states nowhere either.

## Decision

**Draw all ten, at a size taken from the only length the annotation supplies.**

A line annotation states two things with a length: `/L`, which is where the line is, and §12.5.4's
border width, which is how thick it is drawn. `/Rect` is not a third — a writer sizes it *around*
the endings, so taking the size from it would be circular. So the ending's extent is a multiple of
the border width, which makes it right at every scale rather than right at one, and the multiple is
a choice: **four**, written down as `ENDING_SIZE` beside the reason.

That is the same construction §12.5.6.10's marks already use — "the quadrilateral's own height is
the only length the annotation gives", with a thickness of a sixteenth of it — one clause over.

Three more choices, each in the code beside the sentence that leaves it open:

- **Which way an arrowhead points.** The table says nothing. An `OpenArrow` at (x1, y1) points
  *away* from the line, so a line with arrows at both ends has one at each end pointing outwards —
  and Table 179's own `ROpenArrow` and `RClosedArrow`, "in the reverse direction from" it, are the
  standard's way of asking for the other, which is only meaningful once a direction is fixed.
- **The apex angle.** "An acute angle" is the whole of the constraint; 60° at the apex is acute and
  is what an arrowhead is drawn at.
- **Where the endings go when `/LL` moves the line.** Table 178 says `/LE` states the styles "for
  the endpoints defined … by the first and second pairs of coordinates … in the L array" and says,
  four rows earlier, that with `/LL` present those coordinates are "the endpoints of the leader
  lines rather than the endpoints of the line itself". The endpoints the first sentence names are
  therefore not on the line; Figure 80 draws the arrowhead on the line proper, and an ending is an
  ending *of a line*, so that is where it goes.

**And two things stay refused, each with its own sentence.** A `/LE` naming something outside
Table 179 is reported rather than dropped to `None` — a line that quietly lost its arrowheads would
be trap 5 — and §12.5.6.9's *polygon* is reported where it states `/LE` at all, because a polyline
is what the clause calls a polygon "except that the first and last vertex are not implicitly
connected", so a polygon's are and it has no end to decorate. So is a polyline whose
shape comes from Table 181's `/Path`, whose curves this routine does not hold the ends of.

## Consequences

- **`issue13447.pdf` leaves the corpus's incomplete list**, 74 → 73, and it is the only witness
  the corpus has: the report was one document's.
- **Table 181's `/IC` finally does something on a polyline.** The table divides the entry by
  subtype — "[f]or Polyline annotations, the value of the IC key is used to fill only the line
  ending" — and this tree read it, refused the ending, and therefore drew nothing with it. Four of
  Table 179's ten fill and six do not, which is now a test.
- **§12.5.6.7's `/Cap` is still owed and is a different kind of nothing.** A caption needs a font
  and no entry of a line annotation states one; the line is drawn and the caption is named beside
  it, as it has been since session 116. What changed is that the report no longer has to carry two
  refusals in one sentence, so `LINE_ENDINGS_AND_CAPTION` is gone.
- **The oracle sees one page differently**, and it is the page that had the report.
