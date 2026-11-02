# 2598 — A CID on `.notdef` in a CID-keyed CFF program is given a glyph by writing the charset, the FDSelect and the FDArray

Status: implemented and measured. Session 1882 (a `CIDFontType0` over `/FontFile3 /CIDFontType0C`; a Type 1 program
stays owed, ADR 2578 section 4).
Context: ISO 32000-2 §9.7.4.2, §9.7.6.3; ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1
and 6.2.10.9; Adobe Technical Note #5176 sections 5, 13, 18 and 19 and Table 10; A694 item 3; ADRs 2543, 2556, 2557,
2578 (section 4's first item).
Code: `crates/pdf-font/src/cff_cids_given.rs` (new: `with_cids_given`, `CidToGive`, `MOST_CID_BYTES_GIVEN`,
`read_back`), `cff_cids_given_tests.rs` (new), `cff.rs` (`Layout::with_glyph_tables` takes the charset and an optional
`FDSelect`, `font_dict_shifted`, `Moved::fd_select_at`; the name-keyed caller passes `None` and writes the same bytes),
`crates/pdf-transform/src/archive/fonts/cid_cff_notdef_glyph.rs` (new), `fonts/truetype_cmap.rs` (`cff_routed`,
`cid_cff_given`, `CmapRestated::cid_cff`, `cid_cff_sentence`, `NOTDEF_NOT_GIVEN`), `fonts/cid_notdef_glyph.rs`
(`cid_set_with` shared), `archive/rewrite/values.rs`, `tests/archive/cid_cff_notdef_glyph.rs` (new),
`fuzz/fuzz_targets/cff_write.rs` (`cid_keyed`, `cids_asked`), `fuzz/seed_streams.py`.

## 1. Decision

§9.7.4.2 sends a `CIDFontType0`'s CID to its glyph through the program's charset, so a shown CID the charset does not
hold reaches glyph 0 by §9.7.6.3, which ISO 19005-2 section 6.2.11.8 forbids. The remedy is ADR 2543's, written into
the program as ADR 2578 does for a simple font: for each such CID, one glyph whose charstring is glyph 0's with the
width operand restated to the advance the font's `/W` or `/DW` gives the CID, and the CID itself added to the charset.
No content stream, `CMap` or `/CIDToGIDMap` changes (a `CIDFontType0` has none), so no mark moves.

- **Under glyph 0's Font DICT.** A CID-keyed charstring is interpreted against the Private DICT and local subroutines
  its `FDSelect` entry names (TN #5176 section 19), so each copy is placed under glyph 0's. `FDSelect` is restated in
  the source's format (0, or 3 with ranges merged), the `FDArray` rewritten with each Private offset shifted by the
  one offset machinery `Layout` already holds, `CIDCount` raised where a CID passes it. The hypothesis held: the
  INDEX and offset machinery is shared, and `FDSelect` with the `FDArray` beside it is the only new table.
- **Read back before use** (trap 301): `read-fonts` opens the rebuilt program and every charstring, every glyph's CID
  and Font DICT, every Private DICT with its local subroutines, the global subroutines and `CIDCount` are compared.
- **One descendant, every font over it.** Several `Type0` fonts may share one descendant (`issue12963.pdf` has seven
  over one), and its one program serves them all: the CIDs asked for are the union, a CID shown at two widths is
  refused, and the proof (outline, advance, glyph, no drawn CID 0) runs for every font.
- **A shown CID 0 keeps the refusal** (ADR 2557), now under its own sentence (`CID_CFF_0_SHOWN`).
- **Bounds, checked before the work and per copy**: 65 535 glyphs, an INDEX's count, and 4 MiB of copied charstrings
  (`MOST_CID_BYTES_GIVEN`), each tested at and one past the bound.

## 2. Figures

- Census against HEAD's, both no-pack and pack: every count line identical. Three documents move reason and nothing
  moves status: `issue4650.pdf` and `issue11718_reduced.pdf` (pdf.js) and `PDFBOX-3062-005717-p1.pdf` (corpora) go
  from the TrueType `.notdef` sentence to `CID_CFF_0_SHOWN` under all four profile and target headings; each shows its
  own CID 0 through `Identity-H` (2, 1 and 7 codes). `issue12963.pdf` stays refused, its 199 places on glyph 0 past
  the checker's list of 32; with the list lifted it converts at both targets under both profiles, veraPDF passes all
  four outputs and fails the source on 6.2.11.8-1, and its eight pages render with no differing pixel.
- `getrusage` beside HEAD, a program of 30 000 glyphs with an `FDSelect` format 3 of 30 000 ranges and 35 000 CIDs at
  2 000 widths: HEAD refuses in 0.3 ms at 12 160 KiB peak; here 35 000 glyphs are added in 16 ms at 12 356 KiB, and
  35 536 (one past 65 535) are refused in 1.3 ms at 12 548 KiB.
- Plant: an `FDSelect` format 3 whose sentinel is one glyph short is refused by `read_back`, no bytes returned.
- `cff_write`, 600 s, 5.66 million runs on CID-keyed and Type1C seeds: no failure in the CID-keyed branch.

## 3. Not decided here

A shown CID 0 would take an embedded `CMap` sending the code to a fresh CID (ADR 2557). The cap on the checker's list
of places is its own work. No pixel of the viewer can move: the reader is untouched and the name-keyed writer's bytes
are the same, so the six arms were not owed.
