# 764 — The measure a maximum threw away

Sent at what 761 left owed: nine of the eleven priced readings on `rank_the_pages_we_are_alone_on`'s
marked head name a mechanism without naming which of the three measures their number is. The gate
prints both halves' measures now, every row of the head is priced against its own, and one page's
mechanism turned out to explain a different one. Parallel round, worktree `r764`, branch `round-764`.
**No pixel moves and no verdict moves** — the whole diff under `crates/` is `tests/oracle.rs`, and
every non-agreeing per-page line is byte-identical between the run before the round and the run
after it. ADR 0688.

## The instrument

`Distance::nearest` and `consensus_missed_in_three_measures` are each a maximum over three measures
and then a minimum over comparisons, and the first reduction discards a name `worst_ratio` keeps for
the contradicted ranking. `worst_ratio_in_three_measures` keeps it; `AloneOn` carries the measure and
the renderers for each half; `closest_reference_pair` and `nearest_reference` are the two reductions
written once instead of three times. Each row prints `[<measure> v <reference>]` and
`[<measure>, <reference> v <reference>]`, and a count under the list says how many rows divide one
measure by another.

That count is the reason the round exists: **most of the head does.** A note explaining why the
divisor is small has not thereby explained the numerator when the two are different measures.

## Read down the marked head, and it is three shapes

- **The mechanism reaches the measure and the note had not said which.**
  `AMBIGUOUS_OVERSIZED_BORDER` (mean over mean — a border is an area of ink), `AMBIGUOUS_ZERO_AREA_FILL`
  (similarity over similarity, and the similarity orders the three references exactly as that note's
  ink table does), `AMBIGUOUS_GLYPH_COVERAGE`'s `endchar.pdf` (mean over mean, its similarity against
  `mupdf` inside the bound).
- **The two halves are different measures, and naming them makes the row sharper rather than
  softer.** `bug766086.pdf` is 14.9× read like for like on the similarity where the row prints 5.68×;
  `issue16224.pdf` is 6.2× on the similarity and 2.6× on the mean, so trap 9's tenth mechanism holds
  either way; the five `freeculture` pages are 5.3× and 5.1× against a printed 3.39× and 3.30×, and
  on page 322 our own *mean* is inside its bound, so only the similarity puts that page on the list.
  What that says is what is left to explain, and on the book it is placement rather than ink — the
  ladders answer *how much* to four decimals and are silent about *where*, which is page 255's lesson
  arriving on the five pages that stayed.
- **The mechanism explains a different measure.** `AMBIGUOUS_STROKE_ADJUSTMENT`'s `bug1743245.pdf`,
  below.

## The finding

That note argues §10.7.5's single-pixel rule as two camps, in whole-page mean grey — every figure of
which re-takes to a ten-thousandth. The row's number is a **structural similarity against `poppler`**,
and `poppler` is in *our* camp on the mean, so the mechanism cannot be what the number is.

Priced by removal: `/SA true` renamed to `/S1 true` in place, eight bytes for eight, Table 58's
initial value for `SA` being `false`. Control first — on the unedited file, freshly rendered, all
four references and our own raster are byte-identical to the gate's cached panels.

**Not one reference moves by a single bit.** `poppler`, `mupdf`, `ghostscript` and `hayro` render the
`/SA`-free file identically to the original while our own raster moves 18.37 of 255, so on this page
the entry decides a pixel for this tree and for nobody else — `mupdf` and `ghostscript` never read it,
and `poppler` and `hayro` widen a sub-pixel stroke whether the document asks or not. Our agreement
with those two was trap 9's *two answers to two different questions*, in our own camp.

Our nearest falls from 31.43 to 2.62 against a divisor that does not move at all, so the ratio is
0.34 and the page would leave that list entirely. What the 31.43 *is* is the other half of §10.7.5 —
the coordinate adjustment `poppler` implements and this tree does not, which the same note records as
a departure and had never joined to a number. §10.7.5's ledger row carries the measurement now, and
its "nothing reports it because there is no page on which this device could do better" is unchanged.

## Measured

§2's sequence whole and green. `PDFREF_CACHE` on the shared warm cache at a **100% hit rate — 6707
reference renders from disk, 0 produced** — so no gate figure here measures another program. The
removal experiment and the pairwise tables did invoke `pdftoppm -cropbox`, `mutool draw -r`,
`gs -dUseCropBox` and `pdfref-hayro` deliberately; they are pixel arithmetic rather than timings, and
**no timing claim is made**. Load ran from 1 to 31 across the round; the gates were run at load under
6, and every reference invocation taken by hand was outside the harness's 30-second budget.

