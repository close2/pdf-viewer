# ADR 0095 — Which way up a widget is, and what a redaction is not

Status: accepted, 2026-08-01.

## Context

Two `reported` rows in §12.5.6, and the session's finding is that they are opposite kinds of
thing. One was a real gap the tree had grown into; the other was a misreading that had been in
the ledger — and in `doc/HANDOVER.md` — for several sessions.

## Table 192's `/R` is drawn

> The number of degrees by which the widget annotation shall be rotated counterclockwise relative
> to the page. The value shall be a multiple of 90. Default value: 0 .

The row said `/R` "is not read and cannot yet matter: it rotates the widget's contents inside
`/Rect`, which no background filling that rectangle and no border inside it can see." That was
true when it was written and stopped being true in the twenty-third session, when a field's value
began to be laid out (ADR 0032): a line of text has a direction, and a quarter turn is exactly
what moves it.

**The rotation is a `cm`, not a `/Matrix`.** A widget with a stored `/AP` needs none of this — a
producer puts the turn in the stream's own matrix and §12.5.5 maps the rotated bounding box onto
`/Rect`. A *constructed* appearance is written in the page's own space, so §12.5.5's algorithm
reduces to the identity and there is nowhere to put a matrix; the turn goes into the content
stream.

**The contents are laid out in a box at the origin whose sides are `/Rect`'s swapped**, and this
is the half worth stating. A quarter turn of a rectangle exchanges its width and height, so a
field that is wide on the page is a *tall* box in its own axes — and §12.7.4.3's wrapping,
auto-sizing and comb cells all measure against the width the text actually has rather than the
one it would have had unturned. Doing it the other way round — laying out in `/Rect` and rotating
afterwards — would put the text outside the annotation.

A value that is not a multiple of 90 is refused by name. The table says "shall", so a widget
stating 45 has described a rotation no `cm` could be the one it meant, and rounding it would draw
a widget the document did not ask for and say nothing.

No corpus widget states an `/R`, so the fixture is synthetic and measures the *shape of the ink*:
a line of text in a wide short field is wider than it is tall, and turned a quarter it is taller
than it is wide and still inside `/Rect`.

## §12.5.6.23's overlay is not an appearance

The ledger said Table 195's `/OverlayText`, `/Q` and `/DA` "describe text painted over the
redacted region, which needs §12.7.4.3's variable text", and `doc/HANDOVER.md` listed the overlay
as one of "the two edges §12.7.4.3's layout left behind". Reading the whole table says otherwise,
and the words are in every entry:

- `/IC` — "the interior colour with which to fill the redacted region **after the affected content
  has been removed**";
- `/OverlayText` — "drawn over the redacted region **after the affected content has been
  removed**";
- `/RO` — "**After this redaction is applied and the affected content has been removed**, the
  overlay appearance should be drawn";
- `/Repeat`, `/DA` and `/Q` each say how one of those is done.

All six describe the clause's **second phase**, and the clause makes that phase a rewrite of the
document: "remove all content identified by the redaction annotation, as well as the annotation
itself … If a portion of an image is contained in a redaction region, that portion of the image
data shall be destroyed; clipping or image masks shall not be used to hide that data."
`CLAUDE.md` excludes writing files, so this program does not apply redactions and correctly paints
none of the six.

What a redaction annotation looks like in the state this program *does* see, the clause states in
one sentence — "the user can see, move and redefine these annotations" — and no artwork for it.
Drawn from `/AP` where there is one; where there is none the clause states nothing, exactly as
§12.5.6.4's icons do not. The row is `partial` rather than `reported`, because what is missing is
excluded rather than owed.

This is the second time a clause has been read as asking for work it does not ask for — §11.7.4's
overprinting was six `silent` rows a reading of Table 146 removed in the nineteenth session — and
both were found by reading the *whole* table rather than the entry that looked relevant.

## The checker prints the ledger's table titles now

The redaction row cited "Table 193". Table 193 exists, so `the_ledgers_own_prose_names_clauses_
and_tables_that_exist` passed; it is the **watermark annotation's** table, and the number is ISO
32000-1's for a table ISO 32000-2 renumbered to 195. That is precisely the failure the twentieth
session built the tree's table-title printing for — "a table number that names nothing is a defect
this can catch; a table number that names the wrong table reads exactly like a right one, and only
its title gives it away" — and the ledger had been left out of it.

`cargo test -p conformance -- --nocapture` now prints every table the *ledger* cites with its
title, beside the 164 the tree cites. The gate is unchanged; what changed is that the evidence is
on the screen where a person reading the run can see it.

## Consequences

`reported` falls 50 → 49, `partial` rises to 233. 838 tests. No gate moved: no corpus widget
states an `/R` and no corpus redaction annotation lacks an `/AP` in a way this changes.
