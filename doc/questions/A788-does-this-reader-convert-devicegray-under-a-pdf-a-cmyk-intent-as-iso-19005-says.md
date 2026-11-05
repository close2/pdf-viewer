Status: complete
Given: 2026-11-05, in conversation — transcribed by the round
Owes: none

Built as option 2: in a file whose XMP states `pdfaid:part` and whose PDF/A intent (`/S /GTS_PDFA1`)
is a CMYK profile, a `DeviceGray` colour with no `/DefaultGray` is the intent's black ink alone,
by every route a colour takes, soft-mask groups included; the converter writes no `/DefaultGray`
(ADRs 2712 and 2713).

> Q788: yes convert to black ink only

Reading: adopts section 6.2.4.3 in the reader. "Black ink only" is read as option 2: no `/DefaultGray`
is written, so the six soft masks of round 1947 and every archived grey under a CMYK intent draw
through black alone, as their sources do without an intent. Option 1's calibrated grey default
would have routed those greys through the profile again. The corpus measurement the question
asked for is the building round's evidence, not a precondition.
