# 2123 — A TrueType font a photograph takes out of the file needs no route

Session 2123, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R11
(ISO 19005-2 section 6.2.11.6), the next remedy by rank after R06b. ADR 3193; ledger row 7.7.3.4
(`implemented`, before and after; it gains `photographed.rs` and one test); no question.

**Found.** A debugging print at every refusal of `truetype_cmap::restate_one`, over R11's seven
documents: in the three R11 stopped alone (`issue20489`, `issue13316_reduced`,
`sample_fonts_solidconvertor`) the font the restatement refused (no program as written, or one this
reader does not draw from) was one the substitution had photographed the page around, so the row
was refused over fonts no object of the output holds. `SimFang-variant` and `XiaoBiaoSong` are
refused by the substitution first (R10's descriptor inside the font), so no route here reaches
them; `507676` stops on five other rows; `issue6127` is `as-shown`'s R08 stop.

**Built, ADR 3193.** `Substitutes::photographed_away` (the fonts the photographs take out of the
output, `photographed::taken_away`); `truetype_cmap::restated` passes over them; `decision::decide`
answers each of R11's seven rows failing only at such fonts by the photograph, at the losses it
already takes. A page tree node whose every page is photographed is written without the
`/Resources` no page inherits (ISO 32000-2 §7.7.3.4; `Substitutes::inherited_by_none`), and the
report's walk, the 6.1.8 name walk and the writer drop the same keys through
`Substitutes::keys_away`. Two tests, each failing on the tree before.

**Result.** The three convert under `as-if-printed` at 2b and 4 and pass veraPDF 1.30.3 at both;
`sample_fonts_solidconvertor` under `as-shown` converts as before and passes too. Renders: the
photographed pages differ from the source only by the 300 dpi photograph's resampling and the
non-printing annotation `as-if-printed` removes; `issue13316_reduced` is pixel-identical.

**Census.** Re-taken with the pack and without, over this tree: `as-if-printed` converts two more
pdf.js documents and one more corpora document at each part, `as-shown` unchanged; without the pack
`issue15053` stops at R08b as in session 2106. R11 under `as-if-printed` falls from six documents
to three. Sections 1, 2 and 9, the R11 entry and `rank.py`'s R11 status restated;
`doc/pdf-a-mitigations.md`'s restated-`cmap` entry.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after
`too_many_lines` on `restated`); `pdf-sandbox` build 0; nextest `--lib --bins --tests` 0, 1268
passed, none expected; rustdoc 0; conformance 0 (476 passed).
