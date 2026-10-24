# 2320 — A show string's loop state is one struct and its glyph loop is one method per step of §9.4.4

Session 1743, the standing code-quality slot of batch 113. Status: implemented; no arm's behaviour changed, no bug fixed.

`Interpreter::show_text` in `crates/pdf-model/src/content/text/show.rs` was the longest function left
(390 lines) and carried `#[expect(clippy::too_many_lines)]` on the reason that splitting it "would need nine
parameters to carry the loop's state into the piece that was moved". The reason named the cure: the nine are
one string's constants and tallies, and a struct carries them.

**What it is now.** `ShowString` holds what §9.4.4's glyph loop carries from one code to the next: the font, the
state and the state the colours are built from, `GlyphPainting`, the size, horizontal scaling and writing mode,
Table 120's extent, whether the string is reversed (§14.8.2.5.3), the readback pieces and quadrilaterals of a
reversed string, and the string's glyph coverage. `CodeAt` is one code after it was read and placed. `show_text`
is the setup of a string (the refused-font and no-font exits, the paint decisions made once per operator, the
separation decision) and a loop of five calls, one per step of the clause:

- `read_code` — §9.10.2's readback of the code, and the count of one the page showed that no clause could name;
- `place_in_text_layer` — the text rendering matrix, and the code's quadrilateral in the text layer;
- `draw_code` — whether the rendering mode paints, then the outline route or the Type 3 route;
- `draw_program_code` — an outline font's glyph, or why it has none; `account_for_a_missing_mark` — the
  program's own answer for a code that made no mark and did not read back as a space, counted by cause (ADR 0520);
- `advance_past` — §9.4.4's update of the text matrix by the displacement, `Tc`, and `Tw` on a single-byte 32.

**Choices.**

- *The struct is private to the file.* Nothing outside reads a string's loop state; `pub` stays what a sibling
  reads.
- *`ShowString` holds references and `Copy` values and the methods copy them to locals* (`let state = show.state`),
  so a moved body keeps its text and the borrow of `show.coverage` is a field borrow beside the others.
- *`vertical` is a field, not asked of the font per code*, so the loop does the work it did before.
- *Nothing is reordered.* The readback precedes the placement, the placement precedes the paint, the paint precedes
  the advance, as in the clause and as before; the tally and the reversed append follow the loop.
- *The `BDC` arm of `run_marked_content_op` in `content/run.rs` became `begin_marked_properties`* on the same
  reason: it read six property-list entries in turn and was about 90 lines of a 140-line function. The arm is the
  method's body unchanged and `EMC`'s method is as it was.

**What did not move.** No file, no ledger row (the rows name the files, which stand), no behaviour. The multiset of
the two files' non-comment lines against HEAD's: every statement of the glyph loop is present once, verbatim
apart from indentation. What leaves `show.rs` is the `expect`, the three `let mut` that became `let` (they are
moved into a struct that is itself `mut`), `reversing.then_some(&mut pieces)` written `then_some(pieces)`, and the
`coverage` and `reversed_quads` names written through the struct; what arrives is signatures, the two struct
definitions, and `let` bindings of a struct's fields. `run.rs` gains one signature, one call and a closing brace and
loses nothing.
