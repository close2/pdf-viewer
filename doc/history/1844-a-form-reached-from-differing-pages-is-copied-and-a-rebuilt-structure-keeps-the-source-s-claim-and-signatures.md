# 1844 — A form reached from differing pages is copied per page, and a rebuilt structure keeps the source's claim and signatures

Slot 5 of batch 125, 2026-10-28, an "archive" round (Sonnet, high, three sessions); ADRs 2522 and 2523
written; Q710 unused. I worked in `scratchpad/r1844/`. No ledger status moves (ADR 2500's and 2502's owed items).

**Premise** (each owed item is one more input to a pass that exists): held for R04 item 1 (ADR 2522)
and R01's three inputs (ADR 2523). Not for the census: no corpus document is refused for the
differing-pages shape, so R04 item 2 moves tests and no count. ADR 2503's sentence that `027613` is
refused for reaching one of 429 pages ended when round 1846 made the reader reach all 429 (ADR 2526);
ADR 2523 and `doc/pdfa-blocking-issues.md` say so, and ADR 2503 stays as written.

**Built.** Survey `OpenedStream.through_page_table`; `reader_resources.rs` `copy_for_page`, `FormCopy`,
`Edit::Retarget`, bounds of 256 copies and 32 MiB, the rewrite building each copy once; `prepare.rs`
and `mod.rs` `Prepared::raised`, `raised_judgements` (the standard-fourteen definition raises
`fonts/font-programs-embedded`, an encrypted source `file-structure/no-encryption`) and
`conversion_before_deciding`; `recovered.rs` reads the claim and the signatures from the source
(`signatures::find_over`), gives a direct page an object (stating the resolved entries where it names no `/Parent`);
`editions::SHIFTS` gained §9.6.2.2.

**Census** (`archive_corpus` versus HEAD's six arms): converted counts unchanged in all eight lines of
both arms (with the CJK pack pdf.js `as-if-printed` 838 documents at 2b and 841 at part 4, corpora 386
and 385; `as-shown` 797, 801, 333, 331). Freed 0 documents; `refused` rises by 3 documents in every
pdf.js line (116 to 119 at 2b with the pack) because `issue19484_1`, `issue19484_2` and
`poppler-395-0-fuzzed` leave "declined as a source" for other rows (`graphics/only-operators...`;
an unauthorised `signatures/...annotation-rules` and `annotations/appearance-dictionary-present`);
`issue9105_other` now stops at the file-header row. veraPDF 1.30.3 passed all 38 distinct outputs of
the new tests at their flavour, after a planted failure (the program's key renamed) made it fail.

**Memory probe** (peak resident, converter and worker, 2b and 4): 256 copies (78 533 bytes in) 16 to
17 MiB, 257 refused at 15 MiB; 32 MiB of copies (1 059 285 bytes in, 34 621 258 out) 146 MiB in
1.7 s, one copy more refused at 13 MiB; 14 standard-font names on a page 19 MiB; HEAD refuses each at 12 to 14 MiB.

**Gates.** Export of HEAD plus my paths, workers rebuilt before the census: clippy `-D warnings`,
`fmt --check` and rustdoc `-D warnings` exit 0; `--test archive` exit 0 (412 passed, 6 ignored);
`--lib` 136 passed; `cargo test -p pdf-archive` exit 0; `cross_check --ignored`, `--bin cited`,
`--bin quotations` and both censuses exit 0; `gate` is ignored below the gates profile.
`cargo test -p conformance --no-fail-fast` exit 101 once, on `records` alone (this record at 42 lines,
and round 1841's at 41); cut, re-run exit 0.

**Unfinished.** The Type 3 glyph procedure's resources and a font that moved (ADR 2500). Tool uses by session: about 150, 75, 105 (this one 2400 s). **Validation.** An Opus validator failed the first reading on five findings — a block landed twice that broke the formatter and a lint, a licensed sentence quoted, a direct page without a parent called impossible though the reader resolves its inherited values, and two documents stating what no longer was — and passed the round after it fixed all five, the direct page now built from the values the reader resolved and the validator's plant carrying exactly them.
