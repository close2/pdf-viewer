# 882 — A count retired, a lattice cut to what the fill reaches, and a scan that pays for itself: round 879 merged, `batch5/PDFIUM` walked, and `MAX_TILES` gives way to an empty cell that loops nothing, a fill's own reach, and a tiling's copies bounded in commands

Date: 2026-09-03.
ADR: [0810](../adr/0810-a-cell-with-no-marks-is-not-looped-and-the-sites-a-fill-states-are-bounded-by-what-they-cost.md).
Touched: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/pattern.rs`,
`crates/pdf-model/src/content/pattern/reach.rs` (new), `crates/pdf-model/src/colour.rs`,
`crates/pdf-syntax/src/document.rs`, `crates/pdf-model/tests/hostile_budgets.rs`,
`crates/pdf-model/tests/tiling.rs`, `crates/pdf-model/tests/image_reuse.rs`,
`crates/pdf-model/tests/nested_content_window.rs`, `doc/checks/fixed-documents.toml`,
`doc/conformance/ledger.toml`, `doc/performance.md`, `doc/todo/03-more-corpora.md`,
`doc/todo/10-bounds-that-cap-size.md`, `doc/todo/49-restrictions-worth-re-examining.md`;
and the merge commit before this round's own.

**This round was interrupted by a model rate limit and finished on another model.** The agent that
began it did the merge, the walk, the ranking and the whole of ADR 0810's argument and code, and
was cut off with the tree uncommitted and one gate line red. What the second agent did is set out
under *The resumption* below, and the one thing it changed rather than confirmed is the third
bound, which the first agent's design needed and did not have.

## The merge

`round-879` (1b3cdd9d, ADR 0797: §11.3.4's three-component blending spaces and a three-component
mask group taking §11.5.3's Y as three curves) is on `main` as `839a659a`, `--no-ff`, with no
conflicts. The whole `doc/todo/02` §2 sequence ran green on that merged tree before this round's
own change. `r877` and `r879` are closed — both branches were ancestors of `main`, their build
directories are gone with their checkouts, and `r867`, `r881`, `r883` and `r885` are untouched.

## The walk, and the head

`batch5/PDFIUM`, 379 documents, surveyed whole under the four rules — `--data 8 --tree 12`, 3.0 s,
1.30 GiB peak: 7 unopenable, 2 locked, 2 encrypted beyond us, 19 pageless, **66 incomplete**, 0
slow. `doc/todo/03` §43 has the reports by kind and the ranking. **17.4% incomplete is the highest
rate of any tracker walked**, above `poppler`'s 11.5% and for the same reason one step further: a
pdfium issue attachment is a fuzzer's output far more often than a document anybody wrote.

The head was at the light end with both references agreeing to a hundredth: `PDFIUM-1122-0.pdf`,
ours **70.92** against `poppler` 80.51 and `mupdf` 80.52, reporting `LimitReached { limit:
"MAX_TILES" }` and nothing else. One fill, a 10-unit tiling cell over an A4 sheet — **4480 sites of
a two-command cell** — of which the constant afforded 4096, so the top fifth of the sheet was white.

## The count, and the three things that replace it

ADR 0271 had already concluded that "the count is the wrong quantity and a larger count is no
safer": all 48 crawled documents that reached `MAX_TILES` terminate with it lifted. What kept it
was one measurement of the loop rather than of any document — an *empty* cell executes no operator
and, since ADR 0430, copies no command, so its loop ran whatever trip count `/XStep` and `/YStep`
state: 3.6 × 10¹¹ for a thousandth of a unit over 600, about four days.

ADR 0810 retires it, on that measurement turned round. A cell with no marks replicated any number
of times is no marks — §8.7.3.1's replication has nothing to replicate — so `repeat_cell` does not
enter the loop for one, and the four-day loop was the loop *for* that case. A site is copied only
where the fill's interior can reach its cell box: `pattern/reach.rs` scans the path onto the
lattice a row band at a time, in the pattern's own space, conservatively — every cell an edge
passes through, every cell the interior covers on the band's centre line by §8.5.3.3's rule, every
cell under a curve's control box — with the proof that the two rules together miss nothing in the
module's own comment. `7680183.pdf`'s 249 hatched polygons fell from 539 729 sites to 112 499 and
the page from 3.3 s to 2.2 s. And every other site costs its cell's commands, charged **before** the
copy to `MAX_OPERATIONS` and to the tiling's own `MAX_TILE_COPIES`, 65 536 commands — the cost in
its own unit, sixteen times the retired count at a one-command cell — with the prefix cut to whole
rows on ADR 0477's argument and the refusal named by whichever budget is the tighter.

**Why the third survives was measured before it was chosen.** The first draft had no per-tiling
bound at all; on the head and on ADR 0477's two crawl rows it was right, and on
`PDFIUM-1497-2.pdf` — an A3 floor plan with fifty tilings, two of them 448 632 and 389 205 sites of
a four-command cell — it took eleven seconds and 0.9 GiB where the count took 2.2 s and drew the
page *worse*, the two large tilings having spent the whole four million and the frame and title
block after them never drawn. A budget that is the page's lets one operator starve every operator
after it.

The head draws its whole sheet at 80.52 by the ranking's instrument, square for square against
`mupdf` at four times magnification. ADR 0477's two crawl rows moved with it —`7803372.pdf` from
11.1 to 18.9 by the gate's instrument, `4650000.pdf` from 49.2 to 57.3, inside its references'
43.7 to 62.9. `batch5/PDFIUM` is **65 incomplete** after it, `PDFIUM-1497-2.pdf` the one page still
reporting a budget and reporting `MAX_TILE_COPIES` for it. What still wants more than any budget a
page turn can afford is a cell rendered *once* and replicated by the rasteriser, which §8.7.3.1's
NOTE 2 anticipates and `doc/todo/49` now carries with `2760154.pdf` and `PDFIUM-1497-2.pdf` as its
two witnesses.

## The resumption, and the fourth bound

The second agent read the inherited tree against the clause and kept every line of it. Two things
were owed and one was a defect.

Owed: the ledger's §8.7.3.1 row named a test by a name the tree does not hold —
`a_tiling_whose_cell_marks_is_bounded_by_the_operations_budget`, where the file says
`…_by_its_own_budget` — which is what the conformance gate and `nextest` were red on, and the only
thing they were red on.

The defect is the one this round is a lesson about, because it is **the retired constant's own
shape one level in**. `reach.rs` buys its saving with a cost nothing was charging for: `Reach::row`
is one pass over the path's edges *per row band*, and a row that reaches no site spends no copy, so
neither `MAX_OPERATIONS` nor `MAX_TILE_COPIES` ever sees it. Twenty thousand unit squares stacked at
the origin and one more six hundred units above them, at `/YStep 0.0006`, is a lattice of a million
rows of which all but three thousand reach nothing, against a path of a hundred thousand edges —
about 10¹¹ edge tests, which is hours. `Interpreter::MAX_REACH_SCAN` bounds the page's whole scan at
**4 194 304 edge tests**, page-wide because a page may state as many fills as `MAX_OPERATIONS`
affords; past it the caller stops asking and takes every row whole, which is what a stroke already
gets and what the code did before `reach.rs` existed. Running out refuses nothing and is not
reported — it stops a *saving*, and the sites it stops saving are bounded by the two budgets that
bound every other site.

The value is fifteen times the heaviest page measured: `7680183.pdf`'s 249 tilings spend under
300 000 tests between them, `PDFIUM-1497-2.pdf` and `2760154.pdf` fewer than 5000 rows' worth
apiece, and every one of ADR 0810's six before-and-after figures reproduces to the digit with the
bound in place. `hostile_budgets.rs::a_fill_of_many_edges_does_not_pay_for_the_rows_it_cannot_reach`
is the fixture: **0.116 s with the constant, and it does not finish in two minutes with it lifted.**
`Interpreter::reach_scanned` is a new field, so `checkpoint` and `restore` grew the two lines their
exhaustive destructures exist to demand.

The general lesson, and it is ADR 0271's twice over: **a saving is work, and work that no budget
counts is a loop a file can state the trip count of.** Retiring a bound is the moment to ask what
the replacement costs, and the answer here was found by writing the hostile shape rather than by
reasoning about the ordinary one.

## Gates

The whole `doc/todo/02` §2 sequence on the merged and changed `main`, each line under
`tools/bounded.sh` (`--data 8`, `--tree 8` for a build and `12` for a walk), each walk waiting on
any other round's. It ran three times: once on the merge alone, green throughout; once on the
inherited tree, red on `nextest` and on `conformance` for the ledger's stale evidence name and on
nothing else; and once on the finished tree, green throughout.

Formatting and `clippy` under `-D warnings` silent for the workspace and for `fuzz/`; **3028 tests
passed, 20 skipped**; doctests green; the corpus gate at **974 documents, 64 incomplete**; the
oracle at **1945 pages, 1841 complete, 104 incomplete**; the three text gates green (99.67% of
matched words in bounds, 493 of 503 documents fully in); the two censuses green; dates, XMP and
JPEG 2000 green; quorra at **958 pages, 929 agree, 22 differ, 7 refused, 16 not comparable**; fixed
documents **60 of 60**, this round's `PDFIUM-1122-0.pdf` row among them and ADR 0477's two moved to
the bands their whole hatching reads; the transform gate at 178.3 pages/s over a floor of 40; the
writer over 974 documents, 941 attached and read back; conformance green — **875 ledger
subclauses, 0 owing a review, 12 893 citations and 1175 quotations** — re-run after the last ledger
edit.

One mistake of the round's own, recorded because it cost twenty minutes and is general: the
survey was re-run as `safedocs survey <path>` rather than `safedocs survey --dir <path>`, and a
bare positional path is not `--dir` — so it walked the whole 65 944-document cache instead of the
379, exceeded its `RLIMIT_DATA` at 24 threads, and was read as a memory regression of this round's
change until the argument was checked against `main.rs`. A tool that ignores an argument it does
not know turns a typo into a finding.
