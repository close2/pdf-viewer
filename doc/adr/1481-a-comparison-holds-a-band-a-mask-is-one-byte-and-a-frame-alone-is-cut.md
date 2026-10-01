# 1481 — A comparison holds a band, a soft mask is one byte a sample, a `/Matte`'s frame decodes beside its mask, and a frame decoded alone is cut at the rows a pass finds

Status: accepted. Session 1323. Keeps ADR 1472's tiles, ADR 1433's restart bands, ADR 1457's matte
table and ADR 1469's decode beside the image; takes up the cut ADR 1457 section 3 built and set
aside, with the signal that section named. Supersedes nothing.
Context: ISO 32000-2 §7.4.8, §11.6.5.2 and Tables 143–144; ISO/IEC 10918-1 (held as ITU-T T.81,
`doc/md/T.81.md`; cited by section and paraphrased) sections B.1.1.5, F.1.2.1, F.1.2.3, F.2.1.3.1,
F.2.2; `CLAUDE.md` principles 2 and 3 and the rule on optimisations; `doc/questions/A121`;
`doc/habits/measuring.md` 48, 52, 55, 59, 60; traps 50, 66, 73.
Code: `crates/raster-compare/src/ssim.rs` (`Bands`, `BAND_ROWS`), `crates/raster-compare/src/lib.rs`
(`compare_with_tile`); `crates/pdf-model/src/image.rs` (`MaskPlane`, `mask_plane`, `grey_mask`,
`grey_plane_of_samples`, `grey_plane_of_frame`, `MaskSamples`, `opacity_multiplied_in_place`,
`combine_on_the_finer_grid`, `matte_before_samples`, `FrameBeside`, `dct_frame`, `samples_of_frame`,
`invert_matte_in_place`, `InFlight`, `decode_parts_in`, `in_bands`, `RasterCache::parts`);
`crates/pdf-model/src/image/cut.rs`; `crates/pdf-model/src/image/restart.rs` (`plan_rows`,
`Geometry`, `decode_bands` taking a codestream maker, a buffer per thread);
`crates/render-raster/src/tiles.rs` (its module comment states the budget).
Tests: `ssim`'s `a_banded_map_is_the_whole_map`; `image.rs`'s
`a_grey_mask_plane_is_its_decoded_first_channel`, `a_mask_multiplied_in_place_is_the_combination`,
`a_combined_raster_is_its_sources_nearest_samples`, `the_matte_table_route_unpacks_the_per_sample_routes_pixels`
(now also four bytes a pixel and in place); `cut.rs`'s
`a_frame_cut_where_the_pass_finds_its_rows_is_the_whole_frame` over six `cjpeg` fixtures under
`crates/pdf-model/tests/cut/`, and three of its parts; `raster_golden` held 974, moved 0.

## 1. Where the 7.9 GiB was: the comparison, not the tiles

