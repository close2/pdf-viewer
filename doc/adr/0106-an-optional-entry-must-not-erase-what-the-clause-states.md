# ADR 0106 — An optional entry must not erase what the clause states

Status: accepted, 2026-08-01.

## Context

ADR 0075 corrected a refusal on §12.5.6.7's `/LL`:

> It was refused on the true observation that `/L` is not the line — which is a reason to
> compute the line rather than a reason to decline.

Two entries of the same clause kept the shape that correction removed. A line annotation whose
`/LE` named an ending, or whose `/Cap` asked for a caption, was declined **whole**: no line at
all. So was a polyline whose `/LE` named one.

## The argument

Table 178 makes `/L` **required** and `/LE` optional with a default of `[/None /None]`; Table
181 makes `/Vertices` required and `/LE` optional the same way. And §12.5.6.7's first sentence
says what the annotation is:

> The purpose of a line annotation ( PDF 1.3 ) is to display a single straight line on the page.

An annotation that states `/L` has stated a line. `/LE` decorates its ends and `/Cap` writes
text along it; **neither changes the line, and both are optional where the line is required.**
Declining the whole annotation because an optional decoration cannot be sized draws nothing
where the clause states something — which is a larger departure than drawing the line and saying
what is missing.

That is trap 5's fifth deliberate instance of a report accompanying drawing rather than
replacing it, and it earns its place by the same test as the other four: **suppressing either
statement loses information.** The line is drawn *and* the ends are named.

## What was not changed, and why the difference matters

A cloudy `/BE` border (Table 169) stays a whole refusal on a square, circle or polygon. The
distinction is not how confident anybody is about the shape — it is that **an ending is an extra
mark and a cloudy border is a different border**. Table 169 says the border "should be drawn as
a series of convex curved line segments in a manner that simulates the appearance of a cloud";
drawing a straight rectangle in its place puts a shape on the page the file did not describe,
which is the guess principle 5 forbids. Adding nothing where nothing can be derived is not the
same operation as substituting something that can.

## Consequences, measured

**The corpus's one witness cannot show the change.** `issue13447.pdf` is the only document with
an `/LE` that reaches this path, and its arrow-ended line states `/L [176.63 45 177.94 154.5]`
inside a `/Rect [598.31 146.63 537 316.13]` — the line is nowhere near its own rectangle, so
§12.5.5's placement clips away exactly what the change now draws. The display list gains two
commands and the raster is byte-identical, checked by rendering with the change stashed and
restored. That is the file disagreeing with itself, not the reader; the change stands on the
clause.

Both gates are unmoved: 840 agreeing, 65 contradicted, 90 incomplete, 97.9% of `pdftotext`'s
words. The document keeps its report, because a report accompanying a drawing is still a report.

Two constants where there was one refusal, plus a third for both at once — trap 11's other edge
is that a report can hide another report, and a line stating `/LE` *and* `/Cap` now names both.
One test, which draws the same line with and without an ending and checks the report by name.

> **Amended in the five-hundred-and-seventy-fourth**: `/Cap` is no longer one of the two, because
> the caption is drawn (ADR 0409). What survives is the rule this decision is about, and it is
> what let the caption be taken at all — a line whose caption cannot be placed still draws its
> line, and a break in the line that would leave no line is not taken.

## The rule worth carrying

**An optional entry that states no shape must not erase the shape the clause does state.** The
question to ask of any refusal is which entry it is refusing and whether that entry is *additive*
or *substitutive*: the first is a report beside a drawing, the second is a refusal. Both of the
corrections this clause has now needed — ADR 0075's and this one — were refusals of the first
kind wearing the second kind's clothes.
