# ADR 0109 — A *shall* about a picture the standard never draws

Status: accepted, 2026-08-01.

## Context

`CLAUDE.md` names a text annotation's icon as its **standing example** of a place where the
specification defines nothing:

> Where the specification genuinely defines nothing (a `Text` annotation's icon is the standing
> example: §12.5.6.4 requires "predefined icon appearances for at least the following standard
> names" and states not one line of their artwork), say so plainly, make a deliberate choice,
> and document it *as a choice*.

For a hundred and nineteen sessions this tree read that sentence as licence to refuse. One
`match` arm in `appearance.rs` sent `Text`, `FileAttachment`, `Sound` and `Stamp` to the same
report — "its icon's artwork is the processor's own, and no clause states it" — and four ledger
rows said the same thing four times, each pointing at the others.

## The finding

Read the four clauses side by side and they do not say the same thing.

| clause | what it says about a predefined icon |
|---|---|
| §12.5.6.4, Table 175 | "Interactive PDF processors **shall** provide predefined icon appearances for at least the following standard names" |
| §12.5.6.12, Table 184 | "PDF readers **should** provide predefined icon appearances" |
| §12.5.6.15, Table 187 | "PDF readers **should** provide predefined icon appearances" |
| §12.5.6.16, Table 188 | "PDF readers **should** provide predefined icon appearances" |

One obliges and three recommend. The silence about the artwork is real and identical in all
four — `CLAUDE.md`'s sentence is correct about it — but the *obligation to have some* is
normative in exactly one of them. Refusing there is not restraint; it is a conformance failure
against a `shall`, and it had been hiding behind a true statement about a different half of the
same table cell.

This is the shape `CLAUDE.md` already warns about one paragraph later, in the other direction:

> "The specification defines nothing here" is itself a claim about the specification, and it
> decays.

The claim did not decay here — it was never the whole claim. **Reading a silence is not reading
a clause**, and the sentence containing the silence carried the requirement.

## The decision

§12.5.6.4's seven names — `Comment`, `Key`, `Note`, `Help`, `NewParagraph`, `Paragraph`,
`Insert` — are drawn. The three `should` clauses stay refused and named.

The split of what is read from what is invented is the whole of the design, and it is a crate
boundary in miniature:

- **`icon.rs` holds artwork and no PDF at all** — seven shapes on the unit square, this
  processor's invention, documented there as one, alongside `uri.rs` and `accessibility.rs`
  among the modules of `pdf-model` that know no PDF.
- **`appearance::text_icon` holds everything the document gets to say**: which icon (Table 175's
  `/Name`, default `Note`), and the colour behind it (Table 166's `/C` — "The background of the
  annotation's icon when closed", including that table's own "No colour; transparent" for an
  absent or empty array).

Three consequences of that split are worth stating because each was a choice:

**A `/Name` outside the seven is reported by name, not drawn as the note.** "Additional names
may be supported as well" is a permission, and Table 175's default answers an *absent* entry
rather than an unrecognised one. Substituting the note would draw a meaning the file did not ask
for and report nothing — trap 5's failure exactly.

**The symbol is black.** No entry states its colour. Black is the smallest invention available;
choosing black-or-white by the background's luminance would read better on a dark `/C` and would
be a *second* invention stacked on the first with no clause behind either. The cost is written
down rather than hidden.

**The icon is drawn on the largest square that fits inside `/Rect`, centred.** These shapes carry
their meaning in their proportions. §12.5.6.4 would prefer a third answer — the icon "shall not
scale and rotate with the page" — which needs the `NoZoom` flag §12.5.3 records as a departure a
resolution-independent display list cannot express. Until that changes, the icon is the size the
file's rectangle asks for and is not distorted by its shape.

## Consequences, measured

**No page in the corpus witnesses this, and the count that said otherwise was wrong.** Walking
every annotation of all 974 documents by subtype and by whether it carries an `/AP`: **one**
document has a text annotation with no appearance stream, and its `/Rect` is `[50 50 50 50]` —
zero area, so `annotation::decide` returns `Nothing` before any construction is reached. It drew
nothing before this change and draws nothing after it. The ledger row's "1 corpus document" was
a count of the annotation, not of a report; that is the third time a ledger claim has counted
the wrong thing, after the eighty-fifth and hundred-and-eleventh sessions.

The same walk settles the three refusals that stay: all **19** stamps, the **one** file
attachment and the **one** sound annotation in the corpus carry an `/AP`. A producer that cares
what its stamp looks like supplies one, which is why the `should` costs nothing here.

All four gates are unmoved: 840 agreeing and 65 contradicted of 1666, 90 documents drawing
incompletely, 97.9% of `pdftotext`'s words, 1545 dates. Tests 876 → 885. `reported` falls
**35 → 34** and `partial` rises 240 → 241.

Two ledger notes cited the wrong table number by one — §12.5.6.15's entries are Table 187 and
§12.5.6.16's are Table 188, where the rows said 186 and 187. The conformance gate prints a
table's *title* beside every number precisely so that a wrong number gives itself away (ADR
0095), and these two were in the ledger's prose rather than in the tree's, where that check is
the weaker one.

## What this does not license

Inventing a shape wherever a clause names an appearance. The argument here rests on one word,
and the three neighbouring clauses that lack it are still refused in the same commit — which is
the control. **The question to ask of a silence is not "may I fill it" but "does a sentence
around it require me to".**
