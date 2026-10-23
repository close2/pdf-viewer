# 2278 — The three rules the fitting refusal hid: two are the converter's, one the validator's

Session 1722, the archive slot of batch one hundred and eleven. Status: implemented. Context: ADRs
2146, 2147, 2179, 2262; `doc/history/1714`'s "Found, not fixed".

## What was found

Once ADR 2262 named `substitute-stretch`, three documents passed the font row and failed the
output's second verdict, and one converted output failed veraPDF 1.30.3's 6.8. Each was read
against ISO 19005-2 section 6.2.11.6 / ISO 19005-4 section 6.2.10.6, and section 6.8 / 6.9, before
the validator or the converter was touched (principle 5: veraPDF is evidence).

1. **`E3_Mayhew_et_al_2009` at 2b, a `Differences` name — the converter's.** Nine `Type1` Helvetica
   fonts state an encoding dictionary on `WinAnsiEncoding` whose `/Differences` writes `.notdef`
   over codes 0–31. Section 6.2.11.6 lets a non-symbolic TrueType font state a `Differences` array
   only of names the Adobe Glyph List holds, and `.notdef` is not one; a `Type1` font may state it.
   The converter restates the dictionary as `TrueType` for Liberation Sans and carried the encoding
   because its base was admitted. **Decision**: an admitted base is carried only where every
   `Differences` name is one `pdf_font::encoding::text_for` answers — the validator's own reading —
   and otherwise the encoding is restated on `WinAnsiEncoding`, which names no code `.notdef`. A
   code the source named `.notdef` then takes the base's name; no shown code can be one, since
   showing it would reference `.notdef` (section 6.2.11.8), and the outline proof holds every shown
   code to its glyph.
2. **`PDF 2.0 image with BPC` and `… with page level output intent` at 4, a symbolic descriptor's
   encoding — the converter's.** A `Type1` Helvetica with no `/Encoding` and its own descriptor
   flagged Symbolic (`/Flags 4`) was restated as `TrueType` with a `WinAnsiEncoding` dictionary
   beside that flag. The section forbids a symbolic TrueType font an `/Encoding`, and §9.6.5.4
   builds the code-to-name table only for a named encoding or a Nonsymbolic flag, so the output was
   read by this viewer through a table the standard's own procedure would not build. **Decision**:
   the descriptor copy the face is embedded into states Symbolic clear and Nonsymbolic set, every
   other bit the file's. Table 121 makes that the true pair for a Latin face reached through a Latin
   encoding — the reading the descriptor this program writes for a font that stated none already
   takes — and requires the two flags never both set nor both clear. The proof loads the font with
   those flags in place.
3. **`digitally_signed_3D_Portfolio` at 2b, section 6.8 — the validator's.** The output carried five
   embedded PDF documents that state no part of ISO 19005, which the validator passed over "for the
   weaker version" of the part-1 reason. Section 6.8 admits an embedded file on the condition that
   it complies with part 1 or 2, section 6.9 requires that it conform, and both parts this project
   holds require a conforming file to identify its part (sections 6.6.4 and 6.7.3). A document that
   identifies none has established nothing the condition rests on. **Decision**: such a document
   fails `embedded-files/embedded-file-is-itself-pdfa`; one that names part 1 or 3 is still passed
   over, the claim this crate cannot judge (`doc/questions/A17`). `as-if-printed` then discards the
   five, as its configuration says, and names each.

## Drawn, and checked

Page 1 of each source against its `as-if-printed` output, at 100 dpi and 288 dpi: the three font
documents differ by at most 1 level of 255 (the fitting `substitute-stretch` removes, named); the
portfolio's output is pixel for pixel the output HEAD wrote, the attachments being undrawn, and
both differ from the source by up to 64 levels where the existing DefaultCMYK remedy converts its
DeviceCMYK. veraPDF 1.30.3 passes all four outputs (2b, 2b, 4, 4). The census found a fifth
document of the first case, `3253-Article Text-6302-1-10-20190531` in `doc/corpora`, which now
converts at 2b and 4 under both profiles: its twelve pages differ from the source by
at most 1 level at 100 dpi, and veraPDF passes both outputs. ADR 2279 has the census.
