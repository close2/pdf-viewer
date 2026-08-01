# ADR 0107 — A pattern is a group, and the alpha belongs to it

Status: accepted, 2026-08-01.

## Context

ADR 0102 left the 240 `partial` rows as the population with no gate. This session read clause
11's, and §11.6.7's note named a gap with a reason that had expired:

> `tile` replays a tiling pattern's cell with the current blend mode and alpha constants
> inherited rather than reset to their defaults and applied once to the finished pattern. That
> is the closest available approximation **while §11.4.6 does not exist**

§11.4.6's knockout groups were drawn in the seventy-first session and `Command::Group` has been
in the display list since the seventeenth. The approximation had outlived its excuse by
forty-six sessions.

## What the clause says

§11.6.7 states the construction and then states it again from the implementer's end:

> the pattern definition shall be treated as if it were implicitly enclosed in a non-isolated
> transparency group: a non-knockout group for tiling patterns, a knockout group for shading
> patterns. The definition shall not inherit the current values of the graphics state parameters
> at the time it is evaluated; those parameters shall take effect only when the resulting pattern
> is later used to paint an object.

> NOTE 2 In a raster-based implementation of tiling, it is advisable to treat all tiles as a
> single transparency group. This avoids artifacts due to multiple marking of pixels along the
> boundaries between adjacent tiles.

`tile` did the opposite of the first sentence: each cell's `GraphicsState` was given
`state.blend`, `state.fill_alpha` and `state.stroke_alpha`. Two consequences, and only the first
was written down:

- **An `0.5 ca` was applied per mark.** A cell drawing two overlapping shapes composited them
  against each other at half alpha and reached 0.75 where the clause reaches 0.5.
- **The graphics state's soft mask reached the pattern not at all.** It was never copied onto
  the cell and never applied to the result, so a masked pattern fill was unmasked. Nobody had
  noticed, because nothing named it.

## Decision

Every cell runs with the transparency parameters at their defaults — which is what
`GraphicsState::initial` already gives it, so the fix is a deletion — and the state's blend mode,
alpha constant and soft mask are applied once, by a single `Command::Group` over all the tiles.

**Where all three are at their defaults, no group is built.** §11.4.4's NOTE 5 says so — "the
effect of compositing objects as a group is the same as that of compositing them separately
(without grouping)" — and the CPU backend gives every group a page-sized pixmap, so building one
for a pattern that composites trivially would put a buffer behind every patterned page for
nothing.

**The group is isolated where the clause says non-isolated**, and §11.6.7's own NOTE 1 is what
makes that exact: "in the common case in which the pattern consists entirely of objects painted
with the Normal blend mode … the results depend only on the colour, shape, and opacity of the
pattern cell and not on those of the backdrop". A cell that sets a blend mode of its own is the
case it is not, and that is reported with §11.4.4's existing wording.

**A shading pattern's implicit *knockout* group is not built**, and the reason is worth the
sentence: `sh` composites its result once already, so knockout and non-knockout differ only where
the group paints a point twice — which a function of position cannot do.

## What it cost, measured

**No corpus page witnesses the change.** All 122 tiling-pattern paints across the 974 documents
are under a default alpha, blend mode and soft mask, counted by instrumenting the branch — so
every one of them takes the no-group path and both gates are unmoved at 840 agreeing, 65
contradicted and 90 incomplete. This is trap 8 again: the rule is required of any valid PDF and
reachable by no file anybody has.

Two tests carry it instead. The discriminating one draws a cell with two overlapping shapes under
`0.5 ca`, because a cell with *one* shape gives the same answer under either model — and it
asserts the **alpha** rather than the colour, since 0.5 against 0.75 is the whole difference. The
second asserts that a trivially-compositing pattern produces no `Command::Group` at all, which is
the half a future session would otherwise regress by making the group unconditional.

## Consequences

Three of clause 11's `partial` notes named a gap "while §11.4.6 does not exist". This closes the
one that could be closed; the other two are the non-isolated group whose backdrop genuinely has
to be removed, and the group whose blending space is not the device's.

The lesson is the reason to read the `partial` population at all: **a note that gives a reason
gives a trigger, and nothing fires it.** "While X does not exist" is a claim that expires the day
X lands, and no gate in this project can see that day arrive.
