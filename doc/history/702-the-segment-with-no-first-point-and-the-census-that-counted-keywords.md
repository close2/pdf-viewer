# 702 — The segment with no first point, and the census that counted keywords

ISO 32000-2 §8.5.2.1's error is generated now, and the geometry it covers is no longer whichever
first point a library invents. **And the population the row carried for it was measured with the
wrong instrument**: the token census that found the requirement reachable counts *keywords*, the
interpreter also requires the operator's operands to be numbers, and on the one curated witness the
two conditions never meet. The defect's true population over the curated corpora is **zero pages**
and over the `CC-MAIN-2021-31` crawl it is **three** — one of which draws a yellow wedge out of the
corner of the page that is in no content stream anywhere.

Date: 2026-08-24.
ADR: [0563](../adr/0563-the-segment-with-no-first-point-and-the-census-that-counted-keywords.md).

Touched: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/path.rs`,
`crates/pdf-model/src/content/report.rs`, `crates/pdf-model/src/content/run.rs`,
`crates/pdf-model/tests/path_construction.rs`, `crates/pdf-model/tests/corpus.rs`,
`crates/pdf-model/examples/refused_segment_census.rs` (new), `crates/viewer-core/src/report.rs`,
`doc/conformance/ledger.toml` (§8.5.2.1), `doc/todo/01-ledger-partial-rows.md`,
`doc/traps/pixels-and-rasterisers.md` (trap 2), `doc/traps/parsers-and-streams.md` (trap 5),
`doc/traps/instruments-and-reports.md` (trap 13), `crates/pdf-model/tests/hostile_budgets.rs`,
`doc/checks/fixed-documents.toml`, the ADR and this file.

## What the clause says

> The trailing endpoint of the segment most recently added to the current path is referred to as
> the current point. If the current path is empty, the current point shall be undefined. Most
> operators that add a segment to the current path start at the current point; if the current
> point is undefined, an error shall be generated.

It settles which operators — "Most" excludes the two the paragraph above names, "the first one
invoked shall be m or re to begin a new subpath" — and it settles when: the current path is empty.
It settles that an error is raised. **It does not settle what is drawn**, and of the three
candidates the briefing named, two are refused by the clause's own words: beginning a subpath at
the operator's coordinates contradicts the sentence that only `m` and `re` do that (and is not even
defined for `c`, whose first two operands are control points), and treating the stream as damaged
is a consequence the clause never states and would throw away every mark after the error. The third
— add nothing — is taken, and one part of it is *derived* rather than chosen: since the current
point is "the trailing endpoint of the segment most recently added", an operator that added none
leaves it undefined, so the run of segments after the error vanishes whole until an `m` or an `re`.

696's asymmetry was read before anything was built and it holds. Table 58 states `h` as "appending
a straight line segment from the current point to the starting point of the subpath", and with the
path empty there is no starting point either — so `h` adds no segment on that invocation and falls
outside the sentence's antecedent rather than inside its consequence. It is not reported, and that
is trap 11 rather than caution: `Unsupported` says what a page is *missing*, `is_complete` reading
false takes a page out of the oracle's judgement, and an `h` with nothing to close has lost nothing.

## What was built

`content::path::extend_subpath` is the one place `l`, `c`, `v` and `y` reach a path. With the path
empty it appends nothing, counts the operator, and leaves the current point where the clause leaves
it — undefined. The error is `Unsupported::UndefinedCurrentPoint`, this program's thirteenth report
raised while drawing, worded for a person in `viewer-core`.

**It is in `pdf-model` because trap 2 says so, and this instance teaches an edge the other six do
not.** Where a clause calls the input an *error* there is no right answer for a library to agree
with, so no reading of the clause can contradict what a library does: `tiny_skia::PathBuilder`
begins the subpath at the origin of user space and draws an edge from the corner of the page, and
`kurbo::BezPath` fires a `debug_assert!` on a subpath with no move in front of it. **No picture
comparison can see the second of those at all** — it is a crash in one configuration rather than a
pixel in any.

## Planted first, and one half did not fail

- **The unit gate failed on the unfixed tree.** Three new tests in `path_construction.rs` were
  written and run before any source moved: the geometry one printed `[[MoveTo, LineTo], [LineTo(30,
  30)]]` where the clause admits one path, the run one kept two segments the file never anchored,
  and the report one printed `[]`. The fourth, `a_close_with_no_current_point_neither_draws_nor_reports`,
  **passed before the fix** — which is 696's asymmetry confirmed rather than assumed.
- **The corpus sweep did not fail on the unfixed tree, and that is the finding.**
  `paths_beginning_with_a_segment` walks every mark, every group's elements and every clip chain of
  974 first pages and named nothing while the defect was still in the tree. A sweep only ever run
  over a clean population has measured nothing (trap 13), so it is calibrated by
  `the_open_subpath_sweep_names_a_path_that_begins_with_a_segment`, which builds the shape by hand
  in all three places a path reaches a backend from and demands the count be three.

## The census counted keywords

`examples/operator_shape_census` is a lexer, and the right instrument for the question ADR 0548
asked it: is the standard's shape reachable at all. It is the wrong instrument for what the shape
*costs*, because the interpreter asks one thing more — an operator only runs when its operands
parse as numbers.

`issue6342.pdf` is the whole of the difference, and it was opened rather than argued about. The
form `XObject` the file titles "Form XObject with errors" writes byte soup after its third `f`; the
lexer splits that soup into keywords of its own — `c858.7.0`, `c030.177.0`, `c90674`, `c95.c.455`
— each of which clears the pending operands, and the bare `c` operators that survive have too few
numbers in front of them. Its display list holds 36 painted paths and **every one of them begins
with a `MoveTo`**. The page reports thirteen `Unsupported::Operator` and no origin line.

`examples/refused_segment_census` asks the interpreter instead, counting the new report over one
first page per document beside the paths that page paints, so a zero is legible as "no page does
this" rather than as "nothing was interpreted".

| scope | pages reached | pages refusing a segment | segments | painted paths |
|---|---|---|---|---|
| pdf.js, 974 files | 958 | **0** | **0** | 350 338 |
| curated, 1251 files | 1230 | **0** | **0** | 573 389 |
| `CC-MAIN-2021-31`, 65 944 files | 65 659 | **3** | **660** | 114 656 429 |

So the fix moves no corpus pixel, and the requirement is exercised by the crawl alone. The 5010
segments and 133 segments in §8.5.2.1's row are counts of *keywords* and are recorded as the upper
bound they are.

## And then the picture (trap 1)

Both arms were built and run in one sitting, the "before" arm produced by removing the early
`return` at `extend_subpath`'s single site in a scratch copy of `path.rs` rather than by reverting
the tree — 690's method, and the one that does not destroy the round's other edits.

| page | segments refused | pixels moved | what moved |
|---|---|---|---|
| `1284945.pdf` | 8 | **1.09%** of 893 × 1263 | a **yellow wedge running out of the page's bottom-left corner** and a yellow bar down the left edge, beside a logo the file does state and which does not move |
| `4605705.pdf` | 99 | 0.18% of 551 × 813 | an edge at the left margin of a brochure cover |
| `0300856.pdf` | 553 | **0.00%** | nothing — the page is covered black either way, which is why a count of refused segments is not a count of marks |

The wedge is in no content stream anywhere: it is `tiny-skia`'s injected `(0, 0)` joined to
wherever the first surviving segment went, and it is what the round was sent to find.

## Gates

`doc/todo/02` §2 whole, `PDFREF_CACHE` pointing at the shared warm cache
(`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`). **Trap 16: everything below was built and run
over the *whole workspace*** — `cargo clippy --workspace --all-targets` and `cargo nextest run
--workspace` — so the feature unification is the full-graph one. Every line exit 0 on the last run.

Two gates failed on the first run and both were this round's doing, which is what they are for:

- **`hostile_budgets::a_stream_of_many_tokens_and_few_operators_still_draws`** — the fixture that
  proves ADR 0306's operator bound is not a token bound wrote `0 0 0 0 0 0 c n` half a million
  times, with **no `m` in front of any of them**. That is §8.5.2.1's error, so the new report fired
  550 000 times on a fixture whose subject is a budget. The fixture is a conforming stream now
  (`0 0 m 0 0 0 0 0 0 c n`, **6.05 million tokens and 1.65 million operators** where it was 4.4 and
  1.1), and the doc comment says why: a fixture that violates a *different* clause tests two things
  at once.
- **`fixed_documents::every_document_a_round_fixed_is_still_fixed`** — `4605705.pdf` page 1 raised
  `UndefinedCurrentPoint { segments: 99 }`, which its row did not list. The row lists it now, with
  what it is; its ink is **146.026** and the band `145.204 .. 147.204` holds, because what those 99
  segments drew was an edge at the left margin.

- `cargo fmt --all --check`, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`,
  `cargo nextest run --workspace` (**2510 passed, 18 skipped**, 37 s), `cargo test --workspace --doc`,
  `cargo check --manifest-path fuzz/Cargo.toml --bins`.
