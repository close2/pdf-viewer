# 1664 — One widget of a field is a `Field` of its own, and the value stays the field's

Status: accepted and **built**. Session 1414; **superseded in part by ADR 1688 (session 1426)**: the
last choice of section 2, `setFocus` refused on a widget after the first, is answered — a focus
request names its widget. Supersedes ADR 1652 section 2 (the refusal of `getField("name.N")`); builds on ADR 1603's realm table, ADR 1617's drawn properties and ADR 1653's
field table.
Code: `crates/pdf-model/src/view/script_model.rs` (`FieldState`, `WidgetState`, `FieldState::apply`,
`Property::is_widget_level`, `ScriptEdit::Property`'s `widget`, `Overrides`, `widget_state`),
`crates/pdf-model/src/view/scripts.rs` (`apply_property`, `Scripting::one_widget`),
`crates/pdf-model/src/view.rs` (`write_scripted`), `crates/pdf-script/src/engine/bridge.rs`
(`get_field`, `widget_address`, `widget_object`, `widget_of`, `read_property`, `write_property`,
`set_focus`), `crates/pdf-script/src/engine/members.rs` (the captions), `crates/pdf-script/src/wire.rs`
(version 6).
Tests: `crates/pdf-script/tests/realm.rs`
(`one_widget_of_a_field_answers_its_own_widget_members_and_the_field_s_value`),
`crates/pdf-script/tests/wire.rs` (two widgets and a scoped edit round-trip),
`crates/pdf-model/tests/script_properties.rs`
(`a_property_set_on_one_widget_reaches_that_widget_alone`).

## 1. What the two sources split, and where they split it

ISO 32000-2 §12.7.4.2 gives a field one value however many widget annotations show it, and
§12.5.6.19 gives each widget its own presentation entries (Table 191's `/MK` and `/BS`, Table 166's
`/Rect` and `/F`). Adobe's *JavaScript for Acrobat API Reference*, read at `adobe/dc-acrobat-sdk-docs`
commit `ab3b42a7`, splits `Field`'s members the same way on its "Field versus widget attributes"
page, and its "Field" page says what `getField` answers for a field's name followed by a PERIOD and
an index from zero: a `Field` of that one widget, whose widget members read and write that widget
and whose field members read and write the field. Through a `Field` of every widget, a widget member
reads the first widget and a write reaches all of them. The reference is the working source the
owner settled (`doc/todo/56`), so every rule below is a documented choice under principle 5.

## 2. The choices

- **A field's state is the field's members and a list of widget states.** `FieldState` keeps the
  name, type, value, flags, `charLimit` and `page`; `WidgetState` holds `display`, the three
  colours, `borderStyle`, `alignment`, `rect` and the three captions — the members of the
  reference's widget list this bridge carries. Each widget's colour and quadding are read up that
  widget's own chain, so two widgets of one field with their own `/DA` say different things.
- **The index is the field table's order**, which is §12.7.4.1's `/Kids` order — the order a host
  focuses and a calculation walks. The reference counts widgets in the order they were created and
  the file does not record that; `/Kids` is the order the file does record.
- **A member set through one widget's `Field` is scoped only if it is a widget member.** `value`,
  `readonly`, `required`, `charLimit` and the text flags set through `getField("x.1")` reach the
  field, as the reference's table has it; `ScriptEdit::Property` carries `widget: None` for them
  whatever the script addressed.
- **`getField("x.N")` past the field's last widget answers `null`**: no such widget exists, which
  is the same answer as a name no field has. The exact reading still comes first, so a field whose
  own name ends in digits is found as itself (ADR 1652 section 1).
- **`setFocus` through a widget other than the first is refused by name**: a host takes a focus
  request by field and focuses its first widget (`viewer-core`'s `take_focus_request` site), so the
  second widget is a request no host can yet carry.

## 3. A save keeps a one-widget member off the field

`/DA` and `/Q` are the field's entries (Table 228); a widget's siblings inherit them. A text colour
or alignment set on one widget of several is therefore written into that widget's saved appearance
stream and into no field entry, where it would recolour the other widgets. A later set through the
whole field writes the entry as before. `/MK` and `/BS` are the widget's own and are written on it
whichever way the member was set. The cost: a later processor that constructs that widget anew from
its entries alone draws the field's colour, not the one-widget colour; this program's saved stream
shows the colour.

## 4. The wire

`pdf_script::wire::VERSION` is 6: a field's state carries its widget count and each widget's
members, and a property edit carries its optional widget. A worker and a host from different trees
refuse each other by the version byte, as before.

## 5. Measured

See the session's record for the Tier 1 column, the worker column, Tier 0's `script_corpus` and the
corpus. `5712688.pdf`'s `TFTemplate_Format`, which asks for `"@@b12c96nfMM2_3m.0.0"`, was the census
run ADR 1652 section 2 named as the cost of the refusal; it now reads the widget it asks for.
