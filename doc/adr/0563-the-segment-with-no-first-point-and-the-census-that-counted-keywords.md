# ADR 0563 — The segment with no first point, and the census that counted keywords

Status: accepted.

ISO 32000-2 §8.5.2.1's "an error shall be generated" is now generated, and the geometry that error
covers is no longer invented by whichever library the display list reaches. **And the population
the ledger row carried for it was measured with the wrong instrument**: the token census that found
the requirement reachable counts *keywords*, the interpreter additionally requires the operator's
operands to be numbers, and on the one curated witness the two conditions do not meet. Over the
pdf.js corpus and the curated corpora the defect's true population is **zero pages**; over the
`CC-MAIN-2021-31` crawl it is **three**, and one of them draws a yellow wedge out of the corner of
the page that is in no content stream anywhere.

## The clause, read

§8.5.2.1, third paragraph, verbatim:

> The trailing endpoint of the segment most recently added to the current path is referred to as
> the current point. If the current path is empty, the current point shall be undefined. Most
> operators that add a segment to the current path start at the current point; if the current
> point is undefined, an error shall be generated.

Three things it settles and one it does not.

**It settles which operators.** "Most" excludes exactly two, and the paragraph above says which:
"the first one invoked shall be m or re to begin a new subpath". So the sentence governs `l`, `c`,
`v` and `y`.

**It settles when.** "If the current path is empty, the current point shall be undefined" — so the
condition is `path.is_empty()`, and nothing subtler. A path holding only an `m` has a current
point, because Table 58's `m` sets it in its own words; a path whose last command is `h` has one
too, which is what `a_segment_after_a_close_starts_where_the_close_returned_to` has pinned since
the twenty-fourth session.

**It settles that an error is raised**, and that is a `shall` on the processor rather than on the
file.

**It does not settle what is drawn.** No recovery is stated anywhere in the clause, so the drawn
result is a documented choice under `CLAUDE.md` principle 5 — but only one of the three candidates
survives contact with the clause's own words:

| candidate | what the clause says about it |
|---|---|
| begin a subpath at the operator's own coordinates (`l x y` as `m x y`) | contradicted outright — only `m` and `re` begin a subpath — and undefined for `c`, whose first two operands are control points rather than a start |
| treat the stream as damaged from that point | the clause states an error and no consequence for the rest of the stream; §7.8.2's neighbouring error, an unrecognised operator, ends no stream here either, and this would throw away every mark the file states afterwards |
| **add nothing** | the segment has no first endpoint and the clause supplies none, so it describes no geometry |

The third is taken, and one consequence of it is derived rather than chosen: since the current
point is "[t]he trailing endpoint of the segment most recently added", an operator that added none
leaves it undefined, so the *next* segment is refused for the same reason. A run of segments after
the error vanishes whole rather than hanging off a point the file never stated, until an `m` or an
`re`.

## One sentence, two costs — and the `h` half stays where 696 put it

Table 58 states `h` as "Close the current subpath by appending a straight line segment from the
current point to the starting point of the subpath". With the path empty there is no *starting
point of the subpath* either, so `h` adds no segment on that invocation and falls outside the
sentence's antecedent rather than inside its consequence. `content::path::close_subpath` already
pushes nothing onto an empty path, so the standard's error and this tree's silence draw the same
page — which the six-hundred-and-ninety-sixth session measured (ADR 0548) and which
`a_close_with_no_current_point_neither_draws_nor_reports` now pins from both sides.

**It is not reported, and that is trap 11 rather than caution.** `Unsupported` is this program's
statement of what a page is *missing*, and `Interpretation::is_complete` reading false is what
takes a page out of the oracle's judgement altogether. A page whose only anomaly is an `h` with
nothing to close has lost no mark, so a report there would be a false statement about the page and
would cost a gated page for it. The asymmetry the ledger row records is therefore carried all the
way through: one sentence, two costs, two answers.

## Why the refusal is in `pdf-model` and not in a rasteriser (trap 2)

Before this change the segment was appended and the path handed to a backend had no first point,
at which point the *library* decided what to draw:

- `tiny_skia::PathBuilder::line_to` calls `inject_move_to_if_needed`, which begins the subpath at
  **(0, 0)** — the origin of user space — so the page gets an edge running from the corner that no
  operator asked for.
- `kurbo::BezPath::line_to` fires `debug_assert!(!self.0.is_empty(), "uninitialized subpath
  (missing MoveTo)")`, so the same document is a **panic in a debug build** of the graphics backend
  and something else again in release.

Three libraries, no agreement, and nothing in the tree naming the choice: trap 2's first bullet
word for word, and the sixth instance of it after ADR 0535's hairline boundary. The decision is
made once, where the path is built from the content stream — `content::path::extend_subpath`, the
one place `l`, `c`, `v` and `y` reach a path — so every backend receives a path that begins with a
move, and `Unsupported::UndefinedCurrentPoint` names the segments that did not.

## The census counted keywords, and the interpreter counts operands

ADR 0548 built `examples/operator_shape_census` to settle whether §8.5.2.1's error is reachable at
all, and it is the right instrument for that: it lexes a page's `/Contents` and every form
`XObject` its resources reach and counts an `l`, `c`, `v` or `y` **keyword** with no `m` or `re`
before it. It found one curated first page, five crawled ones, and — over the curated corpora's
first hundred pages apiece — twelve documents and 5010 operators.

