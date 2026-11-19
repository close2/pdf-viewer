# 2118 — A Type 1 program under a `CIDFontType0` is written as a CID-keyed CFF

Session 2118, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R10
entry for `issue11740_reduced`, building ADR 3155's construction. ADR 3173; ledger row 9.9.1
(`implemented`, before and after; it gains the converter, its code and four tests); no question.

**Built, ADR 3173.** `pdf_font::type1::as_cid_keyed_cff` runs each Type 1 charstring through the
interpreter the window draws with and writes what it draws as a Type 2 charstring of `rmoveto`,
`rlineto` and `rrcurveto` under the width `hsbw` stated (`embed::accented::Path`, ADR 1486's
writer): `callsubr`, `div`, flex and `seac` arrive as the points and paths they compute, `seac` as
its two components because a CID-keyed charset names no glyphs. The CFF is CID-keyed (`ROS` the
CIDFont's `/CIDSystemInfo`, charset format 2 and `FDSelect` format 3 sending CID *n* to glyph *n*,
one Font DICT, a Private DICT of `0 nominalWidthX`). Before it is returned it is read back through
`pdf_font::cff`, and every glyph's outline must equal the Type 1 draw bit for bit and its width the
stated one; a glyph that differs, does not run, or states no width is refused by index and name.
ADR 3155's operator-by-operator plan is departed from: the interpreter is the definition of what
this reader draws, so the converted glyph is that outline by construction. Stem hints are not
carried (ADR 3173 section 2). `pdf-transform`'s refiling (`font_files::as_cid_keyed`) writes it as
a whole stream under `/FontFile3 /CIDFontType0C`, and the `CIDFontType0` loses `/CIDToGIDMap`.

**Result.** `issue11740_reduced` converts under `as-shown` and `as-if-printed` at 2b and 4; veraPDF
1.30.3 passes all four, and page 1 drawn at 150 dpi is pixel-identical to the source's (ImageMagick
AE 0). The test fixture (a square, a `seac` accent, a flex through section 8.4's subroutines)
passes veraPDF at 2b and 4.

**Found.** A fixture glyph with no `hsbw` was written without a width; this tree's validator passed
it and veraPDF failed it at 6.2.11.5, because `pdf_font::cff::charstring_widths` reads an omitted
Type 2 width as none where TN #5177 gives `defaultWidthX` (ADR 3173 section 5). The converter now
refuses such a glyph; the validator's reading is left.

**Census.** Re-taken with the pack and without, over this tree: every pdf.js line converts one
more document (`issue11740_reduced`), corpora unchanged; without the pack `issue15053` stops at
R08b as in session 2106. R10 falls from 5 documents to 4. Sections 1, 2 and 9 restated, the R10
entry and `rank.py`'s R10 status; `doc/pdf-a-mitigations.md`'s refiling entry.

**Gates.** Over `-p pdf-font -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after
`doc_markdown`, `similar_names` and one `arithmetic_side_effects` in the new code); `pdf-sandbox`
build 0; nextest `--lib --bins --tests` 0, 1707 passed, none expected; rustdoc 0; conformance 0 (476 passed).
