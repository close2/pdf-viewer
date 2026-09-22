# 1201 — Three rasterisers, one of them shipped, and what the library underneath the oracle is still for

Session 1182. Status: **accepted** (a record, and one decision inside it).
Supersedes the *record* of [ADR 0002](0002-cpu-rasteriser-first.md) — CPU backend first behind one
`Rasterizer` trait, `tiny-skia` chosen for maturity, page one on the processor while the device
warms, "[n]either backend is privileged". ADR 0002 stays exactly as written; what is superseded is
the description, not the history.
Builds: ADR 1082 (the coverage integral), ADR 0476, ADR 0226, ADR 0583 (the exact forms it left),
ADR 0697 (the third backend), ADR 0125 (the fallback), ADR 0139 (the strips), ADR 0047 (the blend
modes written twice on purpose), ADR 0219 (the residue), ADR 1102, ADR 1107.
Clauses: ISO 32000-2 §10.7.4, §10.7.1, §8.5.3.3, §11.6.2.

## 1. The three claims, tested against the tree rather than against ADR 0002

`doc/adr_revisit/0002-cpu-rasteriser-first.md` is an argument. Each of its three claims was checked
before anything was written here, and all three hold.

- **Page one is not the processor's.** `CLAUDE.md` principle 2 states it outright — page one goes to
  the graphics device, and "[t]he CPU backend keeps its other two jobs and loses this one" — so the
  sentence that chose `tiny-skia` partly for being on the startup critical path has no antecedent.
  The tree agrees with the principle rather than with the ADR: `viewer-ui` depends on
  `render-raster` and `render-cpu` and on no third backend, and its window's own module comment says
  the display list is drawn onto the surface with `render-raster`.
- **The oracle's scan conversion is in this tree.** `render-cpu/src/area.rs` computes a path's
  coverage as the integral of §8.5.3.3's winding number over §10.7.4's half-open pixel square
  (ADR 1082), and `scan.rs` routes to it from both entry points a mark can arrive at — `intersected`
  for the surface and `mask_fill` for a mask. The library's supersampled converter is what draws
  the cases that module declines, named in section 2.
- **There are three rasterisers and the shipped path is one of them.** `render-cpu`, `render-gpu`
  and `render-raster` (ADR 0697), and it is `render-raster` — over the document renderer this
  project commissioned — that a person's frames go through.

## 2. What `tiny-skia` is still for, enumerated

The dependency is `render-cpu`'s alone; no other crate in the workspace names it. Six modules use
it, and what they use it for is four things, none of which is deciding a coverage:

1. **Geometry and its construction.** `tiny_skia::Path`, `PathBuilder`, `PathSegment`, `Rect`,
   `Point`, `Transform` are the representation every other module here is written against, including
   `area.rs` itself, which reads the library's path and writes coverage of its own.
2. **The stroker and the dasher.** `tiny_skia::PathStroker`, `Stroke`, `StrokeDash`, `LineCap`,
   `LineJoin`, and `Path::stroke`/`Path::dash`, called one step earlier than `stroke_path` would
   call them so that §8.4.3's stroked outline can be *filled* by this crate and so meet §8.5.4's
   clip as a set. The outline is the library's; the coverage of it is not.
3. **Paint and the shaders.** `Paint`, `Shader`, `LinearGradient`, `RadialGradient`, `GradientStop`,
   `SpreadMode`, `Pattern`, `PixmapPaint`, `FilterQuality` — the colour a covered pixel gets, which
   is a different question from how much of it is covered. §8.7.4's shading types 1–7 are
   `shading.rs`'s above them.
4. **The surface and the blitter.** `Pixmap`, `PixmapMut`, `PixmapRef`, `Mask`,
   `PremultipliedColorU`, `BlendMode` and the raster pipeline that composites a coverage buffer
   onto a surface. §11.3.5.3's four non-separable modes are `blend.rs`'s, written here rather than
   shared, for ADR 0047's reason.

The calls that still enter the library's **scan converter** are a short list, and each is a named
residue rather than the general case. `scan::fill`'s and `scan::stroke`'s ordinary paths are taken
where `carries_coverage_as_alpha` is false — anti-aliasing already withdrawn, or §11.4.6's knockout
stating `Source`, where scaling a premultiplied source by a coverage and interpolating a blend by it
are not the same function. `fill_rect` is ADR 0476's exact rectangle and the interior run that keeps
it affordable. And the `fill_path` at the end of `mask_fill` is reached only when `area::region`
declines: a path past its cell budget, a transform that states no bounds, or geometry outside
`SUPERSAMPLED_LIMIT`, where anti-aliasing has already been withdrawn because the library's own 16.16
arithmetic would walk off its run buffer (ADR 0269).

**Decision: the dependency stays, and the boundary is where it should be.** The revisit note asks
whether the oracle should own its compositing too. It should not, on the oracle's own terms: what
makes `render-cpu` an oracle is that it answers §10.7.4's question — how much of this pixel the
shape covers — from the clause rather than from a lattice, and that answer is now this crate's for
every mark that is not one of the residues above. A blitter and a raster pipeline decide no clause;
writing a second one would add a few thousand lines that no requirement is measured against, and it
would remove the one property the library still buys, which is that §11.3.5.3's separable modes and
the premultiplied arithmetic under them are somebody else's code that this tree's four hand-written
modes can be compared with.

## 3. What the other two are, and which one presents

- **`render-raster`** draws through quorra (`raster/crates/raster`, briefed by
  `doc/RENDER_LIBRARY.md`) and is **what the window presents with**, page one included. It is the
  shipped path.
- **`render-gpu`** is Vello on wgpu, headless by construction: a *comparison* backend, held against
  the oracle over `test-scenes`' fixtures and over real pages at a real window's resolution
  (ADR 0127). It has ADR 0002's second-backend role minus the presenting.
- **`render-cpu`** is the correctness oracle and the fallback for a frame the graphics device
  refuses (ADR 0125), which is `CLAUDE.md` principle 2's "other two jobs" named.

## 4. What "neither backend is privileged" means with three of them

ADR 0002's sentence meant that no backend's library model was native to the project, so neither
translation was the real one and a difference between them was a defect rather than a reading. That
property is unchanged by there being three and by one of them shipping, and it is worth saying which
half of it moved.

**Privileged in the display list: still none.** All three consume the same `pdf_render::DisplayList`
and none of their scene models reaches back into it; a difference between any two is a backend
defect, because the document was interpreted once. That is the property the cross-backend comparison
rests on and it is three-way now rather than two-way.

**Privileged as an answer: `render-cpu`, deliberately.** A three-way disagreement needs a tie-break
that is not a vote, and the oracle is it — not because it is the processor's, but because its
coverage is computed from §10.7.4 and §8.5.3.3 and the other two take theirs from a library. Vote is
what principle 5 forbids with other renderers, and it would be no better between our own.

**Privileged on the screen: `render-raster`, by the owner's choice, and it is the corner that has to
be watched.** The shipped path is the one whose defects a person sees and the only one no comparison
can call the odd one out by disagreeing with the other two — a defect shared with the oracle would
look like agreement. `render-raster/tests/corpus.rs` holds it against the oracle's raster over the
whole corpus at the page's own scale and at four times it, which is the instrument that exists
because of exactly this.

## 5. What this does not close

The cross-backend claim for `Stroke::device_width` is still architectural rather than measured on
`render-raster` and `render-gpu` — ADR 1189 section 6's phase ladder on the other two backends
remains unrun, and the ladder this session put under a gate is `render-cpu`'s.
