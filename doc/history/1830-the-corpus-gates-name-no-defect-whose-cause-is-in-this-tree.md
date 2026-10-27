# 1830 — The corpus gates name no defect whose cause is in this tree

Slot 8 of batch one hundred and twenty-three, 2026-10-27, a pixels slot (Sonnet, high); ADRs 2494 and
2495 and Q696 left unused. I worked in `scratchpad/r1830/`. No ledger status moves, no source file, no
held list and no line of `raster_golden.tsv` changes, so the six arms against HEAD's six arms move no page.
**Premise** (the head of the corpus ranking is one defect shared by several pages, fixed at one site):
did not hold. The corpus gate (`tools/state.sh corpus`) counts 59 incomplete documents of 974 and
classifies each by whose defect it is: the file 55, neither the file nor this reader 4, **this reader 0**.
The largest mechanisms are 24 show operators the page could not draw, 18 documents with an Identity CMap
over a font the file did not embed, 13 tokens §7.8.2 admits neither as operand nor as operator.
**Read against the clause, the head and the pages beside it.** The 18: §9.7.5.2 says the Identity-H and
Identity-V CMaps shall not be used with a non-embedded font, and §9.10.3's EXAMPLE 1 says that without a
ToUnicode "no information would be available about what the glyphs mean"; `issue12418_reduced.pdf` (the
oracle's ranking head, handed to this lane by round 1831) states a `/ToUnicode` that is the name
`/Identity-H`, so there is nothing to read the codes by and drawing nothing is the clause's answer. The
resource pages: `issue5954.pdf` (a page's own `/Resources` replaces the inherited one, §7.8.3, Table 31)
and `issue6541.pdf` (a pattern's `/R41` is not in the pattern's own resources). `checkbox_no_appearance.pdf`:
§12.7.5.2.3 puts the on state in `/AP` and states no default caption, so no mark is built (nothing is
invented). `issue19389.pdf`: Table 231's Password flag allows "asterisks or bullet characters", ours are
bullets, and the 0.3 pt clip of the capitals is the baseline choice §12.7.4.3 hands the processor, a 12 pt
`/DA` in an 11.1 pt box. `issue2017r.pdf`: the program's glyphs are named `uniF025` and so on, which the
Adobe Glyph List rule reads as U+F025; the reference's stripped offset is its own heuristic.
**The text gate** (`tools/state.sh text`): 501 of 510 judged documents fully in bounds, 12589 of 12631
matched words (99.67 %), and the nine out of bounds and the 22 under 90 % are each named in
`text_extraction.rs` with a reading. **The silence line** (168 codes over 8 documents) is `.notdef`
reached or an empty glyph, the reader's choice that ADRs 0270 and 0520 argue (`silent_fonts.rs`, `doc/todo/21` §3);
I read the documents only as far as `issue12963.pdf` (an OCR layer's hidden font), not all eight: `issue14821.pdf`'s 3
codes and `issue4650.pdf`'s 2 are unread.
**The cross-backend column** of HEAD's six arms (mean error against the CPU oracle): the largest is
`copy_paste_ligatures.pdf` on the cpu and gpu lanes, and the render-raster gate counts 968 of 968 agree.
That is a difference between two rasterisers over one display list, not a reading of a clause.
**Found, not fixed:** nothing in this tree. **Proposed:** the next corpus slot ranks by `tools/safedocs survey`'s residue (its top
rows: a group's blending colour space under §11.4.7, a font with no outline), the pdf.js list's in-tree head being empty. **For the orchestrator:** `raster_golden.tsv` — no line of mine moves; round 1831 named none.
**Gates.** `tools/state.sh --round 1830 corpus` exit 0 and the full `corpus` gate under `--lock` exit 0
(974 documents in 5.5 s, 59 incomplete, ratchet slack 0); `tools/state.sh --round 1830 text` exit 0 and
`text_extraction` on my export exit 0 (4 tests, 2 gates ran); the worktree's own `text_extraction` run exit
101 on a stale shared sandbox worker, so the figures are the export's. `cargo run -p conformance --bin cited` exit 0. `raster_golden`, the render-raster corpus gate and the
oracle clock gate: not run — no source changed; they stand on HEAD's arms and round 1831's walk. `cargo test -p conformance --no-fail-fast` exit 101, one red, `every_adr_since_the_rule_states_its_decision`,
on ADR 2490 (a `proposed` Status with no owner's question), slot 6's and not mine; every other test passed. The validator's citation findings were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on four citations in the record and PASS on the finding, the clause readings and the gates; the round fixed the findings.
