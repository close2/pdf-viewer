# 1813 — CJK composite fonts converted with the pack installed

Slot 8 of batch one hundred and twenty-one, 2026-10-27, an archive slot; ADRs 2460 and 2461; Q679
unused. I worked in `scratchpad/r1813/`. No ledger status moves (ISO 19005 has no ledger rows).
**Premise** held: the census ran without `QUORRA_CJK_PACK`, and the thirteen pdf.js documents
refused with `PACK_ABSENT`'s sentence are that condition; not that `issue5244` is one of them — it
is an `Adobe-Identity` font, re-keying's (ADR 2378). **Hypothesis** (ten of the 13 convert with
the pack) did not hold: nine did on HEAD, eleven after ADR 2461.
**Measured** (a copy of CI's built pack, digests checked): with it HEAD frees nine under
`as-if-printed` at 2b and 4; `as-shown` stops all thirteen on `machine-face`, its own `stop`. Of the
other four, issue2128r and issue3521 stop on the fence row `bound-names-are-valid-utf8`;
90ms_rksj_h_sample's descendant states no descriptor and was refused with the sentence for an
inline one (trap 11); issue13343's three descendants share one descriptor at three `/DW`s.
**Built.** ADR 2460: every walk of `archive_corpus.rs` that converts prints first whether the pack
was present, and a named pack that does not answer stops it; the pack is read once per walk. ADR
2461: `pack::home_of` gives a descendant a descriptor of its own where it states none (built off
the programs: Table 120's entries, `Symbolic`, the subset's box and heights, the collection's `H`),
writes one inline, or names one holding a subset at other widths (copied); the shared-descriptor
refusal is gone. **Proof**: two pack fixtures in `tests/archive/cjk_pack.rs`, both failing against
HEAD's route with the two refusals the corpus met.
**Census** (converted/conforming/refused/unreadable), pdf.js `as-if-printed`, HEAD without the pack
→ HEAD with it → this with it: 2b 789/2/162/11 → 798/2/153/11 → 800/2/151/11; 4 792/0/161/11 →
801/0/152/11 → 803/0/150/11; `as-shown` and `doc/corpora` unchanged in every column; this tree
without the pack is HEAD's line for line; by name only the two leave, none joins.
**veraPDF 1.30.3**: the eleven at 2b and 4, 22 outputs, 19 PASS; three at 2b (issue11555,
noembed-eucjp, noembed-jis7) fail test 6.2.11.3.1-1, whose `CIDFontSupplement <= CMapSupplement`
reverses the clause (ADR 0922): each states supplement 2 over a CMap of 1, which the clause admits.
**Drawn**: every output page against its source at 100 dpi, 24 pages; the differences are the
pack's face inside the advances the dictionary states (`machine-face`), looked at on six documents.
**Found, not fixed**: issue13343's unused third font keeps the shared descriptor, whose `/FontName`
becomes the first subset's, as HEAD's rename already does.
**Gates.** In an export of HEAD and my hunks: `rustfmt --check --edition 2024` on my three files
exit 0; `cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-transform --all-targets` exit 0; rustdoc `-D warnings --document-private-items` exit 0;
nextest pdf-transform 716 passed, 0 failed; with the pack `--test archive cjk_pack` 3 passed and the
lib digest test 1 passed, exit 0. Census `--tree 6` exit 0 before and after, each with and without
the pack; `archive_corpus --ignored` with the pack exit 0, 7 passed; `cross_check --ignored` exit 0,
13 560 compared, 0 disagreements; `tools/state.sh --round 1813 archive` exit 0 (missed 0, over 0
at six targets; inputs that conformed still do); `cargo tree -i pdf-transform` reaches no corpus
binary, so against HEAD's six arms nothing moves. `cargo test -p conformance` exit 0. 7700 s.
