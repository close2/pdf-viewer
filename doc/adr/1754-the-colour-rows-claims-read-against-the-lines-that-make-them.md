# 1754 — The colour rows' claims, read against the lines that make them

Session 1459. Status: **accepted** — an audit, with three defects in one function fixed and three stale names
corrected. Context: ADR 1712, where §8.6.5.4's note said `/WhitePoint` "is read" while the code read
every `Lab` space as D50. Rows: the thirteen `implemented` rows `8.6.5`–`8.6.5.9`, `8.6.6.3`,
`8.6.6.4` and `8.6.7`. Code: `crates/pdf-colour/src/colour.rs`, `crates/pdf-model/src/content/colour.rs`.

## 1. Why this is coverage work on `implemented` rows

Every `partial` leaf in the ledger is waiting on an owner's answer (Q308, Q271, Q348, A66's trigger,
a policy syntax no signature names), so no round can move one this batch. A false claim in an
`implemented` row is a false `implemented`: that was ADR 1712's finding and the reason for this audit.

## 2. The method

Every sentence in the thirteen notes that says an entry, a parameter or a condition "is read", "are
read", "is applied" or "is executed" (23 of them; `scratchpad/r1459/claims.py` splits the notes and
counts them), checked against the code it names. Every backticked name in the same notes was then
checked to exist (`scratchpad/r1459/names.py`, plus a check that each type-qualified name is a member
of its type). Line numbers are this session's tree.

## 3. The claims

| row | claim | where it holds | verdict |
|---|---|---|---|
| 8.6.5 | four families read, one XYZ-to-pixel route | `parse_at` 2435, `parse_cal_gray` 3025, `parse_cal_rgb` 3043, `parse_icc_based` 2947; `xyz_d50_to_srgb` 5785 is the only matrix (`icc.rs` 1369 calls it) | holds |
| 8.6.5.2 | CalGray `/BlackPoint` read, not applied | parsed at 3030, `black: _` in `to_rgb_at` and `cie_xyz_at` | holds |
| 8.6.5.4 | `/WhitePoint`, `/BlackPoint`, `/Range` read | the `Lab` arm 2435–2455; white applied by `lab_xyz` 5830 and `adapt` in `cie_xyz_at` 3928 | holds (ADR 1712) |
| 8.6.5.4 | missing white read as D50 | `white_point` 5990 | holds |
| 8.6.5.4 | `/BlackPoint` read, not applied | `Lab { white, range, .. }` 3995 | holds |
| 8.6.5.5 | `/Range` read | `icc_range` 6026, called at 2991; `Profile::with_range` `icc.rs` 1254, `encoded` 1325 | holds; **the initial colour did not use it** (§4) |
| 8.6.5.5 | the "from CIE" half read | `B2A1` then `B2A0` at `icc.rs` 963; `mft1`/`mft2`/`mBA ` at 1939–1942; `to_device` 1464 | holds |
| 8.6.5.5 | the closing bullet read by a validator | `pdf-archive` `IccEdition` (`table/graphics.rs` 1375) | holds |
| 8.6.5.5 | `/N` read only where the profile is not | 3015, after the profile and `/Alternate` | holds |
| 8.6.5.5 | the header's intent field read nowhere | no read of bytes 64–67 outside `icc.rs`'s tests | holds |
| 8.6.5.5 | a `'Lab '` data space read | `icc.rs` 888 | holds |
| 8.6.5.6 | the image half read | images `image.rs` 1532, inline images through `inline_image::scan`'s resources, shadings `shading.rs` 420 and 518 | holds |
| 8.6.5.7 | every colour converted when read; the press passthrough performed | `to_cmyk_at` 3790, the `is_of_profile` arm 3805 | holds |
| 8.6.5.8 | the three routes to the intent read | `ri` `run.rs` 968, `/RI` `ext_gstate.rs` 435, `/Intent` `image_intent` `image.rs` 622, which inline images reach through `draw_image` | holds |
| 8.6.5.8 | a display profile's media white read as the PCS's | `media_white_of` `icc.rs` 1909 | holds |
| 8.6.5.9 | `/UseBlackPtComp` read with three values | `ext_gstate.rs` 423; `BlackPoint::applies` `content/colour.rs` 39 | holds; the note named `GraphicsState::black_point`, which is `black_point_under` (`content.rs` 444) |
| 8.6.5.9 | the three intent routes read | as 8.6.5.8 | holds |
| 8.6.5.9 | CalRGB `/BlackPoint` read, not applied | 3043, `black: _` | holds |
| 8.6.5.9 | a shading pattern's parameters read where §11.6.7 says | `PatternInitial::of` `pattern.rs` 117, called once from `run.rs` | holds |
| 8.6.6.3 | the scaling sentence applied | `parse_indexed` 3099 scales by `component_range` 3148 | holds; "five routes" named four, and the JPEG 2000 route is two (`decode_jpx`, `jpx_samples`) |
| 8.6.7 | `/OP`, `/op`, `/OPM` read | `ext_gstate.rs` 400–409 | holds |
| 8.6.7 | Table 57's precedence read | `filling.or(stroking)` at 405 | holds |
| 8.6.7 | the zero test made on the file's tints | `special_overprint` `overprint.rs` 205, `tints.contains(&0.0)` 234 | holds |

Beside the claims, §8.6.5.3's note named `RgbRoute::from_xyz`, which is `components_with_xyz` (1566).
The three stale names are corrected in the notes.

## 4. The defects, which were one sentence's

§8.6.5.1's note says the initial colour has "the nearest valid value substituted where 0.0 is out of
range — `ColourSpace::initial_colour`". It did not hold in three ways, all in that function:

- **A `Lab` `/Range` stated backwards panicked.** The arm called `0.0_f32.clamp(range[0], range[1])`,
  and `f32::clamp` panics when its minimum exceeds its maximum. `[/Lab << … /Range [50 10 …] >>] cs`
  was enough; the conversion beside it (`lab_xyz`) already took the bounds in either order. The
  bounds are now one function, `nearest` 5854, used by both, and the plant is
  `an_initial_colour_is_held_to_the_range_however_it_is_written`, which panics on the old arm.
- **An `ICCBased` space's range was not consulted**, under a comment saying `/Range` "is not read":
  the arm returned zeros. For a conformant file nothing moves, since Table 68's four ranges all
  include 0.0; for a file whose Table 65 `/Range` ADR 1098 takes over the profile's, the initial
  colour is now held to it as every other colour is.
- **A device family under a `/Default` space or an output intent started at the stand-in's initial
  colour.** `/DeviceCMYK cs` with a four-component `ICCBased` stand-in started at `[0 0 0 0]`, no ink,
  which draws the paper; §8.6.8 starts `DeviceCMYK` at `[0 0 0 1]`. That is a decision about what
  §8.6.5.6 passes, and ADR 1755 has it. Its fixture draws white (255, 255, 255) where black (0, 0, 0)
  is owed: `a_device_family_starts_at_its_own_black_whatever_stands_in_for_it`.

## 5. What it moved

HEAD's six corpus arms (`/home/AI/arms-1456/`) and this tree's agree digest for digest — 968,
964, 968, 963, 968 and 964 pages, none moved: no corpus first page selects a device family under
a stand-in and paints before it sets a colour. The population is the fixtures
(trap 8), each run against its planted defect.
