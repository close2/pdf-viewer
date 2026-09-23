# 1323 — A list box's selection is highlighted, in a colour this program chose

Status: accepted and **built**.
Context: `crates/pdf-model/src/variable_text.rs` (`SELECTION_HIGHLIGHT`, `selection_highlight`,
`Request::selected`, `Owed::ListBoxSelection`), `crates/pdf-model/src/appearance.rs`
(`list_box_options`, `field_text`), `crates/pdf-model/tests/variable_text.rs`
(`a_list_boxs_selected_options_are_highlighted_behind_their_text`).
Answers: `doc/questions/A72` — "choose, and bound it" — for the row `doc/todo/65` held it back
for.
Supersedes: ADR 0407's reading that the selection of a list box drawn on the page is reported and
not marked (the rest of 0407, the options drawn from `/TI` in `/Opt`'s order, stands).

## Which side of A72's bound

A72 permits a chosen quantity where a clause names the **kind** of mark and withholds only the
quantity, and forbids one where the clause names no mark. §12.7.5.4 names the selection twice —
"one or more of which shall be selected as the field value", and `/V` "identifies the item or items
currently selected" — for items it requires be "displayed on the screen" in "[a] scrollable list
box". §6.3.2.2 asks a rendering processor for the page's contents, and a list box's contents are
its options *and* which of them its value selects; a list drawn with no selection distinguished
does not show the field's value at all, where a combo box, which draws the value, does. So the kind
is named — a mark distinguishing a selected option from the others — and what is withheld is its
colour and extent. **Near side.** The caret (§12.5.6.11) and the stamp legends (§12.5.6.12) stay on
the far side: each names an artwork and states none of it, which is a shape rather than a quantity.

## The choice

- **What**: each selected option's line is filled behind its text, the full chord the box leaves at
  its baseline, one leading tall from the line's ascent, so adjacent selections join into one band.
- **Colour**: `DeviceRGB 0.6 0.75 0.9`, a light blue on which the black a `/DA` usually sets keeps a
  contrast better than ten to one. Painted inside its own `q`/`Q` between the clip and `BT`, so it
  cannot become the text's colour where the `/DA` states none.
- **Under a `/DA` `Tm`**: the band is carried through the linear part corner by corner, so it is the
  parallelogram the line's glyphs sit in.
- **Reported** as this program's (`Owed::ListBoxSelection`), on the clause's own condition: a `/V`
  naming an option visible from `/TI`. A null `/V` marks and reports nothing.

## Consequences

§12.7.5.4 moves `partial` → `implemented`, and leaves `doc/todo/65`'s not-owed bucket. The corpus
cannot see it: every list-box widget on this disk states an `/AP` `/N` and none sits in a
`/NeedAppearances` document (the row's census), so the mark reaches a page only when a person
chooses an item or a producer asks for regeneration — which is where it was owed.
