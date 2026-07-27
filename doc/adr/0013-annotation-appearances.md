# ADR 0013 — Annotations are drawn from their appearance streams, and never synthesised

Status: accepted, 2026-07-27.

## Context

Clause 12 is 166 subclauses and none of it was implemented. Read as "interactive features"
that reads like a coherent decision: a viewer that does not fill in forms does not need field
values, calculation order, ECMAScript or navigation.

But *drawing* an annotation is not interactivity, and the oracle measured what leaving it out
cost. 148 of 988 corpus first pages carry a visible annotation with an `/AP`, and the gate
contradicted 47 pages that carry one: form fields, highlights, stamps and ink strokes simply
missing from pages that reported nothing wrong. It was the single largest group in the
contradicted list, and the only one whose cause was known before anyone looked at an
artefact.

The reason it was cheap to fix is the reason it was worth separating from the rest of clause
12: **an appearance stream is a form `XObject`**, and the interpreter has run those since
long before annotations existed.

## The decision

A pass over the page's `/Annots` runs after the content stream, and feeds the existing form
machinery. `annotation.rs` decides selection and placement; `content.rs` does the drawing.
The split is not cosmetic — placement is pure arithmetic over dictionaries and is unit-tested
directly, while drawing needs an interpreter and is tested through rendered pixels.

Four parts carry it.

### Visibility comes from Table 167, and its conditions are honoured

`Hidden` (bit 2) and `NoView` (bit 6) mean nothing is drawn. `Invisible` (bit 1) is
conditional in the specification's own words — it "applies only to annotations which do not
belong to one of the standard annotation types and for which no annotation handler is
available" — so it is applied only to subtypes outside Table 171, which is why the list of 28
standard subtypes exists in the source at all. A `Popup` is the window belonging to some
other annotation and is never part of the page.

None of these reaches `unsupported`. An annotation the document asked to hide is not
something we failed to draw, and reporting it would make the corpus counts mean less rather
than more.

### Placement is §12.5.5's algorithm, including the step a square fixture cannot check

The clause has three steps: transform `/BBox` by `/Matrix` and take the smallest upright
rectangle around the resulting quadrilateral; compute `A` mapping that box's corners onto
`/Rect`'s; concatenate to `AA = Matrix × A`.

Step 1 is the one worth stating. It is the *transformed* box that gets measured, so a
`/Matrix` that rotates makes the box larger than the `/BBox` it came from, and the scale onto
`/Rect` must account for it. A reader that measures the untransformed `/BBox` is correct for
every axis-aligned matrix — which is nearly all of them — and pushes half the appearance
outside the annotation for the rest. Both the unit test and the rendered-pixel test use a
rotation and a non-square rectangle, because a square one cannot tell the two axes' scales
apart. This is the same lesson as trap 2 in the handover: a test that cannot fail in the
dimension a defect moves is not a test of it.

### The content is clipped to its `/BBox`

§8.10.2 makes the bounding box the clip for any form `XObject`, and §12.5.5 depends on it:
the entire algorithm is about making that box cover `/Rect`, so content drawn outside the box
lands outside the annotation. A fixture whose appearance fills the whole page in its own
coordinates and must still mark only its 20×20 rectangle pins this.

### Nothing is synthesised, and the absence is reported

An annotation with no appearance stream is reported rather than drawn from `/IC`, `/C`,
`/BS`, `/Border` and its subtype's rules. That is a different drawing routine per subtype —
a cloudy border effect, a rubber stamp's artwork, a line's ending styles — and a guess would
put marks on the page that the document never described. It is also the largest remaining
piece of this area: 64 corpus documents have one.

Two cases are *not* reported, and both are read from the document rather than assumed:

- A `Link` without `/AP` draws its border and nothing else (§12.5.6.5). §12.5.4 puts the
  width in `/BS /W`, falling back to `/Border`'s third element, defaulting to 1. A width of
  zero means there was never anything to draw.
- An `/AS` naming a state the appearance dictionary omits is "displaying nothing", which
  §12.5.5 names as the reasonable behaviour. This is how every unchecked check box with only
  an `On` appearance is written, and an early version of this work conflated it with a real
  gap — the test caught it.

## `/NeedAppearances` is reported, and the stored appearance is still drawn

§12.7.4.3 says a field whose value is not known until viewing time "cannot provide a
statically defined appearance stream" and that "the PDF processor shall construct an
appearance stream dynamically at rendering time". The `/NeedAppearances` flag on the
`/AcroForm` is the writer saying that applies.

Constructing one needs the field value, `/DA`, quadding and a text layout — form work this
crate does not do. So the stored appearance is drawn, because it is the only thing the file
offers and it is usually close, and the fact that the document says it is not the one to draw
is said out loud. `text_field_own_canvas_calc.pdf` is why this exists: its text field is
computed by an ECMAScript calculate action, poppler and mupdf regenerate the appearance and
show an empty field, and we drew the stale grey placeholder while claiming to be complete.

Drawing *and* reporting is unusual in this tree — normally a report means nothing was drawn.
It is right here because the two statements are different: "this is the best the file offers"
and "the document says it is stale" are both true, and suppressing either loses information.

## Consequences

The oracle, over 1794 pages: contradicted pages we call complete fall from 166 to 120, and
`CONTRADICTED_ANNOTATIONS` loses 45 of its 47 entries.

The two that stayed are the interesting part, and the previous handover predicted exactly
this: "if it does not, the remaining entries were never about annotations, and that is worth
knowing too." They were not. Both are small pages with a fractional page box, where our
raster is one pixel smaller than poppler's and mupdf's and exactly the same size as
ghostscript's. On a 72-row page a one-row shift moves everything, and the structural bound
sees a page-wide change. Nothing in ISO 32000-2 says how a fractional page becomes an integer
number of pixels. The list is now `CONTRADICTED_PAGE_ROUNDING` and holds four such pages.

The corpus's incomplete count rises from 291 to 368, entirely as new reporting, and this is
the first time that count has risen *because a feature landed*. The handover's rule still
covers it — a rise that is a new report is the tree becoming more honest — but the shape is
worth noting: implementing something can increase what you are able to notice you are
missing. Before this, an appearance-less annotation was indistinguishable from no annotation
at all, because neither was drawn.

Three things this deliberately does not do, so the next person is not surprised:

- **No transparency group.** §12.5.5 says an appearance without a `/Group` entry "shall be
  treated as a non-isolated, non-knockout transparency group". Transparency groups are
  unimplemented generally (the largest rendering gap in the tree), and annotations do not
  change that.
- **No `NoZoom` or `NoRotate`.** Both make the appearance's size or orientation depend on the
  view rather than on the page, which the display list has no way to express — it is
  resolution-independent by construction. They are rare, and they belong with whatever
  eventually carries view-dependent content.
- **No interactivity.** Field values, calculation order, actions and navigation are still
  entirely absent, and none of them is needed to draw.
