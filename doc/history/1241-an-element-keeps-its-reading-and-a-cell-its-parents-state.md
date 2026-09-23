# 1241 — An element keeps its reading, and a cell its parent's state

Date: 2026-09-23. Branch `batch-1239-1244`, shared worktree. ADRs 1319, 1320.

## Built

- ADR 1319: a scope painting under both readings of `/AIS` seals what it painted so far
  (`Command::Shaped`, stated under the element's own reading) where it turns, so every knockout
  construction reads each element under its own. The seals come off at `finished` wherever the
  element did not become a knockout group's. A statement decides nothing until something the flag
  reinterprets is painted; a stream's end is a restore. Found on the way: a form ending under a
  different `/AIS` left the record reading the rest of the scope under it.
- ADR 1320: a tiling cell starts from its parent stream's state (§8.7.3.1 b), with §11.6.7's three
  resets. §9.3.8 counts a glyph translucent through its cell, and a tiling inside a knockout-capable
  scope is one element. `render-cpu`'s bare knockout draw under a cutting clip was Source scaled by
  the mask, which cleared the accumulation; it is two stages now.

## Rows

§11.4.6, §11.6.4.3, §11.7.4.4 partial → implemented; aggregates §11.6.4, §11.6, §11.7.4, §11.7
follow. §11.4 stays partial for §11.4.3 and §11.4.7. §8.7.3.1 and §9.3.8 kept implemented.

## Gates

`raster_golden` moved nine pages, looked at and regenerated: seven raster-only at clip edges (ADR
1320 section 4), `issue12798_page1_reduced` list-only (a group now its own shape), and
`ContentStreamNoCycleType3insideType3` (the cell strokes in the state its glyph began with).
`render-raster --test corpus` lists that page as differing: raster specks along the cell seams.
The oracle contradicted it; the clause decides it — §9.6.4: "Aside from the CTM, the graphics
state shall be inherited from the graphics state at the point of invocation of the text-showing
operator that caused the glyph description to be invoked." With §9.3.1's text state being
graphics state, the cell starts in mode 2 with the red stroke. Held by name in the oracle's
`CONTRADICTED_PATTERN_CELL_STATE`; the four references draw four different letters.

## Left

`render-gpu` and `render-raster` were not checked for the clipped-Source defect. An element whose
shape cannot be stated keeps the `Mixed` report; no route reaches one.
