# 1890 — A glyph walked from a string's byte to three rasterisers, the sixth teaching document

Slot 10 of batch 129, 2026-10-29 and 2026-11-01, Opus 5.5, two sessions (the first stopped at the
account's weekly limit with no handover; the second finished from the tree it left), a docs round;
ADR 2614 used, ADR 2615 and Q766 unused. Ledger: no row moved — a walk changes no behaviour and
cites the clauses it reads.

**Premise.** The brief's hypothesis held: five stages, six diagrams. The walk describes commit
c53ad5e0 and says so; every claim was checked against `git show HEAD:<path>`, not the shared tree,
where sibling rounds were changing `crates/pdf-font/src/cff.rs`. One premise of the draft did not
hold: the width sources are chosen per font, not per code — a non-empty `/Widths` is the whole
table (`metrics.rs::simple_widths`), and the published metrics apply only to a substitute that
states none.

**Built.** `doc/teaching/how-a-glyph-reaches-the-screen.md`: a byte to a code and a glyph, the
program to an outline under `read-fonts`' operation budget, the outline cache and a refused glyph's
report, the width and the glyph's matrix, and the fill or stroke drawn by `render-cpu`,
`render-raster` and `render-gpu`. The running example is a one-page file using `Times-Roman` named
alone, filled on one line and stroked on the next, drawn with `render_at` and read back with
`readback` from a `dev` build of an export of HEAD; the readback's three lines are in the walk
verbatim. Corrected in the draft: the outline cache's bound (one glyph count per face, class faces
included), the Type 1 budget test's modest case (evaluated, not drawn), `render-raster`'s stroke
(the rasteriser widens from one device width, `kurbo` outlines only an anisotropic placement), the
whole diagram's report edge, the vertical-metrics sentence. `WALKS` gains the walk with six
diagrams; `doc/HANDOVER.md` and `doc/crate-map.md` point at it.

**Plant.** `dash_end.rs::extended_to_a_zero_dash_at_the_end` renamed `dash_ends.rs` in the walk:
the gate failed naming the absent path; restored byte for byte.

**Found, not fixed.** None in the code the walk reads.

**Gates.** `cargo test -p conformance --test teaching` exit 0 in the shared tree and exit 0 in the
HEAD export carrying the walk, `WALKS` and the two pointers; the plant exit 101; `--bin pointers`
exit 0 (none of the eight absent is the walk's), `--bin cited` exit 0, `--bin quotations` exit 0
(no divergence in the walk); `cargo test -p conformance --no-fail-fast` exit 101, 69 test binaries,
one red: `evidence.rs`'s `no_record_since_the_rule_rests_on_a_path_outside_the_tree` on round
1887's record naming its handover path, not this round's. Re-run after the ADR and this record:
exit 101, that red and `records.rs` on this record's unfinished gates sentence; corrected, `--test
records --test evidence --test teaching` exit 101 on the 1887 red alone, records and teaching ok.
