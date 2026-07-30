# ADR 0050 — A list is a list, and a `post` table is wherever the names are

Status: accepted, 2026-07-31.

## Context

`issue215.pdf` was the one page in `CONTRADICTED_UNEXPLAINED` where we differed from **every**
reference while the three of them agreed with each other — 8.5 levels from each, against their
own spread of 0.7 to 1.9. That pattern is what a pairwise table is for, and it is the strongest
signal of our own defect the gate can produce.

The page is a masthead: four references draw **OPENMAGAZIN** in small capitals, we drew
**openmagazin** in lower case.

## What the file says about itself

Three independent statements, all agreeing, none of them anybody else's rendering:

- **`/Differences`** names the eleven glyphs `o.sc p.sc e.sc n.sc m.sc a.sc g.sc a.sc z.sc i.sc
  n.sc` — the small-capital variants, by their conventional suffixed names.
- **`/ToUnicode`** maps those codes to U+F76F, U+F765, U+F76E, U+F76D, U+F769 — the private-use
  block Adobe assigns to small capitals.
- The **`CFF ` charset** in the embedded program names all eleven glyphs.

So the producer said "small capitals" three times. `poppler` said so too, in words:
`Mismatch between font type and embedded font file` — the dictionary says `/Subtype /TrueType`
with `/FontFile2`, and the program's leading bytes are `OTTO`, a CFF-based OpenType.

## Two readings of §9.6.5.4 were wrong, and each hid the other

**A glyph name goes to Unicode "by consulting the Adobe Glyph List and Adobe Glyph List for New
Fonts".** Those are two *lists*, and no entry in either contains a FULL STOP. What
`read_fonts::ps::agl::name_to_char` implements is the wider **Adobe Glyph List Specification**,
whose algorithm for an *unlisted* name strips everything after the first period — so it answers
`o` for `o.sc`. That is a real letter with a real glyph in the (3, 1) subtable, so the chain
"succeeded" and the clause's own next sentence never ran:

> In any of these cases, if the glyph name cannot be mapped as specified, the glyph name shall
> be looked up in the font program's "post" table (if one is present) and the associated glyph
> description shall be used.

**And that sentence would not have helped either.** This font's `post` table is **version
3.0**, which by definition carries no names at all; a CFF-based OpenType keeps its glyph names
in the `CFF ` charset. §9.6.2.1's NOTE 1 is the sentence that makes those the same structure —
a CFF is "an alternative, more compact but functionally equivalent representation of a Type 1
font program", and a Type 1 program's names are its charstring dictionary's keys.

Two defects, each of which alone would have left the page wrong, which is why neither showed up
as a hypothesis until the artefact was opened.

## Decision

`named_glyph` now runs §9.6.5.4's routes in the clause's order and with the clause's meaning:

1. The **listed** route — name to Unicode to the (3, 1) subtable, or to Mac OS Roman and the
   (1, 0) one — attempted only for a name without a period, because that is what "consulting
   the … List" can reach.
2. The program's own names: the `post` table, and then the `CFF ` charset, which is where a
   CFF-based OpenType keeps them. Built once per font, since it inverts the whole charset.
3. **Last**, and only for a suffixed name: the specification's algorithmic form. A font with no
   `o.sc` states nothing better than "an o", and drawing one beats drawing nothing — but it is
   a recovery rather than the clause's route, which is why it sits below the program's names
   rather than above them.

The test puts both glyphs in reach so that a wrong order is a wrong *answer* rather than a
missing one: the (3, 1) subtable holds `o` at glyph 1 and the `post` table names glyph 7
`o.sc`.

## Result

**`issue215.pdf` agrees with the reference consensus.** 811 agreeing and 87 contradicted became
**812 and 86**, and no other page moved — the corpus's incomplete count is unchanged at 95, so
this closed a page rather than trading one for a report.

## The spec track: §14.1 to §14.6, eleven rows

Clause 14's first six subclauses, and the interesting thing about them is how many are
`inapplicable` for a reason the clause states itself. §14.2's procedure sets "shall be used only
when the content stream is printed to a PostScript language compatible output device". §14.5's
page-piece dictionaries hold private data that "can be ignored by general-purpose PDF
processors". §14.3's metadata is interchange, and §14.3.3's information dictionary is deprecated
in PDF 2.0 for everything but two dates.

Two are not. §14.4's file identifier is `implemented`, because §7.6.4.3.2 step (e) takes
`/ID[0]` into the file encryption key — the clause's own purpose for the pair needs a second
document to compare against, and its *encryption* use decides whether a document opens at all.
And §14.6's marked content is `partial` for the reason it has been for several sessions: read
as a bracket by §8.11.3.3's optional content and by §12.7.4.3's splice, and not for any tag's
meaning.

§14.1's opening sentence is worth carrying: "The features described in this clause do not affect
the final appearance of a document." That is true of most of clause 14 and **false of the two
parts this tree implements** — output intents change every pixel a CMYK fill covers, and page
boundaries decide what is drawn at all.

## Consequences

- `CONTRADICTED_UNEXPLAINED` is 42, from 50 three sessions ago, and four of the eight that left
  were fixes rather than reclassifications.
- The pairwise-distance table earns its place as a standing instrument: it found the shared ICC
  profile (ADR 0048) and it found this page, in both cases before an artefact was opened.
