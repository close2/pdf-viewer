# The shipped backend's image filter mixes a transparent sample's colour into its neighbours

Status: **owed upstream**, asked for in `doc/QUORRA_FEEDBACK.md` §39. Nothing in this tree can fix
it: premultiplying the bytes we upload would have quorra's shader multiply by alpha a second time.
Priority: 50 — blocked on a dependency, and it is the *shipped* rasteriser, so a reader sees it.
Corpus: no page names it, and none can — see "Why no gate here can see it" below.
Clauses: §8.9.6.2's interpolation `shall` is where the standard states the rule; §8.9.6.3,
§8.9.6.4, §11.6.5.2 and §7.4.9's opacity channel are the other populations it reaches.
Code: `crates/quorra-gpu/src/shaders/image.wgsl` (`fs_main`), reached through
`crates/render-quorra/src/scene.rs`'s `Self::image`. This side's instrument is
`crates/render-quorra/examples/filtered_edge_colour.rs`.

## The rule

ISO 32000-2 §8.9.6.2, and it is a `shall`:

> If image interpolation (see 8.9.5.3, "Image interpolation") is requested during stencil masking,
> the effect shall be to smooth the edges of the mask, not to interpolate the painted colour
> values.

A stencil decodes to the fill colour where its bits mark and `[0, 0, 0, 0]` where they do not, so
what a filter does with those four components decides which of the clause's two nouns it operates
on. `pdf_render::Image::average_block` states the same rule for the reduction this crate performs
itself: "[a]veraging straight-alpha components directly would let a transparent sample's colour —
which is carried but means nothing — into the answer."

## What it costs

`cargo run --release -p render-quorra --example filtered_edge_colour`, one magnified stencil over
`Medium::NONE`:

```text
   cpu: 160 partly covered pixels, worst departure from the painted colour 0 at x=0 ([0, 0, 0, 0])
 vello: 160 partly covered pixels, worst departure from the painted colour 0 at x=0 ([0, 0, 0, 0])
quorra: 160 partly covered pixels, worst departure from the painted colour 131 at x=75 ([124, 0, 0, 125])
```

131 of 255 on the painted channel. `Image::is_smoothed` turns the filter on for every **reduced**
image as well as for every `/Interpolate`, so the population is not exotic: a scanned page drawn
through a stencil at ordinary zoom is in it, and so is every image carrying an `/SMask`, every image
under an explicit mask, every colour-key range and every JPEG 2000 opacity channel — all of them
arrive as straight-alpha RGBA whose cleared samples are stored black.

## Why no gate here can see it

Every image scene in `render-gpu`'s and `render-quorra`'s cross-backend suites is **opaque**, and on
an opaque raster straight and premultiplied filtering are the same arithmetic. No tolerance could
have been tightened into finding this. The corpus cannot either: a darkened edge is a fraction of a
pixel wide against every reference, and the oracle's four bounds are not a hue at partial coverage.

What exists now is `render-gpu/tests/headless_gpu.rs::cpu_and_gpu_smooth_a_stencils_edges_without_darkening_its_colour`,
which holds the two backends that meet the clause, and the example above, which prints all three.

## What closing it looks like

Either of the two in `doc/QUORRA_FEEDBACK.md` §39.3 — premultiply on upload, which changes
`ImageSpec`'s contract, or four `textureLoad`s and the bilinear weights by hand in `fs_main`. When
one lands: re-run the example, expect a departure of 0 on all three lines, add the scene to
`render-quorra/tests/headless_quorra.rs` beside the one in `headless_gpu.rs`, and take the
paragraph off §8.9.6 and §8.9.6.2's ledger rows.

ADR 0697 has the reading and how the claim came to stand.
