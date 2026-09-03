# ADR 0825 — A rectangle of no area stops only the marks it places: §12.5.5's scale onto no extent is a stored stream's arithmetic, and six subtypes state their geometry elsewhere

Status: accepted. Session 890.
Clauses: ISO 32000-2 §12.5.2 Table 166 (`/Rect`, and the `/AP` row's licence to omit an
appearance dictionary), §12.5.5 (the placement algorithm), §12.5.6.10 Table 182 (`/QuadPoints`),
§12.5.6.7 Table 178 (`/L`), §12.5.6.9 Table 180 (`/Vertices`), §12.5.6.13 Table 186 (`/InkList`),
§12.5.6.6 Table 177 (`/CL`), §12.5.6.5 Table 176 (a *link's* `/QuadPoints`, which is the contrast
the argument turns on), §12.5.6.8 (the square "inscribed within the annotation rectangle").
Code: `crates/pdf-model/src/appearance.rs` (`bounded_by_rect`, `Constructed::bounded`),
`crates/pdf-model/src/annotation.rs` (`decide`'s `Normal::Absent` arm and its retyped-`FreeText`
arm).
Tests: `crates/pdf-model/tests/annotations.rs::a_text_markups_quadrilaterals_are_drawn_where_its_rect_covers_no_area`,
`::a_line_annotation_is_drawn_where_its_rect_covers_no_area`,
`::a_construction_that_rect_places_draws_nothing_where_rect_covers_no_area`;
`doc/checks/fixed-documents.toml`'s row for `batch5/sumatrapdf/sumatrapdf-LINK-1618-0.pdf`.
Measurement: `crates/pdf-model/examples/point_rectangle_census.rs`.
Documents: §12.5.2's, §12.5.5's and §12.5.6.10's ledger rows, `doc/todo/03` §44.

## Context

`batch5/sumatrapdf` (320 documents, the SumatraPDF issue tracker) surveyed at 17 incomplete, and
the ink ranking of those against `pdftoppm -cropbox` and `mutool draw` had two rows within a
hundredth of each other at the head. The first, `sumatrapdf-378-0.pdf` — ours 0 against 3.298 and
3.310, on a `/FontFile2` whose Flate data is `Corrupt` after 409 275 bytes — is **ADR 0459's
decision unchanged and was held by name**: `Corrupt` is the input violating the filter's grammar
at a definite point, and Table 125's stated extent is asked *and* the damage must be a truncation
before a program is read. Two more documents in the same tracker (`-854-0`, `-1505-4`) are that
same shape.

The second is the one this ADR is about. `sumatrapdf-LINK-1618-0.pdf` is `pdfcomment.sty`'s own
demonstration sheet — ours **4.19** against `poppler` 7.48 and `mupdf` 7.93 — and reading the page
rather than the report is what found it (trap 1). Both references draw a line with an arrowhead, a
strikeout, six underlines, two squiggly underlines and two highlights that this tree drew nowhere
**and reported nowhere**.

Every one of those thirteen annotations has the same two properties:

- **no `/AP` at all**, and
- **a `/Rect` written as a point** — `/Rect [ 100.2 732.197 100.2 732.197 ]` and twelve more like
  it — while its own geometry is stated whole: `/QuadPoints [ 100.20018 741.7022 162.38202
  741.7022 100.20018 732.1947 162.38202 732.1947 ]` for the strikeout, `/L [ 100 680 250 680 ]`
  for the line, `/Vertices [ 150 120 260 120 300 150 400 80 ]` for the polyline.

`annotation::decide` dropped all thirteen in one line, before `crate::appearance` was reached:

```rust
let rect = match anchored_icon(&subtype, rect) {
    Some(square) => square,
    None if is_empty(rect) => return Decision::Nothing,
    None => rect,
};
```

## The clause

**The rule that line implements is §12.5.5's, and §12.5.5 is about a stored stream.** Its
algorithm transforms the appearance's `/BBox` by its `/Matrix`, takes the axis-aligned box around
the result, and computes the matrix that maps that box onto `/Rect`; a `/Rect` of no extent makes
that a scale by zero, so nothing a stored stream draws can survive it. That reading is right, it
is why `an_appearance_box_covering_no_area_draws_nothing_and_reports_nothing` stands, and the
guard on the *stored* path is untouched.

**A construction goes through none of it.** `crate::appearance` writes its stream in the page's own
default user space, and `annotation::construct` places it with `Transform::IDENTITY` — the module's
own doc comment says so — so there is no box, no map and no scale. What decides whether `/Rect`
bounds such a construction is the subtype's own clause, and `Constructed::bounded` had already been
answering exactly that question since ADR 0193, for exactly this list:

| clause | table | the entry, in the standard's words |
|---|---|---|
| §12.5.6.7 | 178 | `/L` — "specifying the starting and ending coordinates of the line in default user space" |
| §12.5.6.9 | 180 | `/Vertices` — a `Polygon`'s and a `PolyLine`'s, in default user space |
| §12.5.6.10 | 182 | `/QuadPoints` — "the coordinates of n quadrilaterals in default user space" |
| §12.5.6.13 | 186 | `/InkList` — the ink's points, in default user space |
| §12.5.6.6 | 177 | `/CL` — the callout's "starting, knee point, and ending coordinates of the line in default user space" |

