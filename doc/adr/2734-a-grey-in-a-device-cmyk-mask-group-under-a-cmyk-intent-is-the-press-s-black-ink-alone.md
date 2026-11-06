# 2734 — A grey in a `/DeviceCMYK` mask group under a CMYK intent is the press's black ink alone

Status: accepted. Session 1960. Context: ISO 32000-2 §11.5.3, §11.7.2, §10.4.2.3, §11.6.6, §14.11.5; A23 and
ADR 2552 (the device branch in every space); ADR 1008 (a mask group's `/DeviceCMYK` is the intent's press); ADRs
2708 and 2712 (A788, the PDF/A rule); the Ghent PDF Output Suite's patches 16.8, 16.9 and 16.11 (ADR 2728).

## The defect

Every InDesign glow and feather in the suite's soft-mask patches drew too sharp and too small, in both views. Their
`/Luminosity` groups state `/CS /DeviceCMYK` and `/BC [1 1 1 1]` and paint a `DeviceGray` image: the feather's ramp.
The files are PDF/X-4, so their intent is a CMYK press and not a PDF/A one.

## The diagnosis

Round 1957's hypothesis was that the mask's luminosity was the press's colorimetric `Y`. **That half was wrong**: the
group composites in the intent's press (ADR 1008) and is weighed by §11.5.3's device branch, EXAMPLE 2's four-ink
formula, as A23 chose. **What moved was the grey's way into the press.** A device grey is a colour of another
component count, which §11.7.2 leaves to be "converted or mapped to a CIE-based colour space in an
implementationdependent fashion". This reader separated it by the press's `B2A` table, into cyan, magenta and yellow
as well as black. EXAMPLE 2 weighs that separation darker than black alone, so a mid grey masked as a darker one.
ADR 2708 found the same shape in six archive outputs.

**The plant.** The grey read as the intent's black ink inside such a group (below) draws all three patches like
their reference images in both views. Without it, the fixture test below fails on both its fill and its image.

## The reading

§11.5.3's device branch: "convert the colour to DeviceGray by implementation-defined means and use the resulting gray
value as the luminosity, with no compensation for gamma or other colour calibration". A `B2A` separation of a grey is
exactly a calibration: it finds the four inks that reproduce a neutral colorimetrically. The group's `/CS` names a
device family, and §10.4.2.3 is the conversion between two device families: black `1 − gray` and no other ink. That
is what the same group does on a page with no intent (`InkScale::Double`), and what ISO 19005 section 6.2.4.3 already
asks of a PDF/A file (ADR 2712). So a mask's value no longer depends on whether the file carries an intent.

## Decision

- `soft_mask::content_grey` sets `SoftMaskRequest::grey` to `GreyReading::IntentBlack` when a `/Luminosity` group
  states `/CS /DeviceCMYK`, composites in the intent's press, and the intent is a CMYK profile. Every other mask
  keeps the page's reading.
- The interpreter puts that reading in force for the group's runs and puts the page's back afterwards
  (`enter_mask_compositing`, `leave_mask_compositing`). Fills, strokes, images, shadings and nested masks all reach
  it through the reading, as ADR 2712 built it.
- Not a question for the owner. ISO 32000 leaves the conversion to the processor (§11.7.2), the owner chose the
  device branch (A23), and A788's rule is unchanged: the PDF/A case already read this way. Round 1949's tests hold
  unchanged.

## Not taken

- `DeviceRGB` painted into such a group is still separated by the `B2A` table. §10.4.2.4 is the device route, but it
  needs black generation and undercolour removal, which a mask group has no graphics state to supply (§11.7.5.3). No
  patch exercises it.
- A transparency group that is not a mask keeps the page's reading. There the grey is colour that a person sees, and
  outside PDF/A it is this reader's neutral grey.
