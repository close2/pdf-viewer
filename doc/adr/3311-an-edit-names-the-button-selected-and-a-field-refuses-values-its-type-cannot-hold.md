# 3311 — An edit names the button selected, and a field refuses values its type cannot hold

Session 2152. Status: accepted. Context: audit 2148-B04, B05, B06; ADR 0248; `doc/questions/Q974`.

## Decision

1. **`Entered::Selected(widget)`**: a person selects a check box or radio button by its widget
   annotation, not by its on-state name. §12.7.5.2.4's "selecting any of the buttons automatically
   deselects all the others" is about the button selected; two buttons sharing a name cannot be told
   apart by the name, and the widget can. `ViewState::set_field` makes the value that widget's
   on-state name and records, per widget, `on`: true for the widget named, and for the buttons sharing
   its name where the set is not exclusive (Table 229 bit 26 set, or check boxes); false for the rest.
   `appearance::Field` reads it before the first-kid rule, which stays the rule for a value that names
   no widget (a file's `/V`, an import, a host sending the name). A widget that is not one of the
   field's toggling buttons is no edit. `viewer-confined`'s protocol carries it as tag 3.
2. **`set_field` refuses** a push-button any value (§12.7.5.2.2: it "shall not use the V and DV
   entries"), and typed text into a list box or a combo box whose Table 233 bit 19 Edit is clear
   unless the text is one of its own labels (Edit clear "shall include only a drop-down list").

## Not done here

`viewer_host::clicked` still answers `Clicked::Toggles { name, value }`, so the hosts send the name.
Carrying the widget changes that variant's shape, which every host destructures, among them the
quorra window this round could not touch; the next round on the hosts makes `Toggles` carry the
annotation and sends `Selected`. The audit test `selecting_the_second_of_two_same_named_buttons_turns_it_on_spec_audit`
sends the name and expects the second button, while its sibling B03 test sends the same name to the
same fixture and expects the first: both cannot pass, and Q974 asks whether it may send the widget.
