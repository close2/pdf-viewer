# 1799 — A direct descendant is given its map and a null descendant is refused

Slot 2 of batch one hundred and twenty, 2026-10-27, the archive slot; ADRs 2432 and 2433; Q665
unused. I worked in `scratchpad/r1799/`. No ledger status moves; §9.7.4.2 and §9.7.6.1 stay
`implemented`.
**Built (ADR 2432).** A `CIDFontType2` written inside its Type 0 font's `/DescendantFonts`, embedded
and stating no `/CIDToGIDMap`, gets `Identity` written into the array's element, under ADR 2246's
edition test. `pdf_archive::direct_cid_fonts_stating_no_cid_to_gid_map` lists where each such font
is, and the converter accepts the row's findings at no object only when that list places all of them.
**Premise**: the brief's "where the font program's glyph order makes `/Identity` true" is not what the
remedy reads; ADR 2246 writes the first edition's default and opens no program, and this does too.
**Built (ADR 2433).** A new implemented row, `fonts/descendant-fonts-holds-one-cidfont`, under ISO
19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2: a used Type 0 font's `/DescendantFonts`
must be §9.7.6.1's one-element array holding a dictionary whose `/Subtype` is one of Table 115's two.
The converter refuses it by name (`NotBuiltYet`). **Premise**: my first version walked every font
dictionary and refused `PDFBOX-5784.pdf`, whose Type 0 font with no `/DescendantFonts` is named only
in the form's `/DR` and shown by no content; HEAD's outputs of it pass veraPDF at 2b and 4. The
clause binds fonts used, in any mode, so the row reads `Survey::fonts`, and that document is
converted as before.
**Census** (converted/conforming/refused/unreadable), HEAD → this, all eight lines unchanged: pdf.js
`as-if-printed` 2b 789/2/165/11, 4 795/0/161/11; `as-shown` 2b 764/2/190/11, 4 771/0/185/11;
`doc/corpora` 348/6/120/16, 354/0/121/16, 305/6/164/16, 305/0/170/16. Test-plusminus leaves the
refused lists in all four pdf.js lines; issue12823 joins them in all four; hesr13613 stays refused
and its cid-to-gid sentence now reads the narrowed one. No other line of the census differs.
**veraPDF 1.30.3**: this converter's four Test-plusminus outputs (2b and 4, both profiles) PASS;
HEAD's issue12823 outputs FAIL at 2b and 4, now refused; HEAD's PDFBOX-5784 outputs PASS, unchanged.
**Pixels**: Test-plusminus's page rendered from the source and from the 2b and 4 outputs has one
SHA-256 for all three, and the page shows its text and its ± sign.
**Proof**: the direct-descendant test fails with the direct write disabled in my export.
**Gates.** `rustfmt --check --edition 2024` on my nine files exit 0. In an export of HEAD with my
files and my hunks of the two shared ones: `cargo fmt --check` on pdf-archive and pdf-transform exit
0; `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on both exit 0; rustdoc `-D warnings
--document-private-items` on both exit 0; nextest pdf-archive 300 passed, pdf-transform 709 passed,
0 failed. The census `--tree 6` exit 0 before (HEAD's export) and after. `tools/state.sh --round
1799 archive` exit 0: validator missed 0, over 0 at all six targets; the converter's veraPDF-corpus
lines equal to HEAD's, as a sorted multiset, on everything but the four sentences above; cross-check
exit 0, 13 560 compared, 0 disagreements. The arms: the arm binary's build compiles neither
pdf-archive nor pdf-transform, so against HEAD's six arms nothing can move. `cargo test -p
conformance --no-fail-fast` exit 101 on two record tests, red at the last run on 1800's and 1802's
records, none on mine.
