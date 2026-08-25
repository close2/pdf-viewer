# 761 — The bound a consensus would have set

Sent at the *we are alone* list — 751 gave it a floor, 756 changed its head — with an instruction to
diagnose downward from the head and to state where the round stops. The criterion turned out to be
in the gate already: `Judgement::CORPUS` widens a consensus's bounds to **twice its members' own
spread**, and asking that question of the closest pair splits the list into a head and a tail.
Parallel round, worktree `r761`, branch `round-761`. **No pixel moves and no verdict moves** — the
whole diff under `crates/` is `tests/oracle.rs`, and every non-agreeing per-page line is identical
between the run before the round and the run after it. ADRs 0684 and 0685.

## The criterion, and why it is not a budget

751 required the list's numerator to be outside the class **floor**, on the argument that below it
the nearest reference would have *accepted* the page. The floor is where that argument stops: it is
what `pdfref::decide` returns *because* no consensus formed, the weakest bound in the gate, and the
references on the same page are measured against the same constant and are frequently further
outside it than we are. Above the floor the sentence *we are alone* is true and weak.

So the same question is asked of the bound the gate actually applies everywhere else. Take the
closest pair, widen the class bounds to `Judgement::CORPUS`'s factor times that pair's own
comparison, measure our nearest comparison against the result:

- **`[widened: outside]`** — no reading of these references forgives us. Worth a round.
- **unmarked** — a consensus at that spread would have accepted us. The page is alone against a
  constant and the answer is in the divisor.

**A round works the marked rows and stops at the first unmarked one.** The factor is read off
`Judgement::CORPUS` rather than written down a second time, and the list now prints ten rows or the
whole marked head, whichever is longer — a count naming a head of thirteen under a list of ten is a
queue nobody can open.

**It is an exact test read against a readable one.** Both halves of the printed ratio are a maximum
over three measures, so a ratio at 2.0 or above implies the mark; the converse fails where our worst
measure and the pair's are different measures, and **two pages of this run are exactly that** —
`freeculture.pdf` page 1 at 1.83× and `copy_paste_ligatures.pdf` at 1.65×, ours the structural
similarity on both and theirs the mean. That is why the gate computes it per page.

## What the head turned out to be

The list is 26 and the marked head is 13. Eleven are the pages sessions 518, 744, 751 and 756
already priced, each naming the mechanism in its numerator or its divisor. The two the mark adds
were both diagnosed here:

- **`freeculture.pdf` page 1** — in `AMBIGUOUS_IMAGE_REDUCTION` with no measurement of its own. The
  cover: one 1366 × 2048 JPEG at 201 ppi under an `/SMask` white everywhere, reduced 2.79 to 1, its
  art a field of rules whose period is 38–41 source rows over four sampled columns. Its lettering is
  **inside the image**, so `Interpretation::glyphs` gives it the *vector* tolerance and a page of
  nothing but image edges is held to bounds measured on flat fills — which is why it also heads
  `rank_the_manufactured_ambiguity`. Four ink ladders land inside 0.68 of 255; taking each
  renderer's mean distance from the other three, ours and `mupdf` are joint most central at 14.55
  against `poppler`'s 17.49 and `ghostscript`'s 27.19. Differencing the panels leaves ink on the
  rules' and letterforms' edges and nowhere else, and ours-against-`mupdf` leaves the same pattern
  as `poppler`-against-`mupdf`.
- **`copy_paste_ligatures.pdf`** — priced in ink by an earlier round, which is not the measure any
  of it is taken on. Our number is the mean against `mupdf`, 14.0709 of a bound of 5.00; widen the
  bound by the factor and the **mean goes inside** — 14.0709 against 17.08 — while the similarity
  does not, 0.85082 against 0.89224. And ours against `hayro` is mean 4.3906, similarity 0.98537,
  closer than any two voting references are to each other, which is the two-camp reading at its
  extreme and is evidence about the verdict rather than about us.

## And one page below the mark, because its own note disowned it

`freeculture.pdf` page 255, at 1.35× and unmarked. **Two paragraphs of one doc comment were about
it and did not agree**: a closed-form ink table from the two-hundred-and-thirty-third session under
*nobody here is drawing anything anybody else is not*, and a later paragraph saying **it has never
been opened** and *whatever it is, it is not the diagnosis these paragraphs make*.

Both were right about what they measured. The old table re-takes to the thousandth five hundred
rounds later — and **the ink answers how much and is silent about where**. The page is the book's
other full-page cartoon: one 9258 × 12259 `CCITTFaxDecode` stencil at 2182 ppi reduced 23 to 1, with
a running foot for text. It was the extreme member of `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` on three
of the four measures at once, and it is `AMBIGUOUS_IMAGE_REDUCTION`'s now, beside page 171, with
four ladders converging inside 0.066 of 255 and our render the most central of the five.
`ghostscript` is 15.2 to 16.6 from everybody and carries 20 distinct levels where the other four
carry 255 or 256.

