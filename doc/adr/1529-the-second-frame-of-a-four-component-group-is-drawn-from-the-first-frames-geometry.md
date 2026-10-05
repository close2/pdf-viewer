# 1529 — The second frame of a four-component group is drawn from the first frame's geometry

Status: accepted. Session 1347. Answers ADR 1517 section 6, which named the lever "draw ADR 1471's
two halves as one encode, or name the black half's outlines by the chromatic half's". Amends ADR
1471 in one respect: the frames of one group state their clips by one set of outlines. Supersedes
nothing.
Context: ISO 32000-2 §8.5.3.3, §8.5.4, §10.7.4, §11.3.3, §11.3.4, §11.3.5, §11.4.5, §11.7.2; ADRs
1467, 1471, 1479, 1491, 1503, 1513, 1517; traps 62, 73, 94; `doc/habits/measuring.md` 52, 56, 59,
63, 65; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/meet/kept.rs` (`KeptRegion`, `KeptMeets::find_tile`,
`keep_tile`, `region`, `keep_region`), `encode/coverage.rs` (`Encoder::coverage_tile`),
`encode/meet.rs` (`Encoder::tile_words`, `mark_words`, `meet_residue`'s `own_words`),
`encode/clips.rs` (`Encoder::residue_intersection`), `encode/residue.rs`
(`ResidueRegions::hold_kept`); `crates/render-raster/src/scene.rs` (`ClipOutline`,
`Encoder::clip_outline`), `scene/own_space.rs` (`Encoder::frame_of`);
`crates/render-raster/examples/zoom_frame.rs` (`ZOOM_FRAME_BACKEND=cpu`).
Tests: `raster/crates/raster-gpu/tests/coverage_kept_for_the_next_render.rs` (new, three cases on
three arms; watched failing with the rule left out of a tile's words, with the chain left out, and
with every region kept under one number).

**`crates/render-cpu/` was not opened.** Its figure below was taken through the `Rasterizer` trait
by `zoom_frame`'s new knob, which names the type and calls `rasterize` and nothing else.

## 1. Why the group is two frames, derived

§11.7.2 puts the elements of a group with a blending space of its own in that space: "all blending
and compositing computations shall be done in that space". For four components the space has four
channels and raster's frame has three and an alpha. Two clauses make that enough:

- §11.3.3's formula is per component, and §11.3.4 excludes the spaces where that is wrong because
  "the compositing computations in such spaces do not give meaningful results when applied
  separately to each component": the spaces it admits are composited a component at a time.
  §11.3.5's separable modes are per component by definition, and `pdf_render::blending`'s module
  comment derives the non-separable four from §11.3.5.3's two CMYK bullets on a neutral pair.
- The resolution is not. `pdf_render::resolve_blending` reads all four components of a pixel at
  once and converts them through a grid (`BlendingSpace::convert`), which is multilinear, not
  linear, so it runs once, after both rasters are composited — where §11.7.2's second sentence puts
  the interpretation.

So one render with a four-channel intermediate would be exact, and raster has no such target. The
black is not a separate blend (no `Multiply` of K after the chromatic): it is the fourth channel of
the same compositing. Two renders of the same geometry therefore stay, and what they can share is
everything that is not a colour: each mark's coverage is a function of its outline, transform and
rule (§8.5.3.3), each clip's of its chain's paths (§8.5.4), their meet of the two sets (§10.7.4).
The colour a mark is painted in is in none of them.

## 2. Where the black frame's time went (callgrind, the 1.25× frame of one `zoom_frame` pair, one thread, GPU lane)

`--dump-before`/`--dump-after` on `frame_of` divides each zoom frame into its two renders. Of the
1.25× frame's 1.59 G instructions in the group, the chromatic render was 1.14 G and the black
0.44 G. In the black: `encode_scene` 0.28 G, of it `coverage_tile` 0.115 G (the fill 0.076 G, the
kept meets' lookups 0.035 G) and `plan_group_residue` 0.091 G (the big clip's region, filled again);
`draw_encoded` 0.114 G, the same as the chromatic's; this crate's walk 0.048 G, of it the clips'
regions built and uploaded again 0.038 G. Every outline the black render uploaded was a clip's: a
fill's path is cached by its `Arc` across frames, a clip's region is uploaded per frame.

## 3. Kept

**A whole coverage tile for the next render (`KeptMeets::find_tile`).** `raster::fill_mask` makes a
tile from its place and extent, rule and polylines; the meet makes the rest from those bytes and
the chain. So `coverage_tile` keys the finished tile by the tile, the rule, the polylines and the
chain's number (ADR 1517's), word for word, under the same 1 024-word cap and budget as the meets.
A hit is charged and drains the queue as a miss would; a tile whose meet a budget decided is not
kept. The walk's own tiles skip the meet's own words, which a tile miss cannot hit.

**A chain's region for the next render (`KeptMeets::region`).** The region is a function of the
chain's content, whose words end with the frame's visible rectangle, so it is kept under the
chain's number with the price its admission was asked at. The next render asks the same admission
of `ResidueRegions::admit` at that price and, admitted, is handed the region; declined, it fills per
tile as before. A handed region is not counted in `clip_residue_regions`, which counts what a frame
rasterised.

**One set of clip outlines for the frames of one target (`ClipOutline`).** A clip's region depends
on the clip and the target only, so `render-raster` keeps each uploaded region by the list's clip
id for the frame, and `frame_of` hands the map to each frame of its own and takes it back.

**Byte-identical, and the condition.** Every hit hands back what the miss wrote for the same words.
Skipping a meet changes which chain's edges reach the edge budget first, which ADR 1517's kept meets
already do; a meet the budget decides is not kept, and the corpus below is the evidence that no
page reaches that budget under the change.

## 4. Measured

| black render, 1.25× frame, one thread | before | after |
|---|---:|---:|
| the render (`frame_of`) | 0.443 G | 0.249 G |
| `encode_scene` | 0.281 G | 0.111 G |
| `coverage_tile` | 0.115 G | 0.037 G |
| `plan_group_residue` | 0.091 G | 0.000 G |
| this crate's clips (`clip_chain`) | 0.038 G | 0.013 G |
| `draw_encoded` | 0.114 G | 0.115 G |

The chromatic render moves from 1.142 to 1.148 G (keys built and tiles kept). The pair's two
groups: 1.25× 1.585 → 1.397 G, 1× 1.846 → 1.669 G.

**As latency**, pinned to the eight fastest cores (by `cpuinfo_max_freq`), minima of 3 × 5
interleaved, exported HEAD and HEAD-plus-change trees in their own target directories, the HEAD
binary `md5sum`-equal to the one the round's first measurement used, load 2.9–3.4:

| `bug1721218_reduced.pdf` | before | after | the CPU backend |
|---|---:|---:|---:|
| `zoom_frame`, GPU lane, the 1× frame | 153.1 ms | 145.4 ms | 47.2 ms |
| `zoom_frame`, GPU lane, the 1.25× step | 148.2 ms | 136.2 ms | 56.3 ms |
| `frame_budget` turn (CPU lane, 1 600 × 1 000) | 222.1 ms | 214.0 ms | — |
| `frame_budget` step (compute lane, 2×) | 144.7 ms | 132.3 ms | — |

**The CPU backend's figure, re-taken.** ADR 1471's 84 ms was never re-taken. The same instrument on
the same targets reads 47.2 ms at 1× and 56.3 ms at the step, pinned. Unpinned with four rayon
threads it reads 74.2 and 90.0, which is about where the old figure sat. So the step is 2.4× the
CPU backend, not inside 2× as ADR 1517 section 6 said against the stale figure.

**Corpus.** `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×, exported
trees, per-page digests (`PDFVIEWER_RASTER_TIMES`) compared by name: **0 pages moved on any of the
six arms**; the four pages of ADR 1471 among them. Verdicts equal HEAD's: at 1× CPU and compute
967 / 0 / 0, GPU 966 / 1 / 0; at 4× CPU and compute 963 / 0 / 3, GPU 962 / 0 / 4. One-versus-many
is empty on every run.

## 5. What is left

The black render is now 0.25 G, of which `draw_encoded` is 0.115 G: bind groups built and passes
recorded for 3 518 clipped shadings (`create_bind_group` 30 M, `write_buffer` 20 M), the same in
both renders, and raster's. The chromatic render's encode is 0.99 G, of it the exact meet 0.80 G,
which is the page's own cost and not this construction's: it is what stands between the step and
twice the CPU backend. On the CPU and compute lanes a mark is filled by the fan-out, whose tiles
this memo does not reach; the meets (ADR 1517), the regions and the clips do, and the
`frame_budget` rows above are those lanes. A paint-free encode replayed with the black half's
colours would also save the black render's walk, about 13 ms of its 31; raster's encode writes
colours into its instances as it walks, so that is a change to its encode, not to this memo.
