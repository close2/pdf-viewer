# 1772 — An Adobe-Identity font is re-keyed from its ToUnicode values

Slot 2 of batch one hundred and seventeen, 2026-10-26, an archive round on A570; ADR 2378; ADR 2379
and Q638 unused. I worked in `scratchpad/r1772/`. No ledger status moves; the §9.7.5.2 note grows.
**Built (ADR 2378).** `archive::fonts::rekeyed`: an `Adobe-Identity` composite font the file does
not embed, under `Identity-H`, whose `/ToUnicode` states one usable character for every shown code,
gets the glyphs a shipped face draws for those characters at the document's own CIDs — a `glyf` face
whole with a `/CIDToGIDMap` stream, a `CFF ` face as a subset whose charset carries the CIDs
(`pdf_font::embed::renumbered`, in the writer `pdf-font` already held). The face is the shipped one
for the name, else the CJK pack's first collection reaching every character. New loss
`glyphs-from-to-unicode` beside `machine-face` in the one font decision: `as-if-printed` discards,
`as-shown` stops. Content stream, encoding, `/W` and `/ToUnicode` untouched.
**Premise that did not hold**: A570 reads the population as CJK ("the shipped collection face"); of
pdf.js's 22 `Adobe-Identity` refusals the faces are Latin by name plus one Arabic, and the five with a
`/ToUnicode` show Latin, Cyrillic or Polish — so the base shipped face comes first, a departure from
the letter ADR 2378 states. Q570's "the row refuses the U+0007 and `#` maps" does not hold: the row
tests only >0, U+FEFF, U+FFFE; a value no face draws is refused here instead.
**Hypothesis** "most convert": three of five (issue2840, issue9084, issue20065); issue5244 and
issue5801 (a map stating none of its shown codes) refuse by their own sentences. "Pairs differ only
where the face differs": held. **Census** (converted/conforming/refused/unreadable), HEAD → this:
pdf.js `as-if-printed` 2b 784/2/170/11 → 787/2/167/11, 4 790/0/166/11 → 793/0/163/11; `as-shown`
2b 753/2/201/11 and 4 760/0/196/11 unchanged; `doc/corpora` unchanged in all four. By name the three
leave both `as-if-printed` lists and none joins any list.
**Drawn** (page 1, 100 dpi, looked at): issue2840 (Cyrillic, Liberation Sans Bold) 1 224 of 19 460
pixels differ, at most 255; issue9084 1 425 of 19 460; issue20065 (Foxit serif, renumbered CFF) 271
of 935 000, at most 62 — the same characters in place in the shipped shapes. veraPDF 1.30.3 PASS on
all six outputs, and on veraPDF's own `6-2-11-4-1-t01-fail-e` and `6-2-10-4-1-t01-fail-e`, now freed.
**The rasterized page (next archive slot)**: the 17 Identity fonts stating no `/ToUnicode`, and
issue5801; it needs the window's raster as an image-only page behind `text-as-image`, gated on no text
ever having been machine-readable, under both profiles.
**Proof.** `/CIDToGIDMap` read back per CID (planted off by one: fails); losses named unauthorised;
an unstated code refused; the pack's charset at the document's CIDs (ignored; passes with the pack).
**Gates.** `rustfmt --check` on my files, clippy `-D warnings --all-targets` on pdf-font and
pdf-transform exit 0; rustdoc `-D warnings --document-private-items` exit 0; nextest 998 passed, 0
failed. In an export of HEAD and my hunks: `tools/state.sh --round 1772 archive` exit 0 (validator
missed 0, over 0 at six targets; every conforming input still conforming; cross-check 13 560 compared,
0 disagreements); census `--tree 6` exit 0 before and after; cpu-1x arm against HEAD's six arms, 968
pages, 0 digests differ. `cargo test -p conformance --no-fail-fast` exit 101, one red, not mine (1774's
record). Found, not fixed: issue5244's letters all map through `UniJIS-UTF32-H`, yet a drawn-glyph
check refuses; not traced. 6500 s.
