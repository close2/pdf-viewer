# 1302 — An image pixel is coloured by the sample its centre maps into, on the device too

Status: accepted and **built**. Session 1232.
Answers: the one-pixel image shift `doc/history/1225-*.md` left open — `render-raster` up to 139
levels from the oracle on an image drawn from a half-pixel origin, opaque images included.
Depends on: ADR 0025, ADR 0702 / raster's ADR 0089, ADR 1107 section 3, ADR 1287.
Context: `raster/crates/raster-gpu/src/raster/reduce.rs` (`smoothed`),
`raster/crates/raster-gpu/src/encode/rare.rs` (`texel_transform`),
`raster/crates/raster-gpu/src/device/rare.rs`, `raster/crates/raster-gpu/src/shaders/image.wgsl`,
`crates/render-raster/tests/masked_image_edge.rs`.
Clauses: ISO 32000-2 §10.7.4, §8.9.4, §8.9.5.3.

`§N` is ISO 32000-2 and nothing else.

## Which side was right

§10.7.4 decides a sampled image's pixel by one point: "The position of the centre of such a pixel
-in other words, the point whose coordinate values have fractional parts of one-half -shall be mapped
back into source space to determine how to colour the pixel. There shall not be averaging over the
pixel area." §8.9.4 gives source space one unit per sample: "The upper-left corner of the first
sample is at coordinates (0, 0)". The sample read is the one whose square holds the point — its
floor, the convention §10.7.4 states for pixels ("[a] pixel is a square region identified by the
location of its corner with minimum horizontal and vertical coordinates"), applied to samples, whose
shared edge §8.9.4 does not assign. That is a documented choice where the standard is silent, and it
matters exactly where a pixel centre lands on the edge: an image's origin on a half pixel.

Held against that closed form, `render-cpu` is right in every interior pixel. The device was wrong,
twice: **the CPU needed no change.**

1. **Its copy of the filter rule was stale.** `pdf_render::Image::is_smoothed` answers "point
   sample" at one device pixel per sample (ADR 1107 section 3); raster's `reduce::smoothed`, which
   ADR 0702 makes a copy of it, never took that branch. At a half-pixel phase the linear filter laid
   the mean of four samples on every pixel — a whole image shifted by half a sample, 128 levels on
   the fixture — and the reduced grid of an exact integer reduction took the same path, which is
   `doc/QUORRA_FEEDBACK.md` sections 47 and 48 from the other side.
2. **Its nearest lookup broke ties by float error.** The shader multiplied the unit square's inverse
   — whose `1 ⁄ w` no float holds — back up by the texture's size, landing a hair below the whole
   number on some rows and reading the neighbour: up to 239 levels on a magnified image. The op now
   carries the device → texel transform, composed in `f64` and rounded once, so a native
   placement's coefficients are exactly ±1 and a half pixel and every centre maps to a whole number
   exactly.

## Consequences

- `masked_image_edge.rs` holds an opaque image against the closed form at one sample per pixel,
  magnified 1.5 times, and at scale 1.05, and an exact 2:1 reduction against the oracle; each
  failed with its fix planted away.
- The uniform keeps its size: the six floats that were the unit square's inverse are the texel map.
