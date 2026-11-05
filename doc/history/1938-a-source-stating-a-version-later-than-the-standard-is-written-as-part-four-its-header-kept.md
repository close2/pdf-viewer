# 1938 — A source stating a version later than the standard is written as PDF/A-4, its header kept

Session 1938, a single archive round. ADR 2692; no question; ADR 2693 not used.

**The premise.** It held. `version_for` compared the stated major with the part's and sent every
higher major to the one refusal written for a 2.x source at part 2, so the nine `UnknownFilter-*`
files (`%PDF-3.1` to `%PDF-3.9`) were told at part 4 that the part requires `%PDF-1.n`.
`Version::parse` reads `%PDF-3.1` as 3.1, so `Document::version` is not `None` and ADR 2558's
route never saw them. The reading is right (Annex I's warning needs the number), so `pdf-syntax`
is unchanged. `T01_001` (`%PDF-2.4`) was already refused at 2b only and kept as 2.4 at 4.

**The reading.** §7.5.2's header is `%PDF-1.n` or `%PDF-2.n`, and it says a processor may be able
to process a later version. ISO 19005-4 section 6.1.2 admits `%PDF-2.n` alone. Annex I says a
writer should never lower a file's version.

**The construction (ADR 2692).** At part 4 a major above 2 gets the 2.0 header. The source's first
line and the version it states go into `xmpMM:History` and the report's `recorded`, and that record
is a condition of the header (ADR 2558). Departing from Annex I's *should* is deliberate: the only
way to keep 3.n is to write no PDF/A-4. At part 2 the source is refused as a 2.0 source is, by
`LOWERED_TO_PART_TWO`. That sentence now names PDF/A-2 and section 6.1.2, says PDF/A-4 admits the
header, and is reachable only at part 2. `NO_PLACE_TO_RECORD_HEADER` now covers both kinds.

**Refusals kept.** Every source stating 2.0 or later is still refused at 2b. At 4 the rows refuse
`UnknownFilter-ICC` under every profile, as before, and `-Font` under `as-shown` and
`no-data-loss`. `no-data-loss` also refuses six by `undecodable-stream`.

**Tests.** `tests/archive.rs`: 3.1 at part 4 (header, packet, report) and at 2b (3.1 and 2.0);
a unit test in `archive::header`. A planted false `later_than_the_standard` fails two.

**Census**, compared with round 1937's runs. Shipped: 15 new outputs (8 `as-if-printed`, 7
`as-shown`, at part 4), nothing gone, 5070 identical. Stating: 10 new (8 `state-every-loss`, 2
`no-data-loss`), 4551 identical. Every new output was drawn at 72 dpi beside its source. Six are
identical. `OutlineObjStm` differs in 2560 pixels by one level, which is the output intent's
(ADR 2691). `Font` differs only inside the image, which is R08's photograph (round 1935). R06b now
frees 34 at 2b and none at 4; the table in `pdfa-blocking-issues.md` was regenerated.

**Gates.** fmt `-p pdf-transform`: exit 0. Clippy `-D warnings --all-targets -p pdf-transform`:
exit 0. nextest `-p pdf-transform -p pdf-archive`: 1388 passed, exit 0. Archive tier (`corpus`,
`archive_corpus`, `witnesses`): exit 0 each. Both censuses: exit 0. veraPDF 1.30.3: all 25 new
outputs pass. `cargo test -p conformance` after `git add -N`, and `state.sh docs`: exit 0.
