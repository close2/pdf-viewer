# Session 849 — the 278 are one book, and formation is the floor by another route

2026-09-01. ADR 0776. An oracle round on `doc/todo/12` item 1, the consensus half handed on by
ADR 0774.

**Finding**: the item asked what 278 pages are made of and the gate now counts it every run. Of
the 276 the raise newly convicts today, **272 are `freeculture.pdf`** and the other four are one
page each; **none** is convicted on the differing fraction the raise is about (274 on structural
similarity, 2 on the worst tile); and on **263 of the 276** a reference agrees with *us* more
closely, on the deciding measure, than the convicting set agrees with itself. Then the finding
that closed the item: raising formation **alone**, with our own floor untouched, acquits 27 of the
60 contradicted pages — including every one of the six ADR 0771 refused the *floor* raise for —
because `widened_to` derives our bound from the spread of whatever set formed. The two knobs
`doc/todo/12` is named after are one knob, and the item is answered from both ends.

## Files

- `tools/pdfref/src/lib.rs` — `decide` takes a formation bound and a floor (one value on every
  path that renders); `Triangulation::rejudged` is the counterfactual; two unit tests, the
  calibration and its non-vacuity; `Tolerance::TEXT_HEAVY`'s comment carries the composition.
- `crates/pdf-model/tests/oracle.rs` — `RaisedFormation`, `Standing`, `distance_on`,
  `a_raised_formation_bound`, `what_the_new_convictions_are_made_of`,
  `the_pages_a_raised_formation_bound_would_move`, and `the_censuses_beside_the_verdicts` which
  gathers the five printed censuses out of `report`.
- `doc/adr/0776-the-278-are-one-book.md` — new.
- `doc/todo/12-one-bound-two-jobs.md` — item 1 answered; the file is kept, not deleted, because
  twenty-one comments point at it.
- `doc/todo/00-ambiguous-bucket.md` — the book's section gains the counterfactual, which is its
  own diagnosis arriving from the other side.
- `doc/traps/oracle-and-references.md` — trap 12 gains the widening route.
- `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md`, `doc/todo/README.md`.

## What the round did

The instrument first. `decide` had one `Tolerance` doing the two jobs the item is named after, so
it now takes both and `triangulate_with` passes the same value twice — the item's sentence stated
in the code — and `Triangulation::rejudged` re-runs **`decide` itself** over the comparisons a page
already holds. That is the whole reason the census costs nothing and the whole reason it can be
believed: a counterfactual computed by a second implementation of the consensus rule would be a
measurement of that implementation. It is calibrated by an assertion over all 1876 judged pages,
every run — re-judging at a page's own bounds must reproduce that page's own verdict — plus two
`pdfref` tests, one of which exists only to keep the other from being vacuous.

The raise is ADR 0243's rule and today's numbers: `the_fixed_bounds_against_the_references_own_spread`
re-derives the 99th percentile of the reference-against-reference differing fraction at 11.21% and
12.04% on text and 1.36% and 1.11% on vector, so 0.12 and 0.0136. Both classes are raised rather
than only the one ADR 0243 argued about, because *is this the text class's question or the
measure's?* is a question the census should answer — and it answers it: 275 of 276 are text pages.

ADR 0243's own arm reproduces four hundred sessions later, 493 pages leaving `ambiguous` against
its 457 and 276 contradicted against its 278.

Then the sampling, which is trap 1 and is where the argument came from. Five pages looked at and
all six pairings measured on each with `examples/compare_rasters` over the gate's own panels. On
`freeculture.pdf` page 100 ours, `poppler` and `mupdf` are one picture at 4× and `ghostscript` is
visibly lighter — and the numbers say the same thing: **the tightest agreement on the page is ours
with `mupdf`** (mean 3.13, ssim 0.9558) while the pair that would convict us sits at 0.9315 and
11.19% of channels apart. The four pages outside the book each turned out to be a mechanism
already on record: a shared *gap* on `bug766086.pdf` (neither convicting renderer draws the link
border, ADR 0663 priced the removal), the `libfreetype`-sharing pair on `issue16224.pdf`, and two
pages where a reference is nearer to us than the pair is to itself — one of them
`transparency_group.pdf`, the single vector page, whose pair is admitted at 1.0076% while we sit
0.387% from `poppler`.

The 263-of-276 row was added to the census after the sampling rather than before it, which is the
right order: the sample suggested the shape and the gate then measured it over the population.

**Second track**: the twenty-second sweep (`--bin parts`) over the ledger, three rows of the
clause 8.5.3.3 family. §8.5.3.3.2 and §8.5.3.3.3 said "both rasterisers implement natively" over a
tree with three, and `render_quorra::scene::fill_rule` maps both rules, so the rows say three and
name it. §8.5.3.3.1 — the `partial` row — evidenced its one debt with "neither backend paints
anything there", and the correction is not a larger number: `pdf_render::collapsed`'s
`Extents::collapse` returns `None` for a point and says in its own comment that §8.5.3.3.1 governs
it, so the departure is one decision in the crate every backend reads, pinned by
`a_single_point_subpath_is_not_this_rule`, which the row now cites. A row whose evidence is "both
backends do X" is a row that will be wrong twice.

## Gates

`doc/todo/02` §2 whole, on the change→gate map's first row (`pdf-model`, `pdfref` is the oracle's
harness): `fmt` over both workspaces, `clippy --workspace --all-targets` and `clippy` over `fuzz/`
under `RUSTFLAGS="-D warnings"`, `nextest --workspace`, `--doc`, the sandbox worker built first
(trap 10), then the corpus, oracle, text-extraction, selection and accessibility censuses, dates,
xmp, jpeg2000, quorra's corpus, `fixed_documents` and `cargo test -p conformance`. The oracle
reports the same **980 agrees / 60 contradicted / 836 ambiguous** as before, which is the control:
nothing but `println!`s was added to the judging path.

`--bin parts`, `--bin owed` and `--bin pointers` were run over the moved documents. §5's binaries
were not rebuilt: this round measured nothing outside a §2 gate.
