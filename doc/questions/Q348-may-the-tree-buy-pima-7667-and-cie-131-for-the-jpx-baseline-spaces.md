# Q348 — May the tree buy PIMA 7667 and CIE 131 for the JPX baseline's last three spaces?

Asked by round 1452, a clause round of batch seventy-one. §7.4.9 is `partial` on one reason: its one
sentence addressed to a processor, "PDF processors shall support the JPX baseline set of enumerated
colour spaces", names three spaces this tree draws by §7.4.9's own device fallback because their
defining texts are not held. ITU-T T.801 (the held identical text of ISO/IEC 15444-2) hands each to
another publication in Table M.25 (paraphrased): e-sRGB (`EnumCS` 20) to PIMA 7667, e-sYCC (24) to
that standard's Annex B, and CIEJab (19) to the CIE's Colour Appearance Model 97s, CIE Publication
131.

## The two texts, read on 2026-10-08

- **PIMA 7667:2001**, *Extended sRGB color encoding — e-sRGB*. Sold by IS&T as a legacy standard
  (no longer maintained, offered so that legacy images can be decoded): a 271 KB PDF at USD 25.00,
  USD 20.00 to members. One text closes two of the three spaces, since e-sYCC is its Annex B.
- **CIE 131-1998**, *The CIE 1997 Interim Colour Appearance Model (simple version) CIECAM97s* (ISBN
  978 3 901906 29 9). The CIE's own page lists it under *Superseded and Archived Publications*,
  superseded by CIE 159:2004, and links no shop entry for it; the CIE's store (Accuris) and
  GlobalSpec refuse a scripted read (HTTP 403), so whether it is still sold, and at what price, is
  to be read in a browser. **It is still the text that defines Jab**: T.801 names CIECAM97s and CIE
  131 by name, and its successors define other models — CIE 159:2004 is CIECAM02, itself withdrawn
  in 2022 for CIE 248:2022's CIECAM16 (EUR 153.55 or 164.30 at DIN Media, by format) — whose `J`,
  `a` and `b` are computed differently, so neither can stand in for it. Only a TC draft of 131
  circulates freely, which is not the publication.

## What each would close, and the one count

PIMA 7667 would let `EnumCS` 20 and 24 be drawn as defined; CIE 131 would let 19 be. With both, the
row's one reason is gone and it can move to `implemented`; with one, it stays `partial` on the
other. **No document in reach exercises any of the three**: `examples/jpx_colour_census` over 90 801
files (1 789 with `JPXDecode`, 105 108 images) found no `colr` box stating 19, 20, 21 or 24 (ADR
1510), and §7.4.9's restriction keeps CIEJab out of conforming PDF data altogether, so 19 is a
support obligation for a file that already breaks the clause.

## The question

May the tree buy PIMA 7667:2001 from IS&T (USD 25), and CIE 131-1998 if the CIE or its store still
sells it, so that §7.4.9's three remaining enumerated spaces are drawn as their texts define them?

## Recommendation

**Buy PIMA 7667; buy CIE 131 only if it is sold as the publication.** PIMA 7667 is cheap, is the one
text for two spaces, and is offered for exactly this use. CIE 131 serves a space the clause excludes
from conforming data and no file in reach states, so it is worth buying for the row's completeness
and nothing else; if it is not sold, the row stays `partial` on 19 alone with that sentence.

## What the tree does meanwhile

The three spaces take §7.4.9's own fallback — DeviceGray, DeviceRGB or DeviceCMYK by the number of
ordinary channels — and §7.4.9 stays `partial` on them, as its note says.
