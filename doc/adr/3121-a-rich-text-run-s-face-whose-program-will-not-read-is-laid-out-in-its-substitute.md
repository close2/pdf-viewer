# 3121 — A rich text run's face whose program will not read is laid out in its substitute

Session 2105. Status: **implemented.** Context: ADR 3097 (the plain value's `/DA` font), ADRs 2619 and 2870 with
`doc/questions/A837` (a program present and unreadable is drawn from a substitute face), ADR 1634 (how a rich text
string's faces are chosen). Clauses: ISO 32000-2 §12.7.4.3, §9.5, §9.2.4.

## The gap

ADR 3097 made `LayoutFonts::load` lay a plain value out in the substitute its font dictionary describes, and left
`variable_text::Face::load`, which `rich_text`'s layout loads every face through, refusing such a program with
`Owed::FontUnusable`. A rich value (§12.7.4.3's `/RV`) whose `/DA` font has an unreadable program therefore failed
its own layout and its one-style form (which loads the same face first), and the widget was reported "its /DA's font
is unusable" with no appearance constructed, not even the plain value ADR 3097 expected; the page itself draws such a
font from the substitute (ADR 2619).

## Decision

1. **`Face::load` substitutes as `LayoutFonts::load` does**: where `pdf_font::LoadedFont::load` refuses, it asks
   `LoadedFont::substitute_for_unreadable`, and keeps the refusal in `Face::unreadable`. One rule for every face the
   rich layout holds — the `/DA`'s, a `/DR` face a family resolves to, an invented one (which has no program to
   refuse), a machine face — because each is drawn by the interpreter, which substitutes every one of them.
2. **The layout reports it** with `Owed::FontSubstituted`, in the plain layout's order: after a stood-in name, before
   what the faces could not show. It is reported where a glyph was set in the substitute (`Faces::substituted`);
   a face no glyph used was not drawn.
3. `Owed::FontSubstituted`'s sentence no longer says "its /DA's font": a rich run's face may be a `/DR` face of its
   family. It says "a font program its value is set in is unusable (…)", and the refusal names the font.

Test: `pdf-model` `forms_suite::rich_text::a_rich_string_in_an_unreadable_font_program_is_laid_out_in_its_substitute`,
a `/DA` TrueType font whose `/FontFile2` is no program and a value whose second digit is a green span. With the
change reverted the green is absent and the widget is reported unusable.

## Cost

A run set in a broken program is drawn in a face the producer did not embed; the report says so, as the plain
value's does.
