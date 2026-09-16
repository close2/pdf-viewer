# ADR 1125 — §11.7.5.2's transfer function is chosen by shape, and applied after compositing

Status: accepted, 2026-09-16. Session 1118. Settles the design question `doc/todo/13` says a round
taking §11.7.5.2's last shape inherits — how the clause's "nonzero object shape value" decides the
transfer function at a partly covered pixel — and records the construction that follows from it,
which is deferred rather than started for the reasons below. No pixel moves in this session: the
decision is what a later round builds, and `render-cpu/tests/transfer_edge.rs` is the fixture it
inherits.

## The question

§11.7.5.2 chooses the transfer function at a point by the topmost object covering it, and defines
that object by shape:

> The topmost object at any point shall be defined to be the topmost elementary object in the
> entire page stack that has a nonzero object shape value ( f j) at that point (that is, for which
> the point is inside the object).

*Nonzero*, not one. A pixel an antialiased edge covers partially has a nonzero shape there, so it
is inside that object and takes that object's function — although its colour is a blend of that
object's colour with the backdrop's. The question was whether the edge pixel is inside the object
for the purpose of choosing the function; the clause's own words answer it, and the answer is yes.

## What follows, and where it runs

§11.7.5.3's NOTE says when the chosen function runs:

> This differs from the current halftone and transfer function, whose values are used only when
> all colour compositing has been completed and rasterization is being performed.

So the clause composites the raw colours first and maps the finished pixel once. At an edge pixel
the object encloses, that is `transfer(blend(object, backdrop))`. This tree applies §10.5's
transfer to an object's colour *before* compositing (`pdf-model`'s `fill_paint`, `stroke_paint`
and the image sample map, ADR from the 358th session; a shading's own colours in `shading` and
`mesh`, ADR 0479), so at the same pixel it draws `blend(transfer(object), backdrop)`. The two
agree wherever the object fully covers the pixel — the interior, where the composite *is* the
object's colour — and diverge at the edge by as much as the transfer bends the colour.

The construction that closes the gap is the one `doc/todo/13`'s third piece derives, and this ADR
adopts it unchanged: a per-pixel *transfer identity* rasterised beside the colour — each fully
opaque mark writing its own function's index where its shape is nonzero, each mark that is not
fully opaque writing the page default's — and one pass over the finished raster mapping each pixel
through the function its index names. Trap 2 binds: the channel's meaning is `pdf-render`'s to
state, or two backends answer it apart.

## Why it is deferred rather than built

Three measurements, each taken this session:

- **The population is one document.** `examples/transfer_function_census` over the corpus: 13 state
  a Table 57 `/TR` or `/TR2`, exactly one states anything but `/Identity` or `/Default`
  (`issue6931_reduced.pdf`), and it draws its one image fully opaque with no translucent mark over
  it and no shading. So the change has no oracle witness, which is what makes it a `doc/todo` entry
  and not a defect — the deferral's honesty is a measurement, and it decays the moment a document
  turns up painting a translucent mark over a transferred opaque one.
- **Both backends composite through `tiny-skia`.** `render-cpu` and `render-raster` both hand marks
  to `tiny-skia`'s pipeline, which exposes no per-pixel "which object last wrote me with nonzero
  shape" hook. The index channel is therefore a *second* rasterisation pass over every mark —
  re-running clip, group and mask geometry to write indices — not a field added to an existing
  pass. That is the "matching pass in all three backends" `doc/todo/13` prices, and it is large.
- **It must cost nothing on the 973 documents that state no transfer.** The channel is gated on a
  display list holding any transfer at all; where none does, no second pass and no final map run.
  The gate is cheap but the machinery it guards is not, and building the machinery is the work.

Starting that half-way — the channel in `render-cpu` alone — would make the oracle and
`render-raster` disagree at every transferred edge, regressing their agreement on the one corpus
witness. `CLAUDE.md` principle 1 governs: a change this size is not started until it can be
finished across both backends, so this session records the decision and plants the fixture instead.

## The fixture

`render-cpu/tests/transfer_edge.rs` builds an opaque grey fill with an antialiased edge over white,
under an inverting transfer, and measures three pixels: the interior, where the pipeline and the
clause agree because coverage is one; the edge, where the pipeline draws `blend(transfer(object),
backdrop)` and the clause asks for `transfer(blend(object, backdrop))`, a measured gap of half a
unit at a half-covered edge; and a control with no transfer, unchanged. It is the regression
witness the build owes (trap 8, trap 13): when the channel lands, the edge's asserted value becomes
the clause's, in one place.
