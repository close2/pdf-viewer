# 2334 — A mark that draws nothing places no element

Status: accepted. The orchestrator's, at batch one hundred and fourteen's merge, 2026-10-26; the number
was assigned to the batch's docs slot and unused there. Clause: ISO 32000-2 §14.8.3.3, whose content
rectangle is what an element's own marks drew, and §10.7.4's last paragraph, under which ADR 2330 made
a glyph contour at a single point draw nothing. No ledger row moves. Context: ADRs 2330, 2242 (a red
gate is judged, re-taken, fixed or asked — never accepted); the accessibility census's two floors on
"elements placed by their own marks" in `crates/viewer-core/tests/accessibility_census.rs`.

## 1. What the merge gate read

The census's floor on elements placed by their own marks read two under its bound — 2 639 against
2 641 over the tracked population, and 211 412 against 211 414 over the whole population. Reverting
round 1748's one change to `pdf-font`'s glyph loading, with everything else of the batch in place,
returned both counts to their floors, so the two elements are that change's. A temporary line naming
each element the census counts, run on both trees, named them: page 1 of `issue17147.pdf`, elements 1
and 2. No element was gained and no other was lost.

## 2. Why the count is the honest one

Those two elements had been placed by marks that were glyph contours collapsed to a single point. Such a
contour fills nothing on any rasteriser — it has no edge — and before ADR 2330 the renderer drew nothing
for it either; what placed the elements was the contour's point, carried as a mark with a position and no
extent. ADR 2330 §3 made a glyph whose contours are all single points the blank glyph, under §10.7.4's
"[s]can conversion of character glyphs may be performed by a different algorithm", and a blank glyph
leaves no mark. §14.8.3.3's content rectangle is what the element's content drew; a point that draws
nothing drew nothing, so the two elements are not placed by it. Whether they are placed by any other
route (an annotation, a `/BBox`, §14.8.5.4.5's enclosure) is what the census's other counts say, and
none of those moved.

## 3. Decision

The two floors move down by two, to 2 639 and 211 412, with this ADR cited beside each. The move is
the floor following the renderer's reading, not the floor following a loss: a screen reader told where
an invisible point lies was told nothing it could use. A future change that places those two elements
by a mark that draws is the ratchet's to raise.

## 4. Cost

Two of the tracked population's elements are, as far as their own marks go, placeless. Looking at
`issue17147.pdf` to say what those two elements are — a space-like glyph, a producer's empty run — is
owed to a round that opens the file; this ADR records the count and its cause and not the elements'
content.
