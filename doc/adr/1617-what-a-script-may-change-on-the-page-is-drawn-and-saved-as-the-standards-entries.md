# 1617 — What a script may change on the page is drawn, and saved as the standard's entries

Status: accepted and **built**. Session 1390. Supersedes ADR 1603's decision to keep `textColor`,
`fillColor`, `strokeColor`, `borderStyle`, `alignment`, `charLimit` and `required` without drawing
them; the rest of ADR 1603 stands. Builds on RFC 0008 section 6.4 (a script's write is an edit
beside the document) and ADR 1616.
Code: `crates/pdf-model/src/appearance.rs` (`scripted_entries`, `with_scripted`,
`constructs_for_script`, `for_saving`'s `scripted`, `construct`'s widget arm),
`crates/pdf-model/src/annotation.rs` (the construction a scripted widget takes),
`crates/pdf-model/src/view.rs` (`AnnotationView::scripted`, `write_scripted`, `named_field`),
`crates/pdf-model/src/view/scripts.rs` (the per-widget record), `crates/viewer-core/src/viewer.rs`
(`as_displayed`), `crates/viewer-gtk/src/controls.rs` and `crates/viewer-qt/cpp/window.cpp` (the
editable combo box's commit).
Tests: `crates/pdf-model/tests/script_properties.rs` (one fixture per property),
`crates/viewer-core/tests/field_commit.rs` (`an_accessible_value_is_the_displayed_one`); the drive's
`script_painted` step.

## 1. A property is the entry the standard draws a widget from

A script's property write is an edit, and what the edit *is* is stated in the file's own vocabulary
rather than invented: ISO 32000-2 §12.5.6.19's Table 191 makes `/MK` the dictionary "that shall be
used in constructing a dynamic appearance stream", and §12.7.4.3's Table 228 makes `/DA` a sequence of
operators "that define such properties as the field's text size and colour". So `fillColor` is Table 192's
`/BG`, `strokeColor` its `/BC`, `borderStyle` Table 168's `/S` in `/BS`, `textColor` a colour
operator in `/DA`, `alignment` `/Q`, `charLimit` Table 232's `/MaxLen`, and `required` Table 227's
bit 2 in `/Ff`. `appearance::scripted_entries` is the one place the mapping is made; the property
names and their meanings are Adobe's reference, a documented choice under principle 5.

**`textColor` is appended after the `/DA`'s own operators**, the shape `AFNumber_Format`'s red
negative already takes (ADR 1578 section 2): the producer's font, size and other state are replayed
and the last colour set is the one the text is drawn in. A transparent text colour writes nothing —
an empty array is a background's "no colour", and text has no such state.

## 2. Drawn: a scripted widget is constructed anew

The view state keeps each such property per widget; `AnnotationView::scripted` carries them to the
appearance, and a widget with any is taken off its stored stream and constructed from
`with_scripted`'s dictionary, as a retyped free text annotation is. The stored stream is the
producer's picture of properties the script has changed, and Table 191's `/MK` names construction
as what the entries are for. **A check box and a radio button are the exception**: §12.7.5.2.3
defines their states by appearance streams the value selects among, so constructing one stream
would destroy them; their properties are written to the file and their stored states drawn.

## 3. Saved where the standard keeps each entry

`ViewState::save` writes `/MK` and `/BS` on the widget and `/DA`, `/Q`, `/MaxLen` and `/Ff` on the
field — the nearest dictionary of the widget's chain that states §12.7.4.2's `/T` — after the values
and through `Update::current`, so a widget a person also typed into keeps that rewrite; and the
constructed appearance is written beside them. A field's other widgets inherit what the field now
states, as they inherited what it replaced.

## 4. What an assistive technology reads is what the field displays

`AccessibilityNode::value` was the field's characters while `value_lines` placed the characters of
the appearance, which is drawn through Table 199's `/F`: `12.5` beside the positions of `$12.50`.
`as_displayed` answers both of a page's node sources with what the field displays, and with what a
person typed while they are typing.

## 5. The editable combo box commits like a text field

Table 233 bit 19's text box holds characters until the commit exactly as an entry does, and the
field's `/K` in its commit form and `/V` wait for it. GTK's composed entry and Qt's `QComboBox`
line edit now commit on Enter and on losing the keyboard, as a text field's control does (ADR 1592);
`quorra` already treated a combo box as a single-line field.

## 6. Driven

`script_painted` in the three windows: a push-button whose open action sets `fillColor` to red is
drawn red at `--scripts on` (57 846 to 93 768 pixels, counted through the alpha channel) and its
file's grey at `off`.
