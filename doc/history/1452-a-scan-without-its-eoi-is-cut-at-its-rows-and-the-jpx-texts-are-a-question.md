# 1452 — A scan without its `EOI` is cut at its rows, and the JPX texts are a question

Slot 3 of batch seventy-one, 2026-10-08, a clause round. ADR 1740; ADR 1741 not used; Q348 filed.
No row changed status: §7.4.8 stays `departed` on Table 13's default alone, §7.4.9 stays `partial`
on its three unheld texts, and §7.4 is their aggregate. Every `partial` leaf waits on an answer or a
purchase, so this round worked a `departed` row as the brief said.

**Premises.** Three held. `image/cut.rs` declined a scan with no `EOI` on `scan.ends`. The fork
makes the whole decoder read a complete last row as the frame. ADR 1495's grey frame is then cut
and equals its whole decode. Two did not hold. §7.4.8's note never named the refusal: the decline
only chose a schedule and moved no pixel. So the note gains one sentence, that scheduling is no
second departure. And the "no `colr` box states 19, 20, 21 or 24" census is ADR 1510's, not ADR
1713's.

**The lift (ADR 1740).** The pass admits a scan only when the data holds its last MCU's bits, so
the decline came off. The module comments of `cut.rs` and `restart.rs` now say why. The test that
pinned the refusal is rewritten to show the cut. A new test takes each row-cut fixture without its
`EOI` and ends it at every byte of its last 1 024, or carries five tails on past it: every result is
refused or the whole frame. Two planted patches made the tests fail: the decline put back fails
both, and the end-of-data check taken out fails the truncation test. A temporary census, since
deleted, read the 179 corpus codestreams without their `EOI`: 57 cut with the marker and the same 57
without it, 0 different, 0 cut where the whole decoder refuses, and 134 whole decodes unmoved by the
marker. With the decline planted it fell to 0 cut.

**Q348.** IS&T sells PIMA 7667 as a legacy PDF at USD 25 (USD 20 to members); it covers e-sRGB and
e-sYCC. The CIE lists CIE 131 as superseded and archived and links no shop entry. Its stores refuse
a scripted read (HTTP 403), so the owner checks the price. CIE 131 still defines Jab, because its
successors CIE 159 and 248 are other models. The §7.4.9 note's availability sentence and
`doc/todo/65`'s entry now say this.

**Unfinished.** The six arms were not run: a cut gives the whole decoder's bytes or declines, and
both gates below show no page moved. `jpeg_bands` campaigns belong to slot 6.

**Gates.** `rustfmt --check` on the three files: exit 0. `cargo nextest run -p pdf-model`: 2004
passed, 18 skipped. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets`: exit 0; once
exit 101 on a sibling's mid-edit `appearance.rs`. `cargo test -p conformance`: exit 0, 421 passed;
last run exit 101 on one test, `records`, which names round 1450's half-written record. Behind the
lock, `--tree 6` with the worker built inside the hold: `corpus` exit 0 over 974 documents in 26 s,
1.38 GiB; `raster_golden` exit 0, 974 held and 0 moved, 30 s, 3.31 GiB; the extraction exit 0, 47 s.
