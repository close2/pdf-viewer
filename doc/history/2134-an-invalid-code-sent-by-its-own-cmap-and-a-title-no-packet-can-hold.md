# 2134 — An invalid code sent by its own CMap, and a title no packet can hold

Session 2134, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s long
tail: R03a's `issue11768_reduced`, R07's `poppler-742-0-fuzzed` at part 4, R10's
`issue_cff_unsigned_bbox`. ADRs 3237 to 3240; ledger rows 9.7.6.3, 9.7.5.3, 14.3.3 and 9.6.2.1
(`implemented`, before and after; each gains its code or test); no question.

**Built (R03a), ADRs 3237 and 3239.** Session 2127's construction: the font's embedded CMap gains a
one-code codespace range and a `cidchar` sending the invalid code to a fresh CID past every CID
`Adobe-Japan1-UCS2` maps, the CMap renamed; the proof now splits every shown string again with both
fonts and compares extracted text too. Two stops behind it, found by printing at each `None`: the
program's `hmtx` holds no bearing for 9 562 of its glyphs (read as 0, as `skrifa` reads it), and
veraPDF failed the restated `/CIDSet` at 6.2.11.4.2 — the source fails it too. veraPDF counts a CID
a map stream sends to glyph 0 as present, this tree does not, and Table 115 does not settle it, so
the route leaves `/CIDSet` out wherever its map has such a CID.

**Built (R07), ADR 3238.** The packet that "would not take" the removal's history entry was the
conversion's own fresh one, broken a step earlier: the fuzzed `/Title` holds U+0000 and section
6.1.3's move wrote it into `dc:title` unchecked. A Table 349 value with a character XML 1.0's
`Char` production excludes is now answered by the information rows' second step, as ADR 2681
answers a carried key.

**Not a defect (R10), ADR 3240.** `issue_cff_unsigned_bbox` already converted and passed veraPDF;
`read-fonts` reads a width stated in a subroutine as Technical Note #5177 places it. A test holds
the three shapes; R10's sentence is retired.

**Result.** All three convert at 2b and 4 under both shipped profiles: twelve outputs, veraPDF
1.30.3 passing every one, page 1 at 150 dpi pixel-identical to the source's. The five test fixture
outputs pass veraPDF.

**Census.** With the pack, pdf.js converts one more at 2b and two more at 4 on each profile's line;
corpora unchanged; nothing moved the other way. Sections 1, 2 and 9, the R03a, R07 and R10
entries, and `rank.py`'s three statuses restated; three mitigation entries.

**Gates.** Over `-p pdf-font -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0
(after a lint expectation the delegating fixture builder no longer met, two functions past 100
lines, `as_chunks` and checked arithmetic); `pdf-sandbox` build 0; nextest `--lib --bins
--tests` 0, 1724 passed, none expected; rustdoc 0; `fuzz` `cargo check --bins` 0 (`Code` gained
`in_codespace`); conformance 0.

