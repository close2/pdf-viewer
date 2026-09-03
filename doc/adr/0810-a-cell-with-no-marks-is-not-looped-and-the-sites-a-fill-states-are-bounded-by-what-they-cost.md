# ADR 0810 — A cell with no marks is not looped, and the sites a fill states are bounded by what they cost: `MAX_TILES` is retired for an empty cell that loops nothing, a lattice cut to what the fill reaches, and a tiling's copies bounded in commands

Status: accepted. Session 882. **One citation corrected by ADR 0827**, and the item this ADR
left open is closed by ADR 0828.

> Twice below — in the clause line under this one and in "What the retired count was for" — this
> ADR calls the sentence about a raster-based implementation "§8.7.3.1's NOTE 2" and "the same
> subclause's NOTE 2". The note it quotes is **§11.6.7's** NOTE 2, and §8.7.3.1 has no note about
> replication at all: the two printed under that heading are Table 74's, about a zero-sized `/BBox`
> and about `/XStep` and `/YStep` differing from the box. The note that does say a cell may be
> "evaluated once and then replicated" is §11.6.7's NOTE **1**, and it says it of the opaque
> imaging model. Every quotation below is verbatim and every measurement stands; what was wrong is
> the number under which the note was filed. ADR 0827 has the reading, and ADR 0828 measured what
> the two witnesses named at the end of this ADR are actually cut of.

Clauses: ISO 32000-2 §8.7.3.1 (tiling patterns: the cell "replicated at fixed horizontal and
vertical intervals", painted "as many times as necessary to fill an area", and NOTE 2 on a
raster-based implementation); §8.5.3.3 (the fill rules the lattice is cut by, and the implicit
close).
Code: `crates/pdf-model/src/content/pattern.rs` (`Interpreter::MAX_TILE_COPIES`,
`Interpreter::MAX_REACH_SCAN`, `Interpreter::repeat_cell`, `Interpreter::tile`, `tighter_of`,
`extent`); `crates/pdf-model/src/content/pattern/reach.rs` (new: `Reach`);
`crates/pdf-model/src/content.rs` (`Interpreter::reach_scanned`, and its two lines of
`checkpoint`/`restore`).
Tests: `crates/pdf-model/tests/hostile_budgets.rs::a_tiling_whose_cell_is_empty_loops_nothing_and_reports_nothing`,
`::a_tiling_whose_cell_marks_is_bounded_by_its_own_budget`, `::a_tiling_over_its_budget_keeps_whole_rows`,
`::a_fill_of_many_edges_does_not_pay_for_the_rows_it_cannot_reach`;
`crates/pdf-model/tests/tiling.rs::every_site_the_fill_states_is_painted`; `reach.rs`'s six unit
tests; `doc/checks/fixed-documents.toml`'s rows for `batch5/PDFIUM/PDFIUM-1122-0.pdf`,
`7803372.pdf` and `4650000.pdf`.
Documents: §8.7.3.1's ledger row, `doc/todo/49`, `doc/todo/10`, `doc/performance.md`,
`doc/todo/03` §43.

## Context

