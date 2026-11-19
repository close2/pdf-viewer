# 3097 — A field's `/DA` font whose program will not read is laid out in its substitute

Session 2099. Status: **implemented.** Context: ADR 1501 (a field's characters cross with places), ADR 2619 and
ADR 2870 with `doc/questions/A837` (a program present and unreadable is drawn from a substitute face), the
accessibility census's witness "a filled field whose characters have no place". Clauses: ISO 32000-2 §12.7.4.3,
§9.5, §9.2.4.

## The witness

The census named one field, `multiline` in `PDFBOX-3148-2-fuzzed.pdf` holding `1\n2\n3\n4`. Its `/DA` is
`/Verdana 12 Tf`, a TrueType font whose `/FontFile2` (fuzzed) does not decode, beside its `/Widths` and descriptor.
`pdf_model::variable_text` refused the whole layout with `Owed::FontUnusable`, so `ViewState::field_glyphs`
answered nothing and the node crossed with its value and no place for any character of it.

## Decision

**A model defect, not the census's question.** The page already draws an unreadable program from the substitute
its dictionary describes (`content::font`, ADR 2619): §9.5's font dictionary carries "information that can be used
to provide a substitute when the font program is not available", and §9.2.4's NOTE 2 says the widths it stores
enable "a PDF processor to determine glyph positioning without having to look inside the font program". The layout
alone refused, so the appearance it would write, which the same interpreter draws from that substitute, was never
written, and the census was right to count the field.

1. `LayoutFonts::load` answers a `LayoutFont`: the face, and the program's refusal where
   `pdf_font::LoadedFont::substitute_for_unreadable` stood in. The refusal is kept with the face in the layout's
   own cache, so every layout set in it reports it (trap 5), and a substitute is never put into the interpreter's
   `FontCache`, which serves without the report.
2. `lay_out` reports `Owed::FontSubstituted`: "its /DA's font program is unusable (…), so the value is laid out in
   the substitute face its font dictionary describes (§9.5)", after a stood-in name and before a shortfall.
3. `Owed::FontUnusable` stays for a font that will not load even as a substitute, or whose codes no character
   reaches. Rich text's `Face::load` is unchanged: a rich value whose face will not load falls back to the plain
   layout, which now substitutes.

Test: `viewer-core` `untagged_widgets::a_field_set_in_an_unreadable_font_program_has_places_from_its_substitute`,
the witness's shape built by hand (a `/FontFile2` of bytes that are no font program): two lines, `1` and `2`, inside
the widget. It fails with the change reverted ("a line each: []").

## Cost

A field set in a broken program is now drawn, where the page constructs its appearance, in a face the producer did
not embed; the report says so, as the page's own text in such a font already does.
