# 1483 — The book is ambiguous by the bound its references fail, and the gate says which

Slot 4 of batch seventy-six, 2026-10-09, an oracle round. ADR 1802; ADR 1803 and Q379 not used.
No ledger row moved. The clause reached is §10.7.4, a `departed` row whose status the reading
leaves as it is.

**Premise.** Held. `tools/state.sh oracle-held` printed what the brief quotes. The walk on HEAD
gave exactly the held counts (1026 / 47 / 836 / 46 / 8 / 2 / 3 over 1968 pages), and its
undiagnosed ranking was empty.

**Hypothesis.** Did not hold. No page has left a held group and none sits unheld, so the round read
the top ambiguous group's bound, which was the brief's fallback.

**Read.** `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`, 369 pages. Its note read our band against the 0.90
similarity, but the verdict rests on the references failing to form a pair. The gate's panels were
cropped to one size (`normalise`'s top-left rule) and measured pair by pair with
`examples/compare_rasters`. All 1107 voting pairs are past the 5.00% differing fraction, the
nearest at 5.03%. With that bound taken out, a pair forms on 313 of `freeculture`'s 317 pages and 9
of `pdkids`'s 52. So the book is ambiguous by ADR 0243's bound, and `pdkids` by a wider scatter;
§10.7.4's last sentence licenses both. ADR 0776 is why the bound stays where it is.

**Built.** `FormationStop` (on `RaisedFormation`, through `rejudged`) and
`what_keeps_the_dense_text_apart`, which prints the census for three groups: book 1107/1107 and
322; paper 543/543 and 103; document 177/177 and 44. The book's note cites that line, and its band
was re-derived: mean 2.39 to 8.70, differing 5.63% to 15.78%, ssim 0.7292 to 0.9652. The `oracle`
filter in `tools/state.sh` keeps the census lines.

**Gates.** `rustfmt --check --edition 2024` on `oracle.rs`: exit 0. `clippy -D warnings -p
pdf-model --all-targets`: exit 0, after a first try failed `too_many_lines` on `examine`.
`cargo nextest run -p pdf-model`: exit 0, 2027 passed and 19 skipped. `cargo test -p conformance`:
exit 0. `bash -n tools/state.sh`: exit 0. The oracle walk on HEAD (`--clock`, raw command) exited 0
after a 1591.9 s wait behind the arms export and a 111.7 s hold. After the change,
`tools/state.sh --round 1483 oracle` exited 0 after a 773.0 s wait and a 91.7 s hold, verdicts
unchanged; its census reproduced the scratch's 322, which is trap 13's calibration by a second
instrument. `tools/state.sh --round 1483 golden`: exit 0, held 974 and moved 0. The six arms were
not compared, since no drawing path changed. Two of my waits stopped in the queue at `lane=-`
(exit 143): one `pairs.sh` timed out, and I stopped an `oracle` run to widen its filter first.
Duration 6 300 s.
