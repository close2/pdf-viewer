# 0707 — A zoom is a view change, not new ink

Date: 2026-08-27. Status: accepted, and built.

## The report, and what the trace said

The project owner: *"i have tried the current pdf-viewer and find zooming laggy"*, with a
trace. The document was the ISO 32000-2 specification itself, whose pages carry a
`NoZoom` annotation (§12.5.3), and the trace held the mechanism whole: **234 refusals of
the form** `no reprojection (impossible): another page` **during zoom gestures on one
page**, each leaving the window frozen at the old magnification for the 45–83 ms the real
frame took. No stand-in, no retained page, no movement — then a jump. That is what lag is.

## Two defects, one conflation

§12.5.3 makes a `NoZoom` annotation's placement a function of the magnification, so a
zoom of such a page must re-interpret it — that part is right and stays. What was wrong
is that the re-interpretation went through `Open::stale`, which does two further things,
both of them the *ink's*:

1. **It superseded the ink.** `stale` exists for §8.11's layer switch, §12.7.5's field
   value, §12.5.5's pointer appearance — changes to what the document's state *is*, after
   which a sharp picture of the old state asserts something false and may not be shown.
   A magnification change is none of those: the same layers are on, the same values are
   in the fields. A picture of the old magnification is a picture of this page's ink,
   approximately placed — which is the definition of a stand-in, not a counterfeit.
   With the ink bumped, the host's reprojection *correctly* refused every held picture
   as "of another ink", and the retained low-resolution pages were evicted by the same
   comparison. Nothing was left to show.

2. **It made the refusal structural even where the ink comparison would have passed**,
   because `one_placement` paired the settled arrangement's pages with the asked ones by
   `Arc::ptr_eq` on the display list. A re-interpretation is a new `Arc` over the same
   ink; pairing by address is exactly the defect ADR 0457 removed from the retained
   pages, still present at the other gate.

## The decision

- **`Open::reinterpret`**: the magnification path drops the interpretations (and the
  readbacks beside them, which are invalidated together everywhere else) **without
  touching the ink**. Every other `stale` caller is a genuine ink change and keeps
  superseding it.
- **`one_placement` pairs by `Picture`** — document, page, ink — the identity ADR 0457
  built for precisely this question. What the address test actually guarded, a picture
  of superseded ink standing in, still refuses: another ink is another `Picture`.

The exactness gates are untouched: `depicts` still compares by address and target,
because the base layer's "this rendering *is* the view asked for" is a claim about one
interpretation, and that is the question the `Arc` answers.

## What this does not fix, named

The re-interpretation itself still runs synchronously in the zoom event — 10–19 ms per
wheel tick for two ISO-spec pages, visible in the trace as the event's own duration. It
is bounded by ADR 0256's interpretation cost, it only touches documents with
view-dependent pages, and with stand-ins restored it no longer decides what the person
sees. Moving interpretation off the event thread is a real change to `viewer-core`'s
threading contract and is deliberately not smuggled into this fix.

## Held by

`viewer-core/tests/headless.rs::a_no_zoom_annotation_is_the_one_thing_a_zoom_re_interprets`
now also asserts the two requests carry **the same ink**; the stale module's
`a_reinterpretation_of_the_same_picture_is_still_this_page` holds both directions of the
pairing — a fresh `Arc` of the same picture stands in, the same `Arc` of a superseded
ink refuses.
