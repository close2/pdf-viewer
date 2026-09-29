# 1399 — The first layer's headers repeat nothing, its references come first, and a codestream's device space is a device use

Status: accepted and **built**.
Context: ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3; ITU-T T.801 (held as
`doc/md/T.801.md`, ADR 1383), cited by clause and paraphrased; ISO 32000-2 §7.4.9. Code:
`crates/pdf-model/src/jpeg2000.rs` (`Headers::top_level_spans`, `Headers::jp2_header_contents`,
`Headers::first_codestream_header`, `FirstLayer::contents`, `FragmentList::box_index`),
`crates/pdf-archive/src/table/graphics.rs` (`jpx_cross_references_precede_the_codestream`,
`jpx_first_layer_repeats_no_header_box`, `JPEG2000_DEVICE_COLOUR_IN_THE_CODESTREAM`),
`crates/pdf-archive/src/survey.rs` (`jpeg2000_device_family`), `crates/pdf-archive/tests/jpx_baseline.rs`.
Completes ADR 1383 section 4's two unread parts.

## 1. M.9.2.6's last requirement

M.9.2.6 asks, besides the order ADR 1383 checks, that every fragment a Cross-Reference box the
first layer needs points at lie in the file before the data of the codestream that layer uses:
before its Contiguous Codestream box, or, for a Fragment Table, before the Media Data box holding
that table's first fragment. **Which Cross-Reference boxes** is a reading: those of the first
Codestream Header box and the first Compositing Layer Header box, because M.11.6 applies header
box *i* to codestream *i*, M.11.7 numbers layers by header box, and M.9.2.2 makes the first
layer's codestream the file's first. **Before** means the whole fragment, offset plus length, since
a fragment that begins before the bound and runs past it is not found before it. A Fragment Table
whose first fragment no top-level Media Data box holds is bounded by the fragment itself.

## 2. M.9.2.7's second sentence

M.9.2.7 applies what the JP2 Header box holds to the first codestream and keeps the boxes inside
it out of the Compositing Layer Header box and the Codestream Header box associated with the first
layer. Read **by type, against this file's JP2 Header box**: a box of a type the file's JP2 Header
box holds, found in the first layer's header box — directly, inside its Colour Group box, or as a
Cross-Reference box's `Rtyp`, which M.11.6 and M.11.7 treat as stored there — is the finding. A
type the JP2 Header box does not hold (a `res ` box, a Colour Group where the JP2 Header states no
`colr`) is not. The reason for the reading is the sentence's own first half: a baseline reader
applies the JP2 Header box to the first codestream, so a second statement in the layer's header is
one that reader would not see. ADR 1383's fixture of a Colour Group box beside a JP2 Header `colr`
box now draws this finding, and M.9.2.4 is still judged on the Colour Group, as M.11.7 says.

## 3. The device-colour row's second route

The subclause's device-colour sentence has a second route — an image stating no `ColorSpace`
whose data's own colour definition is effectively a device space — and its row is now delegated
like the first route's: `crate::survey` records, for an image that states no
`ColorSpace` and draws, the device space its data effectively uses, and the six section 6.2.4.3
rows judge it. *Effectively* is two texts. T.801 Table M.25 defines enumerated CMYK (12) as ink
coverages for a device. And §7.4.9's last sentence draws data stating no colour space a processor
supports in the device space of its channel count; the specifications a processor is obliged to
support are M.9.2.4's list and §7.4.9's CMYK, so no specification, a method outside the three, or
an enumeration off that list falls to the channel count. The specification read is the one the
part has a reader use — the only one, or the one at `APPROX` 1, else the first (I.5.3.3). ICC and
calibrated enumerations are not device uses. An image on no page, and a soft mask's image, are the
survey's limits and so the row's.
