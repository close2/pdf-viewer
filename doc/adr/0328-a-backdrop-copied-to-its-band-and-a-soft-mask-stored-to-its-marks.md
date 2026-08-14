# ADR 0328 — A backdrop copied to its band, and a soft mask stored to its marks

Status: accepted, 2026-08-14 (session 493).

## Context

`doc/todo/40` had one measured item left after ADR 0271 removed the arithmetic: **the
copying**. Two buffers in `render-cpu` were allocated and filled at the size of the whole
surface where the work lives in a band of it:

- **`initial_backdrop`** copied the entire surface into a fresh buffer for every
  non-isolated group. `0423548.pdf` — the second-slowest document of a 65 944-document
  crawl — states 132 of them on one 1843 × 5103 page: **4.3 GB copied for 82 MB of band**,
  2.85 s of the 6.6 that remained after ADR 0271.
- **`build_soft_mask`** converted every pixel of a surface-sized buffer into a mask value
  and stored the result whole. `6081357.pdf` states 912 distinct soft masks on a
  4.3-megapixel page; ADR 0271 made the conversion of a transparent pixel one branch, but
  the branch still ran over 3.87 billion pixels and the storage still held — and evicted —
  surface-sized rasters.

The file also named the part to settle first: a clip outside its band admits nothing, so a
band-sized clip mask and "zero elsewhere" are the same thing — but a soft mask outside its
group's marks takes `SoftMask::outside`, which is **255 for a white `/BC`**, so a band
means "constant outside" rather than "nothing outside", and `Built` would have to carry
the constant beside the band. That is exactly what was built.

## Decision

Three changes, all in `crates/render-cpu/src/lib.rs`, none of which moves a byte of any
page:

1. **`initial_backdrop` copies only the group's band.** The buffer still covers the whole
   surface — see the exactness argument below — but only the rows the group's clip and
   mask admit are copied into it.
2. **`build_soft_mask` converts and stores only the rows the mask group's marks can
   reach.** `marked_rows` bounds them from the same per-command extents `misses_surface`
   culls by (`Command::device_bounds`, a group answering through its elements, one row of
   margin as `Band::covering` takes), and widens to the whole surface the moment one leaf
   cannot say. `Built` gains `outside: u8` — zero for a clip and for a clip × soft-mask
   product, `SoftMask::outside()` for a soft mask — and `MaskCache::combine` substitutes
   the constant for the rows the stored raster does not hold.
3. **The one reader that needs a whole-surface raster gets it by expansion, memoised.** A
   command masked by a soft mask *alone* draws over every row, and `tiny-skia` applies a
   mask of exactly the pixmap's size — so `MaskCache::expand_soft_mask` rebuilds the entry
   once, `outside` everywhere with the band laid in, charged to the same budget. A
   document whose masks are all read bare pays what storing every mask whole used to
   cost, and no more.

### Why this is byte-exact where the todo file expected a departure

`doc/todo/40` warned that a parent's mask rows are only *nearly* the prefix's contribution
for a child's band, because `ToDevice` composes the band's first row into the translation
last and ADR 0219 measured what shifting `ty` by a whole number of rows does to
`y·sy + ty`. **That departure lives in the drawing, and nothing here bands the drawing.**
The group buffer and the mask buffer keep the surface's height, so every element draws
under the very transform it draws under on the page, bit for bit. What is banded is:

- **the backdrop copy** — sound because no buffer row outside the group's band can reach
  the page: every composite path (`blend::interpolate`, `blend::composite`, the
  source-over `draw_pixmap`) reads exactly the band's rows, and every operation an
  element performs on a buffer pixel writes that pixel and no other, so a value outside
  the band cannot travel into it. A nested group repeats the argument one level down.
  `the_backdrop_copy_is_the_bands_rows` pins the half that must still hold — inside the
  band the buffer *is* the page.
- **the mask conversion and its storage** — sound because a row outside `marked_rows`'
  answer is a row of the buffer nobody wrote: its pixels are the transparency the buffer
  was allocated as, and the value the whole-surface pass derived from every such pixel
  was already the one constant, `SoftMask::outside()` = `value([0, 0, 0, 0])` (§11.6.5.1;
  the transparent-pixel identity is ADR 0271's, held by
  `the_transparent_pixels_shortcut_is_the_derivation`). Substituting the constant is
  substituting the value the raster held.
  `a_banded_soft_mask_combines_as_a_whole_one_does` and
  `an_expanded_soft_mask_is_the_constant_with_the_band_laid_in` pin the two readers.

