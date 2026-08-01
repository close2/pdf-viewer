# ADR 0093 — The group that need not be built

Status: accepted, 2026-08-01.

## Context

The largest single item on the corpus's incomplete list, after fonts, was one report on nine
documents: *"non-isolated, and an element blends with the backdrop it excludes"*. §11.4.4 states
what a non-isolated group is — its elements are composited onto the group's backdrop and that
backdrop's contribution is removed again afterwards — and this tree drew every group as
§11.4.5's isolated one, which is exact only where every element blends Normal.

Implementing the clause as written looked expensive and was going to be worse than that. The
result step is

    C = Cn + (Cn − C0) × (α0/αgn − α0)
    f = fgn
    α = αgn

and NOTE 4 says the shape and alpha have to be accumulated *separately* from the composite ones.
That is not a convenience: with an opaque backdrop, α0 = 1 makes αn = 1 for every αgn, so the
group alpha cannot be recovered from the raster afterwards. A correct implementation needs a
second alpha channel — in practice a second buffer, rendered onto transparency, purely to
accumulate αgn — plus a per-pixel unpremultiply, a subtraction and a division in eight bits, and
the same again in Vello with a readback so the two backends agree.

## Decision

**None of that, because §11.4.4's NOTE 5 says when the group need not exist at all.**

> As a result of these corrections, the effect of compositing objects as a group is the same as
> that of compositing them separately (without grouping) if the following conditions hold:
>
> The group is non-isolated and has the same knockout attribute as its parent group (see 11.4.5,
> "Isolated groups" and 11.4.6 , 'Knockout groups').
>
> When compositing the group's results with the group backdrop, the Normal blend mode is used,
> and the shape and opacity inputs are always 1.0.

Both conditions are decidable in `pdf-model` at the `Do`. Where they hold, the group's elements
are pushed into the display list **inline**, with no `Command::Group` around them — so every
blend mode inside the group composites against the page it was always going to composite against,
which is exactly what §11.4.4 asks for. The backdrop that would have to be removed is never
introduced.

The clip is not one of the conditions and does not need to be. PDF's clipping is cumulative in
the graphics state, so an element inside the form already carries the clip in force at the `Do`;
applying it once per element is applying it once.

The knockout condition needed a flag. `Interpreter::inside_knockout` says whether the group being
built is, or is inside, a knockout group — because a child flattened into a knockout parent would
stop being *one* element of that parent and become several, which is precisely what §11.4.6 makes
different.

## What it cost and what it bought

- **The corpus's incomplete count falls 89 → 86.** `bug1873345.pdf`, `issue13242.pdf` and
  `nonisolated_blend_smask.pdf` leave the list — the third of those is a file named for this
  feature.
- **Two of the three enter the oracle's *agreeing* set** and the third is ambiguous: 837 → 839
  agreeing, **65 contradicted, unchanged**. Nothing regressed, and the one page worth looking at
  — `issue13242.pdf`, a yellow highlight multiplied over text — has all four panels of its
  side-by-side alike.
- **Text readback rises 97.8% → 97.9%** over a denominator that grew by the three documents'
  words.
- **It is less work than the code it replaces**, measured: `issue13242.pdf` page 1 rasterises in
  **3 968.9 M instructions against 4 070.2 M**, a 2.5% fall, because a page-sized group buffer
  and its composite are gone. Interpretation of the specification's own page is unchanged —
  2 110.60 M against 2 110.61 M in the same sitting, which is a page with no such group.

A correctness fix that is also faster means the old code was doing work that was worse than
useless. This project has met that shape twice before (one mesh raster for 4096 triangles, ADR
0051; a ramp that drops collinear stops, ADR 0068) and it is worth naming a third time: the
expensive design was reached by reading the *formulas* and not the notes around them.

## What is still reported, and why it is the honest remainder

Six corpus documents keep the report: a non-isolated group whose own `Do` states an alpha, a
blend mode or a soft mask, *and* which holds a blending element. NOTE 5's second condition fails
for those, so the backdrop genuinely has to be composited in and removed again, and NOTE 4's
second alpha channel is what that needs. The report's condition is unchanged; what changed is
that the population it names is the one that actually needs the arithmetic.

## Consequences

Two tests changed meaning rather than breaking, and both were rewritten to say the new thing:
`a_form_becomes_a_group_only_for_the_transparency_subtype` now states `/I true` for its positive
case and asserts the flattening as a fourth reason for zero, and
`a_non_isolated_group_reports_only_when_an_element_blends` sets a `gs` at the `Do` so that its
group is one NOTE 5 cannot flatten. A third test is new and is the one that matters:
`a_non_isolated_group_blends_with_the_page_behind_it` renders the same content three ways —
inside a non-isolated group, ungrouped, and inside an isolated group — and asserts that the first
two are pixel-identical and the third is not. That is NOTE 5 stated as an experiment, and it is
what would fail if the flattening condition were ever loosened.
