# 0432 — The border a default draws, and the census that measured the reason it did not

Status: accepted, in the five-hundred-and-ninety-seventh session.
Supersedes the border half of ADR 0329; ADR 0106 supplies the test it turns on.

## The question

§12.5.6.6's free text annotation had one entry of Table 177 left undrawn: `/BS`, the border. It
was refused and reported, on an argument ADR 0329 stated in one sentence — the colour is stated
nowhere, and unlike the callout line beside it, Table 166's `/Border` gives every annotation a
border by default, so drawing one would put a mark on nearly every free text annotation in the
world on the strength of something no producer wrote.

That sentence contains two claims of different kinds, and this round separated them.

## What the standard states, read across the neighbourhood

The shape is stated in full. §12.5.4:

> An annotation may optionally be surrounded by a border when displayed or printed. If present,
> the border shall be drawn completely inside the annotation rectangle.

> If neither the Border nor the BS entry is present, the border shall be drawn as a solid line
> with a width of 1 point.

Table 177 gives this subtype the entry that carries it — a `/BS` "specifying the line width and
dash pattern that shall be used in drawing **the annotation's border**" — and its `/RD` row says
where the mark goes: "Any border styles and/or border effects specified by BS and BE entries,
respectively, shall be applied to the border of the inner rectangle."

**Table 168's `/S` styles this border, and does not style a square's.** §12.5.4 names the subtypes
whose `/BS` supplies less than a whole border style dictionary — "[s]uch dictionaries may also be
used to specify the width and dash pattern for the lines drawn by line, square, circle, and ink
annotations" — and free text is not among the four. `/RD`'s own words are "border styles", plural,
which is Table 168's `/S` by name.

**The colour really is stated nowhere, and this was checked rather than assumed.** `CLAUDE.md`'s
rule is that a claim of silence decays, so the titles around the subject were read before the
silence was recorded again: Table 166's `/C` is a closed list of three purposes — the icon
background, the popup title bar, the border of a *link* — and on a free text annotation it is the
second of them, because a markup annotation has a `/Popup`, has no icon, and is not a link.
§12.5.6.19's Table 192 `/BC` is "the colour of the widget annotation's border" and this is not a
widget. Every sentence in ISO 32000-2 relating a border to a colour was extracted and read: the
standard states a border colour for a link, for a widget, for a collection card, and for a tagged
structure element's `BorderColor`. For an annotation's border in general it states none.

## What was measured, and what it changed

The refusal's second claim — that the default fires on annotations whose producers said nothing —
is a claim about *producers*, which is measurable, and trap 11 says to measure a report's
condition before trusting it. `examples/free_text_census` gained the count:

- **73** free text annotations, in 27 of the corpus's 964 openable documents;
- **67** carry an `/AP` `/N` stream, where §12.5.2 has a reader ignore `/Border` and `/BS`
  outright, so they are not this path's business at all;
- of the **6** that reach a constructed appearance, **all six state `/Border` explicitly** and
  **four state a width of zero**;
- **none** states neither entry. §12.5.4's default fires on nothing in this corpus.

So the population the report was firing on was **two annotations, in one document, and that
document is `poppler-395-0-fuzzed.pdf`**. The picture the refusal was defending against — a mark
appearing on annotations nobody asked about — has no member here at all, and the producers who
want no border say so in the entry the table provides for saying it.

## The decision

**Draw it**, in black, and record black as a choice.

ADR 0106's test is what makes this the same decision as the callout's rather than a different one:
an optional entry that states no shape must not erase the shape the clause does state, and the
question of any refusal is whether what it refuses is *additive* or *substitutive*. No entry
claims this border's colour, so painting in the initial one substitutes for nothing. §8.4.1's
Table 51 gives the graphics state's colour parameter "Initial value: black" — a construction that
names no colour paints in it — and the construction writes it out explicitly only so that the mark
cannot depend on what ran before the appearance.

A cloudy `/BE` is the other side of that test and stays a whole refusal, exactly as it is on a
square: Table 169's border "should be drawn as a series of convex curved line segments", and a
rectangle in its place is a shape the file did not describe. §12.5.4 gives this subtype the entry
in as many words — "Beginning with PDF 1.6, free text annotations may also have a BE entry" — and
nothing had been reading it here. **The note itself is drawn either way**, which is the rest of
ADR 0106's rule: the border is one mark of two and the text is what the subtype *is*.

## What it cost, and what it did not

**No corpus page moves.** The two annotations that were reported are in a fuzzed document, and
every gate is unchanged. That is the honest position rather than a disappointing one: this is a
coverage change, and `CLAUDE.md`'s two denominators say a corpus cannot rank a requirement no
document exercises. Trap 8's shape follows from it — the rule is defended by four hand-built
fixtures differing in `/Border`, in `/BS` and in `/BE` alone.

**One thing this changes that is not a border.** An empty free text annotation with a border is
now a mark on the page where it used to be nothing, because the flag that decides whether the
construction reaches the display list was the callout's alone.

**And a claim one file over was retired with it.** `appearance::free_text`'s doc comment,
`view::add_free_text`'s reason for writing a `/W` of 0, and the §12.5.6.6 ledger row all carried
the same "fires on every annotation" sentence. The `/W` 0 stays and reads better for the change: a
note this program adds asks for no border, in the entry the table provides for saying so.

## The rule worth carrying

**A refusal that rests on a claim about producers has an instrument, and should be made to use
it.** ADR 0329's argument was not wrong about the standard — the default is real and it is a
`shall`. It was a prediction about what files contain, held for a hundred sessions without anybody
counting, and the count took twenty lines in a census that already existed. The tell is
grammatical: a reason that says *would* rather than *does* is a reason nobody has measured.
