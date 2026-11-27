# 3790 — Annex Q's three routes: a `gs` font's glyphs, a form a mask's group drew, a cell's own state

Session 2272. Status: **accepted**. Context: ADR 2994, ADR 3411, ADR 3629, review 2271 findings B01,
B02, B03; ISO 32000-2 Annex Q.2, Q.3, Q.5, §8.4.5 Table 57, §8.7.3.1.

## Context

Review 2271 found three routes of `pdf-archive`'s survey that Annex Q's method does not take as it
reads: a Type 3 font that a graphics state's `/Font` selects was never given to Q.5; a form a soft
mask's group drew was marked visited for the page, so the page's own later `Do` of it was skipped
while the mask's findings were taken back (ADR 2994), and the page read opaque; and a tiling cell
was judged in the state in force at `scn`, where §8.7.3.1's step b) "\[i]nstalls the graphics state
that was in effect at the beginning of the pattern's parent content stream".

## Decision

- **B01**: Table 57's `/Font` is used for rendering like a `Tf` (ADR 3411), so the `gs` route calls
  the same glyph-procedure walk (Q.5: "each object in its CharProc array shall be processed as a form
  XObject") and notes the glyphs shown, as the `Tf` route does.
- **B02**: a visited set is a route. The set is keyed by page, object and `Within` — the page's own
  walk or the one inside a soft mask's group — which the state carries into every stream a group
  invokes. A stream reached twice on one route is still read once.
- **B03**: each stream's walk keeps the state it began in, and a tiling cell starts from it: Annex
  Q.2's four conditions, overprint, colour spaces and text state with the rest. What is not graphics
  state stays the caller's: the route, the group's blending space, and colour operators ignored by an
  enclosing uncoloured cell or a `d1`.

## Consequences

- Each has a survey test that fails without the change; 2271's three audit tests pass.
- A page whose only transparency is inside a form its mask also draws, or inside a `gs`-selected
  Type 3 glyph, is now found transparent and given its blending space; a page whose cell was marked
  only by a `ca` restored before the fill is no longer.
