# 1912 — The image module is a module tree by codec and colour, and nothing it decodes moved

Session 1538, on the project owner's word of 2026-10-14 ("switch to code improvements/refactoring
rounds"). Status: **accepted** and **built** in `pdf-model`. Context: ADRs 0266, 0340, 0456, 1023,
1469, 1481, 1495. No clause is read anew and no ledger row changes status.

## 1. What it was

`crates/pdf-model/src/image.rs` was 8 949 lines: the public entry points, the colour space and
`/Decode` map, the route dispatch, the sample unpacker, the colour conversion, four codecs, the
explicit and the soft mask, the device-scale mask, the caches and 900 lines of unit tests, in the
order they were written. `ahead`, `cut` and `restart` were already children.

## 2. Decision

**One module per concern, named by what it is**, each opening with a comment that says what it is
and which clause governs it; the root's comment ends with a map of them.

| module | what moved into it |
|---|---|
| `image.rs` (342) | `ImageError`, `InFlight`, `MAX_SAMPLES`, `Picture`, `Parts`, `Flattened`, `decode`, `decode_parts`, `sandboxed_decoder`, the re-exports |
| `dispatch.rs` | `decode_parts_in`, `samples_of`, the `/Matte` steps before the samples (`EagerMask` to `invert_matte_in_place`), `filter_samples`, `positive_integer`, `beside` |
| `space.rs`, `unpack.rs`, `convert.rs` | `ColourSpace`, `Decode`, `colour_space`; `Samples`, `unpack`, the eight-bit and packed routes, `sample_rgba`, `palette`, the colour-key test; `convert_three`/`four`, `SampleMemo`, `channel`, `convert_channels` |
| `dct.rs` with `dct/restart.rs`, `dct/cut.rs` | `decode_dct` to `samples_of_frame`, the marker readers, `decode_jpeg`, `banded_decodes`; the two children moved under it unchanged but for their fixtures' paths, one link, and `restart::decode` as `pub(in crate::image)`, the readers it had |
| `jbig2.rs`, `ccitt.rs`, `jpx.rs` | each codec's hand-off to the sandbox and what it delivers |
| `mask.rs`, `combine.rs`, `soft_mask.rs`, `device_scale.rs` | `/Mask` and the reports on it; `MaskSamples`, `combine_on_the_finer_grid`, `alpha_on_grid`; `/SMask`, `/Matte`, the grey plane; `SoftMaskAtDeviceScale` |
| `cache.rs`, `tests.rs` | `RasterCache`, `MaskCache`, `ShapeMasks`, `StreamIdentity`; the sixteen unit tests, one file, each path pointed at its item's module |

**No path outside `image` moved**: every `pub` and `pub(crate)` item is re-exported from the root.
**Visibility is what a sibling module reads**: every moved item was made private and a trial
compile reopened, as `pub(super)`, the 144 items, fields and methods a sibling or the tests name.
An intra-doc link to an item now elsewhere names its path in a reference definition under its
paragraph, so no sentence was reworded. Two ledger rows' test paths follow the tests
(§7.4.6, §7.4.8); the code lists still name `image.rs`, which is the module's root.

**The tests stay one file**, unlike ADR 1914's,
because ten of the sixteen name items of two to four modules (a `/Decode` map with the unpacker,
a `/Matte` inversion with the combination); placing each beside one of its subjects is a choice
per test, not a move, and the six that test one module are a later round's to place.

## 3. The proof that nothing moved

The split was exported alone onto HEAD `512af7e5` (`scratchpad/r1538/split`, ADR 1890) and every
gate below ran on it in one hold. `raster_golden`: held 974, moved 0. **The six arms against
`/home/AI/arms-1534/`, by page: `cpu-1x`, `gpu-1x`, `compute-1x` 968 pages each, `cpu-4x`, `gpu-4x`,
`compute-4x` 965 each, 0 digests and 0 mean errors differ.** `corpus`, `jpeg2000`, `oracle` and
`foreign_corpus` pass; `launch_path` bands 31 figures with 0 outside, `WTPDF`'s `open_kinstructions`
12 754.108 thousand against the 12 752.5 its band was last set from. `pdf-model`'s 2 048 tests pass
and the unit tests are HEAD's sixteen.
