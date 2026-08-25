# ADR 0630 — The other half of the delegated click, and the button a set announced three times

Status: accepted, seven-hundred-and-thirty-fifth session.
Amends ADR 0623's refusal, which was the correct half of this and is now unnecessary for the case
it was written about. Amends ADR 0425's "one definition of a click for the mouse and for a client"
by making it one definition for **three windows** as well. Corrects `viewer_core::referenced_objects`
and `viewer-gtk`'s `write_back`.

## The question

ADR 0623 measured a silence and stopped it: a click on a §12.7 widget did nothing in a host that
delegates the widget's appearance, while `org.a11y.atspi.Action.DoAction` answered `true` — so an
assistive technology was told the click had worked. `viewer-ui` toggled; both native windows
toggled nothing. That round made the refusal explicit, by name, nine of nine. Building the half
that actually performs the click was left owed, with the note that it needs **no new message**.

## Why the refusal was right and is not enough

A person's click on a check box in `pdf-viewer-gtk` lands on a real `GtkCheckButton`, and in
`pdf-viewer-qt` on a real `QCheckBox`. §14.7.5.3's object reference names an *annotation*, and
`viewer_accessibility::Act::Click` resolves a node to a **point on the page** — which is under the
control rather than on it. So the synthetic press went past the widget to the page underneath and
changed nothing, correctly.

What follows from that is not that the click cannot be performed. It is that the *toolkit* is the
wrong route: the value belongs to the field, and `viewer_core::Edit::SetField` is how a field gets
one in every host already.

## What was found before any of it was written

**The rule for what a click on a button field means was written three times, and the three had
stopped agreeing.** `viewer-ui` decided it from a point (`App::toggle_button`, seventy lines),
`viewer-gtk` from a `GtkCheckButton`'s new state (`controls::toggle`) and `viewer-qt` from a
`QAbstractButton`'s (`Host::toggle_control`). Only the first consulted Table 227 bit 1 before
sending an edit; the other two relied on the control being insensitive, which is a fact about a
*person's* click and not about the two other ways one now arrives. That is `viewer-host`'s founding
sentence — the third copy of a decision is where two hosts stop agreeing about a document — and it
had already happened.

## The decision

**§12.7.5.2's rule is one function in `viewer-host`, reached by two doors, and matched
exhaustively in three windows.**

- `viewer_host::form::toggling` is the rule: one widget's flags and the state a click asked for,
  answering a closed [`Clicked`]. Table 227 bit 1 first; then Table 229 bit 15, whose own first
  three words make it a radio button's only; then §12.7.5.2.3's on-state name, which is the file's
  own invention and is reported rather than replaced when a widget states none.
- `viewer_host::form::clicked` is the walk that finds the widget under a point — `Query::FieldAt`
  for the model's own hit test, `Query::Fields` for the flags nothing in the first answer carries,
  and the **last** widget covering the point, because §12.5.2 draws them in `/Annots` order.
- Each host matches `Clicked` exhaustively, so a case added to it fails to compile in three places.
  That is the instrument shape `Tab`, `Key`, `Act`, `Supplied` and `Presenting` already use.

**No message was added, and none changed shape.** `Query::FieldAt` and `Query::Fields` answered
everything this needed, which is why `viewer-ui` had needed nothing added to do it in the first
place. This is the fifth round in a row on that boundary to add nothing at all
(`doc/ui-boundary.md`).

**The refusal is not regressed, it is narrowed to what is still true.** `Clicked::Aimed` is
§12.7.5.3's text field and §12.7.5.4's two controls, where a click asks for a caret at the point it
landed or for Table 234's options on the screen — and *that* a synthetic press at a page coordinate
genuinely cannot do to a `GtkEntry` or a `QComboBox`. `Clicked::note` takes one argument, whether
the caller places a real control, and it is the only answer that argument changes.

## Two defects the measurement found, and neither was the one being built

**A radio set announced every one of its buttons as selected.** `viewer_core::referenced_objects`
keys §14.7.5.3's map by the annotation and stored the **field's** `Control` under each of them —
and `pdf_model::form::Control::RadioButton`'s `on` is documented as "[w]hether any widget of the
set is on". So as soon as one button of a set was chosen, every `Form` element of that set reported
AT-SPI's `checked`. ISO 32000-2 §12.7.5.2.4 says "Like check boxes, individual radio buttons have
two states, on and off", and §12.7.5.2.3 makes the exclusion a `shall` where Table 229 bit 26 is
clear: "at most one radio button in a field shall be set at a time". A screen reader on a
three-button set heard three selected answers — trap 5 aimed at the one person for whom the picture
is no answer.

