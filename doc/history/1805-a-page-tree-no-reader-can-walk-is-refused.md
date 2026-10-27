# 1805 — A page tree no reader can walk is refused, and the appearance-only font is found unwitnessed

Session 1805, Slot 8 of batch one hundred and twenty, an archive round; 2026-10-27. ADRs 2444 and
2445; Q671 unused. I worked in `scratchpad/r1805/`. No ledger status moves.
**Read first.** The converter carried a page tree node as it stood, so `poppler-91414-0-53` and
`-54` (the root's `/Count` names a stream) and `poppler-937-0-fuzzed` (the root's `/Kids` is a bare
reference) converted at 2b and part 4 into files veraPDF says it cannot parse or validate.
**Built (ADR 2444).** `archive::page_tree`: before stage 1, every node `/Pages` reaches is read under
the reader's two bounds, and a node whose `/Kids` is missing or no array, or whose `/Count` is
missing or no integer, refuses the document as `Refusal::Reconstructed` with §7.7.3.2's Table 30
quoted and the object named. Restating the count — Table 30's NOTE asks it of a writer — is not
taken, by ADR 0947's second rule: no checked requirement fails, because section 5.1 is `Unchecked`;
a 5.1 row of its own is the route. **Hypothesis held** for the three; the census found three more
in `doc/corpora` — `T02-02_005_page-tree-no-kids`, `T02-02_008_page-tree-node-no-count` and
`PDFBOX-6040-nodeloop` (a missing `/Count`); at HEAD veraPDF errors on the first's outputs, cannot
parse the second's, and the third was already refused by a requirement.
**Census** (converted/conforming/refused/unreadable), HEAD → this: pdf.js `as-if-printed` 2b
789/2/165/11 → 786/2/165/11, 4 795/0/161/11 → 792/0/161/11; `as-shown` 2b 764 → 761, 4 771 → 768;
`doc/corpora` `as-if-printed` 2b 348/6/120/16 → 346/6/119/16, 4 354/0/121/16 → 352/0/120/16;
`as-shown` 2b 305/6/164 → 303/6/163, 4 305/0/170 → 303/0/169. Every document leaving a count is one
of the six, now printed with the verb's sentence; none joins a refusal list.
**Deferred (ADR 2445); two premises did not hold on the data.** A pikepdf scan of 1 477 files (62
unopenable) found 23 with an unembedded `Adobe-Identity` font, all in page content and none in an
appearance only; two plants calibrated its appearance route. A planted FreeText showing such a font
converts where its `/ToUnicode` answers (ADR 2378 is font-level) and keeps
`COMPOSITE_NOT_SUBSTITUTED` without one. None of the 24 photographed pdf.js conversions lists a
substituted font; a plant (a Helvetica drawing a space beside the unnamed font) does, and the report
and `xmpMM:History` then name a Helvetica the output does not carry.
**Proof.** Five unit tests plant each shape — stream `/Count`, missing and non-array `/Kids`, a
nested node, a leaf — and an indirect integer `/Count` passes. No output changed, so no veraPDF run
is owed on an output; `cargo tree -i pdf-transform` reaches no crate of the corpus binary, so
against HEAD's six arms nothing can move.
**Gates.** `rustfmt --check` on my file and `cargo fmt --check -p pdf-transform` exit 0; in an
export of HEAD and my hunks (a sibling's pdf-syntax edit failed clippy in the worktree): clippy
`-D warnings --all-targets` on pdf-transform exit 0, nextest 711 passed, 0 failed (a first run's
one red was trap 10's stale worker, gone once rebuilt); `tools/state.sh --round 1805 archive` exit
0 (validator missed 0, over 0 at six targets; every conforming input still conforming;
cross-check 13 560 compared, 0 disagreements); census `--tree 6` exit 0 before and after.
`cargo test -p conformance --no-fail-fast` exit 0, 40 suites ok. 5714 s.
