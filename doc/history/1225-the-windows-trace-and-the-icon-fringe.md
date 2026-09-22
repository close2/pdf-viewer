# 1225 — The Windows trace, and the icons whose mask leaked its colour

Kind: measure (the owner's `tmp/win-prob/trace.txt`) and one fix (the owner's icon screenshots).

## The trace
Read whole; the findings are `doc/todo/50`'s new section, with the trace lines copied, so they
are not repeated here. In short: bring-up 110 ms (181.6 last time) is inside what the tree
expects; ~107 ms between `graphics device` and the surface configure coincides with the 109 ms
pipeline compile, in a function whose comment says it cannot wait — priced, owed on that machine;
interpretation is ~730 ms per page turn and 63% of the launch, a fixed cost per interpretation
that a synthetic flat 21 907-page file does not reproduce (12 ms for page 2) — owed the file;
the three resizes were one render; 350.5/268.4 are sums over four frames, not maxima; the 18 MB
frame is encoded scene at ~230 MB/s, not the 58 k-call atlas pathology.

## The icons
Reproduced headlessly, then fixed. The image lane filtered straight-alpha texels with the
hardware sampler and premultiplied afterwards, so the colour under a soft mask's zeros leaked
along its edges — black in every area-averaged variant (`average_block` returns zero for a
transparent cell), which is why it appeared only where a device pixel gathers two samples or
more and vanished on a slight zoom in. A JPEG icon with an `/SMask` at a sweep of zooms: up to 65
levels from the oracle before, within 2 after, at every scale. ADR 1287.

Files: `raster/crates/raster-gpu/src/device/textures.rs`, `.../device/resident.rs`,
`.../shaders/image.wgsl`; `crates/render-raster/tests/masked_image_edge.rs` (new; fails at 59
levels without the fix); `crates/render-raster/tests/corpus.rs` (`22060_A1_01_Plans.pdf` left
`DIFFERS_IN_SHAPE` — its 15–19% heaviness under reduction was this defect); `doc/todo/50`;
`doc/adr/1287`.

## Left open
- `render-raster` differs from the oracle by up to 139 levels on an image drawn at exactly one
  reduced sample per pixel on a half-pixel origin (scale 3.75 in the sweep, opaque images too):
  a one-pixel phase difference, pre-existing, unrelated to the owner's report.
- The owner's own file would confirm the icons are this mechanism; what to send is in the report.
