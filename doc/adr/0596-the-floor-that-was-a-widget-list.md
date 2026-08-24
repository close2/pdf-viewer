# ADR 0596 — The floor that was a widget list, and the half of a flag nobody read

Status: accepted, 2026-08-24. Session 717, the seventh round on the project owner's *"we should
start investing time into the UI (and its API for the native versions)"*, taking **item 7** of
ADR 0509's ordering — the one that file called "a real toolkit floor".

It is not one. What follows is what was actually tried, what reading the flag's second sentence
found in the host that was *ahead*, and the control a tier-2 host turned out never to have had.

No message was added and no variant changed shape, which is the twelfth consecutive round since the
six-hundred-and-seventh in which that has been true. `viewer-core` was not touched at all.

## 1. The item, and the rule it came with

ADR 0509 §3 ranked it last of seven and wrote the block down so that a later round would not spend
itself rediscovering it:

> **Table 233 bit 19's editable combo box in `viewer-gtk` — and this one is a real toolkit floor.**
> *"If set, the combo box shall include an editable text box as well as a drop-down list"*;
> `QComboBox::setEditable` obeys it and `GtkDropDown` is not editable, while `GtkComboBoxText` is
> deprecated in the release this crate binds. `viewer-gtk` carries the flag and reports it, which
> is the honest answer.

And with it, ADR 0508's rule: **before writing that an item is blocked on a toolkit, call the API
the block names.** That is the rule this round is an instance of, for the second time on this one
clause — ADR 0508 was Table 234's `/TI` in the same host.

## 2. The floor was read off the widget list, and the clause does not ask for a widget

Every sentence in the block above is true about *widgets*. `GtkDropDown` has no entry; `GtkComboBox`
and `GtkComboBoxText` are deprecated in GTK 4.10 and this workspace binds `v4_10` with warnings as
errors, so naming either would fail the build. There is no GTK 4 widget that is an editable combo
box.

What the clause asks for is not a widget. ISO 32000-2 §12.7.5.4, Table 233, bit 19: if the flag is
set the combo box **shall include an editable text box as well as a drop-down list**. Two things,
and the clause names them separately — as §12.7.5.4's own prose does one paragraph up, where "[t]he
combo box may be accompanied by an editable text box in which the user can type a value other than
the predefined choices, as directed by the value of the Edit bit in the Ff entry".

A `gtk4::Entry` beside a `gtk4::MenuButton` whose popover holds a `gtk4::ListBox`, in one
`gtk4::Box` carrying GTK's own `linked` style class, is an editable text box and a drop-down list.
Nothing in it is deprecated, and **the `v4_10` feature floor did not move** — which matters, because
what a feature floor costs is a *runtime* requirement on everybody who installs this program, and
ADR 0508 raised one, found the binding could not express what was wanted, and put it back.

The general shape is worth more than the instance and it is now in `doc/traps/`: **a toolkit that
will not hand over a *widget* has usually not withheld the *capability*.** A block written from a
widget list is a claim about a catalogue, not about what the platform can do.

## 3. The half of the flag nobody had read, and it was the host that was ahead

The bit has a second clause: **if clear, it shall include only a drop-down list.** That is a `shall`
in the other direction, and this tree disobeyed it — not in `viewer-gtk`, which builds a drop-down
either way and was therefore accidentally right, but in `viewer-ui`, the host ADR 0509 calls "ahead
on everything that reads a document".

The mechanism is exact and it is nobody's mistake twice over:

- `Answer::Field`'s value is `Some` for a combo box whether or not the flag is set, and that is
  **correct**: the value *is* a text string, §12.7.4.3 lays it out, and a host has to be told what
  the field says in order to show it. `pdf_model::appearance::field_text_value` answers for
  `FieldKind::Text` and `FieldKind::Choice { combo: true }` alike.
- `viewer-ui` read *has a text value* as *takes typed characters*. Its own doc comment says so —
  "§12.7.5.1's four field types are not equal here … and the *core* is what draws that line".

So a person could put the caret in a drop-down whose `/Opt` states Red and Blue and type *Purple*,
and `Edit::SetField` wrote it. The file then holds a `/V` that §12.7.5.4 says is "a text string
representing the selected item, as given in the field dictionary's Opt array" and which is not one
of them.

**This is Table 229 bit 26's shape one flag over** (ADR 0346): a bit whose *set* half was being
reported as unimplemented while its *clear* half was being broken in silence, because a flag reads
as a permission and half of these are prohibitions.