`pdf_model::form::Widget::on` is the per-widget fact and had been sitting beside the field's the
whole time, its own doc comment saying which is which: "[`Control::CheckBox`] carries the same fact
for the field as a whole; this is the per-widget answer a radio set needs." `viewer_core::this_widgets_control`
is one line and the fallback both native hosts already applied to their own buttons.

**And `viewer-gtk` never wrote a toggle back, where `viewer-qt` always did** — a parity defect
shipped since ADR 0244. `write_back` handled a `GtkEntry`, a `GtkTextView` and an editable combo
box, and a `GtkCheckButton`'s state was set when the control was *built* and never again. So a
value the field acquired any other way — an undo, an imported §12.7.8 data set, a click an
assistive technology asked for, or **the other button of a radio set going on** — left the picture
saying one thing and `/V` another. Two buttons of one set showed a tick together, which is the same
`shall` as above seen from the other side. Qt's `applyUpdates` had `button->setChecked(control.on)`
from the start.

**That second one is trap 1 in its purest form**, and it is why this round did not stop at the bus.
With the write-back removed and everything else in place, the AT-SPI client still reports six of
nine widgets toggling — the model really did change — and the GTK window's pixels do not move at
all. The metric was right and the page was wrong.

**And a refused click now puts the button back.** `connect_toggled` fires *after* GTK has flipped
the widget, so returning without an edit left a `GtkCheckButton` showing a state the field does not
hold — a radio button with `NoToggleToOff` unchecking itself on the screen while `/V` still named
it, which is the opposite of what Table 229 bit 15 requires.

## What was measured, and on what

`annotation-button-widget.pdf`, which `doc/verify.md` names as the witness for §14.8.4.7.2's
controls because it labels its own answers, on a real AT-SPI bus under `Xvfb` — a `dbus-python`
client walking from the registry root, calling `Action.DoAction(0)` on each of the nine nodes that
declare `click`, and reading `org.a11y.atspi.Accessible.GetState` back **after each one** rather
than after all nine.

That last detail matters and is where ADR 0623's "three of nine" came from: a batch of nine clicks
followed by one read measures the *net* of a walk in which a radio set's second click undoes its
first, which is a different question from the one being asked.

The document's nine widgets are three check boxes — one off, one on, one with Table 227 bit 1 — and
three radio button fields of two widgets each, two with `/Ff 49152` (Table 229 bits 15 and 16) and
one with `/Ff 49153`, which is those two bits and Table 227 bit 1. Against the file's own initial
state the clause makes **five of the nine toggle**: three are refused on Table 227 and the fourth is
Table 229 bit 15 on the one button of a set that is already on. Along a *sequential* walk the last
of those four becomes a toggle, because the click before it turned that button off.

| | clicks that gave a value | refused by name | `checked` changed |
|---|---|---|---|
| `pdf-viewer` | 6 of 9 | 3 (Table 227) | 6 of 9 |
| `pdf-viewer-gtk` | 6 of 9 | 3 (Table 227) | 6 of 9 |
| `pdf-viewer-qt` | 6 of 9 | 3 (Table 227) | 6 of 9 |

Identical in all three, line for line in each host's `--trace=access` log, where before this round
the two native hosts gave a value to none of the nine. The GTK window's own pixels were
photographed either side of the walk and every one of the nine controls agrees with the field it is
over; a person's mouse click on one radio button of a set now visibly unchecks its sibling.

**The briefing for this round asked for nine of nine and that is not obtainable, on the standard's
own instructions.** Three of the nine widgets belong to fields the document marks read-only, and
`CLAUDE.md` principle 5's whole subject is that the file decides. A host that toggled nine would be
disobeying the document; what parity means here is that all three windows refuse the same four for
the same printed reasons.

## What it costs, and what is left

The rule is one function called from five places and a `Clicked` matched exhaustively in five. The
seventy lines it replaced in `viewer-ui` are gone, `viewer-ui`'s private `covers` is
`viewer_host::covers`, and `viewer-qt`'s `Placement` carries §14.9.3's two names where it carried
one — so every sentence that host says about a field now names it the way the clause says a user
interface shall.

Left, and named rather than left silent:

- **A click on a text or choice field still cannot reach a delegated control**, and that is
  `Clicked::Aimed`'s refusal. What would close it is the host focusing its own widget, which is a
  toolkit call in GTK and a new C++ entry point in Qt — and it is the same item as
  AccessKit's `Action::Focus`, which `doc/todo/31` already carries as reaching
  `Bridge::requested` with `means: None`.
- **AT-SPI's `checked` on a `Form` element reads the tree rather than the page**, which is why both
  native windows were photographed either side of the walk as well: a `GtkCheckButton` and a
  `QCheckBox` show the same nine states, and the same three refusals leave the same three controls
  where the file put them. `pdf-viewer` draws the page's own appearance and is judged by the tree
  alone, which is the one asymmetry left in this measurement.
