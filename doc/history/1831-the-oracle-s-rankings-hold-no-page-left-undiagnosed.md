# 1831 — The oracle's two rankings, read to their heads: no page is left undiagnosed

Slot 9 of batch one hundred and twenty-three, 2026-10-27, an oracle slot (Sonnet, high); ADRs 2496 and
2497 and Q697 left unused. I worked in `scratchpad/r1831/`. No ledger status moves, no source file and
no held constant changes.
**Premise** (the head of the bound ranking is a page this tree reports, its cause one clause misread):
the head is `xobject-image.pdf` page 1, 127.75x on the mean, and it is held by name, but the clause
does not convict the renderer. The walk (`tools/state.sh oracle`'s command, one `--clock` hold, 79 s, load
2.29 at the start) prints `ambiguous, undiagnosed` with no row, and "every one of the 47 pages is held by a
group by name", 0 as a departure of ours, 14 the references', 33 a choice the clause leaves open.
**Read against the clause, the head of each ranking.** `xobject-image.pdf`: Table 87 states `/Width 200
/Height 100` and the DCT frame is 1 x 1; §7.4.8 says the dimensions "shall be stored in the encoded data"
and that DCTDecode "may obtain the parameter values it requires directly from the encoded data", and
Table 87 and §8.9.4 count an image in samples; the clauses speak to a conforming file, so each of the four
references and this tree chooses, and ours draws the frame's sample and reports the contradiction
(`xobject-image.pdf` is on `corpus.rs`'s `INCOMPLETE`). `issue21346.pdf` (37.63x): §11.5.3's
CIE-based branch is the profile's `Y`, the three references take the device branch. The three JBIG2
pages: §7.4.7 hands the decoder to ISO/IEC 14492 and `tests/jbig2.rs` holds this tree to one image for
every coding of `bitmap-*`. `bug1743245.pdf` (4.05x on the other ranking): §10.7.5's last sentence is
what we obey and no reference reads the entry (the note's own re-render with `/SA` renamed), the coordinate half's fit declined by ADR 0848.
`bug766086.pdf`, `bug1552113.pdf`: §12.5.4 and §12.5.5, the divisor a gap two references share. Of the
13 `[widened: outside]` rows, `freeculture.pdf` page 1 alone is `AMBIGUOUS_IMAGE_REDUCTION` and rests
on the departure ADR 0025 records, which is the owner's to reopen; `freeculture.pdf` pages 315, 322, 323,
329 and 333 are `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` (the tolerance's design and a divisor shared through
freetype, ADRs 0647 and 0688), `issue16224.pdf` is `AMBIGUOUS_ONE_LADDER`, `issue4260_reduced.pdf` is
`AMBIGUOUS_ZERO_AREA_FILL` and `endchar.pdf` is `AMBIGUOUS_GLYPH_COVERAGE`, each diagnosed in its note.
**The one population that owes a diagnosis of its pixels** is `AMBIGUOUS_ON_A_PAGE_WE_REPORT` (26 pages):
the module comment says an incomplete page owes none, and `corpus.rs` holds each against its clause
(§9.7.5.2 Identity-H over a non-embedded face for eighteen of them, §7.7.3.4 for `issue5954.pdf`, whose
quoted sentence I re-read). Those are the file's by that gate's reading; the worst, `issue12418_reduced.pdf`
(mean 22.00, ssim 0.6318), is the corpus lane's.
**Found, not fixed:** nothing in the oracle's instrument or constants; the ranking above leaves them as
they were. **Proposed:** none. **For the orchestrator:** `raster_golden.tsv` — no line of mine moves.
**Gates.** The oracle walk exit 0, 3 passed, 79 s; `cargo run -p conformance --bin held` exit 0 (47
contradicted in 15 groups, 837 ambiguous; the walk's `ambiguous, undiagnosed` heading has no row); `--bin cited` exit 0; `cargo test --profile
gates -p pdf-model --test corpus -- --ignored --nocapture` exit 0, 1 passed; no `pdf-model` source
changed, so the six arms against HEAD's six arms move nothing (no export made). `cargo test -p conformance
--no-fail-fast` exit 101, one red, `every_table_designation_names_a_table_the_standard_captions`, on
`crates/pdf-jpx-write/` lines naming Table M.14, M.22, M.23 and M.25 — slot 1's, not mine. The validator's attribution findings were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on the record's attribution of six widened pages and PASS on the conclusion, which its own walk confirmed; the round fixed the finding.
