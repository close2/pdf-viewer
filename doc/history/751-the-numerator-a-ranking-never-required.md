# 751 — The numerator a ranking never required

744's two owed items taken together, because they are one question: what the *we are alone* list
should contain, and what the list as it stood was pointing at. Parallel round, worktree `r751`,
branch `round-751`. **No pixel moves and no verdict moves**: the whole diff under `crates/` is
`tests/oracle.rs`, a test target no library links, and every non-agreeing per-page line and every
census figure is identical between the run before the round and the run after it. ADR 0663 has the
argument.

## What the ranking should contain

**Our own number has to be outside a bound**, and the reason it is not an arbitrary threshold is the
one thing this round had to check before it could act. `Examined::outside_the_bound` refuses to rank
ambiguous pages against their bound, on the ground that "no two references agreed, so the bound
beside them decided nothing" — and it is right, about *ranking by a page-dependent quantity*.
`pdfref::decide` returns the **class `Tolerance` unwidened** where no consensus formed, because
widening is a consensus's and there is none, so on these pages *outside 1* means outside the fixed
floor for the page's tolerance class: the same constant for every text page in the pool. Testing a
page-independent threshold is a different operation from ranking by a page-dependent one.

Below it the numerator says the opposite of the list's name: our nearest inside all three bounds
means that reference, in a consensus, would have **accepted** the page.

**What moves, measured rather than assumed.** The list goes 48 → 26. The printed ten lose exactly
one page and gain exactly one: out goes `issue11403_reduced.pdf`, which led at 9.06× on ours 0.51
over 0.06 with a verdict line reading `differing alone` — the instance 744 named as the defect — and
in comes `endchar.pdf` from rank eleven, which is in the sublist. Nothing else in the ten moves, and
**no page the ranking never printed becomes the head**, which is ADR 0349's standing warning and the
one thing a filter on a ranking can get wrong.

Two things are written into the printed line. The count underneath is **the sublist now rather than
the population** — how many of the listed pages have a closest pair inside all three bounds while we
are outside one, which is 9 of the 26 and is 744's sublist reproduced by the instrument that now
defines it. And the 22 pages the requirement drops **stay printed as a number**, because a caution
nobody can count is trap 11 with the sign reversed.

**In four measures the same requirement changes nothing at all**, and that is the asymmetry as
arithmetic: `consensus_missed_by` is above 1 on every ambiguous page by construction, so *ours >
theirs* already implied *ours > 1* there. The three-measure denominator has no such floor, which is
the whole of what went wrong. It is written into that count anyway, so a fifth measure joining
`Tolerance` has to re-check the implication rather than inherit it.

## What the four turned out to be

Four different shapes, which is the answer to the question 744's trap-9 finding poses about the
sublist:

| page | our number is | the divisor is | and that is |
|---|---|---|---|
| `bug766086.pdf` | 2.58, the **similarity**, against `poppler` | `mupdf` + `ghostscript` at 0.45 | trap 9's *shared gap* |
| `issue16224.pdf` | 1.13 against `mupdf` | `poppler` + `mupdf` at 0.41 | trap 9's **tenth** mechanism |
| `endchar.pdf` | 1.97, the **mean**, against `mupdf` | `poppler` + `ghostscript` at 0.83 | neither |
| `issue12337.pdf` | 1.12, the **mean**, against `ghostscript` | `mupdf` + `ghostscript` at 0.88 | neither |

**A sublist is not a diagnosis.** One of the four is the mechanism 744 measured on `freeculture.pdf`,
one is a different bullet of the same trap, and two are not that trap at all.

**`bug766086.pdf`, the new head, measures one annotation twice.** `AMBIGUOUS_LINK_BORDER` said ours
and `poppler`'s both draw the border and priced the ink — 20.61 against 20.73, which is right and
hid a pixel. The measure this page's 2.58 is taken on is the **structural similarity**. Read off the
two rasters, on a 200 × 50 page where a user unit is a device pixel and `/Rect [5 10 190 40]`:
ours strokes columns 5 and 189 and rows 10 and 39, `poppler` columns 5 and **190** and rows 10 and
**40** — one pixel outside the rectangle on two of the four sides, where §12.5.4 says "the border
shall be drawn completely inside the annotation rectangle". It is `AMBIGUOUS_OVERSIZED_BORDER`'s
finding at a width of 1 instead of 112.

And both halves of its 5.68× are that annotation. Removing it — `/Annots [4 0 R]` → `/Annots []`,
same byte length so the cross-reference table resolves — takes our number from **2.58 bounds to
0.43** while the pair the ratio divides by is **byte-identical to the digit**, because neither of
those two draws the annotation at all.

**`issue12337.pdf`'s numerator is the finding.** One `/Highlight` with no `/AP`, `/QuadPoints` and
`/Rect` the same rectangle. Yellow pixels: ours x 49..296, inside and flush; `poppler` 22..323,
`mupdf` 23..321, `ghostscript` 31..314, `hayro` none. All four that draw it agree about the rows and
bulge only sideways, and the page's worst tile is exactly where our yellow stops and theirs
continues. Removing the annotation moves our nearest 1.12 → 0.61 against a divisor that moves 0.88 →
0.89, so **without it the page is not on the list**. The ranking is right that we are alone, and the
reason is that we are the only one of five inside the region §12.5.6.10 and Table 166 both state.