**Those are counts of keywords, and the interpreter asks a second question the lexer does not.** An
operator only runs when its operands parse as numbers: `b"c" => if let Some(points) =
points_from::<3>(operands)`. `issue6342.pdf` is the whole of the difference. Its form `XObject` —
titled, by the file, "Form XObject with errors" — writes byte soup after its third `f`, and the
lexer splits that soup into keywords like `c858.7.0`, `c030.177.0` and `c90674`, each of which
clears the pending operands. The bare `c` operators that survive have too few numbers in front of
them, and **not one of them ever reaches a path**. Its display list holds 36 painted paths and
every one of them begins with a `MoveTo`.

`examples/refused_segment_census` is the instrument that asks the interpreter instead. It counts
`Unsupported::UndefinedCurrentPoint` over one first page per document, beside the number of paths
that page paints so that a zero is legible as "no page does this" rather than as "nothing was
interpreted".

| scope | pages reached | pages refusing a segment | segments | painted paths |
|---|---|---|---|---|
| pdf.js, 974 files | 958 | **0** | **0** | 350 338 |
| curated, 1251 files | 1230 | **0** | **0** | 573 389 |
| `CC-MAIN-2021-31`, 65 944 files | 65 659 | **3** | **660** | 114 656 429 |

**So the fix changes no corpus pixel and the requirement is exercised only by the crawl**, which is
where the picture was finally looked at (trap 1). Both arms were built and run in one sitting, the
"before" arm produced by removing the early `return` at `extend_subpath`'s single site in a scratch
copy rather than by reverting the tree.

| page | segments refused | pixels moved | what moved |
|---|---|---|---|
| `1284945.pdf` | 8 | **1.09%** of 893 × 1263 | a **yellow wedge running out of the page's bottom-left corner** and a yellow bar down its left edge — geometry anchored at the origin of user space, beside a logo the file does state, which is untouched |
| `4605705.pdf` | 99 | 0.18% of 551 × 813 | an edge at the left margin of a brochure cover |
| `0300856.pdf` | 553 | **0.00%** | nothing: the page is covered black either way, which is why a count of refused segments is not a count of marks |

The first of those is the defect the round was sent to find, and it is a picture rather than an
argument: the wedge is not in the file, it is `tiny-skia`'s injected `(0, 0)` joined to wherever
the first surviving segment went.

## What was planted, and what did not fail

Trap 13, in both of its halves, and they came out differently:

- **The unit gate failed before the fix.** `a_segment_with_no_current_point_states_no_geometry`,
  `a_refused_segment_defines_no_current_point_for_the_next_one` and
  `a_segment_with_no_current_point_is_reported` were written and run against the unfixed tree: the
  first printed `[[MoveTo, LineTo], [LineTo(30, 30)]]` where the clause admits one path, and the
  third printed `[]` where the error belongs. `a_close_with_no_current_point_neither_draws_nor_reports`
  **passed** before the fix, which is 696's asymmetry confirmed rather than assumed.
- **The corpus sweep did not fail before the fix, and that is the finding rather than a pass.**
  `paths_beginning_with_a_segment` in `pdf-model/tests/corpus.rs` walks every mark, every group's
  elements and every clip chain of all 974 first pages and named nothing, on the tree that still
  had the defect. A sweep that is only ever run over a clean population has measured nothing
  (trap 13), so it is calibrated by
  `the_open_subpath_sweep_names_a_path_that_begins_with_a_segment`, which builds a list by hand
  with the shape in all three of the places a path reaches a backend from and demands the count be
  three.

## What the new report found in this tree's own fixtures

Two gates failed on the first run after the change, and both were right to.

**`hostile_budgets::a_stream_of_many_tokens_and_few_operators_still_draws`** is ADR 0306's control:
it proves the four-million-operator bound counts *operators* rather than lexer tokens, by stating a
stream that is token-heavy and operator-light. It did that with `0 0 0 0 0 0 c\nn\n` repeated
550 000 times — **and no `m` in front of any of them**, which is exactly the error this ADR is
about. So the fixture had been violating §8.5.2.1 since it was written, and nothing could say so
because nothing refused it. It is a conforming stream now (`0 0 m 0 0 0 0 0 0 c\nn\n`, 6.05 million
tokens and 1.65 million operators), and the lesson is small and general: **a fixture that violates a
clause other than the one it is about tests two things at once, and one of them silently.**

**`fixed_documents`** named `4605705.pdf` page 1, whose row lists the reports the round that fixed
it recorded. 99 of that page's 2194 noise operators are this clause's, so the row lists a fourth
name now. Its ink is 146.026 and the band 145.204 .. 147.204 holds.

## Consequences

- `doc/conformance/ledger.toml` §8.5.2.1 goes `partial` → `implemented`, with the two refused
  candidates, the interpreter-level population and the `h` asymmetry written into the row.
- A thirteenth report, `Unsupported::UndefinedCurrentPoint`, whose gated-page cost is **zero** on
  today's corpus because no page fires it.
- The corpus gate carries a new assertion that no path reaching a backend begins with a segment.
  It is a zero that is now enforced rather than a zero nobody had asked for.
- `examples/refused_segment_census` is the instrument for the row's population, and the token
  census's figures are recorded as the upper bound they are.
- Trap 2 gains a seventh instance, trap 5 a thirteenth report, and trap 13 a second shape: a census
  derived from the *clause* is not a census of the *defect*, because the code has conditions the
  clause does not.
