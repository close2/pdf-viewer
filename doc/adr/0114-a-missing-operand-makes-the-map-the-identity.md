# ADR 0114 — A missing operand makes the map the identity

Status: accepted, 2026-08-01.

## Context

ADR 0113 gave an appearance stream with no `/BBox` a default box, on the ground that
§12.5.5's algorithm needs the entry only for a *scale* and refusing discards the whole
annotation. The corpus's other annotation-placement report is the same algorithm's other
operand: `issue14438.pdf`, "Ink: no usable /Rect".

## The argument

§12.5.5 maps the appearance's transformed bounding box onto `/Rect`. The two are the same kind
of thing — a box in a coordinate space — and the map between them is determined by both. **A
missing operand therefore makes the map the identity, whichever operand it is.** ADR 0113 read
that one way; this reads it the other. One rule, stated once, applied twice, and the symmetry is
the argument rather than a coincidence to be noticed afterwards.

So an annotation with no `/Rect` but a stored appearance is placed where the appearance's own
`/BBox`, mapped through its `/Matrix`, puts it.

**Where there is no stored appearance the refusal stands**, and the boundary is worth stating:
every construction in `crate::appearance` is *written into* the rectangle, so there is nothing
whose own box could stand in for it. `/Rect` is required by Table 166 and this is where that
still bites.

## What the witness actually says

`issue14438.pdf` states four ink annotations with no `/Rect` at all, each with an appearance
stream whose `/BBox` is `[0 0 0 0]`. Under the rule above the derived rectangle covers no area,
which Table 166 already excuses a writer from supplying an appearance for, and the annotation
draws nothing — **which is what the file says**, not a gap.

That is the finding worth keeping: the old report named the missing `/Rect`, and the missing
`/Rect` is the one entry that could not have changed the picture. **A report can name a true
absence and still be about the wrong thing.**

## Consequences, measured

`issue14438.pdf` keeps its `CompositedInParts` report and loses its annotation one; the corpus
count stays at 89 because the document has both. All four gates unmoved — 841 agreeing and 65
contradicted, 97.9% of `pdftotext`'s words, 1545 dates. Tests 892 → 894, one for each direction:
an appearance placed by its own box where `/Rect` is absent, and an appearance box of no area
drawing nothing *and reporting nothing*.

The annotation row is now 8 reports over 8 documents, from 10: this session and the last took
`Ink: no usable /Rect` and the empty-`/Subtype` wording, and neither cost a line of the drawing
path.
