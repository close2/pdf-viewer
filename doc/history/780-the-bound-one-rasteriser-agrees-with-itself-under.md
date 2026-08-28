# 780 — The bound one rasteriser agrees with itself under

Sent at the oracle's contradicted pages, a robustness round told to take the sharpest page or
bound the ranking names and to prefer one where several pages fall to one mechanism. The ranking
named a bound, and the tail under it — the population session 737 recorded as *ordered and not
yet read in that order* and 741 and 764 carried unchanged — turned out to be one mechanism 32
pages wide. Parallel round, worktree `r780`, branch `round-780`, a fifth round. **No pixel moves
and no verdict moves**: the whole diff under `crates/` is `tests/oracle.rs`, and the census and
every non-agreeing per-page line are identical between the run before the round and the run
after it. ADR 0717.

## What the instruments named

The pristine baseline run (983 agrees / 61 contradicted / 836 ambiguous, the whole sequence's
figures below) prints a fully priced pool: `--bin unpriced` reports every failing bound named by
the note that holds its page, and everything above rank ten of `rank_the_contradicted_by_the_bound`
is diagnosed — the JBIG2 decoder pages, the `DeviceCMYK` press pages, the link borders. What the
ranking *names* under its head is the population: most of the pool is furthest outside on the
differing fraction, `doc/todo/12`'s bound, spanning about 1.0x to 29x. Read off the per-page
lines, 32 of those pages are convicted by `poppler` and `mupdf` **alone** — all 27 of
`CONTRADICTED_GLYPH_EDGES`, four of `CONTRADICTED_SUBSTITUTED_FONT`'s, and
`CONTRADICTED_SUBPIXEL_IMAGE`'s one — and those two are the one voting pair that hints its glyphs
through a single rasteriser, re-checked on this machine rather than inherited: `objdump -p` names
`NEEDED libfreetype.so.6` in both `pdftoppm` and `mutool`, and `libgs.so.10` names no FreeType
and defines `FT_*` symbols of its own statically linked copy.

## The measurement

Trap 9's tenth mechanism was measured on the ambiguous pool's ranking divisor (ADRs 0647, 0663);
on a contradicted page the same pair sits in `widened_to`, deriving the bound the verdict rests
on. Six `examples/compare_rasters` invocations per page over the baseline run's artefacts,
panels cropped top-left to the common size where they differ, calibrated at both ends before
anything was read (trap 13): ours-v-`poppler` on `issue6069.pdf` reproduces the gate's line
digit for digit, and `poppler`-v-`mupdf` on the same page prints differing 3.2738%, ADR 0606's
figure to the fourth decimal.

Over all 32 pages: the convicting pair's differing fraction runs **0.00% to 4.37%, median
2.33%** — an exact printed 0.00% on three pages — while **every pair containing `ghostscript`
runs 5.32% to 13.37%, median 6.8%**. The distributions do not overlap, so on every page the only
two renderers inside the class floor of each other are the two hinting through one FreeType. The
control: `ghostscript` fails the same differing-fraction bound against **both** members of the
convicting pair on **32 of 32** pages, and further than we do on 27 of them (our best-against-
the-pair median 5.70% against its 6.75%; the five exceptions are named in the ADR). Put the
third voting reference where our render stands and the same consensus contradicts it everywhere
— trap 12's `colors.pdf` control taken over a population instead of a page.

## What was decided, and what was not

The gate prints the population now: `rank_the_contradicted_by_the_bound` counts, under its
differing-fraction line, the pages convicted by the sharing pair, citing trap 9 and ADR 0717.
Calibrated before being believed — the count printed is 32, the by-hand count off the baseline
log. The notes carry the measurement where the pages live: `CONTRADICTED_GLYPH_EDGES` the whole
table and control, `CONTRADICTED_SUBSTITUTED_FONT` its four members, trap 9's tenth bullet the
contradicted-pool paragraph, `doc/todo/12` the per-verdict control beside ADR 0243's population
figure, `doc/oracle-and-corpus.md` §3b the new line.

**No verdict moves, no bound moves, no consensus rule changes.** The measurement is not evidence
our phases are right — agreement runs in one direction only, and `hayro` is refused a vote on
exactly the shared-code ground. Moving the bound is still `doc/todo/12`, whose requirement 2 (a
floor from a fourth independent rasteriser) this measurement does not meet; disqualifying the
pair is the move trap 9's fifth bullet already prices — nothing would be left to vote on text.

## Measured

