# 1195 — A redaction cuts a painted path to the region's complement

Status: accepted. Session 1179.
Context: `crates/pdf-transform/src/redact/paths.rs` (the whole module),
`crates/pdf-transform/src/redact.rs` (`PathObject`, `begin_subpath`, `extend_subpath`,
`curve_subpath`, `add_rectangle`, `close_subpath`, `open_path`, `paint_path`, `is_path_operator`,
`mapping`), `crates/pdf-transform/src/lib.rs` (`Origin::Redacted { paths }`),
`crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1124 (the removal and its refusal set), ADR 1126 (the image samples).
Clauses: ISO 32000-2 §12.5.6.23, §8.2, §8.5.2.1, §8.5.2.2, §8.5.3.1, §8.5.3.2, §8.5.4, §8.3.4,
§7.3.3.

## 1. The clause makes it a removal, so a clip is not an answer

§12.5.6.23 requires a processor applying a redaction to "remove all content identified by the
redaction annotation, as well as the annotation itself", and to "remove all traces of the specified
content". The sentence after it says what that excludes for an image — "clipping or image masks
shall not be used to hide that data" — and the reason is not about images: a clipped mark is a mark
whose description is still in the file. A painted path's description *is* its coordinates, so the
only removal that removes anything is one that changes them.

ADR 1124 and ADR 1126 refused every painted path meeting the region, on the reading that "a vector
mark is not removable the way text is, because a glyph's advance box is the unit removed and a path
has no such per-region unit without subtraction". That reading holds; what it left open was the
subtraction, and this round does it. Deleting the whole painting operator stays wrong for the
reason those ADRs give — it removes marks outside the region, which is content the annotation did
not identify.

## 2. Nine cells, eight of them the complement

The region is an axis-aligned box in the display list's space. Its four edge lines cut the plane
into a three-by-three grid of slabs; the middle cell **is** the region and the other eight tile its
complement with pairwise disjoint interiors. Each of the eight is an intersection of at most four
half-planes, so each is convex, and clipping a polygon to a convex window is Sutherland–Hodgman:
one pass per half-plane, exact, and orientation-preserving.

`P \ R` is therefore the union of eight clips, taken subpath by subpath. That is what makes it
correct for **both** fill rules: each subpath is clipped on its own, so an even-odd hole is still a
hole; the cells do not overlap, so no interior is wound twice; and the degenerate edges the
algorithm lays along a window boundary enclose no area, which neither rule can see. It is a *fill's*
construction, which is why §8.5.3.2's stroke is refused rather than cut — a stroke's marks are the
outline of the path, and splitting a path introduces caps and joins the producer never wrote.

The cut is taken in the path's **own user space**, with the half-plane test evaluated on the
display-space image of each point. A vertex that survives is the producer's own pair, copied; only
a vertex the cut creates is arithmetic, interpolated along the source edge. So the geometry outside
the region is the file's own numbers, and a path the region misses keeps its bytes untouched.

## 3. The margin is checked, not assumed

A created vertex is written back as decimal text and read back by a processor whose reals are
single precision (§7.3.3). Both roundings can move a cut edge, and moving it *into* the region
would leave a sliver of the removed marks alive — a redaction that looks applied and is not. So the
region is widened by a hundredth of a point before the cut, which over-removes by an invisible
amount in the safe direction, and `Cut::margin_holds` refuses the page unless the worst
displacement the two roundings can produce — half an ulp of the largest created coordinate plus
half a unit in the sixth decimal place, carried into display space by the mapping's norm — is
strictly smaller than that widening. A margin nobody has checked is not a margin (trap 38).

## 4. What is refused, each by name

A stroke (§8.5.3.2); a path carrying a §8.5.2.2 Bézier segment, because the crossing parameter is a
root this build does not solve and flattening would approximate the producer's geometry; a path that
is also §8.5.4's clipping boundary, because cutting its geometry would move the boundary every mark
after the painting operator is held to — content the annotation did not identify; a path object
§8.2 says should hold only construction operators and which another operator interrupted, so the
byte range the cut would replace is not the path's alone; a singular transform (§8.3.4); a cut
whose surviving pieces exceed this build's bound; and a cut whose margin does not hold. Over-refusal
leaks nothing, which is the direction trap 5 asks for.

## 5. What measured it

The construction's own unit tests hold the areas: a corner bite, a band across the middle, a path
wholly inside, a path clear of the region, a ring's two subpaths, and the same path under a scaling
map cut where the region is rather than where the numbers are. End to end,
`the_cut_page_is_pixel_identical_outside_the_region_and_empty_inside_it` rasterises before and
after at 150 dpi: nothing is drawn inside the region, every pixel outside it is identical, a
one-pixel band at the boundary is skipped because the widening is deliberate — and the original
marks the region, so the comparison could have failed.

**No corpus document exercises it.** The nine documents on this disk carrying a `/Redact` are all
veraPDF and Isartor fixtures, and not one has a painted path, a form or an image under its region:
the verb applied all nine before this round and applies all nine after it. The instrument was
calibrated rather than believed — a planted fixture with a stroke under the region is named by the
same command (trap 13) — so the zero is a fact about the corpus and not about the census.
