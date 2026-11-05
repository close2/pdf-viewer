# Q788 — Does this reader convert DeviceGray under a PDF/A CMYK intent as ISO 19005 says?

Source: round 1947 (ADR 2708), which traced the six `as-shown` outputs still drawing up to 25 levels
from their sources under the assumed-inks intent (ADR 2700 section 3).

## What the round found

- **The cause is a grey inside a soft mask.** It is not the page's blending space.
  - In four documents, a `/Luminosity` group stating `/CS /DeviceCMYK` paints `DeviceGray` images,
    fills or shadings.
  - In `bug1703683_page2_reduced` and `issue13520`, a `/DeviceGray` mask group draws an isolated
    `/DeviceCMYK` group of black.
- **What the source does.** With no intent, the grey and the CMYK meet through ISO 32000-2
  §10.4.2.3's classic conversion, black alone, and the mask's luminosity is the grey again.
- **What the archive does.** Under the intent, this reader converts between them through the
  profile's own separation (ADR 0796, `B2A`). §11.5.3's device branch, which this reader takes in
  every space (A23, ADR 2552), weighs that separation differently.
- **What ISO 32000-2 says.** Alone, it leaves the conversion to the implementation (§11.7.2: a
  device colour of another count "shall be converted or mapped to a CIE-based colour space in an
  implementationdependent fashion").
- **What ISO 19005 says.** ISO 19005-2 and ISO 19005-4 section 6.2.4.3 do not leave it open. Where a
  file sets no device-independent `DefaultGray` and its PDF/A output intent has a CMYK profile, a
  conforming reader converts a `DeviceGray` colour to CMYK by the classic method (black alone) and
  renders it through that profile. That rule draws all six masks as their sources.

## The cost of following it

The rule binds every `DeviceGray` colour in such a file, not only a mask's. A plain grey would then
draw through the press's black ink rather than as this reader's neutral grey. Under the assumed inks,
`0 g` would draw as about (35, 31, 32) rather than (0, 0, 0). Two effects follow:
- a third-party PDF/A file with a CMYK intent would draw as ISO 19005 asks;
- the converter's archives would draw their grey text lighter than their sources, unless it also
  writes a device-independent `/DefaultGray` that matches this reader's grey. That default would
  bring back today's colorimetric separation inside the masks.

So no archive keeps both the source's plain greys and the source's masks.

## The options put

1. **Follow section 6.2.4.3 in the reader, and keep the archive's greys with a `/DefaultGray`**
   (recommended). Where a file identifying as PDF/A (`pdfaid:part`) states no `DefaultGray`, the
   reader converts `DeviceGray` to the CMYK intent's black. The converter writes a calibrated grey
   default wherever it adds a CMYK intent over `DeviceGray` content. The six masks then stay the
   standard's difference, recorded in `doc/pdfa-blocking-issues.md` section 8. This needs a
   corpus measurement first, since it moves every PDF/A file with a CMYK intent and grey content.
2. Follow section 6.2.4.3 in the reader, and write no `/DefaultGray`. The six masks draw as their
   sources, and every archived grey under a CMYK intent moves.
3. Leave the reader on ISO 32000-2's implementation-dependent conversion. Nothing moves, and a
   PDF/A file draws its greys as no conforming PDF/A reader would.

## What the tree does meanwhile

Option 3. The six outputs draw as before, and the conversion report says why. The test
`drawn_through_the_intent::a_grey_painted_into_a_cmyk_mask_masks_by_its_grey_in_the_source` pins the
source and that the archive differs, so an answer is seen to move it.
