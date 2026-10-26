# 1780 — A page whose Adobe-Identity text no reader can name is photographed

Slot 2 of batch one hundred and eighteen, 2026-10-26, an archive round on A570; ADR 2394; ADR 2395
and Q646 unused. I worked in `scratchpad/r1780/`. No ledger status moves; the §9.7.5.2 note grows.
**Built (ADR 2394).** `archive::fonts::photographed`: an `Adobe-Identity` font the file does not
embed and no face can be keyed to (no `/ToUnicode`, or one re-keying refuses) is deferred; each
page whose content selects it (the interpreter's own resource ledger) is interpreted once without
its annotations, gated, drawn by the oracle backend at 300 dpi and replaced by one `ICCBased` sRGB
image placed through the inverse of `base_transform`. New loss `text-as-image` in the one font
decision; `as-if-printed` carries it through its `discard`, `as-shown` through a new row key
`text-as-image = "discard"` beside its `stop`. Offered at 2b and part 4, never 2u or 2a. Reported
per page (`Conversion::photographed`, JSON `photographed_pages`, the file's `xmpMM:History`).
**The gate** refuses by name: any character in the page's readback, any interpreter report but the
undrawn fonts', a structure tree or optional content in the document; a font with a map keeps
re-keying's sentence. **Premise held**: the population is 17 fonts with no map plus issue5801, and
the window draws them blank. **Hypothesis** held: of the 19 pdf.js documents (those and issue5244)
12 convert at 2b and 13 at 4 under each profile; the gate refused where text was readable —
ThuluthFeatures, issue13916, issue6127, issue7835, issue5244 — and on bug1365930's structure tree;
issue20453 stops at 2b on its metadata. **Census** (converted/conforming/refused/unreadable), HEAD →
this, pdf.js: `as-if-printed` 2b 778/2/176/11 → 790/2/164/11, 4 784/0/172/11 → 797/0/159/11;
`as-shown` 2b 753/2/201/11 → 765/2/189/11, 4 760/0/196/11 → 773/0/183/11; `doc/corpora` unchanged
in all four; by name the twelve or thirteen leave the lists and none joins.
**Drawn** (looked at): issue11915's box survives, its text blank as in the window; source against
output at 300 dpi 0 of 12 211 200 pixels differ, issue15443 0 of 174 306; at 100 dpi issue11915
9 858 of 1 357 200 differ, at most 105 — the resampling the loss names. veraPDF 1.30.3: 48 of 50
outputs PASS; issue20453 at 4 FAILs 6.7.2.1-4, -5 and 6.7.3-1 on the source's own packet (an
undeclared `dc:` prefix the part 4 rows accept), not on the page.
**Proof.** The rotation test planted (inverse dropped) fails at 68 levels; refusal, authorisation,
Level U and profile tests; a direct font matched by the ledger's own spelling.
**Traced (issue5244)**: UniJIS-UTF32-H reaches all twelve Polish letters, and the pack's Japan1
program has no outline at nine of those CIDs (Ą ą ć Ę ę Ł ł ź ż; `あ` at 843 draws), so the
drawn-glyph check is right; the pack fills a CID only from Source Han Sans JP's own `cmap`.
**Gates.** `rustfmt --check` on my files and `cargo fmt --check -p pdf-transform` exit 0; clippy
`-D warnings --all-targets` on pdf-transform exit 0; rustdoc `-D warnings --document-private-items`
exit 0; nextest 706 passed, 0 failed. In an export of HEAD and my hunks: `tools/state.sh --round
1780 archive` exit 0 (validator missed 0, over 0 at six targets; every conforming input still
conforming; cross-check 13 560 compared, 0 disagreements); census `--tree 6` exit 0 before and
after; veraPDF 48 PASS, 2 FAIL (above). The arms: `cargo tree -i pdf-transform` reaches no crate
of the corpus binary, so against HEAD's six arms nothing can move. `cargo test -p conformance
--no-fail-fast` exit 0 after this paragraph replaced the one red it had, which was mine. 6900 s.
