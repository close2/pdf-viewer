# 1566 — A click on a field a control stands over gives that control the keyboard

Session 1365. Status: **accepted**. Amends ADR 0630's refusal of `Clicked::Aimed` in the two native
windows; closes the item `doc/todo/31` listed as "the same item as `Action::Focus`".
Context: ADRs 0623, 0630, 1357, 1369; `crates/viewer-host/src/form.rs`; ISO 32000-2 §12.7.5.3,
§12.7.5.4.

## The question

An assistive technology clicks a form field by asking the node for its `click` action, and
`viewer_accessibility::Act::Click` turns that into a point on the page. `quorra` draws its own
fields, so a click at that point puts a caret in a text field or opens a choice's options there.
`quorra-gtk` and `quorra-qt` place a real `GtkEntry`, `QLineEdit` or list over each such widget, and
a synthetic press at a page coordinate goes under the control. So both refused the click by name
(`Clicked::note(true)`), and a screen reader user could not type into any text field in two of the
three windows. The drive confirmed it: both saved an empty `/V` after a click and a typed `z`.

## Decision

1. **`Clicked::Aimed` carries the widget annotation under the point**, found by the same
   last-covering-widget rule the toggling kinds use. A field with several widgets is aimed at the
   one under the point.
2. **The native windows give the keyboard to the control placed over that annotation.** This is the
   move §12.5.1's tab walk already ends in (ADR 1357). GTK calls `grab_focus` from an idle. Qt keeps
   the target and C++ asks for it after the drain (`aimed_control`) and calls `setFocus`, because
   Rust never calls a Qt object. A widget with no control placed over it is said by name.
3. **`Clicked::note` loses its `placed` parameter.** `Aimed` was the only answer it changed, and no
   window refuses `Aimed` now.

What happens next in the control is the toolkit's. GTK's entry selects its text when it takes the
keyboard, so a typed character replaces the value. Qt's does not, so it is inserted. Neither native
window opens a combo box's list on the click; the toolkit's own key opens it. `quorra` opens its own
list, because it draws one.

## What the standard and the platform settle

- §12.7.5.3 and §12.7.5.4 say what a text field and a choice field hold, and say nothing about which
  widget toolkit holds the caret. The point of the work is that the same click reaches the same
  field in every window.
- **`Action::SetValue` on a text field cannot arrive over AT-SPI.** `doc/todo/31` named it as the
  next action to take. `accesskit_atspi_common` 0.19.1 implements no `EditableText` interface and
  raises `SetValue` only from the `Value` interface, with `ActionData::NumericValue`. A text field's
  value is not a number, so nothing would ever send this. Typing after a click is how a client
  enters text here.

## What it was driven against

`tools/drive-windows.sh` step `33-aimed-field` performs the `click` action of a text field's node
in the document's tree, types `z`, saves, and reads `/V` from the saved file. It does this for two
fields: the empty `A` of the form fixture and `N` of the field fixture, which holds `123`. All three
windows now save `A` as `z`. `N` is `z` in `quorra-gtk` and `123z` in `quorra` and `quorra-qt`.
