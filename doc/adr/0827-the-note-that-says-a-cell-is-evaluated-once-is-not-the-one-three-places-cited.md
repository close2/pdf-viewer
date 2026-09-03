# ADR 0827 — The note that says a cell is evaluated once is not the one three places cited: §8.7.3.1 has no NOTE about replication, §11.6.7's NOTE 1 has one, and it is informative and conditional

Status: accepted. Session 891.
Clauses: ISO 32000-2 §8.7.3.1 (tiling patterns, Table 74 and its two notes); §11.6.7 (patterns
and transparency, NOTE 1 and NOTE 2); §11.6.2 (a single object's portions); §8.7.2.
Code: `crates/pdf-model/src/content/pattern.rs` (`Interpreter::MAX_TILE_COPIES`'s doc comment).
Documents: `doc/conformance/ledger.toml`'s §8.7.3.1 and §11.6.7 rows, `doc/todo/49`,
`doc/adr/0810` (its correction line).

## What was claimed, and where

`doc/todo/49` carried a standing item whose whole premise was one citation:

> what they want is a cell rendered *once* and replicated by the rasteriser — §8.7.3.1's NOTE 2's
> own suggestion — which is a paint the display list does not have and three backends would have
> to draw.

ADR 0810 stated it twice, once in its own clause header ("§8.7.3.1 … and NOTE 2 on a raster-based
implementation") and once in its argument, and `pattern.rs` stated it a third time in the doc
comment above `MAX_TILE_COPIES` — "§8.7.3.1's NOTE 2 anticipates exactly that implementation".
The §8.7.3.1 ledger row closed on the same sentence.

## What the standard prints

**§8.7.3.1 has no note about replication.** The two notes printed under its heading are inside
Table 74, and they are about entries of the table:

> NOTE 1 A BBox of zero height or width will still paint one pixel (see 10.7.4, "Scan conversion
> rules").

> NOTE 2 XStep and YStep can differ from the dimensions of the pattern cell implied by the BBox
> entry. This allows tiling with irregularly shaped figures.

`pdf_render::repeat` already cites the second of those correctly, for the reason it exists — a
`/BBox` many steps across — so the tree held both readings at once.

**The note that says it is §11.6.7's NOTE 1**, and reading it whole changes what it grants:

> Unlike the opaque imaging model, in which the pattern cell of a tiling pattern can be evaluated
> once and then replicated indefinitely to fill the painted area, the effect in the general
> transparent case is as if the pattern definition were re-executed independently for each tile,
> taking into account the colour of the backdrop at each point. However, in the common case in
> which the pattern consists entirely of objects painted with the Normal blend mode, this
> behaviour can be optimised by treating the pattern cell as if it were an isolated group. Since
> in this case the results depend only on the colour, shape, and opacity of the pattern cell and
> not on those of the backdrop, the pattern cell can be evaluated once and then replicated, just
> as in opaque painting.

So the sentence the item leant on describes the *opaque* imaging model, and in the model this
program implements it is a permission with a condition attached — every object of the cell painted
with the Normal blend mode.

**§11.6.7's NOTE 2, which ADR 0810 quoted accurately, is about something else**: "[i]n a
raster-based implementation of tiling, it is advisable to treat all tiles as a single transparency
group. This avoids artifacts due to multiple marking of pixels along the boundaries between
adjacent tiles." That is advice about *compositing the tiles together*, and `Interpreter::tile` has
followed it since the hundred-and-seventeenth session. It says nothing about drawing the cell once.

## What binds, as against what a note suggests

A NOTE is informative. What §8.7.3.1 states in its own sentences is:

> When performing painting operations such as S (stroke) or f (fill), the PDF processor shall paint
> the cell on the current page as many times as necessary to fill an area.

and, on the lattice,

> The placement of pattern cells in the tiling is based on the location of one key pattern cell,
> which is then displaced by multiples of XStep and YStep to replicate the pattern.

with the order left open — "unspecified and unpredictable", which Errata Collection 3's Issue #428
extends with "(implementation dependent)". None of that says *where* the replication happens. A
processor that re-runs the content stream per site, one that copies the cell's marks, and one that
blits a tile raster all satisfy the same sentences; the standard's only constraints on the third
are §11.6.2's — "Portions of an object shall not be composited with one another, even if they are
described in a way that would seem to cause overlaps" — read through §11.6.7's rule that a
pattern's evaluation yields *the object's* shape.

**So the item was never resting on a suggestion of the standard's**, and the two witnesses named
under it were owed an engineering argument rather than a clause. ADR 0828 makes it, with the
measurement.

## Why three places kept it and a fourth caught it

`doc/todo/11` made the same misattribution and corrected itself four sessions later, in a
parenthesis: "(This file said §8.7.3.1's NOTE 2 for four sessions. The note is §11.6.7's.)" That
correction reached the file that made the mistake and none of the three that had copied it — which
is the shape `CLAUDE.md` names as a claim about the specification decaying, one file at a time. The
grep that finds the rest of it is `grep -rn "8\.7\.3\.1's NOTE" doc/ crates/ tools/`, and it is
worth running whenever a note is cited by number: a note's number is scoped to the subclause or
the table that prints it, and a citation that names the wrong one cannot be checked by the
quotation gate, which reads blockquotes rather than the sentence around them.

## What changed

Nothing executable. Four sentences: `pattern.rs`'s doc comment, the §8.7.3.1 ledger row's closing
sentence, `doc/todo/49`'s item, and a correction line at the head of ADR 0810 — which keeps its
argument, because what it measured and decided is unaffected by which note anticipated the
implementation it did not build. §11.6.7's ledger row gains NOTE 1's other half, which nothing in
this tree had read: the row already turned on the note's isolation condition and never recorded
that the same sentence is where "evaluated once and then replicated" comes from.

## What this tree does about NOTE 1, since it now has a reader

`Cell::repeat` evaluates the cell once and replicates its *commands* (ADR 0430), which is the
note's optimisation carried out in geometry rather than on a raster — and it is unconditional
where the note's is not. The note's condition exists because a tile blitted as pixels cannot take
the backdrop under each site into account; a copied command can, because every site composites
where it is drawn. So this tree meets the note at every blend mode and owes it nothing.
