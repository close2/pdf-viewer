# 1921 — A document holding none of its pages is refused, and the stating profiles take a recovered structure

Session 1921, a single archive round on the server VM. ADRs 2669 and 2670; no question, trap or
habit. Worked in the rounds worktree and its `scratchpad/r1921/`.

**Why.** The owner's A775 (Reading 2, all profiles) and A757 (Reading 1), both of 2026-11-03, both
about A727's loss `page-not-held` (ADR 2596).

**A775 (ADR 2670).** `pdf_archive::NoPageHeld` is a root `/Count` (or, where the root states none, a
linearization `/N`) of one or more where this reader draws no page, and its `Display` is the one
sentence: `page-not-held: its page tree root's /Count states 1 page and the file holds 0 of them (ISO
32000-2 7.7.3.2, Table 30), so it holds nothing a reader sees to archive (doc/questions/A775)`. The
checker records it at a new row, `file-structure/page-tree-holds-a-page-it-counts` (section 5.1 of both
parts, Table 30's `/Count` narrowed to that one disagreement); the converter refuses with it, exit 4,
before any rebuild, under every profile. `T02-03_006` is a witness of the row. Four corpora documents
meet it (`T02-03_006`, `REDHAT-1531897-0` by its `/N`, `poppler-85140-0`, `poppler-937-0-fuzzed`), all
refused before by the two lower-bound profiles' census of round 1901; only their sentence moves.

**A757 (ADR 2669).** `[profile] recovered-structure` takes `append-statement` as `page-not-held` does,
and the appended page lists the structure: the count the source states and where, the pages held, those
found by declaration, the catalog kept or written, `/Kids` naming nothing, pages read in part. Both
profiles carry `recovered-structure` and `page-not-held` as `append-statement`.

**The census.** `the_stating_profiles_list_what_real_producers_files_refuse_on`, new in
`archive_corpus.rs`, before and after the two profile lines: nothing newly refused. Converting now and
not before, corpora + pdf.js: `state-every-loss` 15 + 3 at 2b and 16 + 3 at part 4, `no-data-loss`
15 + 1 and 16 + 1; nineteen documents in all, listed in ADR 2669. All 70 outputs passed veraPDF 1.30.3.

**Ledger.** 7.7.3.2 stays `implemented`; its note and pointers name both answers.

**Gates.** `cargo fmt --check` and `clippy -D warnings --all-targets` of `pdf-archive` and
`pdf-transform`: exit 0. `nextest` of both: 1322 passed, exit 0. The archive tier behind the walk
lock (`pdf-archive --test corpus`, `pdf-transform --test archive_corpus`, the witnesses): exit 0.
`cargo test -p conformance` after `git add -N`: exit 0. No device frame.

**Unfinished.** Four documents stay refused because the appended page cannot be composed (ADR 2669);
the remedy for the Defence document, a page extending a number tree's labels, is not built.