## 4. Where the sentence lives now, and why not in the core

`viewer_host::form::ControlKind::takes_typed_characters` is the one statement, matched exhaustively
over the enumeration so that a ninth control kind fails to compile rather than falling into
whichever arm it lands in — `Key::ALL`'s and `Tab::ALL`'s mechanism (ADRs 0526, 0564) applied to the
third thing a host builds.

**It is a host's question and not the core's**, and the division is the one this tree already draws:

- A host that places somebody else's widgets obeys the flag by *choosing the widget*. A `GtkEntry`
  takes characters and a `GtkDropDown` has nowhere to put one; `QComboBox::setEditable` is one
  property. Both native hosts obey by construction and always did in the clear direction.
- A host that draws the page's own appearance has no widget to be constrained by, so it has to ask.
- A C caller places its own toolkit and already has the flag — `PDFV_FIELD_FLAG_EDITABLE` since
  ADR 0346 — so nothing was owed there.

Putting the refusal in `viewer-core` was considered and declined. `ViewState::set_field` already
drops an out-of-range index and cuts a multiple selection to one where Table 233 bit 22 is clear, so
there is a precedent for the core enforcing a choice field's flags — but those two are about a
*value* the clause defines, and bit 19 is about what the reader is **shown**. A core that refused
the edit would leave a host free to draw a text box the clause forbids and then find its keystrokes
silently discarded, which is a worse failure than the one being fixed. The three hosts differ in
what they draw, so they are what have to obey.

## 5. The tier-2 host could not choose an option at all, in either control

Reading the clause for bit 19 found the larger gap. `viewer-ui` sends no `Command::Delegate`, so no
`GtkDropDown` and no `QComboBox` is ever placed over its page; `Entered::Chosen` — the variant
`Edit::SetField` grew in the four-hundred-and-twelfth session precisely so that §12.7.5.4 could say
which options are selected (ADR 0248) — **occurs nowhere in `viewer-ui`**. Both of the clause's
controls were unusable there: a list box drew its options (ADR 0407) and no press could select one,
and a combo box could only be given a value by typing a label, which is the violation in §3.

So the refusal in §3 had to come with the control it refuses on behalf of, or the round would have
closed a clause violation by taking a capability away.

`viewer_ui::chrome::ChoiceList` is that control: Table 234's options listed under (or over) the
widget in the window's own device pixels, the selection marked in this interface's own colour, and a
press on a row sending `Entered::Chosen`. Three things about it are decisions rather than drawing:

- **The geometry is computed once and used twice.** The same value answers *where the rows are* for
  the drawing and *which option a point is on* for the press. Two derivations of one layout is how a
  control comes to show one row and act on another, and no gate in this tree rasterises chrome.
- **Where it starts is the clause's for one control and not the other.** Table 234's `/TI` is "the
  index in the Opt array of the first option visible in the list" and it says "[f]or scrollable list
  boxes"; a drop-down states nothing, so it opens at its own value. A list that opened at row 0 with
  the selection forty rows below would be showing the document's data and hiding its answer.
- **The mark over the selection is this host's and the options are the clause's**, which is ADR
  0407's division held rather than restated: §12.7.5.4 requires the options be "displayed on the
  screen" and states no highlight at all, so the highlight is drawn in the interface's colour
  exactly as a text selection is, and never in a way that takes the row down with it (ADR 0106).

Table 233 bit 22 comes with it, because a list box that can be selected in is a list box that can be
multiply selected in: a press adds or removes one row and the list stays up, and the indices are
sorted, which is what Table 234's `/I` requires of them.

## 6. What was run, and what only the screen could say

The two new tests were each run against an injected defect before being believed (trap 13):
`takes_typed_characters` returning true for every combo box, and the drawn rows offset from the rows
the layout reports. Both failed as intended and both pass with the defects removed.

Driven under `Xvfb` on `doc/pdf.js/test/pdfs/issue17492.pdf`, whose first page states an editable
combo box (`country`, 28 options, `/V` naming *Spain*) beside a non-editable one — so one screenshot
shows both halves of the flag at once. `doc/history/717-*.md` carries what each host showed.

## 7. What this does not do

It adds no message, no `Query` and no C entry point. It does not give `viewer-ui`'s list a keyboard:
Up, Down and Enter would be this host's convention and no clause states one, and Escape closing it is
here only because a control that can be opened must be closable without picking something. It does
not touch `pdf-model`, so no page draws differently and no gate that rasterises one could see this
round at all.
