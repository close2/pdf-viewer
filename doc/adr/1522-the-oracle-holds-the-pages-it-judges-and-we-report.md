# 1522 — The oracle holds the ambiguous and geometry pages this tree reports

Session 1343. Status: accepted. Amends ADR 1512 section 3 (the 26 pages "held by no group by
construction"). Code: `AMBIGUOUS_ON_A_PAGE_WE_REPORT`, `GEOMETRY_ON_A_PAGE_WE_REPORT`, the two
ratchets in `check_the_ratchets` and
`the_pages_ambiguous_on_a_page_we_report_are_on_the_corpus_incomplete_list` in
`crates/pdf-model/tests/oracle.rs`.

## 1. The difference

The walk printed 836 ambiguous and 3 our-geometry pages; `oracle-held` printed 810 and 0. Every
group above holds complete pages only, so the 26 ambiguous and 3 geometry pages this tree draws
and reports were counted and watched by nothing. Listed from the run of 2026-10-05: all 26 are
pages of documents on `corpus.rs`'s `INCOMPLETE` (25 first pages and `issue6127.pdf` page 2); the
3 geometry pages report a `/MediaBox` the standard does not admit (absent, or enclosing no area)
and are drawn on this reader's default.

## 2. The decision

Not a sixth verdict: `ambiguous` is the right word for what the references did, and `(incomplete)`
already rides beside it. Two groups, held in both directions over incomplete pages, mirroring
`CONTRADICTED_ON_A_PAGE_WE_REPORT`. The reason each page is short is not written again: the test
reads `INCOMPLETE` out of `corpus.rs` and requires every ambiguous member's document to be on it,
so the mechanism is one fact in one file and the oracle adds only the verdict. Afterwards
`oracle-held` prints 836 ambiguous and 3 our geometry, the walk's totals.

## 3. What would reopen it

An incomplete page whose verdict is `contradicted` or `not comparable` is already held; one that
`agrees` is not held anywhere and does not need to be, since agreement is the default the walk
assumes. A page joining either group is a picture that moved under a standing report.
