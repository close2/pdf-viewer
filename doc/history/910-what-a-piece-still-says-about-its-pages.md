# 910 — What a piece still says about its pages, and the last mode of `split`

Date: 2026-09-04.
ADRs: [0862](../adr/0862-what-a-piece-still-says-about-its-pages-and-the-three-clauses-that-decide-it.md),
[0863](../adr/0863-a-piece-begins-where-the-outline-lands-and-the-front-matter-is-a-piece-too.md).
Touched: `crates/pdf-transform/src/split.rs`, `crates/pdf-transform/src/merge.rs`,
`crates/pdf-transform/src/lib.rs`, `crates/pdf-transform/src/bin/pdf-transform.rs`,
`crates/pdf-transform/tests/split.rs`, `crates/pdf-transform/tests/split_corpus.rs`,
`crates/pdf-transform/tests/foreign_corpus.rs`, `crates/pdf-transform/tests/support/mod.rs`,
`doc/conformance/ledger.toml` (§7.7.4, §7.9.6, §12.3.2.4, §12.3.3, §12.4.2),
`doc/state-of-play.md`, `doc/todo/57-the-transform-suite.md`, ADRs 0862 and 0863, this file; and
the merge of `main` before them. **No pixel moves** — nothing here is on a path that draws, and
the two walks that draw one say so: 965 of the corpus's pieces are bit-identical to their source
page before and after.

## The two items were one item

`doc/todo/57` §1 carried them separately and they are the same thing seen from two sides.
`split --at-bookmarks` did not land in session 886 because it wants an outline subset for the
piece; the outline subset is one of the three things a piece could carry and did not. So the
round did the carrying first and the mode fell out of it.

**Three clauses, three different verdicts, and the point of ADR 0862 is that they are different.**

- **§12.3.3's outline is *permitted*.** "A PDF document may contain a document outline", so a
  piece without one conforms and nothing requires a derivative to keep its source's. What binds is
  the shape once there is one, and every conditional entry of Table 150 and Table 151 is rebuilt
  over the subset rather than carried — `/First`, `/Last`, `/Parent`, `/Prev`, `/Next`, and
  `/Count` from the clause's own three steps run over the kept items. An item is kept when its own
  destination lands in the piece **or when a descendant's does**, because Table 151 makes
  `/Parent` required and dropping an ancestor orphans what is under it.
- **§12.4.2's labels are permitted and the source's own tree is *forbidden*.** Three sentences say
  it together: a page index is "the page's relative position within the document", each key is
  "the page index of the first page in a labelling range", and "[t]he tree shall include a value
  for page index 0". A piece is a document whose indices run from 0, so a piece beginning at the
  source's page 13 that carried the tree unchanged would state one whose lowest key is 12 —
  non-conforming, not merely lossy. What is written instead is `merge::page_labels`, literally:
  the function moved out of `merge` and takes the labels and the output's order, because two
  implementations of one clause are how they come to disagree.
- **§12.3.2.4's named destinations are carried by whatever still names them**, and this is the one
  where the asymmetry with everything else `split` does is the argument. Every reference to a page
  the piece does not hold becomes §7.3.10's null; a named destination is **not an indirect
  reference** — it is "a name object ( PDF 1.1 ) or a byte string ( PDF 1.2 )" whose meaning comes
  from the `/Dests` of the catalog or of the name dictionary and from nowhere else — so §7.3.10
  has nothing to say about one, and a piece that kept a link and dropped both tables would state a
  destination the standard gives no meaning to at all. The entries that resolve into the piece are
  carried, the rest dropped and counted, and an item kept only as an ancestor loses its own
  destination rather than naming nothing.

`NOT_CARRIED` is eight entries now instead of twelve, and the four that left it are named in
`doc/state-of-play.md` and in five ledger rows.

## Where a piece begins

**At every selected page an outline item at the stated depth or shallower resolves to, and it runs
to the page before the next such page.** The resolution is `pdf_model::retrieval::sections` — ADR
0257's machinery, reused as RFC 0002 §6.1 asks — so a named destination goes through §12.3.2.4's
two tables here exactly as it does when a reader follows a link. Three edges, each decided:

- **The pages before the first mark are a piece with no title.** A split whose pieces do not cover
  the selection has lost pages, and front matter ahead of the first bookmark is that case.
- **Two items on one page start one piece**, and the first in the outline's own order names it, so
  the answer is a function of the file rather than of the iteration.
- **A document whose outline resolves nowhere at that depth is `Refusal::NoBookmarks` at exit 2** —
  the same shape as `Refusal::Selection`, because the request is well formed and the document
  simply does not hold what it asked for. A verb that answered by writing one piece would have cut
  nowhere while saying it cut at the bookmarks.

`%t` is a title for this mode and still refused for the other three. The flag needed one thing the
argument reader did not have, and it is three lines: an **optional** value written inline, because
`--at-bookmarks in.pdf` would otherwise read the input as a depth.

## What the walks were given to ask

`support::check_navigation` is four clause-derived properties in `check_structure`'s discipline
(trap 8) — asked of the *output*, never compared with what the writer meant. Every outline item
resolves to a page this document holds; Table 150's `/First`/`/Last` are present where there are
entries and Table 151's `/Prev`/`/Next`/`/Parent` are right at every position; §12.4.2's tree has
a value for index 0, no key twice and no key past the last page; every §12.3.2.4 entry, in either
home, resolves to a page. Beside them, one source-relative check: page 1 of a piece carries the
label the source's page 1 carried.

