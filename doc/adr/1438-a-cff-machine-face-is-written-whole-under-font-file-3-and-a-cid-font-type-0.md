# 1438 — A CFF machine face is written whole under `/FontFile3` and a `CIDFontType0`

Session 1301. Status: **accepted** and built.
Context: `crates/pdf-font/src/embed.rs` (`Outlines`, `SystemInfo`, `Refusal::NoOutlines`,
`Refusal::CidsAreNotGlyphs`), `crates/pdf-font/src/embed/cff.rs` (new),
`crates/pdf-model/src/variable_text.rs` (`cff_font_for_a_file`),
`crates/pdf-model/tests/machine_cff_face_written.rs` (new).
Amends: ADR 1425, whose `Refusal::NotTrueType` refused every face without `glyf`.
Clauses: ISO 32000-2 §9.9.1 (Tables 124, 125), §9.9.2, §9.7.4.2, Table 117, Table 119.

## 1. Which row of Table 124

Table 124's `OpenType` row for `/FontFile3` places a program with a `CFF ` table under a
`CIDFontType0` whether or not its Top DICT uses CIDFont operators, and says of both cases: "In
addition to the "CFF " table, the font program shall include the "cmap" table." Table 125's
`/Subtype` is "OpenType for OpenType fonts". So a `CFF ` machine face is written as an OpenType
program (`OTTO`) under `/FontFile3 /Subtype /OpenType`, and the descendant becomes a
`CIDFontType0`: `/CIDToGIDMap` is dropped (Table 117 states it for Type 2 only), `/Length1` is not
written, and the Type 0 font's `/BaseFont` is the CIDFont's name, a hyphen and `Identity-H`, as
Table 119 recommends for a Type 0 descendant.

## 2. Whether the stream's CIDs still reach their glyphs

The layout draws each glyph's index as its CID and the stream is written as drawn. §9.7.4.2: for a
Top DICT without CIDFont operators "[t]he CIDs shall be used directly as GID values" — always
right here. For a CID-keyed one the CID goes through the charset, so the write checks every glyph
the value shows has its own index as its CID, and refuses by name
(`Refusal::CidsAreNotGlyphs`) where one does not; the alternative, an embedded CMap as the Type 0
`/Encoding`, is not built. A CID-keyed program's `ROS` becomes `/CIDSystemInfo`, which §9.7.4.2
says "should be copied into the PDF CIDFont dictionary".

## 3. Not subset, and what that costs

Subsetting CFF means rebuilding the `CharStrings` INDEX, the charset, a CID-keyed face's
`FDSelect` and the subroutines the kept charstrings call. That is a separate job. The `CFF ` table
is carried whole with `cmap`, `head`, `hhea`, `hmtx`, `maxp`, `OS/2` and `post`; layout,
vertical and variation tables are dropped. Measured on this machine: Source Sans 3 Regular writes
190 221 bytes (162 621 of it `CFF `); Noto Sans Duployan 631 423 (595 731). The fixture's saved
file is 252 423 bytes after Flate, and it shows five glyphs. With no subset there is no §9.9.2 tag,
so the name is the face's PostScript name. `fsType`'s restricted and bitmap-only bits refuse, as
for `glyf`; its no-subsetting bit asks for nothing more, because nothing is subset.

## 4. The fixture, and why it is not Arabic here

The contract's instrument, `fc-list ':fontformat=CFF:lang=ar'`, finds no face on this machine. The
compiled-in Helvetica draws Latin, Greek and Cyrillic, so none of those values reaches a machine
face. `machine_cff_face_written.rs` therefore turns the machine's fonts off, hands the layout one
face through `pdf_font::provider`'s port (the confined worker's own route), and chooses an Arabic
`CFF ` face where one exists, otherwise one covering a Duployan value (Noto Sans Duployan here). If
neither exists it skips with ADR 1154's sentence. The test saves the value, runs `qpdf --check`
(clean), stops the port, re-opens the file and asserts `CIDFontType0`, `/FontFile3 /OpenType`, no
`/CIDToGIDMap` and no `/Length1`. The page is then drawn from the file with the port asked zero
times, and it is byte-identical to the screen at 2x. Truncating the written program to 2000 bytes
makes it fail. `embed::cff`'s unit test takes this machine's first `OTTO` face and checks the
tables, the `OTTO` version, the identity glyph map and the charmap an independent reader
(`skrifa`) finds.

## 5. Left

CFF subsetting. A CID-keyed face whose charset renumbers (an embedded CMap would carry it). And
the drawing path still describes every machine face as a `CIDFontType2` in memory (ADR 1414),
which a `CFF ` program is not. It draws correctly because the loader goes by the program, but the
in-memory dictionary is not a truthful description of that program.
