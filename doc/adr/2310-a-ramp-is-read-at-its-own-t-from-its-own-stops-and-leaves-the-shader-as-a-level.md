# 2310 — A ramp is read at its own `t` from its own stops, and leaves the shader as a level

Status: accepted for integration. Session 1738, 2026-10-24, the pixels slot of batch one hundred and
thirteen, under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause:
ISO 32000-2 §8.7.4.5.3 (the row stays `implemented`, its code and tests amended), with §8.7.4.5.4,
§7.10.3 and §7.10.4. Context: ADRs 0011, 1389, 1567, 2286; raster's ADR 0055; habit 179. Amends ADR
0011 on how a ramp is sampled and ADR 1389 §3 on where a step is found; supersedes ADR 1567's
table-making, which has nothing left to do. Code: `raster-gpu/src/device/ramp.rs` (`ramp_table`,
`colour_for`, `MAX_RAMP_STOPS`), `shaders/shading.wgsl` (`ramp_colour`, `ramp_interpolated`,
`ramp_stop`), `device/resident.rs` (`Device::ensure_paint_textures`), `resources.rs`
(`ResourceStore::upload_ramp`), `error/resource.rs` (`ResourceProblem::RampTooLong`). Tests:
`raster-gpu/tests/a_steep_ramp_is_read_at_its_own_t.rs` (red on the tree before, 128 of 256 rows a
level off), `device/ramp/tests.rs`.

## 1. The page, and which converter was wrong

`issue14165.pdf` was the first page of the ranking no ADR or todo entry names (record 1726): one
full-page axial shading whose `/Function` is a type 0 function of 5 120 samples, a grey that crosses
the whole range eleven times along a 921.6-unit axis. Against HEAD's six arms it read 0.1320 of 255
at 1x and 0.1234 at 4x on all three lanes alike, one digest per scale — so not a coverage lane, and
the glyph quantum is not on the page. At 1x, 176 430 of 1 002 822 pixels differ by one level, in
whole rows and in both directions.

The two converters' arithmetic, row by row in column 600 against the display list's own ramp (232
stops) evaluated at each row centre's `t` in `f64`: the oracle is the level nearest the ramp on 841
of 842 rows; raster is the level nearest the ramp on 693. Raster's 149 others are exactly a 4 096-texel
table read at its nearest texel (that model misses one row): the table moved `t` by up to half of
`1/4095`, which on this ramp's slope — up to 3 260 levels per unit of `t` — is four tenths of a level.
**The lane was wrong, not the oracle**: §8.7.4.5.3 gives a point the function at its own `t` (Table
79: "The variable t becomes the input argument to the colour function(s)"), and ADR 0011's grid
answered it at a neighbour's. The table's own comment promised "a fraction of one 8-bit level between
any two stops", which holds for a ramp that crosses the range about once and for no steeper one.

## 2. The construction

The table is the ramp's **stops**: stop `i` in column `i mod W` of block `⌊i / W⌋` (`W` the stop
count up to 4 096), five rows a block — the offset, red, green, blue and alpha, each texel an `f32`'s
bytes, read back exactly through `ramp_word` as row 1's bounds already were. The shader finds the
first stop above `t` by halving (§7.10.4's intervals closed on the left; at or below the first offset
the first stop's colour, past the last the last's) and interpolates between it and the one before,
at `t` — `colour_for` states it in Rust, and `the_table_is_the_ramps_own_colour_at_every_t` holds it
bit for bit to `ramp_color_at` over 600 ramps with coincident pairs, runs of three and uncovered ends,
on every stop, an ulp either side and between. A hard step is now found by the same comparison as
every other stop, so ADR 1389 §3's segments and `MAX_SEGMENTS` are gone with the grid, and
`shading_steps.rs` holds still. A ramp past `MAX_RAMP_STOPS` (4 096 × 819) is refused at upload by
name rather than drawn in part; `pdf_render::Ramp` stays far below it. A two-stop ramp is now 40 bytes
of texture rather than 48 KiB, and the table is written, not sampled, so ADR 1567's threads and
`texel_byte` have nothing to do and are removed.

## 3. What the adapter does with a fragment's colour

With the table gone the arms' adapter (RADV, Radeon 890M) still drew 51 of the 842 rows a level
**low**, every one with the ramp's fraction between 0.50 and 0.61, while llvmpipe drew all 842
exactly. A constant ramp isolates it: RADV writes 200.58/255 as 200 and 239.58/255 as 240, which
is a half float rounded toward zero and then rounded to the byte — up to an eighth of a level off a
component above one half. The table's bytes had hidden it, since a level survives that export. So
`ramp_colour` hands the target the level nearest the answer (`nearest_level`, ADR 2311 §2 on why its
product with 255 is taken exactly), which is the byte the table's texels held and the byte every
adapter now writes: after it, both adapters draw the fixture of the page's own ramp on all 842 rows.
The same export under the function lane, and why a solid paint is not rounded the same way, is ADR
2311's.

## 4. The pixels

`issue14165.pdf` after: 303 pixels differ at 1x, all in the last column, which the page's 0.52-unit
remainder covers partly (the oracle's coverage lattice, ADR 2166 §3), and 10 493 of 16 041 784 at 4x,
the last column and the last row the same way and one row whose centre's colour is 137.5000 exactly,
a tie each converter's `f32` breaks its own way. The corpus figures are the record's.