Before and after, the census and every non-agreeing per-page line are identical: 983 agrees, 61
contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render.
The only lines that differ between the runs are the ranking's own — the two bracketed measures per
row and the new count.

`fmt` clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2684 passed, 18 skipped**, doctests clean, the fuzz check clean, and the corpus, oracle, text
extraction, both censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents and conformance
(**192 passed**) gates all green.

Sweeps: `--bin unpriced` **93 failing bounds over 61 pages, 93 named, 0 not**, unchanged, still
`issue6069.pdf`. `--bin overtaken` **45** where 761 recorded 46; the notes this round rewrote cite
0688, which is what clears a hit. `--bin quoted` **237 figures read, 123 confirmed, 101
contradicted**, where 761 recorded 211, 106 and 105 — the read count rises with this round's new
pairwise tables and the *contradicted* count falls, which is the half worth reporting: the first
draft of those tables and sentences produced five new hits, every one of them a **bound multiple
written after a measure's name** where the sweep and a reader both take it for a level of 255. That
is ADR 0499's units lesson arriving as a writing rule — **put the number before the measure's name,
or a bound multiple reads as the measure** — and all five are gone. `--bin pointers` 131 absent and
`--bin quotations` 38 diverging, both unchanged.

Not a fifth round (`tools/round.sh`) and no pixel moved, so §5's installed binaries were not rebuilt
and `doc/todo/00` step 7 was not re-run — neither has an input that changed, and the byte-identical
per-page lines say so a second way. `examples/render_at` and `examples/compare_rasters` were built in
release from this worktree, with `pdf-sandbox-worker` beside them and no stale copy under
`examples/` (trap 10 and its twin). `compare_rasters` was validated against `AMBIGUOUS_LINK_BORDER`'s
existing table before any new number was read off it, and every panel cropped by hand was cropped
top-left to the smallest, which is what `normalise::to_common_size` does.

**One process mistake, recorded because it nearly cost a neighbour.** Two of this round's document
edits were written into the *main* tree instead of the worktree. Both were moved to `r764` and both
files restored from `HEAD`; `git status` on `/home/cl/projects/pdf-viewer` is clean but for
`.claude/`. The lesson is the briefing's own and it is about paths rather than care: an absolute path
under `doc/` is the main tree's unless it names the worktree.

## Changed

- `crates/pdf-model/tests/oracle.rs` — `worst_ratio_in_three_measures`, `AloneOn`,
  `closest_reference_pair`, `nearest_reference`, the `Examined` field, the two bracketed measures on
  each row of `rank_the_pages_we_are_alone_on` and the mixed-measure count; the measure named in
  `AMBIGUOUS_LINK_BORDER`, `AMBIGUOUS_OVERSIZED_BORDER`, `AMBIGUOUS_STROKE_ADJUSTMENT`,
  `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`, `AMBIGUOUS_ZERO_AREA_FILL`, `AMBIGUOUS_ONE_LADDER` and
  `AMBIGUOUS_GLYPH_COVERAGE`, with the pairwise table each reading is taken from.
- `doc/conformance/ledger.toml` — §10.7.5 gains the removal measurement and the number its
  unimplemented first requirement had never had. No status moves; the row was already `partial`.
- `doc/todo/00-ambiguous-bucket.md` — the brackets, and where a mechanism has to be priced.
- `doc/traps/oracle-and-references.md` — trap 9's tenth bullet gains the mixed-measure reading,
  because the round reading that bullet is the round deciding whether a removal is worth taking.
- ADR 0688.

## Owed

- **The thirteen unmarked rows below the head still have not been read as a list**, unchanged from
  761. All thirteen are like for like on both halves, which the new count says without naming them.
- **No sweep asks this question mechanically.** `--bin unpriced`'s right-hand side is the gate's
  per-page line and this number is not on it, so a sweep over the ranking rows needs its own parser
  and its own population; ADR 0688 says why it was not built now and when it should be.
- **The like-for-like ratio is in the notes and not on the row.** A reader of a mixed row wants the
  divisor re-read on our measure, and the honest form of it is a minimum over *all* pairs on that
  measure rather than the same pair re-read — a fourth quantity nobody has argued for.
- Unchanged from 751, 756 and 761: the 22 dropped pages of the ranking, `AMBIGUOUS_ONE_LADDER`'s hold
  on `issue12337.pdf`, `doc/todo/12`'s 278 pages, `poppler`'s border placement not reported to
  `poppler`, `border_overhang_census` having no crawl scope, and `AMBIGUOUS_IMAGE_REDUCTION`'s prose
  sections not matching its array.
