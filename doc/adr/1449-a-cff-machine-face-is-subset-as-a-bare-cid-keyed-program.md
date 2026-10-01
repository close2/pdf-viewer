# 1449 — A CFF machine face is subset as a bare CID-keyed program

Session 1307. Status: **accepted** and built.
Context: `crates/pdf-font/src/embed/cff.rs` (`subset`, `Closure`, `written`),
`crates/pdf-font/src/embed/reach.rs` (new), `crates/pdf-font/src/cff.rs` (`Parts`),
`crates/pdf-model/src/variable_text.rs` (`cff_font_for_a_file`),
`crates/pdf-model/tests/machine_cff_face_written.rs`.
Amends: ADR 1438 section 3 ("Not subset"), which it replaces where the face permits subsetting.
Clauses: ISO 32000-2 §9.9.1 (Tables 124, 125), §9.9.2, §9.7.4.2, Table 117.

## 1. Bare, under `CIDFontType0C`, not the `OpenType` wrapper

Table 124 admits a "Type 0 CIDFont program represented in the Compact Font Format (CFF)" under
`/FontFile3 /Subtype /CIDFontType0C` for a `CIDFontType0`. The `OpenType` row would also serve, but
it requires the `cmap` table beside `CFF `, and a subset's `cmap`, `hmtx`, `hhea` and `maxp` would
all have to be rebuilt for glyphs no reader of a `CIDFontType0` reaches through them: §9.7.4.2 goes
through the CFF charset and §9.7.4.3 through `/W`. The wrapper's one extra is `OS/2`'s `fsType`.
§9.9.1 makes the licence the writer's to check ("[o]ne of the conditions may be that the font program
cannot be embedded, in which case it should not be incorporated into a PDF file") and gives a
reader without it the narrowest use: "embedded font programs shall be used only to view and print
the document and not for any other purposes". `embed::for_embedding` checks restricted and
bitmap-only before anything is written. A face whose `fsType` forbids subsetting is still written
whole under `OpenType` (ADR 1438).

## 2. Always CID-keyed, CIDs are the face's glyph indices

The layout shows each glyph's index in the face as its CID, and the stream is written as drawn. The
subset is always written with CID-keyed Top DICT operators, and its charset gives each kept glyph
(renumbered densely from 0) its old index as its CID. Under §9.7.4.2's first case, "[t]he CIDs shall
be used to determine the GID value for the glyph procedure using the charset table in the CFF
program", so every CID reaches its glyph. Table 117 gives a `CIDFontType0` no `/CIDToGIDMap`, and none
is needed. A name-keyed face becomes one Font DICT holding its Private DICT. `ROS` is the face's own
where it is CID-keyed and its charset already gives every kept glyph its own index. Otherwise it is
`Adobe-Identity-0`. The subset route therefore no longer refuses a CID-keyed face that renumbers. The
whole route still does.

## 3. Subroutines: counts kept, unreached stubbed

Adobe Technical Note #5177 section 4.7 adds a bias to every call operand, and the bias depends on the
INDEX count (TN #5176 section 16). So a kept INDEX keeps its count. An unreached position becomes a
one-byte `endchar`, and no call in any kept charstring is rewritten. An INDEX with no reached entry
is written empty. The alternative, renumbering both the INDEX and every call operand, would rewrite
charstring bytes whose operand widths change, for a few kilobytes of saving. Reachability is found by
running each kept charstring as an interpreter does (`embed::reach`): one operand stack across calls,
stems counted so a `hintmask`'s data bytes are skipped, and calls followed to TN #5177's depth of ten.
An arithmetic, storage or reserved operator can compute a subroutine number, so it makes the walk keep
every subroutine: the subset is larger, but no glyph draws wrong. An accented-character `endchar`
names glyphs by standard code, which a CID-keyed program cannot carry, so that face is written whole.

## 4. Offsets

Every offset in the Top DICT and each Font DICT's `Private` is a five-byte integer. This is TN #5176's
way to fix a DICT's length before the offsets it states are known. The layout is measured once with
placeholders and written again with the real values at the same lengths. The order is TN #5176
section 2's: header, Name INDEX (the §9.9.2 name, which Table 117 says `/BaseFont` equals), Top DICT,
String INDEX (the face's `Notice`, `Copyright` and name strings carried), Global Subrs, charset
(format 0), `FDSelect` (format 0), `CharStrings`, `FDArray`, and then each Private DICT followed
directly by its local Subrs. `UniqueID`, `XUID` and `UIDBase` are dropped because they identify the
whole face.

## 5. Measured

Noto Sans Duployan Bold, the value `machine_cff_face_written.rs` saves (seven glyphs): the whole
`CFF ` table is 608 614 bytes, the subset 4 125, and the saved file 5 108 (it was 252 423 under ADR
1438). Source Code Pro, seven glyphs: 99 290 bytes whole, 3 953 subset. Every one of the machine's 77
`CFF ` faces was subset to all of its glyphs and to three, and every kept glyph's outline through
`pdf_font::cff::draw` was identical to the whole face's. Six variable faces carry `CFF2` and no `CFF `,
and they are refused as before (`NoOutlines`). The file's raster at 2x is byte-identical to the
screen's, and `qpdf --check` is clean.

## 6. Left

A face whose charstrings use the accented `endchar` is written whole rather than subset. The
arithmetic fallback keeps every subroutine, and no face on this machine meets it.