The brief's premise was that the tiled frame held every tile until the stitch. It does not:
`render_in_tiles` reserves the page raster, then draws, reads back, copies and drops one tile at a
time. Measured on `issue19517.pdf` (12 608 × 16 806 at 1×) with `VmHWM` reset between stages: the
display list 0.54 GB; raster's whole frame **0.99 GB over the device's own** (the 847 MB page and one
tile's readback); the CPU backend's frame 0.93 GB; and `raster_compare::compare` of the two **7.45
GB**. The structural-similarity map held nine `f32` planes of the page at once — two luminances,
five blurred statistics, the blur's intermediate and the map — 847 MB each.

**Built: the map a band at a time.** `ssim::Bands` computes a band of at least 256 rows with the
window's five-row margin above and below it; a horizontal pass reads only its row and the vertical
pass's clamped rows fall inside the margin, so every entry is the same operations in the same order
and the map is the same to the bit (`a_banded_map_is_the_whole_map`, every band height across band
boundaries). `compare_with_tile` asks for each row of tiles. A band of a tile's 32 rows did 31% more
horizontal work and measured 17% slower; at 256 the comparison is faster than before (8.1–8.4 →
7.8 s on this page) and adds 0.1 GB instead of 7.45 — the same `Comparison`, field for field.

**The corpus walk at 1×** (`render-raster --test corpus`, peak by `getrusage`, the tree before and
after in one sitting): **11.31 → 5.15 GiB**, 68.3 → 62.4 s, 966 agree both ways, 1-vs-N at 0. Its
peak is now the three rasters of the largest page — the CPU backend's, raster's, and the one-thread
backend's beside it — and no longer the comparison.

**The budget, stated.** A tiled target costs the host `width × height × 4` bytes plus one tile's
readback; the raster is reserved before the first tile and refused as `Allocation` with its size if
it cannot be held. How large a target may be asked for is the caller's pixel budget
(`TargetSpec::for_page`, 2^28 pixels in the correctness gate). No second ceiling was invented: the
brief's frame-budget ceiling is raster's device-side `max_frame_bytes`, which bounds a tile
(a quarter of it, ADR 1472) and is not a host bound.

## 2. A soft mask is one byte a sample

`issue13931.pdf`'s mask (2 996 × 4 256, `FlateDecode`, `DeviceGray`, `/Matte`) was decoded to an RGBA
raster (`unpack` 147 M instructions), its first byte copied out for the inversion (12 M), and read
four bytes a pixel by the multiplication (83 M). Every consumer reads one number a sample — Table
143 makes the mask `DeviceGray` — so a decoded mask is now `MaskPlane`, one byte a sample, shared
with the inversion by `Arc` where the grids are one. Two direct routes write it without the raster:
raw eight-bit samples through `unpack`'s own `/Decode` table, and a one-component `DCTDecode` frame
asked of `zune-jpeg` as `Luma` (restart bands and `DNL` honoured as `decode_jpeg` honours them). Any
other mask takes `decode` and its first channel, as before. Both routes are held to `decode`'s bytes
(`a_grey_mask_plane_is_its_decoded_first_channel`). Callgrind on `issue13931.pdf`, one thread:
926 → 793 M.

## 3. A `/Matte`'s frame decodes beside its mask, and is inverted where it lies

ADR 1469 left the mask of a `/Matte`'d image decoded before the image, because the inversion reads
it. The clause orders the inversion before the colour conversion — "inversion of the pre-blending
shall precede the colour conversion" — and neither the codec nor the mask's decode reads the other.
So a `DCTDecode` parent of at least `PARALLEL_PIXELS` decodes its frame (`dct_frame`) on its thread
and its mask on the pool, and the inversion runs on the pair after both: on `issue13931.pdf` the mask
was 6 ms spent before the frame's 9.5 began. And the inversion of a frame covering its grid writes
each pixel over the pixel it reads (`invert_matte_in_place`) rather than into a second raster; the
same table entries, `the_matte_table_route_…` holding the three routes to one answer. Callgrind 793 →
627 M, the fixed-size walk over pixels in place halving the table route's 421 M.

## 4. The signal, and the cut

ADR 1457 section 3 built a cut of a frame with no restart interval and set it aside: it took 2–3 ms
off a page of one photograph and added 8–10 to Plans, because the pass is serial and Plans' other
frames were already decoding on the pool. **The signal is `InFlight`**, answered in `RasterCache::parts`: `Alone`
where the page offered nothing to be decoded ahead (ADR 1321's walk found fewer than two candidates,
so it installed no table), the thread is not one of rayon's, and rayon has two threads or more;
every other caller says `AmongOthers`. The third condition was found by callgrind: at one thread
ADR 1321 installs no table, so without it Plans' eight frames were each cut and decoded serially,
2 147 → 2 605 M instructions. A decode beside its own mask counts as alone: one pool task leaves
the rest idle.

**The cut was rebuilt** (`image/cut.rs`; ADR 1457's code was never merged): a Huffman-only pass
records each MCU row's first bit and each component's first block — where its DC code opens and
closes, and the value it decodes to; a band is the header with its lines, the scan header, the
first DC of each component re-coded against a prediction of zero by section F.1.2.1's categories,
the rest of its rows' bits copied, one-bit padding (section F.1.2.3) and `EOI`, every `FF` stuffed
(section B.1.1.5). Declined: a code no table holds, categories past section F.1.2's bounds, a run
past the 63rd coefficient, data ending before the last MCU, any `FF` but a stuffed pair, and a table
without the code a band's first DC needs (`hv_optimised`'s fixture declines every band that way). The
pass costs 1.6 ms on `issue13931.pdf`. ADR 1433's band plan and band decoder serve both cuts; a band
decodes into its thread's buffer rather than a fresh one (`decode_into`), which took a band of
`issue13931.pdf` from 3.8 to 2.8 ms while eight decode at once.

**Byte for byte over the corpus** (a scratch census, removed since): 1 477 files, 1 196 `DCTDecode`
frames, 106 of a megasample or more in the admitted shapes with no interval, 104 cut, **104 identical
to the whole frame's decode**, 2 declined.

**Measured with the signal** (`frame_budget`, pinned, minimum of three runs of five rounds, one
binary with scratch switches, load 1.5–2.5): the cut alone and the buffer together −0.9 ms on
`issue13931.pdf` and −1.4 on `carp2010-2.pdf` p5, the two single-photo pages without an interval;
Plans, `images.pdf`, `arsimon_1.pdf` p56 (two frames, so not alone) and the photograph moved inside
their spread. It pays where it is taken and is not taken anywhere else, so it is kept. A band of
1 024 lines was faster on `issue13931.pdf` and slower on `carp`; 256, ADR 1433's, was kept.

## 5. The whole, against the tree before (same sitting, two binaries, `md5sum`-distinct)

| page, turn | pinned (load 1.5–2.0) | unpinned (load 1.2–1.4) |
|---|---|---|
| `issue13931.pdf` | 37.87 (32.73) → **22.60 (17.54)** | 39.78 → 25.74 |
| `images.pdf` | 34.90 (28.70) → **31.09 (24.62)** | 35.85 → 31.51 |
| `22060_A1_01_Plans.pdf` | 51.38 (36.16) → **47.77 (32.43)** | 50.32 → 50.02 |
| `carp2010-2.pdf` p5 | 19.20 (13.27) → **17.85 (12.02)** | 20.80 → 19.27 |
| `issue12841_reduced.pdf` | 37.66 → 37.03 | 34.78 → 34.79 |

Milliseconds, `budget (interp)`. Callgrind, one thread (where the cut is not taken):
`issue13931.pdf` 926 → 627 M, Plans 2 147 → 2 067 M, `images.pdf` 704 → 577 M. A first build of
the plane route left `DCTDecode` masks to `decode` and its first channel, and Plans read 51.03 →
52.28 against the same before; the `Luma` route is what took it below.

## 6. What is left

`issue13931.pdf`'s interpretation is the frame (5–6 ms banded), the inversion (2.7 ms, table lookups
over 51 MB), the multiplication (2.4 ms) and the copy of the finished raster into the `Arc` an `Image`
holds; the inversion could write `α` into the alpha byte and leave the multiplication nothing to do,
but that moves which step applies §11.6.5.2's opacity and was not built. Plans' interpretation is
still its eight frames' Huffman on busy cores. The corpus walk's peak is now three rasters of one page,
which only a comparison that does not hold the one-thread backend's whole raster could lower.
