# Q974 — May audit 2148's B04 test select the second button by its widget?

Source: round 2152, fixing audit 2148 (`doc/reviews/2148-specification-audit-12-7.md`, finding
2148-B04). ADR 3311.

## The two tests

Both open the same fixture (a radio set, `RadiosInUnison` clear, two buttons whose on state is `Yes`)
and make the same call on a fresh view, `view.set_field(&document, "r", &Entered::Text("Yes"))`:

- `a_saved_radio_set_keeps_at_most_one_button_on_spec_audit` (B03) holds as its control that the view
  then shows `[true, false]` — the first button on;
- `selecting_the_second_of_two_same_named_buttons_turns_it_on_spec_audit` (B04) expects `[false, true]`.

No implementation can pass both: the call is the same and carries nothing that names the second
button. The finding itself says why — "a host sends a button's `on_state` to `set_field`, which takes
no widget" — and its suggested fix is "an edit that can name the widget a person selected".

## What this tree does now

`Entered::Selected(widget)` is that edit (ADR 3311): selecting the second button by its widget shows it
on, alone, and the save writes it so (`forms_values_read_back::the_button_selected_is_the_one_shown_and_saved`).
The name alone keeps the first-kid rule the B03 control pins. The B04 test stays failing and stays in
`expected-failures-current.json`.

## The question

May the B04 test send `Entered::Selected(second.annotation)` instead of `Entered::Text(state)`,
keeping its assertions? **Recommendation: yes** — it then tests the finding's own fix, and B03's
control stays as it is.
