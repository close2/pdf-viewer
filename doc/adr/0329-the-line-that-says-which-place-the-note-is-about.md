# ADR 0329 — The line that says which place the note is about

Status: accepted, 2026-08-14. Session 494. Amends the §12.5.6.6 ledger row, extends ADR 0193's
table with a fifth entry, and closes the first of the two items `doc/todo/33` still carried.
Changes nothing any earlier ADR decided.

## The question

ADR 0238 gave §12.5.6.6's free text annotation a constructed appearance and named what it left
out: Table 177's `/CL` callout line and the `/LE` that ends it, reported by name because "the
clause states the geometry … and states no colour to draw it in". ADR 0304 restated the same
refusal from the other side. Ninety-three sessions later it is still the only entry of Table 177
this program owes a mark for, and the reason it was refused does not survive being read against
the entry beside it.

**No corpus document can decide this.** `examples/free_text_census` counts **0 of 73** free text
annotations stating a `/CL`, on every page rather than on first pages, so everything below is
trap 8's territory: the rule is defended by pairs of hand-built files differing in the rule and
by nothing else. That is not a weakness of the work; it is the only instrument there is for a
requirement the world's files do not exercise, and `CLAUDE.md`'s two denominators say so.

## What the standard states

Table 177, in full, on the entry:

> (Optional; meaningful only if IT is FreeTextCallout; PDF 1.6) An array of four or six numbers
> specifying a callout line attached to the free text annotation. Six numbers [ x 1 y 1 x 2 y 2 x
> 3 y 3 ] represent the starting, knee point, and ending coordinates of the line in default user
> space, as shown in "Figure 79 - Free text annotation with callout". Four numbers [ x 1 y 1 x 2 y
> 2 ] represent the starting and ending coordinates of the line.

and on `/LE`:

> The name shall specify the line ending style for the endpoint defined by the pairs of
> coordinates ( x 1 , y 1 ).

Three things follow that the refusal had not weighed.

**The geometry is complete.** Two or three points, one of Table 179's ten shapes at the first of
them, and Figure 79 draws the result — a note, a bent line, an arrowhead at the far end from the
note. Nothing here is a shape the standard names without describing.

**`/RD` is the entry that makes it fit.** The same table says the inner rectangle "is where the
annotation's text should be displayed" and that border styles "shall be applied to the border of
the inner rectangle". So `/Rect` on a callout annotation is not the text's box — the text's box is
inside it, and what occupies the difference is the line. Reading the two entries apart is what
makes each look under-specified.

**The condition to draw is on another entry's *value*.** "[M]eaningful only if IT is
FreeTextCallout", with `/IT` defaulting to `FreeText` and its third value stating outright that
"no callout line is drawn". The markdown conversion drops two of those three values and the
default line, which `pdftotext -layout` over `doc/`'s PDF shows — `doc/HANDOVER.md`'s standing
caveat about tables, earning its keep for the second time on this same table.

## The decision

**Table 177's `/CL` is drawn, with `/LE`'s ending at the endpoint the table names, wherever the
annotation states no appearance stream and its `/IT` is `FreeTextCallout`.**

### Why this is drawn where `/BS`'s border is still refused

Both entries lack a colour and the refusal cited exactly that, so the interesting question is what
separates them. It is not the silence, which they share. It is **which of the two the file asked
for**:

- Table 166's `/Border` states "Default value: [0 0 1]", so a border is what an annotation saying
  nothing at all about one *has*. Drawing it in a colour of this program's choosing would put a
  mark on nearly every free text annotation in the world on the strength of a default — and that
  default is the reason `ViewState::add_free_text` writes `/BS /W 0` on the annotations this
  program creates (ADR 0238).
- A callout exists only where a producer wrote four or six numbers *and* an intent. Those numbers
  say the one thing no other mark on the page says: which place the note is about. Not drawing
  them loses information the file states and nothing else carries.

That is ADR 0106's own test — is the entry a refusal refuses additive or substitutive — asked
about the *reason for the mark* rather than about the mark.

### The colour is omitted rather than invented

The construction states black, and the point is that this is the absence of a choice rather than
one. §8.4.1's Table 51 gives the graphics state's colour parameter "Initial value: black", so a
content stream that names no colour paints in it; drawing the line takes no entry, no convention
and no other renderer's output. It is nevertheless *written out* — `0 G` — so that the mark cannot
depend on what ran before the appearance was invoked.

