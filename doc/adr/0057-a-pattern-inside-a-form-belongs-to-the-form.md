# ADR 0057 — A pattern inside a form belongs to the form

Status: accepted, 2026-07-31.

## Context

`issue6231_1.pdf` sat second in the unexplained contradicted list at a ratio of 3.17, with a
worst tile of 126.61 against a bound of 40.00 and all three references agreeing. Opening the
side-by-side answered the question before any measurement did: it is a TeX plot, three
references draw a blue-to-red surface inside its axes, and **we draw the axes and nothing
else — with `unsupported: []` beside it.** Trap 1's archetype, and trap 5's: a page that
looks plausible and is missing its subject.

The display list held the surface all along. 79 commands, one of them a `Fill` whose paint is a
type 5 lattice-form Gouraud mesh with every triangle and every colour in it. The mesh was simply
in the wrong place — about 180 points below and 140 to the left — so every triangle fell outside
the fill's clip and the rasteriser drew nothing. Nothing to report, because nothing failed.

## The clause

The file paints the surface inside a form XObject: `/Fm1 Do`, and inside the form
`/Pattern cs /pgfpatPlotsurface0 scn … re f`. §8.7.2 has two consecutive sentences about where a
pattern's matrix maps *to*, and this tree had implemented the first:

> If a pattern is used on a page, the pattern appears in the Pattern subdictionary of that
> page's resource dictionary, and the pattern matrix maps pattern space to the default (initial)
> coordinate space of the page.

> Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ), the pattern
> matrix maps pattern space to the form's default user space (that is, the form coordinate space
> at the time the form is painted with the Do operator).

The interpreter carries the parent space as `base` and set it once, from the page. It is now
saved, replaced with the form's own space while a form's content runs, and restored — where the
form's own space is the CTM at the `Do` with the form's `/Matrix` applied, because §8.10.1's
step b) concatenates that matrix before the content stream runs, so that *is* the space the
content starts in.

## What it was worth

**Three pages left the contradicted list**: `issue6231_1.pdf` and both pages of
`issue6961.pdf`. 820 agreeing and 78 contradicted, from 817 and 81.

The test is a fixture in which the two readings land in different pixels: a form whose `/Matrix`
shifts it ten units right, painted with a cell that draws a five-unit square every twenty. The
squares start at x = 10 under the clause and at x = 0 under the old code.

## The finding behind the finding, for the second session running

**The ledger's row for §8.7.2 already said this.** In as many words: "Inside a form XObject the
parent is the form, 'that is, the form coordinate space at the time the form is painted with the
Do operator', and that is what `base` holds while a form is being run." It was written from the
clause in the twentieth session, it was true of no code, and its status was `implemented`.

That is the second such row in two sessions — ADR 0056 found §8.7.3.1's "`/BBox` clips the cell"
in the same condition. Two is a pattern, and the pattern has a shape: **a row written while
*reading* a clause describes what the code should do, and nothing in the gate can tell that from
what it does.**

So the instrument gets one cheap addition rather than a resolution to be careful. Both wrong rows
named a whole test *file* as their evidence — `tests/tiling.rs` — which passes whatever it
contains. A row naming `file.rs::a_test` is a claim something would fail if it stopped being
true; a row naming `file.rs` is a claim nothing checks. The conformance gate now counts the
second kind and ratchets it:

    59 of the implemented rows name a test file rather than a test

`FILE_ONLY_EVIDENCE_CEILING` may only fall. It is deliberately a count and not a rule: the gate
cannot tell whether a named test actually covers its clause, and turning it into a rule today
would mean writing fifty-nine tests against the clock, which is the rubber stamp the ledger
exists to prevent (the same argument `REVIEW_OWED` was written with). What it does is bound the
population in which a false claim can hide, and say out loud how large that population is.

## Consequences

- 820 pages agree, 78 are contradicted, and `CONTRADICTED_UNEXPLAINED` is 32.
- §8.7.2's row names a test that fails if the claim stops being true.
- Every pattern inside every form is now positioned by the form. No corpus document reports a
  new limit and the corpus gate's wall time is unchanged.
