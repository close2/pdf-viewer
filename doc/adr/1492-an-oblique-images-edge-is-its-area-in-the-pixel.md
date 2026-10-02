# 1492 — An oblique image's edge is its area in the pixel

Status: accepted. Session 1328. Amends raster's ADR 0011 (an oblique placement was hard-edged by
pixel centre, "the stated cost of the rare-rare case") and ADR 1480 section 1 (an oblique image
met its residue by `min`). Supersedes nothing.
Context: ISO 32000-2 §8.3.3, §8.9.4, §8.9.5, §10.7.4, §11.6.4.2; the §10.7.4 ledger row's
departure (1); ADRs 0805, 1435, 1467, 1480, 1491; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/shaders/image.wgsl` (`oblique_area`, `keep_side`, the oblique
branch of `shape_at`), `encode/rare.rs` (`Encoder::image_residue` takes the parallelogram),
`encode/meet.rs` (`Encoder::meet_residue_with_shape` in place of `meet_residue_with_rectangle`,
`clip_to_rect`, `polygon_overlap`, `rectangle_overlap`).
Tests: `raster-gpu/tests/an_image_meets_its_residue_as_a_set.rs`'s
`an_oblique_images_edge_is_its_area_and_meets_a_polygon_clip_as_a_set` (new; watched failing
twice — with HEAD's shader, pixel `(5, 2)` drew 0 against 11.63 levels, and with the oblique meet
switched off, pixel `(6, 2)` drew 55 against 33.38); `tests/m7.rs`'s oblique test, comment only.

**`crates/render-cpu/` was not opened.** Every expected value is a polygon's area in a pixel.

## 1. The reading

§10.7.4's image paragraph says "only those pixels whose centres lie within the region shall be
painted". Raster's oblique lane did exactly that, while its axis-preserving lane painted each
pixel's overlap with the image's rectangle. The tree's reading of that paragraph is not the
literal one: the §10.7.4 row's departure (1) is "a partly covered pixel is partly painted", and
ADR 0805 reads the image paragraph's edge as departure (1) in the paragraph's own words. The CPU
backend paints an image's edge that way, by the row's note; that file was not read here. So one
renderer drew an image's edge two ways, depending on the placement. The row decides it: an
oblique image's edge pixel is painted by the parallelogram's area in it, as an axis-preserving
image's already was. The row's departure stands as written. Nothing here is a decision about the
row.

## 2. The construction

The shader already carries the device-to-texel transform (ADR 1302's single rounding). The pixel,
cut first by the clip rectangle, is mapped into texel space. Its corners are taken as offsets from
the pixel centre's texel point, so the arithmetic works on quantities the size of a pixel rather
than on page coordinates. The resulting parallelogram is cut by the image's box `[0, w] × [0, h]`,
at most eight corners. Its shoelace area is then divided by the transform's determinant, which
gives the area in device space. The colour is still sampled at the centre's texel, as for an
axis-preserving edge. Under a residue clip, the encoder replaces the tile's byte in each pixel
that both sets cut. The new value is the area of the parallelogram met with the clip rectangle and
the chain, from ADR 1467's edges. That is the same rule ADR 1480 states for the rectangle, now
reached through one function for both placements.

## 3. Corpus

`render-raster --test corpus` with per-page digests, from exported trees each with its own target
directory (ADR 1491 section 5 has the table). Pages with an oblique image were counted by a probe
in `encode_image` (removed). Four pages move at 1× on all three lanes, the same four:
`image-rotated-black-white-ratio.pdf`, `issue17147.pdf`, `bug946506.pdf` and `issue19360.pdf`.
Each one has an oblique image and was already in the agree count. At 4×, three of them move, and
`images.pdf` with them; it has oblique images too. Against the CPU backend's bytes, every page moved
closer or stayed equal: `issue17147.pdf` went 0.0321 → 0.0000 at 1× and 0.0057 → 0.0000 at 4×,
`bug946506.pdf` went 0.0928 → 0.0831 on the CPU lane and 0.1199 → 0.1101 on the GPU lane at 1×,
and the rest are equal to four places. Verdicts are unchanged. The gate tolerated the difference
before, because these pages were within its bound. With this change the two lanes now draw an
image's edge one way.

**For the ledger round (the §10.7.4 row, which this round does not touch).** The row's departure
(1) is the tree's reading for an image's edge, and both of raster's placements now follow it. The
row's `code` list names raster's `encode/fill.rs` and `encode/replay.rs` but not the image lane;
`raster/crates/raster-gpu/src/shaders/image.wgsl` and `encode/rare.rs` are where departure (1) is
carried out for an image.
