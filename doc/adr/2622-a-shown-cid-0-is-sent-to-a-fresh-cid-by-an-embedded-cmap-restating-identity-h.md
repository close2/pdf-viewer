# 2622 — A shown CID 0 is sent to a fresh CID by an embedded CMap restating `Identity-H`

Status: implemented. Session 1894; the six documents convert at both targets and pass veraPDF at both (section 4).
Context: ISO 32000-2 §9.7.4.2, §9.7.4.3, §9.7.5.3 and §9.7.5.4, §9.7.6.2, §9.7.6.3, §9.10.2, Table 115, Table 116,
Table 118; ISO 19005-2 sections 6.2.11.3.1, 6.2.11.3.3 and 6.2.11.8; ISO 19005-4 section 6.2.10.9; ADRs 2342, 2556,
2557 (section 3, which this builds), 2582, 2598.
Code: `crates/pdf-transform/src/archive/fonts/fresh_cid.rs` (new: `planned`, `FreshCid`), `fonts/embedded_cmap.rs`
(new: `cid_cmap_stream`, `system_info`, moved from `font_files/glyph_indices.rs`, which calls them and writes the same
bytes), `fonts/cid_notdef_glyph.rs` (`given`, `map_restated`, `map_entries`, `proved`, `falls_back_to_cid_0`,
`FreshWritten`, `CID_0_SHOWN`, and `given`'s parts `fresh_planned`, `cid_set_restated`, `objects_written`,
`program_in_budget`), `fonts/cid_cff_notdef_glyph.rs` (`given`, `cids_asked`, `proved`, `CID_CFF_0_SHOWN`,
`fresh_planned`, `proved_for_every_font`), `fonts/truetype_cmap.rs` (`CmapRestated::fresh_cids`, `widths`,
`fresh_cid_sentence`), `rewrite/values.rs` (`send_cid_0_to_fresh_cid`), `fonts/notdef_glyph.rs` (`glyph_count`),
`crates/pdf-font/src/cff_cids_given.rs` (`highest_cid`), `crates/pdf-transform/tests/archive_corpus.rs`
(`width_statements`, `embedded_encodings`).

## 1. The construction

ADR 2557 kept the refusal for a code whose own CID is 0: §9.7.4.2 makes CID 0 the `CIDFont`'s `.notdef` character
whatever glyph draws it, and the code is the content stream's. What §9.7.6.2 reads the code through is the `Type0`
font's `/Encoding`, so that is what changes. `Identity-H` (Table 116: the two-byte code is the CID) is restated as an
embedded `CMap` (§9.7.5.3, Table 118) with the same one codespace, a `cidrange` per first byte as `Identity-H` states
them, code `<0000>` sent to a fresh CID, and a `notdefrange` to CID 0 (§9.7.5.4 item d). The fresh CID is one past the
highest CID the pages show, the program holds (the identity's glyph count or the map stream's entries for a
`CIDFontType2`, the charset for a `CIDFontType0`) and `/W` states; `/W` gains it at the width CID 0 had (§9.7.4.3).
It draws what CID 0 drew: under `CIDFontType2` the map sends it to CID 0's glyph, or to ADR 2556's copy where that
was glyph 0, the identity restated as a stream of each glyph's own index; under `CIDFontType0` ADR 2598's
`with_cids_given` gives it the copy of glyph 0. The content stream, the `ToUnicode` and `/CIDSystemInfo` are kept.

## 2. Where it stops

- **`Identity-H` only.** Another predefined `CMap` is restated with its own codespace and every mapping, `Identity-V`
  with `/W2`; neither is written, and the font keeps `CID_0_SHOWN` / `CID_CFF_0_SHOWN`, which now name the conditions.
- **§9.10.2.** A `ToUnicode` is keyed by code and does not move. Without one, the third method reads a CID of the
  Adobe GB1, CNS1, Japan1, Korea1 or KR collection as a character, and a fresh CID would read as one CID 0 never was:
  such a font keeps the refusal. The six documents are `Adobe-Identity-0`, five with a `ToUnicode`.
- **A code that reaches CID 0 by §9.7.6.3's fallbacks**, its own CID another, is no code of CID 0 and keeps the
  refusal (`falls_back_to_cid_0`), as before.
- **Bounds.** `/W` is read to 131 072 entries (`MOST_WIDTH_ENTRIES`, tested at the bound); the map to 65 536; the
  fresh CID must fit 16 bits; the `CMap` is a fixed 257 ranges.

## 3. The proof

The font is loaded again from the new encoding, widths, map, descriptor and program held in memory, and every shown
code must select the CID it selected — a code of CID 0 the fresh CID — draw the glyph and outline it drew at its
width, and not be drawn as CID 0. The plant (the `<0000>` range one CID off) fails all three fresh-CID tests.

## 4. Measured

The census against HEAD's, no-pack and with the CJK pack alike: every line of both profiles at both targets gains
three converted documents in each corpus and loses the six from both `shows CID 0` rows, except `as-shown` part 4 over
`corpora`, which gains two — `4387ba48….pdf` stays refused there on the `DeviceCMYK` row it already carried. One
document keeps a `shows CID 0` line, `issue11768_reduced.pdf`, which is not of the six: its `/Encoding` is not a name
and its descendant is `Adobe-Japan1` without `ToUnicode`, which section 2 leaves refused, and it is refused on
`cid-system-info-agrees-with-the-cmap` beside it. `rank.py` agrees (R03a's CID 0 line: one document, alone in none).
Each of the six, converted under `as-if-printed` at 2b and at 4, passes veraPDF 1.30.3 at its target (twelve of
twelve), states its fresh CID in `xmpMM:History`, and keeps its page count. The planted failure: the `<0000>` range of
two outputs (`issue7544.pdf`, a `CIDFontType2`; `PDFBOX-3062-005717-p1.pdf`, a `CIDFontType0`) written back to CID 0
after `qpdf --qdf`; veraPDF adds exactly 6.2.11.8 to the 6.1.7.1 the QDF rewrite costs the control, and the in-tree
checker, reading `drawn_cid`, refuses both on `fonts/no-notdef-glyph-shown` and passes both controls. The plant of
section 3 in the shipped binary: both documents refused (exit 4) and no file written. Memory, by `getrusage`, under a
`/W` of 131 073 widths on `issue7544.pdf`: HEAD 42 848 KiB, this round 44 864 KiB, both refused (exit 4) with nothing
written; on the document itself HEAD 12 976 KiB refused, this round 14 380 KiB converted.

The corpus gate's width check compared each `/W` whole, so the fresh CID's added width read as a width rewritten:
`/W` is now compared statement by statement (§9.7.4.3's `c [w …]` and `c_first c_last w` are independent), and a
converted file may add one `W` statement for each `Type0` font that gained an embedded `CMap`. Every statement the
source made must still be there. The file that showed it, veraPDF's own `6-2-11-8-t01-fail-c.pdf`, now converts at
2b and at 4 and passes veraPDF 1.30.3 at both.