**What was deliberately not taken is the other half of the file's item**: drawing the
buffers band-sized, and building a clip chain's intermediates once each. Both would put a
band's first row into the drawing arithmetic, which is ADR 0219's supersample and a
departure this backend — the oracle — does not take for speed. `doc/todo/40` keeps that
half, with this ADR as the record that the two halves have different exactness arguments.

## Measurement

Callgrind instruction counts — the machine ran ten parallel rounds at a load average of
70 to 90, so a wall clock is not evidence (`doc/habits.md` *Measuring*) — with both arms
built `--release` from this tree in one sitting: the base arm at `ffc587ef` (the round's
base commit), the new arm with this change and nothing else. The exact invocations are in
the history file.

| instrument, `I refs` | base | banded | |
|---|---|---|---|
| `open_one` `0423548.pdf` (open + interpret + rasterise) | 149 475 924 050 | **76 481 113 199** | **−48.8%** |
| `open_one` `6081357.pdf` | 81 900 867 207 | **17 023 010 209** | **−79.2%** |
| `callgrind_rasterise` `bug1721218_reduced.pdf` p1, 20 renders | 13 826 922 716 | **13 670 680 697** | **−1.1%** |
| `callgrind_rasterise` ISO 32000-2 p101, 20 renders | 5 517 832 572 | 5 517 098 548 | −0.013% |
| `callgrind_rasterise` ISO 32000-2 p6, 20 renders | 4 004 078 177 | 4 006 888 935 | +0.070% |

(The worst page's base is 13.83 G today where ADR 0236 recorded 20.03 G — the sessions
between kept taking pieces off it, ADR 0271's transparent-pixel shortcut the largest —
which is why the A/B is two binaries from one sitting and not a comparison against a
recorded figure.)

Peak resident, kernel's `ru_maxrss` over three samples an arm, no valgrind:

| peak RSS, kB, median of three (range) | base | banded |
|---|---|---|
| `0423548.pdf` | 1 143 068 (1 139 404 – 1 156 036) | 1 130 540 (1 113 268 – 1 203 648) |
| `6081357.pdf` | 208 788 (207 332 – 209 000) | 203 460 (193 836 – 203 868) |

Memory moves inside the spread — the mask cache was bounded by `MASK_BUDGET` before and
after, and the group buffer's untouched rows were lazily-zeroed pages that were allocated
either way — so the claim this ADR makes is about instructions, not bytes held.

The two specification pages are the regression check: a page whose masks and groups
already reach most of the surface pays only the band bookkeeping, and it measures at the
level of run-to-run noise (±0.07% here, against ADR 0236's ±0.08% on the same pages).
`bug1721218_reduced.pdf`'s −1.1% says its three soft masks were already cheap after ADR
0271; its clip chains — `MaskCache::get`'s remaining share — are the half not taken.

## Proof the pixels held

- **42 renders byte-identical**, `cmp` on `open_one`'s PNGs from both arms: twenty
  mask/group corpus documents (`bug1721218_reduced`, `alphatrans`, `colorkeymask`, the
  eight `knockout_*`, `nonisolated_blend_smask`, the five `smask_*`, `transparency_group`,
  `transparent`, `issue19971`) at scale 1.0 **and** 2.0, plus `0423548.pdf` and
  `6081357.pdf` at 1.0.
- Every gate of `doc/todo/02` §2 run on this tree and identical to the base, line for
  line — corpus, oracle, quorra, text extraction, dates, XMP, JPEG 2000, conformance.
  The history file carries the printed lines.

## Cost, written down

- `Built` carries one more byte per cached mask, and `MaskCache` one more method. The
  band computation is one `device_bounds` walk per soft-mask build — the same extents the
  strip planner already asks for, memoised in each `Path`'s hull.
- A mask read bare *and* combined is stored expanded from the first bare read on, so a
  page mixing both readers holds what it held before this change, not less.
- The readability cost is three blocks of slice arithmetic and the `outside`
  substitution in `combine`, each carrying its argument in a comment; the exactness
  reasoning above is the part a maintainer must not lose, and it lives in
  `initial_backdrop`'s and `admit_soft_mask`'s doc comments.

## What this leaves

`doc/todo/40` keeps the chain-sharing item (one crop and one intersect per chain, ~17% of
the corpus's worst page) with its unresolved ADR 0219 question, and the quorra half
(`pdf_render::cropped_rectangle` is exported for it; nobody upstream calls it). Both are
in the todo file with this round's re-statement.