**`issue16224.pdf` is trap 9's tenth mechanism at single-page scale**: the `libfreetype.so.6` pair
0.41 bounds apart while each is 3.05 and 3.11 from `ghostscript`, and ours 1.13 from `mupdf`, less
than half `ghostscript`'s distance from the same reference. Over the whole pool the mechanism holds
— `poppler` + `mupdf` is the closest pair on 23 of the 48 and on 137 of the other 788, while
`poppler` + `ghostscript`, the pair sharing no glyph rasteriser, is closest on 2 of the 48 against
333 of the 788, where it is the commonest closest pair there is.

**`endchar.pdf`'s limit stood on one ladder and has four.** Ours 59.4874 → 59.8367 → 61.1486 →
60.9729 at 1×, 4×, 8× and 32×; `mupdf` 58.1554 → 60.9314, `poppler` 59.0589 → 60.9757, `ghostscript`
59.6630 → 61.0843. Four independent ladders inside **0.153 of 255**, ours between two of them, so
the coverage is agreed and what is left is §10.7.4's glyph scan conversion on a 15 × 34 raster.
Three of the four figures the note was written on reproduce to the hundredth and **the fourth is
ours** — 59.4874 against a recorded 59.39 — so the number that moved in the rounds between is this
tree's own while every reference is where it was.

## The instrument this round adds

**Take the mechanism out of the document and re-measure *both* halves of the ratio.** It is trap 9's
own instrument, pointed at our own accusation instead of somebody else's excuse, and it settled two
of these four pages in minutes. A denominator that does not move is a denominator that was never
about the page.

## Measured

Four full oracle runs and §2's sequence whole, the last of it after the round's final edit.
`PDFREF_CACHE` on the shared warm cache at a **100% hit rate — 6707 reference renders from disk, 0
produced**, so the gate spawned no reference renderer and no gate figure here measures another
program. The ink ladders and the annotation-removal runs *did* invoke `pdftoppm -cropbox`,
`mutool draw` and `gs -dUseCropBox` deliberately, and they are ink and pixel arithmetic rather than
timings. Load ran from 0.5 to 18 across the round, which is what three parallel neighbours cost;
**no timing claim is made and none was needed**.

Before and after the change, every non-agreeing per-page line and the whole census are identical:
983 agrees, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable,
18 no render. The only lines that differ between the two runs are the ranking's own.

§2's sequence, run whole and green, and its core four re-run after the round's last edit: `fmt`
clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2656 passed, 18 skipped**, doctests clean, the fuzz check clean, and the corpus (**974 documents,
67 incomplete**), oracle, text extraction (99.2% against `pdftotext`, 99.8% against PDFBox), both
censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents (**40 checked, 0 absent**) and
conformance (**192 passed**) gates all green.

Sweeps: `--bin unpriced` **93 failing bounds over 61 pages, 93 named, 0 not**, unchanged, still
naming `issue6069.pdf` as the one page whose line cannot say what its verdict rests on. `--bin
overtaken` **48**, unchanged, and the one hit this round adds is ADR 0663 joining ADR 0647 on a note
already listed. `--bin pointers` 131 absent and `--bin quotations` 38 diverging, both unchanged.
**`--bin quoted` moved, 170 figures read → 190, 100 confirmed → 101**, and the whole of the movement
is this round's two `examples/compare_rasters` tables: nineteen pairwise figures the gate does not
print, because the gate's line is our render against the *worst* member of a consensus and every row
of those tables is one named pair. Both notes now say so above the table, which is the sweep working
rather than a defect — a figure written finer is another instrument's, and the instrument is named.

Not a fifth round (`tools/round.sh`) and no pixel moved, so §5's installed binaries were not rebuilt
and `doc/todo/00` step 7 was not re-run — neither has an input that changed. The `examples/render_at`
and `examples/compare_rasters` used for the ladders were built in release from this worktree this
round, with `pdf-sandbox-worker` beside them and no stale copy under `examples/` (trap 10 and its
twin).

## Changed

- `crates/pdf-model/tests/oracle.rs` — `rank_the_pages_we_are_alone_on` requires the numerator
  outside a bound and prints the sublist count and the dropped count; three group notes gain their
  measurements: `AMBIGUOUS_LINK_BORDER`, `AMBIGUOUS_ONE_LADDER`, `AMBIGUOUS_GLYPH_COVERAGE`.
- `doc/todo/00-ambiguous-bucket.md` — step 1's pointer, the third ranking's entry, the reading of
  the four, and the removal instrument.
- `doc/traps/oracle-and-references.md` — trap 9's tenth bullet gains the whole-pool measurement and
  the shared-gap divisor.
- ADR 0663.
- No ledger row: the round implements no normative requirement. The five clauses its comments cite —
  §10.7.4, §12.5.2, §12.5.4, §12.5.5 and §12.5.6.10 — are already `partial`, `partial`, `partial`,
  `partial` and `implemented`, so §3's requirement is met without an edit.

## Owed

- **The 22 dropped pages are a population and nobody has asked what they are.** Nine of them are
  `standard_fonts.pdf` and four `freeculture.pdf`; whether that concentration is the same
  denominator effect acting where our own number is inside is unmeasured.
- **`AMBIGUOUS_ONE_LADDER` holds `issue12337.pdf` on an ink argument that could not see its
  finding**, and the page arguably belongs with `AMBIGUOUS_MARKUP_ARTWORK`'s clause. The note now
  carries both; moving the name between groups was not taken.
- **`doc/todo/12`'s 278 pages**, unchanged from 741 and 744.
- Unchanged from 744: `Distance` and `outside_the_bound` disagree about the contradicted pool with
  nothing stating which a round reaches for first; a *width* division and a *camp* division are
  treated alike; a voting reference whose raster is constant still votes; `freeculture.pdf` page
  255; the owner's `git stash drop`.
