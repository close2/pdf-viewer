# 1491 — A clip's residue is one byte whichever region asks, and a region is priced by its edges

Status: accepted. Session 1328. Answers ADR 1479 section 3, which named two levers: "ADR 0049's
admission rule pricing a tile's edges as well as its bytes" and "the exact meet counting a
pixel's left winding from whichever side holds fewer partial edges". Amends raster's ADR 0049:
its rule, and its finding that a tile is the crop of its region only "to within the 1-of-255
residual". Supersedes nothing.
Context: ISO 32000-2 §8.5.3.3, §8.5.4, §10.7.4; raster ADRs 0049, 0079, 0080; ADRs 1389, 1467,
1479; habits 52, 53, 56; traps 73, 92; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/fill/real.rs` (new: `Real`), `raster/fill.rs`
(`clip_mask`, the generic `fill_over` and deposits), `raster/fill/exact.rs` (generic), `encode/clips.rs`
(`row_pieces`, `intersect_links`), `encode/residue.rs` (`Fill`, `PIECE_BYTES`,
`ResidueRegions::admit`), `raster/meet.rs` (`sort_row`); `raster/crates/raster-pages/src/page.rs`
(two recorded rows).
Tests: `raster/tests/fill.rs`'s `a_clip_tile_is_the_crop_of_its_region_and_the_area_rounded`
(new; the `f32` arm differs, the `f64` arm differs on no pixel of 2 863 228); `encode/residue.rs`'s
`a_region_whose_tiles_pay_its_edges_is_kept` (new); `raster/meet.rs`'s
`the_winding_into_a_pixel_is_counted_from_the_side_with_fewer_partial_edges` (new; watched failing
with the right count's sign left positive).

**`crates/render-cpu/` was not opened.**

## 1. Why admitting the region moved bytes (habit 53 first)

A probe compared every per-tile residue fill on `bug1721218_reduced.pdf` (one `zoom_frame` pair,
185 196 pixel asks) with the region crop at the same pixel. It also compared both with the set's
area from `area_in_pixel` and with an independent `f64` integral of the winding; those two agree.
Three pixels differed. Their areas are 65.500037, 135.500024 and 59.500014 levels. The crop missed
the rounded area at all three; the tile missed it at two. Two different tiles over pixel
`(452, 582)` gave 65 and 66. **Neither construction is wrong by geometry.** Each pixel lies within
1.5·10⁻⁷ of a rounding boundary, and `f32` sums cannot decide which side it falls on. Only the
order of the sums differs (ADR 0049). Moving only the prefix sum to `f64` changed nothing; with
the accumulator in `f64`, one pixel still differed, from deposits taken in `f32`. The complex
pixels' correction (`exact.rs`, ADR 1389) is the remaining `f32` step: the 40-curve probe's last
differing pixel came from it.

**The fix: a clip's residue is filled in `f64` from the same `f32` points**, through `clip_mask`.
The generic code is the `f32` code with its type named, and that includes `exact.rs`. A mark
keeps `f32`, because its bytes are the compute lane's (ADR 0080), and WGSL has no `f64`. After the
change, all 185 196 asks equal the rounded area, and the 40-curve probe differs on no pixel. On
the page there is one byte per pixel, whichever region asks. So admission moves nothing, and the
rule is a question of cost alone.

## 2. The rule, priced

A fill costs its bytes plus the row pieces of the edges in its rows. Measured on `clip_mask`: a
rectangle filled over a 1000 × 1000 region costs 6.4–6.6 ns a byte. A 100 000-gon over a 210 × 210
region costs 103–143 ns a row piece once its bytes are subtracted. That is 16 to 22 bytes per
piece, and `PIECE_BYTES = 16` takes the lower figure so the rule leans towards declining. The
rule admits a region when `region + 16·pieces ≤ uses × (tile + 16·pieces·tile_rows/region_rows)`.
`row_pieces` counts the pieces in one pass over the chain's points. On `bug1721218`, the
111 677-edge chain was declined at 24 840 bytes against 3 025 × 4. It is now admitted. Archetypes
(the counters are exact): dense text goes from 0 regions / 40 tiles to 2 / 0, and artwork from
67 / 380 to 164 / 68. Texels and tiles are unchanged. `examples/residue_clip`'s steady minimum
improves by 5–9 % in three alternating pairs at load 13. A page-sized clip over forty small
marks is still declined (`a_region_costlier_than_its_tiles_is_not_kept`).

## 3. The meet's winding from the fewer side

Along a horizontal line, the crossings of closed subpaths sum to zero. So the winding entering a
pixel equals minus the crossings through it and to its right. `sort_row` counts from whichever
side holds fewer partial edges. Windings are integers, so this moves nothing (corpus: 0 pages).

## 4. Measured (callgrind, one `zoom_frame` pair at one thread; wall: minima of five rounds, load 8–10)

| tree | instructions | 1× frame | 1.25× frame |
|---|---:|---:|---:|
| HEAD | 18.47 G | 593 ms | 514 ms |
| `f64` residue | 18.25 G | 681 ms | 489 ms |
| + the priced rule (and ADR 1492) | 11.19 G | 362 ms | 337 ms |
| + the winding from the fewer side | 9.51 G | 301 ms | 274 ms |

Residue filling goes from 8.37 G to 1.13 G, and `area_in_pixel` from 4.95 G to 3.26 G. The CPU
backend draws the page in 84 ms. What is left is the exact meet (34 % of the instructions) and
the page's interpretation.

## 5. Corpus

`render-raster --test corpus`, per-page digests, exported trees with their own target
directories, all against HEAD. Verdicts are unchanged on every run. At 1×: CPU and compute
966 agree / 0 differ / 1 refused, GPU 965 / 1 / 1. At 4×: CPU and compute 962 / 0 / 4, GPU
961 / 0 / 5. One-versus-many is 0 on all sixteen runs.

| step | 1× pages moved | 4× pages moved |
|---|---|---|
| `f64` residue (section 1) | 0 | 8, every one equal to the oracle to four places |
| + the priced rule and ADR 1492 | 4, the oblique-image pages (ADR 1492) | 4: three of the same, and `images.pdf`, which has oblique images |
| + the fewer side (section 3) | 0 | 0 |

The 4× `f64` pages are `bug1703683_page2_reduced`, `image-rotated-black-white-ratio`,
`issue13520`, `issue17147`, `issue6961`, `pattern_text_embedded_font`, `personwithdog` and
`text_clip_cff_cid`. Each moves by the rounding section 1 describes, toward the rounded area,
which the new fill test holds by construction. The priced rule moves no page by itself: every
page that moved between the `f64` tree and the full one has an oblique image (counted by a
probe in `encode_image`, removed). `bug1721218_reduced.pdf`'s digest is the same in all four
trees. Its time in the 1× walk is 1 138 ms on HEAD, 377 ms with this change and 328 ms with the
fewer side, at differing load.
