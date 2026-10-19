# 1564 — A transfer function is the operator's loss, and a halftone's name a press's

Slot 1 of batch ninety, 2026-10-15, an archive round. ADRs 1964, 1965; no Q.

**The premise held; the hypothesis half held.** The census (`examples/archive_unbuilt`, 1 450 of
1 477 pdf.js and corpora documents) re-ranked `doc/todo/72` as 1558 left it: 36 sites unbuilt at
2b, `fonts/embedded-programs-define-every-glyph-shown` 29 (fence) — **29 after ADR 1943's route**,
the figure 1553 printed; the validator asks `pdf_font`'s own `glyph_index`, so the route reaches it
and moved none. The transfer-function site was no longer in the census: ADR 1952's `Stated` row is
not a site. **The `discard` hypothesis did not hold.** Rendered (trap 182), `issue6931_reduced.pdf`
draws its logo red under its four sampled functions and a near-black negative without them: a
graphics state's `/TR` replaces the device's initial transfer on a printer as on a screen (§8.4.1
Table 52, §10.5), so ADR 1952 section 3 (b)'s premise, that a press replaces it, is false.

**Built.** `pdf-transform`: `/TR` other than `Identity` is `Loss::TransferFunction` (per-document,
ADR 1210's shape): each function written into the output's `xmpMM:History` in clause 7's syntax,
data decoded, before it goes, and named in the report; the record is a condition. Both shipped
profiles say `stop` (ADR 1964). `HalftoneName` is `Loss::HalftoneName` — a press's pointer, nothing
a screen draws (§10.6.1, §10.6.5.1); `as-shown` says `discard` and `endchar.pdf` converts (ADR
1965). The census example takes `--who <site>` and names each site's witnesses. The identity
sentence cites Table 52, which ADR 1952 called 53. Next ranked: OPM 1 (`102952.pdf`; this reader
simulates OPM, `overprint.rs`, so OPM 0 can move pixels) and ICC tags (`issue14814.pdf`).

**Pixels.** `endchar.pdf` page 1 at 72 and 720 dpi before and after: at most 7 of 255, all the
GRACoL output intent's (the report names it). veraPDF `6-2-5-t04` (name): identical. `6-2-5-t01`
(function, authorised): eyes black to white, 0.03 % of pixels — the loss, stated. veraPDF 1.30.3:
all three outputs compliant.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-transform
--all-targets`: exit 0. `nextest -p pdf-transform`: 511 passed, 0 failed. `cargo test -p
conformance`, this record in place: 435 passed, 1 failed — `records`, on sibling 1567's 43-line
record, not this round's. `tools/state.sh docs`: exit 0. One `--tree 12` hold, 285 s:
`t3-archive_corpus` exit 0 (authorised run 2b: 481 converted, 129 refused; 4: 222, 137 — a control
without the two losses, applied and reversed as a patch, 479/131 and 220/139: the four are veraPDF
`6-2-5-t01`/`t04` at both parts), `t3-foreign_corpus` exit 0 (0 panicked), `t3-archive-val` exit 0,
veraPDF corpus census exit 0, 2 908 of 2 908 documents, 27 s: 35 unbuilt at 2b. pdf.js and
corpora census after: exit 0, 35 unbuilt at 2b (36 before). `--remedy-sites --to 2b`: 35 of 75
sites not built (36 of 74). Duration: 6 150 s.
