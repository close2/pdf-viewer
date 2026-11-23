# 3445 — A text-showing operator with no font uses the fill colour space

Session 2186. Status: **accepted** and built. Context: ISO 32000-2 §9.3.1, §9.3.6; ISO 19005-2
section 6.2.4.3, ISO 19005-4 section 6.2.4.3; `pdf_archive::survey`.

## Reading

`T02-05-01_004` and `_008` show text in a text object that ran no `Tf` (one names no font, one
states a size with no operator). §9.3.1 gives the font parameter no initial value, so this reader
draws no glyph and says the page is incomplete. The survey returned before recording the paint,
so `graphics/device-gray-needs-a-default-or-an-output-intent` saw no use of the initial
`DeviceGray`, and veraPDF failed 6.2.4.3 on all four outputs of each.

Section 6.2.4.3 restricts the colour space a painting operator *uses*. §9.3.6 makes a text-showing
operator fill or stroke as its rendering mode says, with the font deciding only the shapes; a
reader that supplies a face where none was selected paints in the fill colour space in force. So
the use is the operator's, as it already is for `B` with no current path, and only mode 3, which
paints nothing, uses none.

## Decision

`Walk::show` records the paint whether or not a font is selected. The converter's existing
remedy for the initial `DeviceGray` (the output intent `as-shown` states) now reaches both
documents. `PDFBOX-3148-2-fuzzed`, the third document veraPDF fails at 6.2.4.3, is another case:
its widget appearance is `ASCII85Decode` data with no `~>`, which this reader decodes to `/Tx `
and veraPDF to `/Tx B` (section 1 of `doc/pdfa-blocking-issues.md`).