**A disclaimer inside a group note is a page nobody is holding**, whatever its rank, and that is the
one exception the criterion admits. `grep` finds them in a second.

## Measured

§2's sequence whole and green, its core four re-run after the round's final edit. `PDFREF_CACHE` on
the shared warm cache at a **100% hit rate — 6707 reference renders from disk, 0 produced** — so no
gate figure here measures another program. The ink ladders and the pairwise tables *did* invoke
`pdftoppm -cropbox`, `mutool draw -r` and `gs -dUseCropBox` deliberately; they are ink and pixel
arithmetic rather than timings, and **no timing claim is made**. Load ran from 1 to 22 across the
round, which is what parallel neighbours cost; every reference invocation here was taken by hand
and outside the harness's 30-second budget, so none of it could be a measurement of the load.

Before and after, the census and every non-agreeing per-page line are identical: 983 agrees, 61
contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render.
The only lines that differ between the two runs are the ranking's own — the marks, the extra rows
and the new count.

`fmt` clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2676 passed, 18 skipped**, doctests clean, the fuzz check clean, and the corpus, oracle, text
extraction, both censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents and conformance
(**192 passed**) gates all green.

Sweeps: `--bin unpriced` **93 failing bounds over 61 pages, 93 named, 0 not**, unchanged, still
`issue6069.pdf`. `--bin overtaken` **46**, where 751 and 756 both recorded 48; the three notes this
round rewrote now cite the newest ADRs in the tree, which is exactly what clears a hit, and the one
hit printed under a note of this round's is 751's pre-existing `0647`/`issue12337.pdf`. **The drop
is consistent with those citations and was not separately attributed** — the sweep has no *before*
that a round may take without disturbing a neighbour's tree. `--bin quoted` **211 figures read, 106
confirmed**, up from 190 and 101, and the movement is this round's new pairwise figures — numbers
the gate does not print, because the gate's line is our render against a consensus's *worst* member
and every row of those tables is one named pair. All three tables say so above themselves, which is
751's own correction applied on arrival rather than after a sweep. `--bin pointers` 131 absent and
`--bin quotations` 38 diverging, both unchanged.

Not a fifth round (`tools/round.sh`) and no pixel moved, so §5's installed binaries were not rebuilt
and `doc/todo/00` step 7 was not re-run — neither has an input that changed, and the byte-identical
per-page lines say so a second way. The `examples/render_at` and `examples/compare_rasters` used for
the ladders were built in release from this worktree this round, with `pdf-sandbox-worker` beside
them and no stale copy under `examples/` (trap 10 and its twin).

## Changed

- `crates/pdf-model/tests/oracle.rs` — `outside_what_the_closest_pair_would_allow` and
  `corpus_widening_factor`, the `Examined` field, the mark and the count on
  `rank_the_pages_we_are_alone_on`; `freeculture.pdf page 255` moved from
  `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` to `AMBIGUOUS_IMAGE_REDUCTION` (370 → 369 and 17 → 18); the
  readings of page 255, `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf`; the dense-text
  band re-derived from this run's own per-page lines.
- `doc/todo/00-ambiguous-bucket.md` — the mark, where a round stops, and the disclaimer exception.
- `doc/traps/oracle-and-references.md` — trap 9's tenth bullet gains the mark, because the round that
  reads that bullet is the round deciding whether a removal is worth taking.
- ADRs 0684 and 0685.
- No ledger row: the round implements no normative requirement, and the one clause its comments
  cite, §10.7.4, is already `partial`.

## Owed

- **Nine of the eleven older head readings price a mechanism without naming which of the three
  measures their number is.** `outside_by_in_three_measures` returns a maximum and discards the
  name where `worst_ratio` keeps it, so the gate could print it and `--bin unpriced`'s rule could be
  asked of this list as well as of the contradicted one.
- **The thirteen unmarked rows below the head have not been read as a list**, and the criterion says
  they are the divisor's rather than ours. Whether that is true page by page is unmeasured, and
  `freeculture.pdf` page 255 is one instance of it being worth opening anyway.
- **`AMBIGUOUS_IMAGE_REDUCTION` is now eighteen pages and its note is numbered by prose sections that
  never matched the array.** Nothing turns on it; a reader counting "a seventeenth" against the
  array will not get the same number.
- Unchanged from 751 and 756: the 22 dropped pages of the ranking, `AMBIGUOUS_ONE_LADDER`'s hold on
  `issue12337.pdf`, `doc/todo/12`'s 278 pages, `poppler`'s border placement not reported to
  `poppler`, `border_overhang_census` having no crawl scope.