So the tree held two answers to one question, and the wrong one ran first: `bounded` said "`/Rect`
does not bound these marks", and forty lines away a `/Rect` of no area erased them.

**The silence is a silence, and that is checkable rather than assumed.** §12.5.6.5's Table 176
states a fallback to `/Rect` in as many words, for a *link's* activation region:

> If this entry is not present, or the PDF processor does not recognise it, or if any coordinates
> in the QuadPoints array lie outside the region specified by Rect then the activation region for
> the link annotation shall be defined by its Rect entry.

§12.5.6.10's Table 182 states the same array — for a *mark* rather than for a region — and gives it
no such sentence. Neither does Table 178, Table 180, Table 186 or Table 177. Where the standard
means the rectangle to win it says so, once, and this is the clause that shows it doing so.

**Table 166's `/AP` row is evidence in the same direction rather than against.** It frees a writer
from supplying an appearance dictionary for

> Annotations where the value of the Rect key consists of an array where the value at index 1 is
> equal to the value at index 3 and the value at index 2 is equal to the value at index 4.

— a point, which is precisely what these thirteen state. The standard therefore *anticipates* a
file with a point `/Rect` and no appearance dictionary; a reader excused from finding a stream for
it is a reader that has to construct one, and constructing a text markup's needs no rectangle.

## Decision

`Constructed::bounded` becomes a predicate, `appearance::bounded_by_rect(subtype)`, so the list
exists once and both questions ask it:

- **the `/BBox` clip**, which is what it already decided — a construction for one of the six gets
  `bbox: None` and is not clipped to a rectangle that does not contain it;
- **the empty-`/Rect` guard**, which is new. `decide`'s `Normal::Absent` arm returns
  `Decision::Nothing` for a rectangle of no area only where `bounded_by_rect` is true; the other
  six go on to `construct`, and draw whatever their own entries state.

The retyped-`FreeText` arm above it loses its own copy of the guard for the same reason and by the
same predicate: `FreeText` is one of the six, Table 177's `/CL` reaches the page whatever `/Rect`
says, and §12.7.4.3's value is clipped to the box it is laid out in rather than to a `/BBox`
(`variable_text::lay_out`, ADR 0193's fifth entry). One question, one answer, in both places.

**What is deliberately unchanged**: the stored-appearance guard, and every construction `/Rect`
*places*. §12.5.6.8's square is "inscribed within the annotation rectangle", an icon is drawn on
the largest square inside it, a border runs along it, a field's text is laid out in it — each of
those inscribes nothing in no area, so returning `Decision::Nothing` for them is right and is what
keeps this from becoming a licence. `a_construction_that_rect_places_draws_nothing_where_rect_covers_no_area`
is that half, and it is the test that *passes* against the old code.

## Consequences

`sumatrapdf-LINK-1618-0.pdf` draws twelve of the thirteen — the line with its `/LE` arrowheads, the
strikeout, the six underlines, the two squigglies and the two highlights — and **names the
thirteenth where it was silent**: the `PolyLine`'s `/BE` is `/S /C`, so §12.5.4's cloudy border
refusal (ADR 0106) now reports it, exactly as the `Square` beside it was already reported. The page
moves from 4.19 to 4.54 by the ranking's instrument, and
from **8.385 to 9.078** by the fixed-documents gate's, between references at 7.48 and 7.93; what is
left between it and them is the cloudy `Square`'s blue interior, three `Text` icons outside
§12.5.6.4's seven names, and three `/DA` fonts the `/DR` does not define — three held decisions and
a stand-in, none of them this defect.

**The population is small and was measured rather than guessed.**
`examples/point_rectangle_census` reads `/Annots` on every page with `pdf_syntax` alone (trap 8)
and counts an annotation whose `/Rect` covers no area, whether it states an `/AP`, and whether it
states the entry its own clause puts in default user space. Over `batch5`'s 6119 files, 6074 open:
**122 documents state such an annotation, 1237 of them state no `/AP` at all, and 57 of those — over
six documents — still state their own geometry.** The other 1180 are `Link` (1009), `Widget` (213),
`Text` (87) and one `RichMedia`: subtypes `/Rect` places, or which draw nothing anyway, so they are
unchanged by this and the census says so.

**Trap 5 is the part worth keeping.** Twelve marks were missing from a page and nothing said so,
because the drop happened before the subtype's clause was consulted and a report is written from
what a clause asks for. A guard that runs in front of the code that knows what is owed cannot
report what it cost — which is the same shape as ADR 0193's own finding one level up, where the
clip did the erasing instead.

**And the general lesson is a duplicated predicate.** `bounded` was the correct reading of these
six clauses, written down, tested, and carried in a struct field — while a second copy of the same
question, spelled `is_empty(rect)`, answered it the other way in a function forty lines off. Neither
was wrong on its own; what was wrong is that there were two. A rule this project derives once
belongs in one named place that every caller asks, and `bounded_by_rect` is that place now.