Three oracle runs. The pristine baseline and the after-run used `PDFREF_CACHE` on the shared
warm cache at a **100.0% hit rate — 6707 reference renders from disk, 0 produced** — so no
measurement figure of the round measures another program; the §2 line then re-rendered
everything fresh into this worktree's own cache and reproduced every verdict (above). The
`objdump`/`nm` checks and the `compare_rasters` sweep are pixel and symbol arithmetic, not
timings; **no timing claim is made**. Load ran from 0.7 to 36 across the round with three
parallel neighbours building; the one wall-clock casualty is the `outlines` flake above.

Before and after, the census and every non-agreeing per-page line are identical: 983 agrees, 61
contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no
render. The only lines that differ are the ranking's own — the one added count.

A fifth round (`tools/round.sh`), so §2's sequence ran whole and green after the final edit:
`fmt` clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2717 passed, 18 skipped**, doctests clean, the fuzz check clean, and the corpus (**974
documents, 67 incomplete**), oracle, text extraction (99.2% against `pdftotext`, 99.8% against
PDFBox), both censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents (**40 checked, 0
absent**) and conformance (**207 passed**) gates all green — and §5 rebuilt and installed the
eight binaries into `target/` from this worktree's own build directory (`cargo metadata`'s
answer, not the literal path trap 15 warns about).

Two events of the sequence are worth their lines. **`outlines.rs`'s
`an_outline_resolves_against_the_page_tree_once` failed one `nextest` run at a one-minute load
above 20** — its bound is a wall-clock *ratio* (a 20 ms resolution against a 2 ms single
search), and under three neighbouring rounds' builds the two sections drew different schedulers;
it passed in isolation and in the full re-run, and the whole diff of this round is a test file
its assertion never touches. The shape is the one session 776 fixed for the launch test,
arriving in a different test. And **the §2 oracle line ran against this worktree's own empty
cache — 6698 reference renders produced fresh, 0.1% hit rate — and every page's verdict is
identical to the warm-cache baseline's**; the only per-page differences are renderer
failure-message texts, among them the briefing's known r707 contamination, which the fresh
cache does not carry.

Sweeps, before and after against the pristine baseline: `--bin unpriced` **93 failing bounds
over 61 pages, 93 named, 0 not**, unchanged both sides, still naming `issue6069.pdf` as the one
page whose line cannot say what its verdict rests on. `--bin quoted` **237 figures read, 123
confirmed**, unchanged both sides — this round's new figures are `compare_rasters`' and every
note says so above them, which is 751's correction applied on arrival. `--bin overtaken` 45 →
**47**: ADR 0717 adds passing-mention hits on notes already listed (its pages are the pool's
standing witnesses), the one first-rung hit it created — `CONTRADICTED_SUBPIXEL_IMAGE`, whose
member the ADR names — was cleared by citing the ADR in that note, and the one residual
`names a member` hit is a document-stem collision: the ADR is about `pdfbox/unencrypted.pdf`
**page 2**, contradicted, and `AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE` holds the same document's
*ambiguous* pages. `--bin pointers` **98 absent** and `--bin quotations` **38 diverging**, both
unchanged. Step 7 was not re-run: no pixel moved and the byte-identical per-page lines say so a
second way.

## Changed

- `crates/pdf-model/tests/oracle.rs` — the sharing-pair count in
  `rank_the_contradicted_by_the_bound`; the measurement section on `CONTRADICTED_GLYPH_EDGES`;
  the four-members section on `CONTRADICTED_SUBSTITUTED_FONT`; the bound paragraph on
  `CONTRADICTED_SUBPIXEL_IMAGE`.
- `doc/traps/oracle-and-references.md` — trap 9's tenth bullet gains the contradicted-pool
  paragraph.
- `doc/todo/12-one-bound-two-jobs.md` — the per-verdict control.
- `doc/oracle-and-corpus.md` §3b — the new printed line.
- ADR 0717.
- No ledger row: the round implements no normative requirement and reads no clause anew; the
  clauses its edited notes already cite are non-`unreviewed`.

## Owed

- **The ten differing-fraction pages outside the 32 are the colour mechanisms** (`mupdf` +
  `ghostscript` convictions); their groups price them and no population question is open there.
- **`doc/todo/12` is sharpened, not advanced**: the per-verdict control says what the bound is
  made of on this population, and requirement 2 — a fourth independent rasteriser — still has no
  candidate Arch packages.
- Unchanged from 751, 761 and 764: the 22 dropped pages of the *we are alone* ranking, its 13
  unmarked rows unread as a list, `AMBIGUOUS_ONE_LADDER`'s hold on `issue12337.pdf`,
  `border_overhang_census` having no crawl scope, and the owner's `git stash drop`.
