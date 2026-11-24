# 3569 — A repaired font program is written as this reader read it, and the program classes return to section 6.2.11.4.1

Session 2217. Status: **accepted** and built; extends ADRs 3256, 3566 and 3567. Context: ADR 3565
(the classes); ADRs 2636, 3375 and 3446 (the refile remedies and their proofs); the owner's PDF/A
stance — archive every PDF it can see, the elegant remedy first, else a stated loss; ISO 32000-2
§9.9, Table 124 and Table 125; ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1.

## Context

ADR 3567 left the program repairs (`sfnt-loca-extent`, `sfnt-loca-format`, `sfnt-loca-order`,
`sfnt-table-past-the-program`, `collection-cut-to-one-face`, `cff-font-dicts-rewritten`) without a
route: the program stream crossed byte for byte and the output's check refused seven pdf.js
documents by the class's name. ADR 3566 kept the classes off section 6.2.11.4.1's row because the
converter read that row's failure as a font with no program.

## Decision

1. **`pdf_font::as_read::program_as_read`** hands over the bytes the loader drew from — the one
   face of a collection, the `loca` repairs made, a CID-keyed CFF's Font DICTs made readable — and
   for an sfnt whose directory names a table past the program's end, the directory laid out again
   without those records (`sfnt::without_records_past_the_end`): `read-fonts` hands over no data
   for a range the bytes do not hold, so no glyph drew from one. It proves before returning: the
   bytes read back with no repair, and for an sfnt every glyph draws the same outline, point for
   point bit for bit through `skrifa`, at the same `hmtx` advance, selected through the same
   `cmap` bytes. For the `loca`, collection and CFF repairs the written bytes are the drawn ones,
   so the comparison holds by construction and the read-back is the proof; for the cut directory
   the comparison is what proves it.
2. **The converter writes it at the program's own object** (`archive::fonts::as_read`, beside ADR
   3256's program written whole): the producer's dictionary, one `FlateDecode`, `/Length1` for a
   `/FontFile2`, and the `/Subtype` Table 124 gives the bytes' format for a `/FontFile3`. No loss
   word: the window drew these bytes, and nothing is invented.
3. **The metrics route restates advances in the same bytes.** Where `fonts/widths-agree-with-the-
   program` also fails at the font, `restate_metrics` takes the program as read rather than the
   file's bytes, and its stream is the one written (`issue7074_reduced`).
4. **The six program classes breach section 6.2.11.4.1 again** (`pdf_archive::repairs::MAPPING`),
   read at the programs of the fonts a content stream renders (`record_logged_at`); the converter
   asks the row's own reading, `pdf_archive::program_read_repaired`, of the same descriptor.
   `font-file-subtype-disagrees` stays section 5.1's alone (ADR 3566): the filing row reads it,
   and the program route states the right `/Subtype` whenever it writes.
5. **Refused by name** (`NOT_WRITTEN_AS_READ`): a program the proof does not hold for, one the
   loader refuses, and bytes whose format Table 124 files under another key than the one used.

## Consequences

- Seven of the fifteen pdf.js documents ADR 3567 named convert at 2b with the program route alone
  (`issue11131_reduced`, `issue17671`, `issue2537r`, `issue3405r`, `issue7074_reduced`,
  `issue8480`, `issue9278`); `bug1650302_reduced` also needs ADR 3570.
- The `quorra-transform-worker` binary runs the conversion confined, so a check of a change to the
  converter or the validator builds `-p pdf-transform --bins`, not the one binary.
