# 2102 — A Type 0 font naming no CIDFont, and a Type 3 code past LastChar, are given glyphs that draw nothing

Session 2102, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s next
remedies. ADRs 3109 and 3110; ledger rows 9.7.6.1 and 9.6.4 (`implemented`, before and after; the
first gains the construction and its test); no question.

**Built (a), ADR 3109.** `issue12823`'s `/FT19` (`/DescendantFonts [ null ]`, `Identity-H`). Round
2097's reading that the window does not move the text matrix was wrong: the interpreter places the
codes of a font it cannot load by `LoadedFont::metrics_only` (ADR 1094), each at Table 115's `/DW`
default of 1000 plus §9.4.4's spacings, and the two shows' `ET` is why emptying them changed no
pixel. So the written `CIDFontType2` moves by that width — one empty glyph restated to it, `/DW`
stating it, every CID mapped to it — and the content walk the brief asked for is not owed: `Tc`,
`Tw`, `TJ` and `Th` act alike on both paths, and §9.3.6 clips nothing for glyphs without
outlines. A survey record of spacings was built, then removed once the width was read. The route
is `embed_pass`'s, ahead of every other, reported and recorded on A48's condition
(`SubstitutedFont::descendant_written`); elsewhere refused by name. Converts at 2b and 4 under
both shipped profiles; veraPDF 1.30.3 passes all four outputs; page 1 at 150 and 300 dpi is
byte-identical to the source's.

**Built (b), ADR 3110.** `Type3Test` and `Type3WordSpacing` show code 32 below `/FirstChar` 97.
ADR 2918's empty procedure refused a code with no `/Widths` entry; Table 110 gives it 0, which this
reader lays it out by. Both convert at 2b and 4 under both profiles; veraPDF passes all eight
outputs (closing ADR 2652's 6.2.11.4.1-2 entry for them); page 1 at 300 dpi byte-identical.

**Census.** Re-taken by section 1's commands at this tree, with the pack and without, whose
census lines are the same. Against session 2097's, every line converts one more corpora document
(`Type3Test`) and two more pdf.js documents (`issue12823`, `Type3WordSpacing`) and refuses as many
fewer; no other document moved. R03a falls from 8 to 6 documents, R10 from 7 to 6.

**Tests.** `tests/archive/no_descendant.rs` (two: the page drawn pixel for pixel with a character
spacing and a later glyph, its control moving when the string is emptied; another `CMap` refused);
`tests/archive.rs`'s null-descendant test now asserts the remedy; `type3_glyphs.rs` gains the
out-of-range code under `3 Tw`; `truetype_cmap.rs`'s refused-anyway fixture moves to
`cmap-embedded-or-predefined`, the row it needs still refused by name.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after two lints of
1.100 in the new code); nextest `--lib --bins --tests` 0, 1255 passed, none expected; rustdoc 0;
`fuzz` `cargo check --bins` 0 (`SubstitutedFont` gained a public field); conformance 0.
