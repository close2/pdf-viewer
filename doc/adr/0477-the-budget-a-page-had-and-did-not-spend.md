# ADR 0477 — The budget a page had, and did not spend

Date: 2026-08-22 (session 647)
Status: accepted.

## Context

`doc/todo/03`'s chunk, for the ninth round running and the last one: the **3944 crawled documents
no chunk had ranked** — archives `7557` and `7803` whole, and all eighty-one of the twenty-four-member
archives. With them the SafeDocs crawl is **65 944 of 65 944 ranked**, which is the first time this
project can say anything about the population rather than about a sample of it.

The deepest row of the chunk is `7803372.pdf` at **−12.251 of 255** — a French school-canteen menu
whose *Jeudi* and *Vendredi* columns are hatched, which `pdftoppm`, `mutool` and `gs` fill (their
inks agree within 1.1) and which this tree drew as two small patches and a great deal of white.
Page one reports one thing: `LimitReached { limit: "MAX_TILES" }`.

That report is ADR 0271's bound, and its value is not what this ADR moves. What it moves is what
the bound *does* when it is reached.

## What the file states

Objects 16 through 43 are twenty-eight `/PatternType 1` dictionaries, all the same shape:

```text
<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 2 /BBox [0 0 1.6 1.6]
   /Matrix [1 0 0 1 0 595.4] /XStep 1.6 /YStep 1.6
   /Resources << /XObject << /Image17 17 0 R >> >> >>
```

The cell is one `Do` of an 8 × 8 one-bit image — an `iTextSharp`-family hatch, which is the
commonest producer in the population that reaches this bound at all. At 1.6 units a side, the two
table columns want something over twenty thousand sites apiece against a `MAX_TILES` of 4096, so
the fill was refused. **Refused entirely**: the check sat in front of the interpretation of the
cell, so a page that could afford four thousand sites was given none.

## The clause

§8.7.3.1 puts the requirement on the processor rather than on the file:

> When performing painting operations such as S (stroke) or f (fill), the PDF processor shall
> paint the cell on the current page as many times as necessary to fill an area.

A budget is this project's own answer to a file that asks for more times than there is time for
(principle 3: "Rust does not prevent resource exhaustion"), and nothing in ISO 32000-2 sets one.
What the standard does decide is that painting the cell **no** times is the furthest a processor
can get from that sentence, and that every site the bound *does* afford is a mark the producer
stated.

The order is the implementation's, which matters because it is what makes "a prefix" a legitimate
answer rather than an arbitrary one. The clause: "[t]he order in which individual tiles (instances
of the cell) are painted is unspecified and unpredictable", and Errata Collection 3's **Issue
#428** (`Review`/`Accepted`, p. 235) adds "(implementation dependent)" to the end of that very
sentence. A processor that stops after N sites has stopped in an order the standard handed it.

## The precedent, which is this subclause's own

This is §7.8.2's prefix rule, and the tree already applies it to the *other* of the two things a
tiling is made of:

- a page `/Contents` part that decoded only as far as its damage draws its prefix and reports the
  shortfall (ADR 0343);
- a tiling **cell's** content stream does the same, since ADR 0359 — the clause makes the cell a
  content stream, so §7.8.2 reaches it;
- an image whose samples stop short of the grid its dictionary infers paints the samples that
  arrived and leaves the rest (ADR 0356).

So the cell's *content* has drawn its prefix for a hundred and twenty sessions while the cell's
*replication* threw one away. The §8.7.3.1 ledger row carries both halves and the asymmetry was
legible in it the whole time.

ADR 0343's own test decides it the same way: the marks are **additive**, not substitutive. A site
is one more copy of the cell, not a different picture of it — so drawing four thousand of twenty
thousand loses nothing that drawing zero of twenty thousand keeps, and the report is what keeps a
tiling cut short distinguishable from a tiling the producer meant to be sparse.

## The decision

**Spend the budget instead of discarding it.** When the lattice a fill needs exceeds `MAX_TILES`,
report `MAX_TILES` exactly as before and then narrow the span to the prefix the bound affords —
whole rows first, laid down from the corner `Interpreter::tile` already interprets the cell at.
`affordable_span` is nine lines and states the arithmetic.

**The value stays at 4096 and so does the worst case.** The bound was always sized as *the number
of sites a fill may cost*; before this change a fill either cost at most 4096 sites or cost none,
and after it a fill costs at most 4096 sites. No page can now do work that the old check would have
refused — which is the property that makes this a change to the refusal rather than to the budget,
and is why it is separable from `doc/todo/49`'s open question about bounding work rather than count.

**What it does not do** is make these pages right. `7803372.pdf` goes from **9.083 to 11.096** of
ink against three references between 21.3 and 22.4: a fifth of the hatching where there was none.
The remaining four fifths are `doc/todo/49`'s and stay there.

## What moves, and how the reach was bounded

**The population is measured and not inferred**: `examples/open_one` over every one of the 65 944
crawled documents says **48 report `MAX_TILES` on page one**, spread over 35 archives — 0.073% of
the web, and the same 48 ADR 0271 counted, re-measured rather than copied out of a document.

**The reach is bounded by the code and confirmed by measurement, and it is worth being exact about
which is which.** The diff is entirely inside the `total > MAX_TILES` branch, and that branch is
the one that raises the report — so a page whose raster can change is a page that reports
`MAX_TILES`, and the 48 above are the whole of what can move. That is a proof rather than a sample,
and it is the reason the confirming run below is 8011 documents rather than all 65 944.

**Confirmed over 8011 documents, twice, with this tree's own panel** (631's rule — a reference's
panel differs run to run, so a reach is measured against ourselves), page one at scale 1, the two
binaries differing in this change alone: this round's whole chunk (3944), four previously-ranked
archives chosen because each holds documents that reach this bound (`0100`, `1530`, `6204`,
`7188`), every one of the 48, and every row of `doc/checks/fixed-documents.toml`. **42 rows move
and every one of them reports `MAX_TILES`.** Forty of the 42 gain ink; two lose a little
(`1530611.pdf` by 0.003, `6081466.pdf` by 0.056), which is a hatch laid over something darker than
the page and is the change working rather than against it.

**A forty-third row moved and was not this change**, which is worth recording because it is 626's
lesson on our *own* instrument rather than on a reference's: `1530980.pdf` is a 30 MB document that
takes about 9 s to draw, and under a load average above 100 it lost the harness's 30-second budget
in the first pass and not in the second. Re-measured alone, before and after agree at **88.5497**
to four decimals. A wall-clock budget is a measurement of the machine as well as of the tree,
whichever program is holding it.

`doc/history/647-*.md` has the table.

## What was checked and is not the answer

- **Not the bound's value.** Raising it is `doc/todo/49`'s and is refused there for a measured
  reason: `7680183.pdf` wants 42 282 tiles and took 14.2 s while `2760154.pdf` wants 765 440 and
  took 8.7, so the count is not the cost and a larger count is not safer.
- **Not `MAX_OPERATIONS` doing the job instead.** An empty cell executes no operator, so the trip
  count would be bounded by nothing — which is exactly what
  `hostile_budgets.rs::a_tiling_whose_cell_is_empty_is_refused_by_name` exists to hold, and it
  still holds: the bound is reported by name and the sites it affords are four thousand copies of
  nothing.
- **Not a partial row.** The prefix is whole rows wherever more than one row is affordable, because
  a ragged right edge would put a boundary on the page that the file states nowhere. Where a single
  row is itself over budget the row is cut, since a prefix of one row is all there is to keep.
