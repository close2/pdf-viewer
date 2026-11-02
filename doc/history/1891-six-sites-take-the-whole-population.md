# 1891 — six more converter sites take the whole population

Batch 130, slot 1, an "archive" round, in two sessions. ADR 2616 written; ADR 2617 and Q767 unused.

**What.** ADR 2602's rule carried to the six sites round 1884 found reading the checker's thirty-two
kept places as the set: `boundaries::removal`, `external::external_stream_data`,
`remedies::embedded_streams`, `transcode::prepare`, `font_widths::restated`, `prepare::unicode_fonts`.
`pdf_archive::Named` keeps every distinct place in the order found, bounded at 4096 places and a
mebibyte of their names; `sites::places_failed` hands it to all six or refuses by name. The report's
printed list is unchanged: `Outcome::Failed` is still the thirty-two kept places and "… and N more".

**The premise that did not hold.** The brief expected the survey to serve four sites and its font set
two. None of the six is about page content streams; two need more than the object a place names (a
page and its boundary entry, a file specification and its `/EF` key), so the place list serves six.

**Bounds the lift needed (trap 305).** Pages enumerated once (boundaries); set dedupe where a scan was
quadratic (external data, embedded files); 4096 × 256 width entries read (widths, shared arrays);
thirty-two images at the ordinary decoder's maximum (transcode, HEAD's own ceiling); the survey read
once by object (`/ToUnicode`). Each is tested at its bound.

**Found, not fixed.** `archive/halftones.rs:233` and `archive/transfer.rs:86` still refuse by name when
the kept list was shortened rather than take the population (the plant's one red).

**Gates.** Export of HEAD plus the eleven files: clippy `--no-deps -D warnings` on pdf-archive and
pdf-transform exit 0, `cargo fmt --check` exit 0. `cargo test -p pdf-transform --test archive --test
profiles` exit 0 (520 and 13 passed); `--lib` exit 0 (183 passed); `cargo test -p pdf-archive` exit 0
(311 passed). The five new whole-population tests fail at HEAD (exit 101, 5 failed). Plant, `MAX_KEPT`
at 2: `--test archive` 519 passed, 1 failed (the halftone refusal above), every whole-population test
passing. Census no pack and with a copy of the CJK pack: exit 0 each, against HEAD's 0 census lines
moved (only test durations and cargo's 60-second notice differ); `rank.py` exit 0. veraPDF: 10 freed
outputs PASS (2b ×4, 2u, 4 ×4, 4f), the planted output (one media box 2 units wide) FAIL. `getrusage`
beside HEAD on 1 000-item inputs, peak KiB HEAD/this tree, HEAD exit 4 and this tree exit 0 at each:
boundaries 22 736/21 672, external 20 388/20 924, widths 23 584/24 192, `/ToUnicode` 30 964/34 024,
embedded files 25 960/26 576, JPEG 2000 26 084/35 108; 4096 places both refusing 31 804–32 640 vs
33 016–33 224 KiB. `--bin cited` exit 0, `--bin quotations` exit 0. `cargo test -p conformance
--no-fail-fast` exit 101: two reds, both siblings' records in progress (1892's Gates paragraph, 1894's
path outside the tree). `archive_corpus` (the other six, `--ignored`, gates profile) exit 0, 6 passed.