- Both trap-10 builds.
- **corpus** — 974 documents in 5.2 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
  **67 incomplete**, 0 slow. The new report costs this gate nothing: no first page of the 974 fires
  it. The new `open_subpaths` assertion names no document.
- **oracle** — 1945 pages in 56.5 s (1838 complete): **983 agrees, 65 contradicted, 832 ambiguous,
  3 our geometry, 2 reference geometry, 42 not comparable, 18 no render**. Every one of those is the
  figure the briefing quoted, so nothing moved — and 696's failing page does not fail here.
- **text extraction** — 99.2% (22834/23013 words) over 974 documents, 22 below 90%; 99.8%
  (14257/14281) against PDFBox over 40. **selection census** 98.91% over 453 documents,
  **accessibility census** 104 with structure of 988, **dates**, **xmp**, **jpeg2000**,
  **fixed documents** (40 checked, 0 absent), **quorra corpus** (957 pages, 932 agree, 23 differ,
  2 refused, 17 not comparable) — all exit 0.
- `cargo test -p conformance` — **875 rows: 444 implemented, 223 partial, 18 reported, 69
  inapplicable, 8 writer-side, 113 out-of-scope, 0 unreviewed**; 1051 quotations all verbatim; the
  notes name 2687 clauses and 275 tables, all of which exist. One status moved and it is §8.5.2.1's.

