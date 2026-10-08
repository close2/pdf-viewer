# 1688 — A focus request names its widget, and the host focuses that widget

Status: accepted and **built**. Session 1426. Supersedes ADR 1664 section 2's last choice (the
refusal of `setFocus` through a widget after the first); builds on ADR 1615's focus request.
Code: `crates/pdf-model/src/view/script_model.rs` (`ScriptEdit::Focus`'s `widget`),
`crates/pdf-model/src/view/scripts.rs` (`FocusRequest`, `ask_focus`, `take_focus_request`),
`crates/pdf-script/src/engine/bridge.rs` (`set_focus`), `crates/pdf-script/src/wire.rs` (version 7),
`crates/viewer-core/src/viewer.rs` (`carry_out_focus_requests`).
Tests: `crates/pdf-script/tests/realm.rs` (`set_focus_through_one_widget_asks_for_that_widget`),
`crates/pdf-script/tests/wire.rs` (both shapes round-trip), `crates/pdf-model/tests/script_properties.rs`
(`a_focus_request_names_the_widget_its_field_stood_for`), `crates/viewer-core/tests/script_levels.rs`
(`a_focus_request_through_the_second_widget_focuses_the_second_widget`).

## 1. The choice

Adobe's *JavaScript for Acrobat API Reference* (commit `ab3b42a7`, the source the owner settled in
`doc/todo/56`) lists `setFocus` among the methods of a widget, and its "Field" page makes
`getField("name.N")` that field's widget N from zero (ADR 1664). So a `Field` of one widget asks
for that widget and a `Field` of every widget for the first, which is what the host did for every
request until now. A documented choice under principle 5, as every API member is.

- **The index crosses as the property's does**: `ScriptEdit::Focus` carries `widget: Option<u32>`,
  counted in the field table's order (§12.7.4.1's `/Kids`), `None` for a `Field` of every widget.
  The wire's version is 7.
- **The view state checks the place once**, against the same table the host reads
  (`widgets_by_field_name`), and holds a `FocusRequest { field, widget }` with `None` read as 0. A
  place past the field's last widget is a sentence in the report and no request: the realm already
  answers `getField("x.N")` past the last widget with `null`, so only a hand-built outcome reaches
  this.
- **The host focuses the widget the request names** and turns to the page that widget's Table 166
  `/P` names.

## 2. Found beside it

`carry_out_focus_requests` read `/P` through `Document::get_key`, which resolves the reference to
the page dictionary, so `as_reference` was always `None` and no focus request ever turned a page:
a focus on a widget of another page put the keyboard on a widget the reader could not see. Table
166 makes `/P` "[a]n indirect reference to the page object", so it is now read unresolved, as the
view state's own `field_page` already read it. The new viewer test fails on the old read (page 0
where the widget is on page 1) and on `widgets.first()` (object 6 where 7 was asked). A report sentence of the same function carried 26 spaces where a line continuation had been
lost; it is one sentence again.

## 3. What stays

A widget with no `/P` is focused where it is and no page is turned, as before: Table 166 makes
`/P` optional for a widget, and finding its page means walking every page's `/Annots`, which a
focus request does not yet pay for.
