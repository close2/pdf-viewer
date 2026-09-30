# 1413 — A right-to-left value is displayed in UAX #9's order, by a reviewed crate

Session 1288. Status: **accepted** and built.
Context: `crates/pdf-font/src/shaping/bidi.rs`, `crates/pdf-font/tests/bidi_character_test.rs`,
`crates/pdf-model/src/variable_text.rs` (`Order`, `encode`, `write_lines`, `comb`), `Cargo.toml`,
`data/unicode/BidiCharacterTest.txt`, `data/unicode/BidiMirroring.txt`.
Builds: ADR 0348's third capability. Clauses: ISO 32000-2 §12.7.4.3, Table 228, Table 147, §7.9.2.2.1, §7.3.8.1.

## 1. Why the value needs an order at all

§12.7.4.3 has the processor write the content stream that shows a field's value, and §7.9.2.2.1
makes that value Unicode text, whose encoding forms "are described in The Unicode Standard by the
Unicode Consortium". Unicode stores text in logical order; the order it is displayed in is Unicode
Standard Annex #9's to state. ISO 32000-2 states nothing about direction for variable text — its
one `Direction` entry (Table 147) is about page order and "has no direct effect on the document's
contents" — so a value is resolved with rules P2 and P3 finding each paragraph's level.

## 2. Decision: take `unicode-bidi`, measure it, and do the line rules here

- **Resolution is Servo's `unicode-bidi` 0.3.18.** It was already in the lock under `stringprep`
  (§7.6.4.1's SASLprep), so no package is added to the graph; it is MIT or Apache-2.0, which
  `deny.toml` admits. `doc/stack.md`'s rule prefers a reviewed dependency where one exists, and
  one exists. Its one `unsafe` block reinterprets a `u8` slice as `Level`, a `repr(transparent)`
  newtype, independent of input.
- **It is measured, not trusted.** `tests/bidi_character_test.rs` reads the UCD's own
  `BidiCharacterTest.txt` (version 18.0.0, fetched 2026-09-29) at test time and resolves every
  line through `pdf_font::shaping`, comparing paragraph level, per-character levels and visual
  order. Every line agrees. The crate's tables are Unicode 16.0; the file is 18.0, and no line
  disagrees, so no character the file exercises changed class between the two.
- **Rule L1 is this tree's**, because it depends on where a line ends and only the layout knows:
  `Paragraphs::line_levels` resets separators and trailing white space per laid-out line, per
  paragraph. It is inside the measurement above — each case is one line. Rule L2 is the crate's
  `reorder_visual` over the levels of each code's source byte (`Order`).
- **Rule L4 is data**: `BidiMirroring.txt` compiled in; a character at an odd level is asked of
  the font by its mirror. **Rule L3 is not applied**: the annex leaves it to what the font's marks
  expect, and the faces measured here (DejaVu Sans, Noto Sans Arabic) give U+064B zero advance and
  an outline to the right of its origin, which overhangs the base that follows it in display order.
- Caret, click and selection keep working in logical offsets: a boundary is drawn at the edge of
  the code after it on that code's reading side, and a selected range is one box per displayed
  run. For a left-to-right value every mapping is the identity and nothing is collected.

## 3. Table 228's `/Q` names sides

"0 Left-justified 1 Centred 2 Right-justified" — the clause names sides of the box and says
nothing about a paragraph's start or end, so a right-to-left value under `/Q 0` starts at the left
side and under `/Q 2` ends at the right one. `a_right_to_left_value_is_quadded_to_the_side_q_names`.

That holds for every line, a wrapped one included. A right-to-left line wider than the box is
broken at its last space that fits, in logical order, so the words read first stay on the line and
the rest continue on the next; under `/Q 0` that continuation starts at the box's left side, which
is where Table 228 puts every line of the field, and a short one therefore sits alone at the left.
The trailing space a soft break leaves is set back to the paragraph level by rule L1 and displayed
at the line's left end. `freetext_no_appearance.pdf`'s short lines are exactly this: its stored
lines are longer than its box, and the fragments that sit alone at the left are the ends of those
lines, not breaks inside words (`a_wrapped_right_to_left_value_breaks_between_words`).

## 4. Cost

Every value is resolved once (`Paragraphs::new`), which is a pass over the text with the crate's
table lookups; a value that resolves to level 0 throughout stops there. The line-break character is
now a `Placed::Space` rather than any code 32, so a no-break space no longer ends a wrapped line —
which is what `substitutable`'s own comment already said happened.
