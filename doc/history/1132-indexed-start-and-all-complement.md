# Round 1132 — §8.6.6's Indexed start and the /All complement

Contract: §8.6.6's tiling/lookup residue — resolve the two calibrated defects the aggregate
note names that are not §8.6.6.5's (the DeviceN spot case stays blocked on unstated blending).

## Finding: both are already correct, and the calibration holds

Neither is a live defect. `ColourSpace::entry_of` computes `start = index.min(hival) *
components` (colour.rs:2099) — §8.6.3's 0-based index into the base's components, clamped to
`[0, hival]` — and `to_rgb_at`'s `AllColourants` arm complements the tint, `1.0 - channel(t)`
(colour.rs:2427), which is §8.6.6.4's "complemented by subtracting from 1" on an additive
device. The fixtures the contract describes already exist and pass:

- `colour::tests::an_indexed_space_reads_its_table` builds exactly
  `[/Indexed /DeviceRGB 2 <ff0000 00ff00 0000ff>]` and asserts index 1 selects green (0,1,0),
  index 99 clamps to blue — index 1 is not clamped to 0.
- `colour_paths::the_all_colourant_is_a_complemented_tint` asserts tint 1 -> black, 0 -> white.

## Calibration (trap 13) — planted each, watched it fail, reverted

- `start` forced to 0: `an_indexed_space_reads_its_table` (index 1 drew red not green) **and**
  `indexed_out_of_range::an_out_of_range_index_is_adjusted` both FAIL — the note's "both
  indexed tests". `/None` unaffected.
- `/All` complement dropped (`grey = channel(t)`): `the_all_colourant_is_a_complemented_tint`
  FAILS. colour.rs restored from backup; `git status` clean on it.

## Census (trap 8) — new instrument

Added `pdf-model --example indexed_all_census`: walks every object and each image's
`/ColorSpace` with `pdf_syntax` alone (not the reader it measures) for §8.6.3 arrays whose
family is `/Indexed`, `/I` or `/Separation`. Over pdf.js + corpora-own + the IndexedColor
witness the Indexed bases seen are DeviceRGB (commonest), DeviceGray, DeviceCMYK, ICCBased,
Lab, DeviceN and Separation, hival 0..255; a Separation `/All` is rare. The contract's
`DeviceRGB hival=2` shape is corpus-real (S2.pdf); grep finds a fraction (streams hide the rest).

## Changes

No source change — the two clauses are correct and calibrated. Added the census example;
appended its command (not its counts) to the §8.6.6 aggregate note. §8.6.6.3 and §8.6.6.4 stay
`implemented`; §8.6.6.5's DeviceN spot case left blocked as briefed.
