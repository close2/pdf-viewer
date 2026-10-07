# 1604 — A toolkit's control shows a committed value as the field displays it, and the field's characters while it holds the keyboard

Session 1384. Status: **accepted** and **built**. Builds on ADR 1592 (the commit from every window),
whose section 5 named this as the remainder, and on ADR 1579 (`ViewState::displayed_value`).
Context: ISO 32000-2 §12.6.3 (Table 199's `/K` and `/F`), §12.7.4.3, §12.7.5.3 (Table 231 bit 13);
Adobe's *JavaScript for Acrobat API Reference*, the working source the owner named for event order.
Code: `crates/viewer-core/src/query.rs` (`FormField::displayed`), `crates/viewer-core/src/viewer.rs`
(`Viewer::form_fields`, `displayed`, `states_a_format`), `crates/pdf-model/src/view/scripts.rs`
(`ViewState::displayed_values`), `crates/viewer-confined/src/protocol/panels.rs` (the wire),
`crates/viewer-ffi/src/abi.rs` (`quorra_field_displayed`), `crates/viewer-gtk/src/controls.rs`
(`FieldChange::Holds`, `holds_on_entering`), `crates/viewer-gtk/src/host.rs` (`write_back`),
`crates/viewer-qt/src/bridge.rs` (`QtControl::displayed`), `crates/viewer-qt/cpp/window.cpp`
(`placeControls`, the `focusChanged` connection, `commitsWhenFinished`); for section 5,
`crates/viewer-core/src/command.rs` (`Command::Presented`), `crates/viewer-core/src/viewer.rs`
(`open_sequence`), the four windows beside `Command::Report`, `quorra_presented`. Instrument:
`crates/viewer-core/examples/fields_cost.rs`. Tests: `crates/viewer-core/tests/field_commit.rs`
(`a_committed_value_is_answered_as_the_field_displays_it`), `tests/headless.rs`,
`tests/open_sequence.rs`; drive step
`39-field-shown`.

## 1. The displayed value crosses beside the value

`Query::Fields` carried what a field *holds* and not what it *displays*, so the two toolkit windows,
which draw the page without the widget appearances under their own controls (ADR 0245), showed `12.5`
where `quorra` draws `$12.50`. `FormField::displayed` is `pdf-model`'s answer — Table 199's `/F`,
"performed before the field is formatted to display its value" — taken as `ViewState` gives it,
whatever it runs to make it: this window decides nothing about a format. It crosses the confined
wire beside the value and reaches a C host as `quorra_field_displayed`, so the four consumers of
`Query::Fields` stay level. It is `None` exactly where `value` is; a password field displays its echo.

## 2. Which string a control shows: the keyboard decides

A text field's control shows the field's characters while it holds the keyboard and `displayed` while
it does not — on open, after a commit, after the keyboard leaves it. Typing starts from the
characters because Table 199's `/K` checks what is typed against the field's own grammar:
`AFNumber_Keystroke` refuses a `$` or a thousands comma, so a person whose typing began from
`$12.50` would have every keystroke refused. The rule is the keyboard's rather than the commit's
because a control that holds the keyboard is where a caret is, and one toolkit signal — GTK's focus
controller, Qt's `focusChanged` — moves between the two strings. GTK and Qt select a control's whole
text as a Tab gives it the keyboard; the characters replacing the displayed string keep that
selection, so a person who tabs in and types replaces the value as the platform promises.

## 3. Enter hands the keyboard back to the page

Under section 2 a control committed by Enter would go on showing the characters until something else
took the keyboard. Adobe's reference lists Enter beside a click elsewhere and a tab among the ways a
value is committed as the field is left (`event.commitKey`), so Enter in a single-line control
commits (ADR 1592) and gives the keyboard to the page. The control losing the keyboard commits a
second time, which finds nothing typed and does nothing (ADR 1592 section 1).

## 4. What it costs, measured

The toolkit windows ask `Query::Fields` on every repaint. `displayed_value` walks §12.7.4.1's field
tree for the one field asked, so a column computed field by field grew with the square of the form:
`prefilled_f1040.pdf`'s 116 fields went from 0.6–0.9 ms to 24.5 ms, none of them formatted, and
`160F-2019.pdf`'s first page — the most formatted page of `doc/pdf.js`, 28 of 76 widgets — from 0.54
to 5.2 ms. Two changes took it out: a field stating no `/F` is not asked (`states_a_format`; it
displays its characters, which is the method's own answer), and the rest are asked together through
`ViewState::displayed_values`, which walks the tree once. Best of fifty after: the 116 fields 0.80 ms
against 0.65 without the column, `160F-2019.pdf` 0.41–0.85 against 0.54, and 120 fields each
stating `AFNumber_Format` 0.72 against 0.35. `displayed_values` is an additive edit to a file round
1383 owns this batch: `displayed_value` keeps its signature and its body, now read through the table.

## 5. The open sequence's call, from every window

ADR 1602 leaves `ViewState::run_open_scripts` for a host to call once page one is presented. It is
`Command::Presented`, which `viewer-core` answers by running the sequence for the focused document
once — a second message for the same document does nothing — and saying what it said as
`Event::Reported` with no page; the confined wire carries it to the worker, which holds the view, and
a C host has `quorra_presented`. Each of the four windows sends it beside `Command::Report`, under
`viewer_host::report::Due`'s rule: once per opened document, after the frame that put it on the
screen. That is ADR 1044's moment for the same reason — *when opened* is not *before the first
frame* — and one rule rather than a second means a document opened later in a running window gets
its sequence exactly as it gets its report. With no runner supplied the sequence runs nothing and
says, once, how many document-level scripts went unrun.

## 6. What is not done

The editable combo box's text keeps the value: it has no commit wiring of its own in either toolkit
(ADR 1592's `commits_on_enter` is the single-line entry's). `viewer-core`'s accessibility nodes carry
`value`, so a screen reader reading `quorra`'s own form node of an unfocused field hears `12.5`;
`displayed_value`'s doc comment names that reader as the one who wants the displayed string, and
`AccessibilityNode::value` taking it is the next change, in a file this round does not own.
