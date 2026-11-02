# 1884 — a remedy takes the whole population, and not the places a report keeps

Session 1884, batch 129, slot 4 (archive, robustness), in three sessions; the second ended at the
account's weekly limit. ADR 2602; ADR 2603 and Q760 unused.

**What changed.** Two converter sites read the checker's kept places (`MAX_KEPT`, 32) as the whole
set. `sites::page_resources` now takes its pages from the survey's opened streams (a page, named by
no object, referenced, stating no `/Resources`), every page when the survey stopped short; the page
tree is enumerated once and each node's inherited value read once, and copies of a dictionary
written directly in a node are budgeted at 32 MiB by weight, past which the row is refused by name.
`fonts/truetype_cmap.rs` loses both "more places than kept" refusals, since its fonts come from
`objects_named` (bounded at `MAX_NAMED`), and its refusal loses the clause about the first places.
Files: `crates/pdf-transform/src/archive/{sites.rs,prepare.rs,fonts/truetype_cmap.rs}`,
`tests/archive.rs`, `tests/archive/{whole_population.rs,truetype_cmap.rs}`, ADR 2602, `doc/pdfa-blocking-issues.md`.

**Census against HEAD's.** Seven pdf.js documents written that were refused, at both targets,
both profiles, with and without the CJK pack: bug1132849, issue13211, issue6068, issue6889,
issue6894, issue6961 (off `non-symbolic-truetype-differences-are-listed-names`, which keeps
issue13316_reduced) and issue14117 (off `truetype-codes-reach-glyphs-by-the-standard-route`).
Every other moved line is the build's output, a timing, or a summary count following those seven.
No corpus document reaches the page-resources site's 33 pages. The census ran on HEAD and this
round alone, so no sibling's move is in it. `issue12963.pdf` (199 places), refused there on the
`.notdef` rows for a CID-keyed CFF program, converts in the merged tree beside ADR 2598.

**Found, not fixed.** Six more sites iterate `Outcome::Failed { places, .. }` and ignore `total`:
`archive/boundaries.rs:153`, `external.rs:104`, `remedies.rs:1114`, `transcode.rs:292`,
`font_widths.rs:43`, `prepare.rs:102` (`unicode_fonts`); and a direct inherited dictionary
written once as a shared object, not copied per page, would change HEAD's bytes.

**Gates.** HEAD probe: the three new tests fail. `cargo test -p pdf-transform --test archive
--test profiles` exit 0 (505 passed, 6 ignored; 13 passed); `cargo test -p pdf-archive` exit 0
(309 passed); `cargo clippy --no-deps -p pdf-transform --all-targets -- -D warnings` exit 0;
`cargo check --tests -p pdf-transform` in the merged tree exit 0. Plant: `MAX_KEPT` 2, 36 of 36
tests pass; with the two sites at HEAD, 6 of 36 fail. Census exit 0 both arms (199.92 s, 176.46 s).
veraPDF PASS at 2b and 4 on the three fixtures, the seven freed documents and issue12963; the
planted header failed. `getrusage` (not a clock hold): 1 000 pages by reference, HEAD exit 4
16 544 KiB, here exit 0 17 288 KiB; 100 000 pages under a 4 000-entry direct dictionary, both
exit 4, HEAD 612 044 KiB and 257.93 s user, here 393 940 KiB and 109.92 s. `--bin cited` exit 0,
`--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101, 468 passed and 2
failed, one on 1887's record (a path outside the tree) and one on 1890's (no gates paragraph).
