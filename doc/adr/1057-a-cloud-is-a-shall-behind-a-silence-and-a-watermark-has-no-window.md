# 1057 — A cloudy border is a `shall` behind a silence, and a watermark has no window

Session 1040. Status: **accepted**. Adds `pdf_model::cloud`; §12.5.4's cloudy `/BE` is drawn on
a square, a circle, a polygon and a free text annotation where it was refused and reported; a
watermark annotation is given no popup window and no pointer. Moves no page of the tracked corpus:
`examples/cloudy_border_census` finds two cloudy annotations under `doc/` and both carry an `/AP`.

`§N` is ISO 32000-2 and nothing else.

## 1. The rule the refusal rested on, and why the cloud is not under it

`appearance.rs`'s module comment states the line this crate draws: a subtype whose clause states a
*shape* is constructed, and one whose clause "names an appearance without stating what it looks
like is refused and reported". §12.5.6.12's stamp legends are the standing example — "**should**
provide predefined icon appearances" — and the cloudy border was filed beside them on the strength
of Table 169's two `should`s: "the border should be drawn as a series of convex curved line
segments in a manner that simulates the appearance of a cloud".

Read whole, the entry is not that shape. The sentence that *introduces* it is a `shall`, in §12.5.4:

> Beginning with PDF 1.5, some annotations (square, circle, and polygon) may have a BE entry, which
> is a border effect dictionary that specifies an effect that shall be applied to the border of
> the annotations.

and Table 181 says it again of a polygon's — "[a] border effect dictionary that shall describe an
effect applied to the border". The `should`s are inside that `shall`: they say what the effect looks
like, and the obligation to apply it is normative. That is exactly the construction ADR 0192 met in
Table 179 — "[t]wo short lines meeting in an acute angle" states no dimension either — and resolved
the same way: **a `shall` behind a silence about artwork is a reason to choose, not to decline**.
The stamp's `should provide` is different in kind: there the *whole* obligation is the
recommendation, and there is no `shall` for a choice to discharge. Both rules stand; the cloud was
on the wrong side of the line between them.

Refusing had a cost the rule was meant to avoid: an annotation that asked for a cloud drew nothing
at all, and "drawing nothing" is as much a mark the file did not describe as a straight border is.

## 2. What is chosen, and what each choice rests on

Every number below is a choice, made once here so that a later round does not re-litigate it; none
is derived from the standard, which states none, and none is fitted to another renderer.

- **A scallop is a semicircle on a chord of the border's path, bulging outward.** "Convex curved
  line segments" that "simulate the appearance of a cloud" is satisfied by the least curve that
  meets its neighbours at a point; the cusps between them are what a drawn cloud has.
- **The cusps sit where the straight border sat.** On a square, circle or free text the line is
  inside the shape by half its width (§12.5.4: "the border shall be drawn completely inside the
  annotation rectangle"), so the cusps sit a radius inside *that* and the arcs reach back out to it:
  the cloud occupies the band the straight line did and the inscribed rectangle or ellipse is the
  cloud's outer reach. A polygon's line straddles its own vertices and so does its cloud.
- **Each edge is divided evenly**, into the whole number of chords nearest the chosen length, so
  that every corner is a cusp; an ellipse and a `/Path` curve are single edges whose cusps are
  spaced by arc length.
- **The radius is two points plus two per unit of `/I`, and never less than the line's width.**
  Table 169 gives `/I` "in the range 0 to 2" with a default of 0 and no unit. `/I 0` is the smallest
  cloud rather than none, because `/S /C` has already asked for one and the table would otherwise
  make the entry's default cancel it. A radius below the stroke's width is a bumpy line rather than
  a curve anybody can see — the first render showed it on a six-point free-text border.
- **The stroke's joins are round.** Two arcs meet at a cusp by reversing direction, the one angle
  §8.4.3.5's miter length has no bound at; the first render showed a spike at every cusp whose arcs'
  tangents did not align exactly. A round join is the line's own half-width at every cusp.
- **`/IC` fills the cloud**, because the cloud is the annotation's rectangle or ellipse under its
  effect and Table 180 fills "the annotation's rectangle or ellipse".
- **A polyline's `/BE` is not read.** Table 181: "(Optional; meaningful only for polygon
  annotations)". Refusing it, as before, named a gap the clause does not state (trap 11).

**The two witnesses are evidence about scale and nothing more.** `AnnotationTypes.pdf`'s producer
drew its `/I 1` cloud with cusps 7.5 apart and its `/I 2` cloud with cusps 16.7 apart, in arcs that
are within a tenth of a point of semicircles; this choice gives 8 and 12. Same order, same shape, not
a match — and principle 5 says which way that inference runs.

## 3. §12.5.6.22's sentence is about the annotation and it binds the pointer

> Watermark annotations shall have no popup window nor other interactive elements.

Nothing read it. A `/Popup` written on a watermark opened a window, a popup annotation whose
`/Parent` is a watermark was listed as one, and `annotation_at` — the one function every press,
hover and §12.6.3 trigger goes through — found a watermark's rectangle like any other. Now
`annotation::interacts` answers `false` for the subtype whatever its `/F` says, and `popup::popup_of`
and `popup::read` answer `None` for the window. Table 171 makes a watermark no markup annotation,
so the window was never one §12.5.6.14 associates with a parent either; the sentence just says it
outright.

## 4. What this does not decide

The printing half of §12.5.6.22 — tiling and n-up, "[i]n situations other than the usual case where
the PDF page size equals the media size" — waits on a print path (RFC 0004) and keeps that row
`partial`. Table 168's `B` and `I` are the same construction as §1 — "[a] simulated embossed
rectangle" is a `shall` behind a silence about two colours — and are not taken here: that is one
more choice, and it is a separate ADR's.
