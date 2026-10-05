# 1510 — JPX enumerations the held texts do not define are counted, not guessed, and §7.4.9 stays `partial`

Session 1337. Status: accepted. Builds on ADR 1383. Code: `crates/pdf-model/examples/jpx_colour_census.rs`.
Row: §7.4.9.

## 1. The question

§7.4.9 asks a reader to "support the JPX baseline set of enumerated colour spaces". T.801 Table
M.25 hands e-sRGB (20) and e-sYCC (24) to PIMA 7667, CIEJab (19) to CIE Publication 131, ROMM-RGB
(21) to PIMA 7666 and sYCC (18) to IEC 61966-2-1 Amd. 1. None of those is held; ITU-T T.4 Annex E,
held, gives no white point but D50's for a CIE Lab (14) under another illuminant. Does the row's
`partial` rest on anything a document in reach uses?

## 2. The census

`examples/jpx_colour_census` reads only the JP2 headers of every `JPXDecode` image, counting each
`colr` box by method and enumeration, overall and on images with no `/ColorSpace` — the only ones
where §7.4.9 lets the box decide ("the colour space specifications in the JPEG 2000 data shall be
ignored" otherwise). Population: every document under the pdf.js corpus, `doc/corpora/`,
`doc/corpora-own/`, `doc/*.pdf` and `corpus-cache/` (90 801 files), prefiltered by the bytes
`JPXDecode`, which an image dictionary cannot hide in an object stream: 1 791 files, 1 789 opened.

| specification | boxes | deciding | documents |
|---|---|---|---|
| 12 CMYK | 25 008 | 8 | 379 |
| 14 CIE Lab, D65 stated | 4 | 0 | 2 |
| 16 sRGB | 61 903 | 1 210 | 1 285 |
| 17 greyscale | 17 631 | 14 | 785 |
| 18 sYCC | 3 | 2 | 3 |
| method 2, 3, 4 | 8 034, 8 033, 603 | 0 | 262, 261, 57 |

105 108 images, 103 868 beside a `/ColorSpace`, 509 bare codestreams, 9 headers unread. **No box
states 19, 20, 21 or 24**; the two non-D50 Lab documents (`PDFBOX-3599-0.pdf`,
`poppler-LINK-613-0.pdf`) state a `/ColorSpace` over it.

## 3. The decision

The row stays `partial`: the undrawn enumerations are in scope, and the clause's support sentence
names the whole baseline set, not the set files use. What changes is the note: it states the census
and that the residue is a requirement no file in reach exercises, so the case for buying PIMA 7667
or CIE 131 is a census line that is not empty rather than an argument. No guess at a definition is
drawn; §7.4.9's own device fallback stands. sRGB's fixture already existed
(`srgb_is_drawn_as_its_samples`, a generated lossless codestream held to its own samples), so no new
codestream was made; the row now names its fixtures by function.
