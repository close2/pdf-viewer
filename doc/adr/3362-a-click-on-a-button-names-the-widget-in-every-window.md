# 3362 — A click on a button names the widget, in every window

Session 2165. Status: accepted and built, on the project owner's answer to `doc/questions/Q974`
(`doc/questions/A974`, 2026-11-20: "yes" — audit 2148's B04 test sends `Entered::Selected` of the
second button and keeps its assertions; B03's control stays `Entered::Text("Yes")`). Context: ADR
3311, which gave the model `Entered::Selected` and left the hosts sending the name; audit 2148's
finding B04; ISO 32000-2 §12.7.5.2.3, §12.7.5.2.4.

## Decision

1. **The audit test names the widget.** `forms_values_spec_audit::selecting_the_second_of_two_
   same_named_buttons_turns_it_on_spec_audit` sends `Entered::Selected(second.annotation)`; every
   assertion stays, and its doc comment now says what a host sends. It passes on ADR 3311's model,
   and B04 leaves `expected-failures-current.json`; 2148's `findings.json` marks it fixed.
2. **`Clicked::Toggles` carries the edit.** `viewer_host::toggling` takes the widget the click
   reached, and `Toggles` gains `entered`: `Entered::Selected(widget)` when a button goes on, and
   `Entered::Text("Off")` when one goes off — §12.7.5.2.3's off state belongs to no widget, so the
   name is the whole of it. `value` stays the state's name, for the sentence a window says.
3. **Every window sends `entered`.** `viewer_host::clicked` (the pointer path of `quorra` and the
   accessibility click of GTK and Qt), GTK's `GtkCheckButton`, Qt's `QAbstractButton` and the C
   ABI's `toggle_field` all reach `toggling` and all dispatch `Edit::SetField` with `entered`, so
   §12.7.5.2.4's "selecting any of the buttons automatically deselects all the others" is about the
   button a person selected in all of them. `viewer-confined`'s wire already carries `Selected`
   (ADR 3311).

## The test

`viewer-host`'s `host_mappings::a_click_on_the_second_of_two_same_named_radio_buttons_turns_on_that_button`
writes two radio buttons sharing `/Yes` with `RadiosInUnison` clear, clicks the second's middle
through `viewer_host::clicked`, asserts the edit names that widget, sends it, and reads back
`Query::Fields`: the second is on and the first is off. Sending the name instead, as before, turns
the first on, and the assertion fails.