`--at-bookmarks` gets its own two, and they are the mode's rule stated back to it: the pieces
cover the document's pages exactly once, and every piece but the leading one begins on a page a
level-1 item resolves to.

**The foreign readback gained a sixth lane** because an at-bookmarks piece is a different *shape*
to show another reader — several pages carrying an outline, labels and named destinations, where
`split`'s lane writes one page. It fits the existing comparison for free: the mode's first piece
always states the source's page 1 as its own page 1.

## The gates

**The whole `doc/todo/02` §2 sequence ran twice — on the finished working tree and again on the
merged one — all twenty-six lines, every one exit 0 both times.** The figures below are the
second run's, because that is the tree the branch head is and `doc/todo/02` §2's rule is that a
number belongs to the round that ran the gate last. Both runs were on a quiet machine waited for.
The wait is itself worth recording: `/proc/PID/exe` naming a binary in another round's build
directory is what finds a neighbour's walk, since a command line cannot be grepped without
matching the grep, a build cannot be told from a walk by its name, and a sweep is invisible to a
list of gate binaries.

`cargo fmt --all --check`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`, both
`fuzz/` lines: clean. `Summary [69.216s] 3196 tests run: 3196 passed (1 slow), 27 skipped`;
doctests green. Corpus **974 documents in 10.7s — 0 unopenable, 9 locked, 1 encrypted beyond us,
5 pageless, 64 incomplete, 0 slow**; oracle **1945 pages in 33.0s (1841 we call complete, 104
incomplete)** with `our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`;
text extraction **99.3% (24014/24193 words), 22 below 90%** and the PDFBox lane **99.8%
(14257/14281)**; selection census **1000/1011 words (98.91%) over 453 documents**; accessibility
census **102 853 elements reached, 57 116 a caret can move through**; dates **1514 of 1545
(97.99%)**; XMP **318 of 319 streams read**; quorra **958 pages compared: 929 agree, 22 differ, 7
refused, 16 not comparable**; fixed documents **70 checked, 0 absent, 70 rows**; the transform gate
**164.4 pages/s over a floor of 40**; `writer_corpus` **941 attached, read back and removed, 0
unexplained refusals**; `merge_corpus` **966 merged, 965 bit-identical, every reconciliation
counter at zero**; `pages_corpus` **966 edited, 0 label faults**; `optimize_corpus` **26.71%
saved, every property counter at zero**; conformance **875 subclauses, 14 021 citations, 1244
quotations verbatim, 0 cited clauses owing a review**.

**`split_corpus`, this round's own:** 974 documents in 56.6s, 966 split and re-read as one page,
**965 drawn bit-identically**, content streams differing 0, §14.7 faults 0 — and the new rows,
**§12.3.3 outlines carried 147 with 180 items resolving; §12.4.2 labels carried 22; §12.3.2.4
destinations carried 68; faults 0; page 1's label changed 0; `--at-bookmarks` over the 23
documents whose outline names two or more pages at level 1, 0 refused, 0 pages lost or duplicated,
0 pieces cut where nothing lands.** Every one of those counts is identical across the two runs.

**The foreign readback:** 203 of 974 documents in 123.9s against `pdftoppm 26.08.0` and
`mutool 1.28.0`. The new lane, **`bookmarks`: 5 written, qpdf held 5, qpdf gained a warning 0,
poppler identical 5, mupdf identical 5, §14.7 shapes agreed 3, §14.7 faults 0, drew differently 0,
0 refused by name.** Five is what a stride-8 sample of a corpus where 23 documents have a
cuttable outline can offer, and it is said as such rather than rounded up.

**`main` was merged in after the commit and before the second run**, bringing round 908 (which
touches `pdf-syntax`'s parser, so the whole sequence is what the merge is owed rather than the
core four). Git found no conflict. `doc/conformance/ledger.toml` was checked **row by row** rather
than by reading the diff: `main`'s two changed rows (§7.3.7, §7.3.10) and this round's five
(§7.7.4, §7.9.6, §12.3.2.4, §12.3.3, §12.4.2) are each identical to the side that wrote them, no
third row moved, and the count is 875 on all four versions.

**§5 ran, this being a fifth round, and it ran twice for the reason session 905 recorded**: the
first install was from the pre-merge tree, so the eight binaries and `libviewer_ffi.so` were
rebuilt with `--release` in one invocation from the merged tree and installed again into the
project's own `target/` — a stale binary is a measurement of the past.

## What is left, and whether `r867` can close

`split` is complete against RFC 0002 §6.1 and the suite's five writing verbs were already done, so
**the transform stream is finished except for four things, and two of those are the owner's**: the
aligned rotated comparison ADR 0831 §1 priced (a change to `render`'s *report*, not to the
renderer), a per-input password for `merge` (nobody has asked), the confinement tranche, and RFC
§13's second question — a DCT encoder, which `optimize --images` and JPEG output from `render`
both wait on. The first two are small and unasked-for; the last two are decisions rather than work.
**On that reading `r867` can be closed by the next main round**, and `doc/todo/57` says so in its
own header now.
