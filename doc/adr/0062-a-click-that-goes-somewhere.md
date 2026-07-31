# ADR 0062 — A click that goes somewhere

Status: accepted, 2026-07-31.

## Context

The ledger reached zero unreviewed rows in the fifty-sixth session, and what replaced "read the
next family" as the specification track is its **195 `silent` rows**. Almost all of clause 12's
are the same shape: this program renders pages correctly and does nothing when a person clicks
on one.

§12.5.6.5's first sentence is where that starts. A link annotation "represents either a
hypertext link to a destination elsewhere in the document … or an action to be performed", and
this tree drew its border (ADR 0030) and ignored where it pointed. Everything underneath had
been built in the meantime: §12.3.2's destinations resolve to a page index (ADR 0054), and the
viewer already turns pages.

## Decision

**`pdf_model::link` is the region and the target; `viewer-ui` is the mouse.** The split matters:
everything with a clause behind it is in the model and is tested headlessly, and what is left in
the binary is a cursor position, a scale and a page number.

Three things the clause decides, each in the model:

- **The activation region is `/QuadPoints` where the clause admits them.** Table 176 makes them
  "n quadrilaterals in default user space that comprise the region in which the link should be
  activated" and then states three conditions under which they are ignored. The third is the one
  a lenient reader gets wrong: "if any coordinates in the QuadPoints array lie outside the region
  specified by Rect then the activation region … shall be defined by its Rect entry". A stray
  quadrilateral is not a wider region — it is **no** region, and the rectangle stands.
- **`/Dest` and `/A` are exclusive**, and `/Dest` is read first because it states the same thing
  directly. A go-to action's `/D` is the other route; a URI, a launch or an ECMAScript action
  leads nowhere here, which is not a gap — principle 3's sandbox is why §12.6.4.5 is absent, and
  a URI needs a network this program does not have.
- **Overlapping links resolve to the last one.** The clause states no rule, so the document's own
  order stands and the annotation drawn on top is the one under the cursor.

Point-in-quadrilateral is the crossing-number rule rather than a winding rule, because the
clause's "counterclockwise order" is not what real files write and a winding test would answer
differently for the two orders.

**Mapping a click back to the page is one function in `pdf-model`**, `user_space_at`, which is
the inverse of the transform every page is drawn under. §12.5.2 puts an annotation's `/Rect` "in
default user space units", and §7.7.3.3's `/Rotate` and `/CropBox` are exactly what stand between
that and a pixel — so a viewer that inverted the scale alone would work on every unrotated page
and fail on every rotated one.

## What the corpus says

**54 documents have a link on page one, 33 125 links in all — 32 768 of them in one file**,
`bug1978317.pdf`, which is a stress test for precisely that. The other 53 share 357.

**36 lead to a page of their own document.** The rest are URIs, which is what a web page printed
to PDF produces, and the test asserts the two-sided fact rather than the ratio: a link whose
destination resolves must name a page inside the document, and every link must have a region.

## Consequences

- §12.6.4.2's go-to action is `implemented`: every place the standard puts one — a link's `/A`,
  an outline item's, and the catalog's `/OpenAction` — is read and each turns the page.
- §12.5.6.5 is still `partial`, for `/H`'s highlighting mode: a response to a mouse this program
  does not draw.
- The viewer's `render` grew a second return value, the scale it drew at, because a click has to
  be mapped through exactly the transform the frame on screen was drawn with — not one the
  handler computes for itself from a window size that may have changed since.
