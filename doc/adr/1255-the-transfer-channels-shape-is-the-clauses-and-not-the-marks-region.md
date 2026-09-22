# 1255 — §11.7.5.2's channel reads §11.6.4.2's shape, not the mark's region

Status: accepted. Session 1209.
Amends: ADR 1125's last section, whose `shape_of` painted every shape solid white.
Depends on: ADR 1148 (the channel), ADR 0479 (a shading's colours are sampled under the function),
ADR 0430 (a tiling cell is interpreted once), ADR 1022 (`SampleAlpha` is a field of the raster),
ADR 1218 (the stencil and its own soft mask kept apart).
Context: `crates/pdf-render/src/transfer_channel.rs`,
`crates/pdf-model/tests/transfer_functions.rs`, `crates/pdf-model/src/content/pattern.rs`.
Clauses: ISO 32000-2 §11.7.5.2, §11.6.4.2, §11.6.5.2, §10.5.

`§N` is ISO 32000-2 and nothing else.

## 1. The quantity the channel is choosing by

> The topmost object at any point shall be defined to be the topmost elementary object in the
> entire page stack that has a nonzero object shape value ( f j) at that point (that is, for which
> the point is inside the object).

`resolve_transfers` walks the runs from the top and takes the first whose coverage at a pixel is
nonzero, so the *shape* it rasterises is the whole of what decides which function a pixel takes.
`shape_of` built that shape by taking the mark's geometry and painting it **solid white**, which is
the mark's region rather than §11.6.4.2's shape, and the clause states them apart for two kinds of
object.

> For objects painted with the sh operator (8.7.4.2, "Shading operator"), the shape shall be 1.0
> inside and 0.0 outside the bounds of the shading's painti ng geometry, disregarding the
> Background entry in the shading dictionary (see 8.7.4.3, "Shading dictionaries").

An axial ramp that does not extend, a mesh's triangles and a sampled grid's cover each leave part
of the path they were painted through unmarked, and a shape of solid white claimed all of it. The
consequence is a *wrong function*, not a wrong colour: such a mark occluded pixels it never
painted, so an older fully opaque mark's function stopped reaching them and they took the page's
default instead.

> For images (8.9, "Images"), the shape shall be 1.0 inside the image rectangle and 0.0 outside it.
> This may be further modified by an explicit or colour key mask (8.9.6.3, "Explicit masking" and
> 8.9.6.4, "Colour key masking").

An `/SMask` is in neither of the two that sentence admits — §11.6.5.2 makes it opacity — so an
image carrying one covers its whole rectangle, and `shape_of` handed the channel the image's own
samples, whose alpha *is* that mask. The error runs the other way there: the transparent part of
such an image stopped occluding, and an older function reached a pixel the clause hands the
default.

## 2. What it reads now

The same reading `pdf_model::transparency::shape_without_the_mask_and_the_constants` already
states for §11.4.6's knockout shape, moved to the one place the channel's rule is written (trap 2):
a shading's shape is `Shading::opaque`, every other paint's is its path; an image's is the unit
square where `SampleAlpha` says opacity, its own samples where it says shape, and its producer's
separated shape where the raster carries both. `SampleAlpha::Both` with no separable shape is the
one residue, and it is the same pair ADR 1218 left — there the product under-states the shape, and
neither answer is the clause's.

Two fixtures in `tests/transfer_functions.rs` hold each against a value derived from the clause,
and each fails on the shape it replaces (trap 13). The first paints a white square under an
inverting transfer and covers the page with a shading pattern whose axis stops half way and does
not extend: the left half is the shading's green at the page's default and the right half is the
square, inverted to black. The second covers the same square with a two-sample image whose soft
mask empties its right half: both halves are inside the rectangle, so neither is inverted.

## 3. The two paints that keep §11.7.5.2 `partial`, and what closing the first now costs

`Interpreter::mark_transfer` puts §10.5's function **inside the colour** for two marks rather than
on the mark: a shading pattern's (and `sh`'s), and every mark inside a tiling cell.

- **The shading.** ADR 0479 put the function where a shading's colours are made because mapping a
  simplified ramp's two stops draws the chord between the transferred ends rather than the curve.
  That argument is about mapping *the ramp*; it does not reach the channel, which maps the finished
  pixel once, where §11.7.5.3's NOTE puts it — "only when all colour compositing has been
  completed and rasterization is being performed" — and therefore has no stops to interpolate
  between. What blocked carrying it on the mark was the shape: a channel that occluded the whole
  path could not stand for a shading. §1 removes that, so the change is two hunks in
  `content/pattern.rs` — dropping the `PatternPaint::Shading` arm of `mark_transfer`, and building
  `sh`'s `Colouring` without the transfer while `draw_mark` carries it — and the report
  `Unsupported::TransferFunction` narrows to the tiling cell alone. **Designed here and not built**:
  `content/pattern.rs` was not this session's to write.
- **The tiling cell.** ADR 0430 interprets a cell once and copies it to every site, so a cell's
  marks reach the channel only through `occlude_tiling`, which walks the finished tiling and pushes
  every leaf as an occluder carrying no function. Carrying each leaf's own function would mean
  recording it per cell mark and replaying it per site through `pdf_render::Cell`, which is a
  change to the replication rather than to the channel. It stays.

The population either would move is still zero: `examples/transfer_function_census` finds one
document in `doc/pdf.js` stating a function that is not `/Identity` or `/Default`, and it paints one
fully opaque image.
