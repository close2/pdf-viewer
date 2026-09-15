# 1108 — The domain a colour key is written in

Contract: §8.9.6.4, colour key masking — the row's `partial`, and the arm it said was refused.

## The three witnesses, read

`examples/colour_key_mask_census`: 3 open, 3 state a colour key, 0 over a codestream filter —
`colorkeymask.pdf` `/Mask [255 255 0 255 0 255]` unfiltered, `issue14821.pdf` `/Mask[ 0 0 0 0 0 0]`
and `issue15629.pdf` `/Mask [251 251]` both Flate; the row's sentence holds and all three interpret
completely. `issue15629.pdf`'s space is `[/Indexed /DeviceRGB 255 …]`, so its two integers range
over *index* values — §8.6.6.3 makes that the one component of an Indexed space.

## The refused arm, and the reason that was wrong

Not the lossy filters — ADR 0832 settled those. `JPXDecode` was refused because Table 87 makes
`/BitsPerComponent` "ignored if present", which was read as withdrawing the domain §8.9.6.4 bounds
its integers by. Its next sentence says the opposite — "The bit depth is determined by the PDF
processor in the process of decoding the JPEG 2000 image" — and §7.4.9 says from what: a packaging
carrying "the colour space, bits per component, and image dimensions", which `pdf_model::jpeg2000`
reads out of `ihdr`, `bpcc` and `SIZ` without decoding a sample. Trap 40: a gap, not a departure.

## Built

Applied where the samples still carry the domain the file wrote its integers in: an `Indexed`
dictionary space at any precision (§7.4.9's precedence makes the decoder hand indices back
unscaled; §8.6.6.3 caps `hival` at 255), and eight unsigned bits per component otherwise. Any
other precision, or signed samples, is reported as `MaskEntry::Unusable` naming what it read;
`mask_entry` and `unapplied_mask` take the stream rather than its dictionary. ADR 1121.

Four fixtures in `tests/image_masks.rs`, all generated — no corpus holds a colour key on a
`JPXDecode` image. Calibrated by planting: *accept every depth* fails only the refusal test;
*never apply the ranges* fails all three JPEG 2000 tests; testing the base space's components
instead of the index fails the new Indexed test **and nothing else**. The eight-bit fixture states
`/BitsPerComponent 4`, so "shall be ignored if present" is asserted, not assumed.

## Left owed

The residue is this tree's eight-bit image pipeline, not a silence in the standard, so §8.9.6.4
stays `partial` and is **not** a `departed` candidate; §8.9.6's note is corrected to match. No
corpus rendering moves: the change reaches the `JPXDecode` route alone.
