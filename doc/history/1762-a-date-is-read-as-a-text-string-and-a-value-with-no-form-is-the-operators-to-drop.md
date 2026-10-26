# 1762 — A date is read as a text string, and a value with no form is the operator's to drop

Slot 3 of batch one hundred and sixteen, 2026-10-26, an archive round; ADR 2358; ADR 2359 unused; no
question. I worked in `scratchpad/r1762/`. No ledger status moves; the §7.9.4 row's note grows.
**The sentence.** The census under both profiles ranked ISO 19005-4 section 6.1.3's "states a
CreationDate or ModDate that is not §7.9.4's date" first among refusals a conversion answers: 17
documents at part 4 under each profile (6 pdf.js, 11 corpora). Above it and not taken: smoothing,
machine face, substitute stretch (the profiles' `stop`s, Q585), fences, composite fonts (Q570,
Q596), `DeviceCMYK` beside an RGB intent (a supplied press, ADR 2130), part 2's PDF 2.0 header.
**Premise** (habit 105): 3 of the 17 dates are dates — two UTF-16BE, one behind UTF-8 lead bytes —
read off raw bytes as UTF-8; 14 are none (`D:1910…`, no `D:`, "Fri, Jan 20, 1995", `()`, month
42). 13 more (9 pdf.js, 4 corpora) were refused at the same rows as "nothing asked for": `<< >>`.
**Built (ADR 2358).** `prepare_information` decodes a date as §7.9.2.2's text string first
(`attach::modification_date` too); a value of no form §14.3.3 states is dropped under the new row
key `unreadable = "discard"`, named in `moved_information`, `stop` the default and a `declined`
naming the key; `as-if-printed`, `as-shown`, `only-metadata-loss` say `discard`. A kept `/ModDate`
beside `/PieceInfo` is never dropped, and is read only where the packet states `xmp:ModifyDate`. An
empty or dangling `/Info` (§7.3.9) moves nothing and goes. Four tests in `tests/archive.rs`; plants
(trap 13): raw-UTF-8 dates, an ignored answer, the old empty refusal — each fails its test.
**Census**, converted/refused at PDF/A-4 (2b unchanged), HEAD → this: pdf.js `as-if-printed`
769/187 → 781/175, `as-shown` 748/208 → 760/196; corpora 344/131 → 355/120, 295/180 → 306/169. 23
leave each part 4 list, none joins any; the other 6 date witnesses stop on fences, fonts, CMYK.
**Drawn** (looked at): page 1 of the 23, source against both outputs, 100 dpi: 18 identical;
`pdf20-utf8-test`, `issue12010_reduced` by 1 level (DefaultRGB, ADR 2114) and
`poppler-91414-0-53/54`'s stroked text by up to 12 (its embedded face) — other remedies on those
pages, since this change writes no content; `poppler-937-0-fuzzed` has no page here, either side.
**veraPDF 1.30.3** at 4 over the 46 outputs: 36 PASS, 4 FAIL (`issue12823`, `Test-plusminus`), 6
unparsed (`poppler-91414-0-53/54`, `poppler-937-0-fuzzed`), each failing the same way at 2b already.
**Gates.** `rustfmt --check` (my files), `cargo fmt --check`, `-D warnings` clippy `--all-targets`,
rustdoc `-D warnings --document-private-items`, all `-p pdf-transform`: exit 0; nextest 694 passed,
0 failed, 16 skipped. Census `--tree 6`: exit 0 before (HEAD's export) and after (mine).
`tools/state.sh --round 1762 archive` in my export: exit 0; validator missed 0, over 0 at all six;
converter 2b 526/85, 4 271/88, every conforming input still conforming; cross-check 13 560
compared, 0 disagreements. A doc-comment escape followed the walks; rustdoc, clippy, nextest reran.
`cargo tree -i pdf-transform -p render-raster`: no match (`pdf-font`, the control, resolves), so no
arm moves. `cargo test -p conformance --no-fail-fast`: exit 101, one red, not mine (1764's record).
**Found, not fixed.** `fonts::for_each_font` visits indirect font dictionaries only, so a direct
`CIDFontType2` with no `/CIDToGIDMap` (`Test-plusminus`) and a font resource resolving to null
(`issue12823`) pass this validator and fail veraPDF. A root page node whose `/Count` is a stream, or
with no `/Kids`, is carried into an unparsable file: `refuse_a_document_only_recovery_reads`'s. 5346 s.
