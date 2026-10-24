# 1730 — A placeholder leaves the producer's CMap at part 4, the fence is damage, and two veraPDF disagreements were ours

Slot 3 of batch one hundred and twelve, 2026-10-23, an archive round. ADRs 2294, 2295; no question.
**Read first.** The fence's 33: 28 `govdocs1-error-pdfs`, four malformed tests, one JHOVE error file
— damaged streams §7.8.2 makes an error; the fence stands, named (ADR 2294 section 2). The 32
`to-unicode-values-are-usable` fonts: all composite, 13 identity spans, 19 single codes to U+0000/FEFF.
**Built.** (1) ADR 2294: where §9.10.2 derives nothing, the producer's `CMap` is restated without its
placeholder statements — spans split, every other byte kept, proved code by code through
`pdf_font`'s reader — at part 4, whose mapping is a recommendation, and at part 2 Level U/A only
where no shown code loses its mapping. (2) ADR 2295: `metadata/properties-use-known-schemas` holds
an `xmpMM` structure to the 2005 `ResourceEvent`, `ResourceRef` and `Version` fields (veraPDF's
6.6.2.3.1 on `stEvt:changed` and `stRef:originalDocumentID`), one finding per property, and the cut
walks every metadata stream where the row's findings were capped; a stream with no profile file
signature is short of it at part 2 (veraPDF's 6.2.4.2), answered by the existing alternate route.
**Drawn** (100 dpi): `me450f12project10_report` pages 156–157 byte-identical to HEAD's output; 26
part 4 outputs page 1 within one level of the source; three looked at. veraPDF 1.30.3: `me450`,
`s41589-021-00892-6`, `102952` 2b PASS (HEAD's `s41589` failed 6.6.2.3.1); 21 of 26 part 4 PASS.
**Census** (both profiles, against HEAD's export, converted / conforming / refused / unreadable):
part 4 pdf.js 723/0/233/11 → 751/0/205/11 and 704/0/252/11 → 732/0/224/11; `doc/corpora` 336/0/139/16
→ 339/0/136/16 and 290/0/185/16 → 292/0/183/16; 2b pdf.js unchanged, `doc/corpora` 343/6/125/16 →
344/6/124/16 and 302/6/167/16 → 303/6/166/16 (HEAD's findings cap, ADR 2295). Nothing newly refused;
the first build refused two on that cap, and the cut now reaches past it.
**Hypotheses.** "The fence is one producer's unbalanced `q`/`Q`": did not hold, it is damage. "A
third of the 32 restate from glyph names": did not hold, none — all composite; the clause's last
sentence answers them instead. "One of the two veraPDF rules is ours": both were. **Premise that
did not hold**: the brief's 19005-4 section 6.2.10.5 is font metrics; the rule is 6.2.10.7.
**Gates.** Worktree: `rustfmt --check` on my seven Rust files exit 0. Export of this slot alone
(habit 138; a sibling's worktree build gives the confined worker SIGSYS): `cargo fmt --check` exit 0;
`-D warnings` clippy `--all-targets` on pdf-archive and pdf-transform exit 0; nextest 961 passed, 0
failed, 16 skipped; rustdoc `-D warnings` (and `--document-private-items`) on both exit 0. Plant
(each behaviour taken out, these tests kept): the three new tests fail (trap 13). `tools/state.sh
--round 1730 archive` exit 0 in both exports: validator over the veraPDF corpus identical, missed 0,
over 0; converter over veraPDF part 4 267 → 270, the rest identical; cross-check 13 560 compared, 0
disagreements. Census alone exit 0 in both. `t3-foreign_corpus` not run (it reads back the five
writers); arms not compared (`cargo tree -p render-raster` reaches neither crate); `tools/state.sh
docs`' workspace walk stopped in its queue, the scoped rustdoc standing for it. `cargo test -p
conformance`: 463 passed, 1 failed, a sibling's record (1728); `evidence`, `adr_status` exit 0.
**Found, not fixed.** Five part 4 outputs fail veraPDF's 6.2.10.4.1 (a `FontFile3` with no
`Subtype`, which this validator passes); the other 2005 structured XMP types are not held to their
fields; part 4's tag row reads a stream that is no profile as clean (`doc/todo/72`). 8439 s.