§5's binaries were deliberately not installed: this is a parallel round told not to merge, and no
launch or frame figure was taken.

## Sweeps

Fourteen conformance sweeps plus `spec-errata check`, `applied` and `emit`, run after the edit.
**No hit in any of them names this round's material** — greps for `0563`, `refused_segment`,
`extend_subpath`, `UndefinedCurrentPoint`, `8.5.2.1` and the three crawled page names come back
empty across the whole sweep log — which is the check a delta is a proxy for.

- `ledger` implemented 443 → **444**, partial 224 → **223**, which is §8.5.2.1's row and nothing
  else.
- `owed` **223** `partial` rows over 3684 terms, with **175 unnamed over 111 rows** unchanged.
- `pointers` 7668 with **absent unmoved at 130** and 13 undefined symbol pointers; every path this
  round wrote exists.
- `quotations` 5723 over 866 documents with **diverging unmoved at 34**, and in the ledger 1876 over
  794 notes with **diverging unmoved at 2**.
- `tables` 6150 sentences and 2297 attributed key citations, with **absent unmoved at 101** and the
  denial count unmoved at 6 — so no wrong table number in the prose this round wrote, which is what
  that sweep is for.
- `counts` 7252 sentences, 391 attributed. `overtaken` 520 decision records, 43 overtaken, none of
  them named by ADR 0563. `unread`, `capabilities`, `callers`, `inapplicable`, `entries`,
  `overstated`, `blockers` and `quoted` print nothing of this round's.
- `spec-errata applied`'s **90 / 10 / 171 split is unchanged** — no new place quotes struck text —
  and `check` names nothing under clause 8.5.
- **`spec-errata emit` was run on §8.5.2.1 and there is no erratum on it.** Errata Collection 3
  touches §8.5.3.1 (Issue #549, *generate an error* → *be ignored*) and §8.5.3.2 (#103, #434) and
  leaves this clause's sentence exactly as printed, which is what makes the two deliberately
  different rather than an oversight a reader may borrow across.

## Measurement conditions

The machine ran **three other rounds** throughout. Load average over 24 cores was **15 at the start
of the gate sequence, 44 during it and 5.6 at the end**, so **no timing figure was taken** and none
is quoted above as one: every number is a count, a verdict or a raster value, and a loaded machine
moves none of those. The oracle is the one gate whose *verdicts* a loaded machine can move — it
spawns three reference renderers under time budgets — and it read 99.8% from the cache with 15
renders produced, so 6692 of its 6707 comparisons never ran another program at all.

`PDFREF_CACHE` pointed at the **shared** warm cache rather than at a copy, which is what the
briefing asked for and is 690's recorded caveat rather than this round's choice.
