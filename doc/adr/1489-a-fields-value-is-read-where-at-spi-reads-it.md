# 1489 — A field's value is read where AT-SPI reads it

Session 1327. Status: **accepted**.
Context: `viewer_core::AccessibilityNode`, `viewer-confined`'s `protocol/panels.rs`,
`viewer_accessibility::tree`, `tools/drive-windows.sh` step `24-reopened`. Amends ADR 1478 section 2
(the drive's one golden, which this retires) and builds on ADRs 1369, 1381 and 1394 (which publish a
field as the control its §12.7.5 type is, named by `/TU` or its §12.7.4.2 name).

## 1. What was missing, measured against the tree rather than the brief

A check box already crossed with its state (`Control::CheckBox { on }`, published as `toggled`, which
AT-SPI reads as `checked`), and a choice field with its options and the indices selected
(`ChoiceControl::options`, `selected`). What crossed with nothing was a **text field's text**: a
`TextInput` node with its §14.9.3 name and no contents, so a screen reader announced a filled field as
empty. §12.7.4.3's value is a string neither `Control` variant says.

## 2. The node gains one field, in the shape `Answer::Fields` gives it

`AccessibilityNode::value: Option<pdf_model::view::ShownValue>` — the same type and the same reading
(`pdf_model::form::fields` with the view's state) as `FormField::value`, so a field typed into answers
with what was typed, and Table 231 bit 14's password answers with its echo and `obscured`. Filled for a
`Form` element through its §14.7.5.3 reference and for an unreached widget (ADRs 1369, 1381). It
crosses the confined pipe through `encode_shown`, the one encoder `Answer::Field` and `Fields` already
share, after the node's control; the greeting moved `PDFVCF05` → `PDFVCF06`, because an older worker
would send a node the host misreads. No message was added.

## 3. The mapping, decided by what the adapter exposes

`accesskit_atspi_common` 0.19 gives `org.a11y.atspi.Value` only to a node with a *numeric* value and
`org.a11y.atspi.Text` only where `accesskit_consumer::Node::supports_text_ranges` holds: a text input
with `TextRun` children. A string `value` on the control reaches no AT-SPI interface. So:

| field | published as | AT-SPI reads it through |
|---|---|---|
| text field, editable combo box | one `TextRun` below the control holding the text, and the same string as the control's `value` | `Text` (`GetText`, `GetCharacterCount`) |
| choice field | each `/Opt` entry a `ListBoxOption`, `selected` where `ChoiceControl::selected` says; a combo box's shown text also its `value` | `Selection` (`GetSelectedChild`) |
| check box, radio button | unchanged: `toggled` | the `checked` state |

An empty field still has a run, of no characters: "empty" is a statement and a missing interface is
not one. The run states the widget's rectangle and **no character positions** — the characters are
laid out in an appearance stream, not read back from a content stream — so `GetCharacterExtents` on a
field answers nothing rather than a guess (`doc/todo/31`).

## 4. Two consequences, accepted

`accesskit_consumer` gathers a page's text from every run below the page node, so the page's `Text`
interface now includes each field's contents where its control sits, as a sighted reader sees them
drawn; and an untagged page with a filled field now declares `SetTextSelection`, which is decided by
whether a `TextRun` is among the page's nodes rather than by an identifier range. The runs and items
below a widget take identifiers from the page's own count, continued across the widgets' band, so
none meets an identifier a line's run took
(`crates/viewer-accessibility/tests/tree.rs`, `a_fields_run_takes_no_identifier_a_lines_run_took`).

## 5. The census and the drive

`accessibility_census` counts "fields with a value published", a floor new with this decision; every
one of them crossed with no value before it. `24-reopened` reads `quorra`'s own form nodes off the bus
— `Text` for A, B, C, `checked` for D, the selected item for E — exactly as it reads the toolkits'
widgets, and the golden, its `--goldens` and `--regolden` options and `golden()` are gone: a picture a
person looked at once is not needed where the window states the value.
