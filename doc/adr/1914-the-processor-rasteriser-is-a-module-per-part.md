# 1914 — The processor rasteriser is a module per part, and nothing it draws moved

Session 1539, on the project owner's word of 2026-10-14 ("switch to code improvements/refactoring
rounds"). Status: **accepted** and **built** in `render-cpu`. Context: ADRs 0139, 0219, 1023, 1841,
1882, 1896. No clause is read anew and no ledger row changes status.

## 1. What it was

`crates/render-cpu/src/lib.rs` was 7 544 lines: the public type and its `Rasterizer` implementation,
then the strip planner, the command walk, every kernel, the groups, the soft masks, the mask cache,
the compositing choices, the surface geometry and 900 lines of unit tests, in the order they were
written. A reader looking for how a stroke is drawn met the planner first and the mask cache after.

## 2. Decision

**One module per part, named by what it is**, each opening with a comment that says what it is and
which ADRs argued it:

| module | what moved into it |
|---|---|
| `lib.rs` (605) | `CpuRasterizer`, its builders, `Rasterizer for CpuRasterizer`, `CpuRasterError`, `HIGH_PRECISION_PIPELINE`, and a map of the modules |
| `strips.rs` | `MAX_STRIPS` (re-exported), `MIN_STRIP_ROWS`, `MAX_REPLAY`, `PIXEL_PASS_ROWS`, `spot_planes`, `encode_in_strips[_watched]`, `plan_strips`, `strips_in`, `whole_target`, `drawn_row_by_row`, `converted_row_by_row` |
| `encode.rs` | `encode`, `encode_command`, `encode_group_command`, `encode_shaped`, `draw` (the dispatch), `coverage_twin`, `marks_reach` |
| `fill.rs`, `stroke.rs`, `image.rs` | the three kernels: `draw_fill`, `filled_as_a_raster`, `rectangular_mark`; `draw_stroke`, `zero_length_dashes`, `SolidOutline`, `draw_stroked_outline`, `draw_long_mitres`, `outlined_whole`; `ImagePlacement`, `draw_image` |
| `stroke/folding.rs`, `stroke/sub_pixel_rule.rs` | the folding stroke's pieces (`folding_stroke_outline` to `distance_to_triangle`); the sub-pixel rule's two constructions |
| `groups.rs`, `soft_mask.rs`, `mask_cache.rs` | `Group`, `draw_group` and its backdrop steps; `build_soft_mask`, `marked_rows`, `mask_values`, the consumers' reach; `MaskCache` and its keys, `crop_to_mask`, `room_for_marks`, `state_regions` |
| `compose.rs`, `surface.rs` | `Compose`, `carries_coverage_as_alpha`, `premultiply`; `Band`, `Surface`, `ToDevice`, `band_pixels`, `misses_surface` |

`images.rs` is `reduced_images.rs`, after the type it holds, so that it does not sit beside the image
kernel under one letter's difference. `thin_segment`, `scan`, `area`, `shading`, `blend`, `convert`
and `glimpse` were already modules and keep their contents; their paths into the moved items changed.

**Visibility is what a sibling reads.** Each moved item is private unless another module of the
crate names it, which a trial compile with `pub(crate)` removed decided item by item (81 kept, the
rest private). **The unit tests went to the module they test**, so an item only its tests read
stays private. The crate comment's third job was a retired sentence — page one on the processor
while the device starts — and states the current one instead (`CLAUDE.md` principle 2, ADR 1831).

## 3. The proof that nothing moved

The split was exported alone onto HEAD `512af7e5` (`scratchpad/r1539/split`) and walked in one hold:
`raster_golden` held 974 pages, moved 0; its division test drew 967 first pages at 2, 4, 8 and 16
strips, 176 moving at some division by at most 1 308 pixels and 3 levels, the figures ADR 1882
recorded; `drawn_while_read` passed. The two processor arms against `/home/AI/arms-1534/`, by page:
`cpu-1x` 968 pages and `cpu-4x` 965, **0 digests and 0 mean errors differ**. The crate's own 204
tests pass, and the count of `#[test]` in the crate is HEAD's, 207.
