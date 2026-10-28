# 2522 — A standard-font name the conversion defines is a raised failure of the embedding requirement, and is embedded once per name

Status: implemented. Session 1844, the R04 slot of batch one hundred and twenty-five (Sonnet, high).
The first of the two items ADR 2500's "What remains" and ADR 2490's row R04 left owed. Context:
ADR 2500 (the standard-fourteen name defined as a `/Type1` font, whose embedding the check of the
output refused by name), ADR 1990 (a requirement the source met and the output would fail is held
like any other), ADR 0816 (no content-stream byte is edited), ADR 0947 (a shape a remedy cannot
carry out is a refusal naming it); `crates/pdf-transform/src/archive/{prepare,fonts,reader_resources}.rs`.
Q710 is unused.

## The reading

ISO 19005-2 section 6.2.11.4.1 requires every font the file uses to be embedded,
and ISO 32000-2 §9.6.2.2 names fourteen fonts a PDF processor must supply. A page that draws with a
resource name the producer never defined is drawn by this reader with the standard font the name
spells; ADR 2500 therefore defines that name as an object of
the category it was looked up in, a `/Type1` font with the standard `/BaseFont`. That object is
new: it is in no stage-1 judgement, because the producer's file defined no font at all, so
`fonts/font-programs-embedded` met in the source and the embedding route, which works from the
failed judgements, was never asked for it. The output then failed the requirement the table says
the conversion meets, and the check refused the file by name (round 1833's re-check).

## The decision

1. **A second planning pass over the added definitions.** The definitions are worked out before the
   fonts are, and the font names the page-resources pass defined and the reader drew text with are
   collected (`ReaderResources::rendered_standard_fonts`: a font nothing drew with owes no program,
   section 6.2.11.4.1 binds the fonts a page uses). Where there are any, `fonts/font-programs-embedded`
   is **raised**: it is decided as a failed requirement of the source would have been, with the
   number of fonts as its places, and the output is held to it afterwards like every other
   (`Prepared::raised`, `raised_judgements` in `archive/mod.rs`). The identifiers leave
   `Conversion::conformed`, because the source did not meet them in the file this conversion writes.
2. **The embedding route embeds the face the reader drew with**, once per name: the definition is
   one object however many forms and pages reach the name, the font program is one stream, and a
   font this conversion added is written whole into the output rather than restated as an edit to an
   object the source does not hold (`Substitutes::added_fonts`).
3. **A route that cannot embed keeps the refusal it had**: the check of the output, which holds the
   file to every requirement the table says the conversion meets, still names the font if the
   embedding route did not write a program for it (ADR 2500, "Where the empty font is refused").

## Proof

- One test, `a_standard_font_name_nothing_defines_is_the_font_the_reader_drew_not_an_empty_one`,
  converts a page drawing `/Helvetica` and one drawing `/Times-Roman` with the name undefined, each at
  2b and at part 4, 4 conversions. Each output meets its target by this tree's own check, carries a
  font program in its descriptor (a `TrueType` dictionary for Helvetica's `glyf` face, `Type1` for
  Times-Roman's CFF), and passes veraPDF 1.30.3 at its flavour, 4 of 4. The raise was planted out
  (`raised` always empty): the test fails at the output's own check, so the pass is not vacuous.
- `a_standard_font_defined_for_a_name_is_one_object_whatever_number_of_streams_name_it` pins the
  once-per-name rule: a page and 60 forms that all show `/Helvetica` reach one font object, and its
  face is embedded once.

## Consequences

The R04 list loses its font entry; what remains of R04 is the Type 3 glyph procedure's resources
and a font that moved, both stated in ADR 2500 and unchanged.