Two colours were considered and both were rejected as inventions. Table 166's `/C` is "the
background of the annotation's icon when closed, the title bar of the annotation's popup window,
[and] the border of a link annotation", an exhaustive list with none of this subtype's marks in
it. Table 177's `/DA` is "the default appearance string that shall be used in formatting the
text", and a line is not text — borrowing its colour would also mean translating a nonstroking
operator into a stroking one, which is a second invention on top of the first.

The **width** is a choice and is recorded as one: one point, §12.5.4's own number for a line width
when nothing else states one. `/BS` is not it, because Table 177's `/RD` row binds `/BS` to the
border of the inner rectangle — the one entry of this subtype carrying a line width is a statement
about a different mark, and a `/BS` of `/W 0` means "no border" rather than "no callout".

### An intent the table calls meaningless is drawn *and reported* as nothing

`FreeTextTypeWriter` and the default `FreeText` draw no line for a `/CL` they state, and **owe
nothing for it**. A report names what this program owes; the table owes a mark only under the
intent it says the entry is meaningful for, so a report here would fire on a condition the clause
does not have — trap 11's shape, which has cost this project four populations of noise. The
silence is the reading, not a gap in it.

What *is* reported is a `/CL` whose length is neither four nor six — which of five numbers a
reader should keep is not something the table answers — and a `/LE` naming a style outside Table
179's ten. The second is reported *beside* a drawn line rather than instead of it, because `/CL`
is what states the line and `/LE` only decorates it: ADR 0075's rule, one entry over.

### A fifth row for ADR 0193's table

That ADR decided that "a constructed appearance is bounded by `/Rect` only where the clause it is
built from bounds it by `/Rect`", and listed four entries whose coordinates the standard puts "in
default user space". **Table 177's `/CL` uses those same words and was not on the list**, which
nothing could have noticed while the entry was refused. A free text annotation is therefore
unbounded now, and joining costs its *text* nothing: §12.7.4.3's own example puts "any required
graphics state changes, such as clipping" inside the construction, so `variable_text::lay_out`
clips the value to its box itself and has since before `bounded` existed. A conforming callout is
inside `/Rect` anyway — that is what `/RD` is for — and a file whose `/Rect` is wrong now loses
nothing, which is the position ADR 0193 put four other subtypes in.

**One asymmetry follows and is left standing deliberately.** An `/AP` this program *writes* — for
an annotation a person added or retyped — is a form `XObject`, and §8.10.2 makes a `/BBox`
compulsory, so `view::write_added_appearance` states `/Rect` and the next reader will clip to it.
That is §12.5.5's own rectangle rather than a box this program invented, and widening it to the
union of `/Rect` and the marks is the alternative ADR 0193 already rejected: it would still be
invented and would still clip something one day. A conforming callout is inside `/Rect` because
`/RD` is what makes room for it, and no annotation this program creates states a `/CL` at all.

### How a stored appearance interacts with all of it

It does not. §12.5.5 makes an appearance stream "a self-contained content stream", so where one
exists it is the whole appearance and nothing is added over it. §12.5.2's list of entries a reader
ignores while rendering an appearance dictionary names `/LE` and not `/CL`, which looks like a
licence and is not one: the list is about entries that would otherwise *contribute* to a
construction, and a second mark painted over a self-contained stream is not something any sentence
here asks for. This module is reached only where the annotation has no `/AP`, which is where the
question arises at all.

## What it is worth

- **Five fixtures, four pairs, and the pictures were looked at** (trap 1). At four times scale, on
  a 200×100 page with `/Rect [20 40 180 70]`: a two-point `/CL` draws the diagonal, a three-point
  one bends at its knee, `/LE /ClosedArrow` puts an unfilled arrowhead at (x1, y1) pointing away
  from the note — unfilled because Table 179 fills it "with the annotation's interior colour, if
  any" and Table 177 gives this subtype no `/IC` — `/RD [40 0 0 0]` moves the text forty points and
  the line not at all, and `/IT /FreeTextTypeWriter` draws the note alone. The whole callout lies
  below `/Rect` in every one of them, which is the unbounded rule doing its work.
- **Every fixture is a pair differing in one entry**, because a fixture that only shows that
  something was drawn shows nothing: the two-point line is inked at (40, 20) and not at (30, 20)
  and the three-point one is the exact reverse.
- **No corpus page can move**, and identical is the check rather than an excuse: no document in
  the 974 states a `/CL`, so the corpus and the oracle are expected to reproduce line for line.
- **What is still owed on Table 177** is `/BS`'s border, for the reason above, and `/DS`, which
  principle 5 excludes as XFA.
