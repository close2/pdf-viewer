# 2181 — A form drawing from its page, CFF reals with no digit, an empty program, and a bfchar encoding

Session 2181, an archive round in the `lane3` worktree: ADR 3412's five veraPDF items. ADRs 3425 to
3428; no question. Ledger rows 7.8.3, 9.7.4.2, 9.7.5.4, 9.8.1 and 9.9 (`implemented` before and
after).

**`issue6541` (ADR 3425).** §7.8.3 has a form or Type 3 font stating no `/Resources` inherit "from
the resource dictionary of the page on which they are used"; `reach::Lender` adds the names of every
such stream drawn on a page, at any depth and through annotation appearances, to that page's owner
names, so the exemption no longer withdraws findings at the entries they draw.

**`bug1068432`, `bug1308536` (ADR 3426).** New row `fonts/compact-font-reals-state-a-number`
(6.2.11.2 / 6.2.10.2, Technical Note #5176 section 4): a bare CFF's Top, Font or Private DICT real
with no digit before its exponent or end. `Rewrite::CompactFontRealsRestated` writes the zero
`read-fonts` reads, as zero digits ended by 0xf in the operand's own bytes; every glyph's outline is
compared before and after. Both outputs pass veraPDF; page 1 renders identical (AE 0).

**`bug866395` (ADR 3427), ours.** Its `/FontFile3` is a valid Flate stream of no bytes; `pdf_font`
substitutes, so no `Malformed` refusal reached `font-programs-embedded`. `program_unparsed` now
reports an empty program, and the converter embeds the face the window draws. Both outputs pass
veraPDF; page 1 draws the same text in the embedded face (210 and 633 pixels differ at 72 dpi).

**`bug920426` (ADR 3428 item 1).** New row `fonts/encoding-cmap-maps-codes-to-cids` (§9.7.5.4 c));
`Rewrite::EncodingCmapRestated` writes each `beginbfchar` section as `begincidchar` with the CID
`pdf_font::cmap` read, refusing a glyph-name destination or any code selecting another CID. Both
outputs pass veraPDF; renders identical.

**Left (ADR 3428 item 2).** `issue7901`: §9.7.4.2 uses a name-keyed CFF's CIDs as glyph indices and
Table 115 gives `CIDToGIDMap` to Type 2; the remedy (a CID-keyed rewrite whose charset states the
map) is named and not built.

**Census.** With and without the pack, every line equals 2177's and the kept outputs are 2177's path
for path; only the four documents' outputs change size (2602 others differ only in per-run
identifiers). veraPDF: 5516 written as conforming, 63 failing outputs of 19 documents over
16 rules (was 79, 23, 18), 20 unfinished; the four documents above left the list and none joined it.

**Gates.** Over `-p pdf-archive -p pdf-transform -p pdf-font -p viewer-qt`: fmt 0; clippy 0;
`pdf-sandbox` build 0; nextest 2177 passed, 0 failed (manifest difference none); rustdoc 0; `fuzz`
check 0; conformance 478 passed, 0 failed.
