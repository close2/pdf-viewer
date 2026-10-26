# 2348 — A default `/Decode` is copied, and a row the mask leaves transparent is written whole

Status: accepted. Session 1757, 2026-10-26, the measure slot of batch one hundred and fifteen.
Context: ISO 32000-2 §8.9.5.2, §11.6.5.2; ADRs 1433 and 1469 (the eight-bit table route and the
conversion's memo), 1457, 1481 and 1519 (the matte's tables and its inversion in place), 1969 (when
a band moves), 2332 (the round before this one on the same gate); habit 164; trap 13; principle 2's
rule that an optimisation carries its benchmark and its comment. Moves no ledger row and no band.

## 1. The question, and which hypothesis held

After ADR 2332 the turn gate's slowest pages are, in order, `bug1721218_reduced.pdf` (ADR 2349
prices what is left of it), `22060_A1_01_Plans.pdf`, `issue12841_reduced.pdf`, `bug1743245.pdf`
(ADR 2333), `images.pdf` and `issue13931.pdf`; on the four image pages the turn is mostly `interp`.
Callgrind over `pdf-model`'s `examples/callgrind_pages -- <n> <file> 1 kept-revisit` on HEAD's
export, a run of two less a run of one, every thread, gives one steady interpretation of each:

| page | all threads | the largest term in this tree's own code |
|---|---|---|
| `22060_A1_01_Plans.pdf`, four 2 480 × 2 630 `ICCBased` photographs under masks | 2 090.6 M | `convert_three`, 455.0 M, ADR 1469's memo |
| `issue12841_reduced.pdf`, one 5 280 × 3 792 photograph | 1 098.6 M | none: the decoder, `zune-jpeg`, is 87 % of it |
| `issue13931.pdf`, a 2 996 × 4 256 photograph under a `/Matte`'d mask | 692.5 M | `invert_matte_in_place`, 293.4 M |
| `images.pdf`, four eight-bit `DeviceRGB` images under the default `/Decode` | 612.1 M | `unpack`, 213.0 M |

The brief's hypothesis, two pages' profiles naming one lever each that clears its ceiling, **held
for `images.pdf` and `issue13931.pdf`** and for no other: the codec page's time is the decoder's,
and the plan's memo is already one hash probe a pixel, its conversion answered from the memo for
99.97 % of its pixels (ADR 1469), with nothing exact left to take out of it without a table of
2²⁴ entries.

## 2. The decisions

- **A table that maps every byte to itself is not looked up.** `unpack_eight_bit_device` read three
  256-entry tables a pixel, 17 instructions a pixel on `images.pdf`. Under §8.9.5.2's default
  `/Decode` on a device space each table is the identity, which `Decode::is_identity` already asks
  for the `DCTDecode` route; there the pixel is the sample's own bytes and an opaque alpha, written
  four pixels a step as twelve bytes into sixteen, which the compiler widens. Ceiling first: the
  walk with no lookups at all is the whole 213 M less a copy, and the copy measured 75.4 M.
- **A row the mask leaves wholly transparent is the matte throughout.** At a mask value of 0
  §11.6.5.2's pre-blending is the matte whatever the source colour was, so its inversion is this
  tree's choice, and `Prematte::restore_component` answers the matte; each table's row at α = 0 is
  therefore one value. `wholly_transparent_pixel` reads that from the tables rather than
  assuming it, and a row whose mask values are all 0 is filled with that pixel without its samples
  read. `issue13931.pdf`'s mask is 0 on 3 917 of its 4 256 rows. A first form asked α = 0 a pixel:
  the compiler merged the two arms into one store sequence and it read 256 M where HEAD reads 293 M,
  so the row is the unit.

Both are the same bytes by construction, and the tests hold them to it:
`the_table_route_unpacks_the_per_sample_routes_pixels` now carries the default `/Decode` in grey
and RGB beside the stated ones and asserts which route each takes, and
`the_matte_table_route_unpacks_the_per_sample_routes_pixels` a row of mask values all 0. Six planted
defects each fail one of them (trap 13): two bytes of a four-pixel step swapped, red and blue
swapped in the single-pixel tail, grey's alpha made its level, two components of the transparent
pixel swapped, its alpha flipped in grey, and a row of values up to 7 taken for a transparent one.

## 3. The figures

**Instructions**, one steady interpretation, every thread; the display list's `--digest` (which
renders every image's samples) is the same on HEAD and the change on these two pages and on
`22060_A1_01_Plans.pdf` and `issue12841_reduced.pdf`; `raster_golden` holds 974 and moves none, and
against HEAD's six arms no page differs in frame digest or mean error:

| page | HEAD | this change | the term |
|---|---|---|---|
| `images.pdf` | 612.1 M | 432.7 M, −29 % | `unpack` 213.0 → 75.4 M |
| `issue13931.pdf` | 692.5 M | 457.7 M, −34 % | the inversion 293.4 → 42.0 M |

**The clock**, one `--clock` hold on 2026-10-26, HEAD's export interleaved with this round's,
`frame_budget` pinned to the four Zen 5 cores and their siblings, five rounds a run, three runs each
way, load 2.5 to 2.9, ms:

| page, row | HEAD | this change | per interleaved pair |
|---|---|---|---|
| `images.pdf` turn | 36.54, 38.31, 36.91 | 33.25, 33.74, 34.77 | −3.3, −4.6, −2.1 |
| its `interp` | 26.86, 28.50, 27.16 | 23.89, 24.69, 25.18 | −3.0, −3.8, −2.0 |
| `issue13931.pdf` turn | 21.87, 21.86, 22.65 | 19.15, 20.21, 20.84 | −2.7, −1.7, −1.8 |
| its `interp` | 16.38, 15.76, 16.41 | 13.78, 14.46, 14.87 | −2.6, −1.3, −1.5 |

The step rows, which interpret nothing, read inside each other's spread. In the same hold the turn
gate judged 33 figures on each export and none was outside: `images.pdf`'s turn 35.19 → 33.80 ms
and `issue13931.pdf`'s 23.11 → 21.35, inside their bands, so no band moves (ADR 1969). The launch
gate judged 59 figures on each and none was outside; its memory figures read beside each other,
`open_anon_mib` within 0.007 MiB on every document and `peak_anon_mib` within 1.1 MiB.

## 4. What is left on these pages

`images.pdf`'s inflation, 163 to 178 M in `zlib-rs`, and its masks' planes, 52 M; on
`issue13931.pdf` the decoder's colour conversion, 74 M, and the mask's plane, 51 M; on the plan the
memo and the decoder. None is a lever this round priced to its ceiling.
