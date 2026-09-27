# 1334 — A stencil through a pattern under a soft mask is one mask drawn through the other

Status: accepted and **built**.
Context: `crates/pdf-model/src/content/image.rs` (`Interpreter::stencil_through_a_pattern`,
`stencil_mask`); `crates/pdf-model/tests/image_masks.rs`,
`crates/render-raster/tests/stencil_through_a_pattern.rs`.
Supersedes: the refusal ADR 0697 left standing ("two masks on one command").

§8.9.6.2 paints a stencil's places "with the current colour", which §8.7.2 lets be a pattern, and
ADR 0151/0169 draw that as the pattern's fill through the stencil made a §11.5.2 alpha soft mask. A
command has one mask slot, and under a graphics-state soft mask that slot was already taken, so the
case was refused. §11.6.4.3 displaces the state's mask only by "[e]ither form of mask in the image
dictionary", and a stencil is neither, so both masks are owed.

**The composition.** §11.6.4 splits a mark's alpha into shape and opacity, and §11.3.5 composites with
their product. Here the stencil is the shape, the state's mask the opacity, the pattern the colour.
The product is itself an alpha mask: the stencil's raster drawn *through* the state's mask. The
display list states that as a soft mask whose one `Image` command names the state's mask, which was
registered earlier, so every backend meets it in the order it evaluates a mask nested in a mask's
group. No new display-list vocabulary. The knockout group's question — what is this element's shape
(§11.6.4.2) — still has the stencil alone as its answer, recorded apart as ADR 1301 does for a
stencil's own `/SMask`. A stencil whose dictionary carries a mask of its own displaces the state's
and takes the one-mask route. The tiling route hands the product to `Interpreter::tile` as the
group's mask exactly as it handed the stencil.

**The backends.** `render-cpu`, `vello` and `render-raster` each evaluate a mask whose group holds a
masked command, so none refuses. The fixture is a stencil in red under a luminosity mask of grey 0.5
over white, whose painted cell §11.3.5 gives by hand as (255, 127.5, 127.5). All three draw it within
two levels, through both a shading and a tiling pattern. Calibrated by dropping the inner mask, which
draws (255, 0, 0) on all three.
