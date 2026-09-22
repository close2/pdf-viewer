# ADR 1206 — What §11.4.4's second element run costs, on the one curated page that reaches it

Status: accepted. Session 1184.
Prices: ADR 1107, which built the second run and priced it only as "a second run of the elements".
Depends on: ADR 1170, whose synthesised group is what put a page into the population, and round
1188's `crates/pdf-model/examples/non_isolated_group_census`, which counted it.
Context: `crates/render-cpu/src/lib.rs` (`CpuRasterizer::remove_the_backdrop`),
`doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf`.
Clauses: ISO 32000-2 §11.4.4, §11.7.4.3.

## 1. Why there is a number now and was not before

§11.4.4's result step removes the backdrop from a non-isolated group's accumulated colour, and its
NOTE 4 says how:

> For shape and alpha, backdrop removal can be accomplished by maintaining two sets of variables
> to hold the accumulated values.

Under Normal the removal cancels against the re-compositing; under any other mode it does not, and
`render-cpu` runs the group's elements a second time onto transparency for Table 140's group alpha.
ADR 1107 built that and recorded the cost in words, because the population it could measure was
empty: counted on *file-stated* groups it was 0 of 1477 curated first pages. ADR 1170 then had the
interpreter synthesise exactly such a group for §11.7.4.3's implicit overprint construction, so the
premise expired and the population is no longer empty — 2 groups on 1 curated first page, 1233 on
205 crawled ones, counted by round 1188 on `CpuRasterizer::group_buffer`'s own conjunction.

## 2. The measurement

`doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf` page 1 is the whole curated population: two
groups, both §11.7.4's implicit construction, both under `/Multiply`.

`callgrind_rasterise`, **20 draws, one sitting, two arms built from one tree**, the second with
`group_buffer`'s call to `remove_the_backdrop` planted away:

| arm | I refs |
|---|---|
| without the second run | 6 225 323 395 |
| with it | 6 668 515 460 |

**+443 192 065 instructions, +7.12%, or 22.2 M a draw.**

## 3. What the figure is and is not

- **It is the page's marks, not a constant.** The second run re-rasterises the group's own
  elements, so a group of one fill costs one fill and a group of a thousand glyphs costs a
  thousand. Quoting 7.12% of a *different* page would be quoting this page's element count.
- **It is not paid anywhere else.** A page stating no non-isolated group under a mode of its own
  pays the one comparison at the call site; the whole curated corpus except this page is in that
  case, which is the honest reason the figure was not taken sooner rather than an argument that it
  is small.
- **It is the price of the clause, not of a choice.** The alternative to the second run is not a
  cheaper removal — it is the wrong picture, which is what NOTE 3's cancellation stops being when
  the mode is not Normal. A round that wants it cheaper should look for a way to accumulate Table
  140's `αgn` *beside* the first run rather than for a way to skip it; §11.4.8's recurrence for
  shape and alpha reads no colour, which is what makes a second colour pass more than is needed.