`batch5/PDFIUM` (379 documents, pdfium's issue tracker) surveyed at 66 incomplete, and the ink
ranking of those against `pdftoppm -cropbox` and `mutool draw` at 72 dpi had one head with both
references agreeing to a hundredth of a level: `PDFIUM-1122-0.pdf`, ours **70.92** against
`poppler` 80.51 and `mupdf` 80.52, reporting `LimitReached { limit: "MAX_TILES" }` and nothing
else. The page is one fill in a tiling pattern whose cell is a 10-unit square holding a red
square inside a grey grid, `/XStep 10 /YStep 10`, `/BBox [0 10 10 0]`: **4480 sites of a
two-command cell**, of which the constant afforded 4096 — every column and the whole rows from
the bottom that 4096 buys (ADR 0477's prefix) — so the top of the sheet was white. Both references
draw the whole sheet; ours, with the count gone, reads 80.52 by the same instrument and the two
top-left corners agree square for square at four times magnification.

The constant's history is in three ADRs and it is worth stating in one place, because the
decision rests on what each of them measured.

- **ADR 0271** lifted every budget over the 83 crawled documents that reached one. All 48 that
  reached `MAX_TILES` *terminate* — 0.06 s to 14.2 s, wanting 4104 to 895 500 tiles — and it
  concluded that "the count is the wrong quantity and a larger count is no safer". What kept the
  constant was one measurement of the loop rather than of any document: a pattern whose cell is
  *empty* executes no operator, so `MAX_OPERATIONS` never sees it, and with the count lifted
  1 000 000 empty tiles interpreted in 889 ms reporting nothing — 0.89 µs a trip, and
  `/XStep 0.001` over a 600-unit fill states 3.6 × 10¹¹ of them, about four days. That was the
  only bound on the trip count, and `doc/todo/49` recorded it as the reason "the count cannot
  simply be dropped in favour of `MAX_OPERATIONS`".
- **ADR 0430** made a site a *copy*: the cell's content stream is interpreted once, at the first
  site the span reaches, and every other site is its commands displaced
  (`pdf_render::Cell::repeat`), each copy charged to `MAX_OPERATIONS` before it is made (ADR
  0793 moved the check in front of the copy).
- **ADR 0477** found the count refusing the sites it had been sized to afford — the check sat in
  front of the interpretation, so a fill wanting twenty thousand sites was given none — and made
  the shortfall a prefix of whole rows, reported by name, on §8.7.3.1's own sentence: the budget
  decides *how many* and not *whether*.

## The clause

§8.7.3.1 puts the requirement on the processor:

> When performing painting operations such as S (stroke) or f (fill), the PDF processor shall
> paint the cell on the current page as many times as necessary to fill an area.

There is no permission in it to stop short; the one liberty it grants is the order, which the
same subclause says "is unspecified and unpredictable", so a prefix is a choice this tree makes
rather than one the standard makes for it (ADR 0477). A budget is this project's own answer to a
file that states more work than there is time for, on principle 3, and every site it withholds
from a page that would have finished is a departure from the sentence above — ADR 0271
established that on the web that departure was the whole of the population, not one of the 48
being a bomb. The same subclause's NOTE 2 says how the programs that draw those 48 whole do it:
"[i]n a raster-based implementation of tiling, it is advisable to treat all tiles as a single
transparency group" — the cell drawn once and replicated on the raster, at a cost per pixel
rather than per site. This tree replicates *geometry*, so that a tiling stays
resolution-independent and no backend learns what a pattern is (ADR 0430), and a site therefore
costs what a command costs: about 225 bytes of display list and two to three microseconds of
rasterisation, measured on this machine over four million of them.

## The decision

**`MAX_TILES` is retired.** Three things replace it, and none of them is a count of sites.

1. **A cell that drew no command is not looped.** `Interpreter::repeat_cell` returns before the
   loop where `Cell::is_empty()`. The argument is the clause's own: the cell "replicated at fixed
   horizontal and vertical intervals" is *this* cell, and a cell with no marks replicated any
   number of times is no marks — every site is painted, with nothing, and the trip count a file
   states is a cost a copy of nothing never charges. ADR 0271's four-day loop was the loop *for*
   this case, and the case needs no loop.
2. **A site is copied only where the fill's interior can reach its cell.** `pattern/reach.rs`
   scans the path — in pattern space, each subpath closed as §8.5.3.3 closes it for filling,
   each curve kept whole under its control box — onto the lattice one row band at a time, and
   keeps every column whose cell box an edge passes through or the interior covers on the band's
   centre line, by the fill rule. The module's own comment carries the proof that the two rules
   together miss nothing; it is conservative, so a site is kept wherever the scan cannot prove
   the box misses the interior. A stroke's outline is not a region the scan takes, and a stroke
   keeps its hull. On `7680183.pdf`, 249 hatched polygons of a plan, the sites fell from 539 729
   to 112 499 and the page from 3.3 s to 2.2 s.
3. **Every other site costs its cell's commands, and that cost is bounded twice in its own
   unit.** Once for the page, by `MAX_OPERATIONS`, as ADR 0430 charged it; and once for the
   tiling, by `Interpreter::MAX_TILE_COPIES` — **65 536 commands**, sixteen times the retired
   count at a one-command cell — so that one operator's expansion cannot take the page's budget
   from the operators after it. Both are asked before the copy; the prefix a budget affords is
   whole rows, asked for row by row on ADR 0477's argument, and the refusal is reported by
   whichever name is the tighter (`tighter_of`).

4. **And asking which sites the fill reaches is itself bounded, page-wide.** The scan is a
   saving rather than a requirement, and item 2 bought it with a cost nothing was charging for:
   `Reach::row` is one pass over the path's edges per row band, and a row that reaches *no* site
   spends no copy, so neither budget above sees it. That is item 1's shape one level in — a loop
   whose trip count the file states and whose body a budget cannot see — and it is answered the
   same way, in the unit of the work. `Interpreter::MAX_REACH_SCAN`, **4 194 304 edge tests a
   page**; past it the caller stops asking and takes every row whole, which is what a stroke
   already gets and what the code did before `reach.rs` existed. Running out is not a refusal and
   is not reported, because nothing is refused: the sites are all still attempted, and still
   bounded by the two budgets in item 3. **It was found by writing the hostile shape rather than
   by reasoning about the ordinary one**: twenty thousand unit squares stacked at the origin and
   one more six hundred units above them, at `/YStep 0.0006`, is a lattice of a million rows of
   which all but three thousand reach nothing, against a path of a hundred thousand edges —
   about 10^11 edge tests, and the fixture does not finish in **two minutes** with the constant
   lifted where it takes **0.116 s** with it. The value is fifteen times the heaviest page
   measured: `7680183.pdf`'s 249 tilings spend **under 300 000** tests between them, and
   `PDFIUM-1497-2.pdf` and `2760154.pdf` fewer than 5000 rows' worth apiece.

## Why the third survives, which was measured before it was chosen

The first draft of this round had no per-tiling bound: the empty cell not looped, the copies
charged to `MAX_OPERATIONS` alone, and the argument that the page's budget was the work bound
`doc/todo/49` asked for. On the head and on ADR 0477's two crawl rows it was right. On
`PDFIUM-1497-2.pdf` — the tracker's other `MAX_TILES` page, an A3 floor plan with fifty tilings
of a four-command cell, two of them 448 632 and 389 205 sites — it took **eleven seconds and 0.9
GiB** where the count took 2.2 s, and drew the page *worse*: the two large tilings spent the
whole four million, and the frame and title block after them were never drawn (ink 4.66 against
5.79 before, `mupdf` 7.59). A budget that is the page's lets one operator starve every operator
after it, and a fill that wants three quarters of a million copies is a page turn of eleven
seconds however the budget is apportioned — so the copies of a tiling are bounded on their own,
in commands.

**The value is a choice**, and what it admits and refuses was measured over every witness this
round and ADR 0477 had, with `examples/render_at` at scale 1 on four threads, the *before*
binary kept from the merged tree:

| document | tilings | largest, in commands | before | after |
|---|---|---|---|---|
| `PDFIUM-1122-0.pdf` | 1 | 8960 | 1.1 s, ink 70.92 | 1.1 s, **80.52** (references 80.51 / 80.52) |
| `7803372.pdf` (ADR 0477) | 14 | 21 320 | 1.1 s, 5.52 | 1.1 s, 9.45; the gate's band 11.1 → 18.9 |
| `4650000.pdf` (ADR 0477) | 1 | 17 384 | 1.1 s, 24.57 | 1.1 s, 28.66; the gate's band 49.2 → 57.3, inside the references' 43.7–62.9 |
| `7680183.pdf` | 249 | 7610 | 3.3 s, 0.25 GiB | **2.2 s, 0.13 GiB**, the reach's doing |
| `2760154.pdf` | 1 | 762 930 | 1.1 s, 16.72 | 1.1 s, 16.79 — cut at 65 536 rather than 4096 |
| `PDFIUM-1497-2.pdf` | 50 | 1 794 528 | 2.2 s, 5.79 | 2.2 s, 0.19 GiB, 5.95 — its two largest cut |

Every tiling the crawl's witnesses hold draws whole except the two that want a hundred thousand
sites or more, and those two are the same two under any value a page turn can afford: at 262 144
they would still be cut and `PDFIUM-1497-2.pdf` would take twice as long. What they want is
NOTE 2's implementation — a paint the display list does not have and three backends would have
to draw — and `doc/todo/49` carries them as that item's witnesses.

And the hostile shapes are the tests. A one-command cell at a step of 1/1024 over 600 units —
615 425 columns, 3.8 × 10¹¹ sites — copies exactly 65 536 commands and reports
`MAX_TILE_COPIES`, the page's own operators still counted after it; the same cell at 1/8 over
64 units, 521 columns, lays down 125 whole rows and not the 126th; the empty cell at the first
step finishes in the time one fill takes, reporting nothing.

## What was checked and is not the answer

- **Raising the count.** ADR 0271 said it and it holds: a count admits the expensive document
  and refuses the cheap one. The unit was what was wrong — the head's 4480 sites of two commands
  were refused where a cell of forty commands was afforded 163 840.
- **The page's budget alone**, above: right on three witnesses and wrong on the fourth, in both
  time and picture.
- **A separate page-wide budget for copies**, so that copies never starve operators. It fixes
  the starvation and not the eleven seconds, and it doubles the list a page may hold.
- **A per-tiling allowance for the scan** rather than a page-wide one. A page may state as many
  pattern fills as `MAX_OPERATIONS` affords, so a per-tiling number multiplies by that; the same
  argument that makes `MAX_OPERATIONS` the page's makes this the page's.
- **Bounding the loop by wall clock.** A deadline is a fact about the machine, and `interpret`
  is a pure function of the document and the view; a budget in commands is the same on every
  run. The decode deadline is the one clock in this tree and it guards a *decoder*.

## Consequences

- `PDFIUM-1122-0.pdf` draws its whole sheet; `batch5/PDFIUM` is 65 incomplete after it. The two
  crawl rows ADR 0477 added to `doc/checks/fixed-documents.toml` move to the bands their whole
  hatching reads, and a row for the head holds the gate's own reading.
- `doc/pdf.js`'s corpus, oracle and quorra gates move nothing: no page there reached the count,
  and the reach removes only sites the clip removed.
- `LimitReached { limit: "MAX_TILES" }` no longer exists; `MAX_TILE_COPIES` is the report where a
  tiling wants more than its budget. The survey lines of walked trackers are baselines and never
  ratchets (`doc/todo/03`).
- `doc/todo/49`'s first open bound is closed and the item it leaves is one with two witnesses.
- `doc/todo/10`'s table of bounds loses the `MAX_TILES` row's standing and gains
  `MAX_REACH_SCAN`: a saving that costs the page nothing it is not charged for.
