# ADR 0235 — A form is a description, and a check box is a name the file invented

Status: accepted, 2026-08-08 (session 398).

## Context

`doc/todo/37` audited what a native host would not draw itself and found six populations of
interactive chrome crossing `viewer-core`'s boundary. **Five crossed as data and one did not.**

| what `viewer-ui` draws | what a native host uses instead | crossed as, before this round |
|---|---|---|
| §12.5.6.14's popup window | `NSPopover`, `QToolTip`, `GtkPopover` | `Query::Popups` |
| §12.3.3's outline, §8.11.4.3's layers, §7.11.4's files, §12.3.5's collection, §12.4.3's articles, §14.3.3's `/Info` | `QTreeView`, `NSOutlineView` | six queries, all data |
| §12.3.4's thumbnails | an icon view | `Query::Thumbnail` |
| a text selection | the platform's highlight brush | `Query::Selection` |
| §12.5.1's focus ring | the platform's focus ring | `Query::Focus` |
| **§12.7's form fields** | a real `QLineEdit` / `NSTextField` / `GtkEntry` | **nothing** |

A form field has the strongest claim of the six, because a text field *is* a `QLineEdit` — and it
was the one a host could not have. What crossed was addressed to a *point*: `Query::FieldAt` names
the field under a click, `Query::Caret`, `Query::Offset` and `Query::FieldSelection` measure inside
its value (ADRs 0201, 0211, 0225), and `Edit::SetField` puts a string in. A host placing controls
does not have a point. It has a page, and it needs every field on it before anybody clicks.

**And the audit understated the gap.** Reading §12.7.5.2.3 against the code found that a check box
could not be checked at all, by any host, on either drawing path — see below. So the boundary was
missing a *description* and the model was missing the operation the description would have been
used for.

## What was written down first, because this project's commonest finding is a blocker that expired

| what a host needs | crossed before | crosses now |
|---|---|---|
| the field's fully qualified name | `Answer::Field` | `FormField::name` |
| Table 226's `/TU` | `Answer::Field` | `FormField::name` |
| Table 226's `/T` alone | no | `FormField::partial` |
| its text value, through §12.7.5.3's truncation | `Answer::Field` | `FormField::value` |
| a caret, an offset, a selection inside the value | three queries | unchanged |
| **the enumeration of a page's fields** | no | `Query::Fields` |
| **§12.7.5's type** | no | `FormField::control` |
| **Tables 227, 229, 231, 233's flags** | no | `Control`'s arms |
| **Table 232's `/MaxLen`, and a comb's cell count** | no | `TextControl` |
| **Table 234's `/Opt`, `/TI`, `/I`** | no | `ChoiceControl` |
| **a check box's on-state name** | no | `FormWidget::on_state` |
| **Table 230's export value** | no | `FormWidget::export` |
| **each widget's rectangle** | no | `FormWidget::quad` |
| Table 227 bit 1's `ReadOnly` | no | `FormField::read_only` |

## Decision

### One question, answering a description rather than a picture

```
Query::Fields → Answer::Fields(Vec<FormField>)
```

`pdf_model::form::fields(document, page, view)` reads the description; `viewer_core` maps each
widget's `/Rect` through the one arithmetic every shape it hands over goes through. That is the same
split `Query::Popups` makes — `pdf_model::popup::Popup` in default user space,
`viewer_core::PopupWindow` in device pixels — and it is why the crate that reads documents holds no
opinion about pixels.

**What crosses is what decides a *control*, not what decides a mark.** Table 231's Multiline,
Password, Comb and DoNotScroll are applied here already and still are; DoNotSpellCheck, FileSelect
and RichText are *carried*, because each constrains a widget somebody else builds. Table 233 bit
20's `Sort` is deliberately **not** carried, on the table's own instruction — "PDF readers shall
display the options in the order in which they occur in the Opt array" — so handing a host the bit
would be inviting it to break a `shall`.

**One entry per field, with a list of widgets under it**, because §12.7.4.1 makes the value the
field's and lets one field own several rectangles. A host places one control per widget and sends one
edit per field, which is the shape `Edit::SetField` already had.

### §12.7.5.2.3's two sentences, and the one this tree had never read

The clause states both:

> The value of the V key shall also be the value of the AS key. If they are not equal, then the
> value of the AS key shall be used instead of the V key to determine which appearance to use.

This tree obeyed the **second** and not the first. `annotation::stored_appearance` read `/AS` and
never `/V`, which is right for a file as written — and wrong the moment the *reader* is what changed
`/V`. The first sentence is an invariant, and a processor that changes one entry is the one that has
to carry the other.

So: a person checked a box, `/V` became `Yes`, `/AS` stayed `Off`, and the widget went on drawing the
state it was saved in. Measured on `issue17492.pdf`: **527 display-list commands before the edit and
527 after**, against 528 with the fix. The constructed path was wrong too, and differently —
`Field::is_on` compared the replaced value against `Off` as a *name*, and `ViewState::set_field`
encodes what a host sends as §7.9.2.2's text string, so the comparison never matched and every edited
check box read as off.

`appearance::appearance_state` is the answer, and it is narrow on purpose:

- **`None` means the file's `/AS` decides**, which is every annotation in a document nothing has
  been done to and every widget that is not one of §12.7.5.2's two toggling kinds. A text field's
  value is laid out rather than selected among; a push-button has none (§12.7.5.2.2).
- **A name or a text string**, because Table 230 spells a button's export values as text strings for
  the same reason a host has to send one.
- **A widget whose `/AP /N` states no stream under that name is off.** That is §12.7.5.2.4 read
  forwards: "[t]he parent field's V entry holds a name object corresponding to the appearance state
  of whichever child field is currently in the on state", so a radio button the value does not name
  is one of the others and draws its `Off` state rather than nothing. Applied only where the file
  states a state subdictionary to check against.

**Why the two halves are one round.** A host that could enumerate a form but not check a box would
know what to send and never see it work; one that could check a box but not enumerate the form could
see it work and never learn the name to send. Neither half is a feature on its own.

### Why the on-state name and Table 230's export value are two fields

They are two strings, and the clause says why:

> When this entry is present, the names used to represent the on state in the AP dictionary of each
> annotation may use numerical position (starting with 0) of the annotation in the Kids array,
> encoded as a name object (for example: /0, /1).

So `/AP` may say `/0` where `/Opt` says `Rot`. Only the first selects an appearance; only the second
is worth showing a person or exporting. A single field would have had to choose, and whichever it
chose the other would be lost at the caller — the same argument ADR 0167 made for a field's two
names.

### A choice field's selection comes from the value, and `/I` breaks a tie

§12.7.5.4 is explicit about the precedence: "[i]f the items identified by this entry differ from
those in the V entry of the field dictionary ... the V entry shall be used." So the value decides,
matched against Table 234's labels — "the name string is the second of the two array elements" —
and `/I` is consulted only for the case the clause states it for, two options carrying the same
label. Answering from `/I` alone would have been the easier code and the wrong clause.

## Consequences

- **Three consumers, which is what keeps the vocabulary from going stale** (ADR 0178).
  `viewer-core/tests/headless.rs` enumerates a real form and checks a box with the name the page
  gave it; `viewer-confined`'s transport carries the answer as its twelfth panel encoding, compared
  field for field against the same document read unconfined; and **`viewer-ui` toggles a check box
  and a radio button from a click**, which it could not do before because it had no name to send.
  Table 229 bit 15's `NoToggleToOff` is what decides whether a click on a button already on turns it
  off, and that is now a host's to obey.
- **The confined host has no less than the unconfined one.** `protocol::panels` destructures every
  field of `FormField`, `FormWidget`, `Control`, `TextControl`, `ChoiceControl` and `Choice` with no
  `..`, so a field added later fails to compile rather than stopping crossing — the property that
  module was written to hold, extended to six more types.
- **Driven in the real window**, which is where the last four defects of this shape were found (ADR
  0126). Under `Xvfb` with `lavapipe`, a click at `[57.7 499.8 69.0 510.7]`'s own centre — device
  `(122, 400)` for a 595×842 page in an 800×1000 window — printed `note: setting the field
  typeScript to Yes` and `note: this document has unsaved changes`, and a second click printed
  `note: setting the field typeScript to Off`. Neither headless test exercises the loop from a
  pointer, which is the gap that ADR exists for.
- **Nothing about the render path changed.** The corpus's 67 incomplete documents, the oracle's 904
  agreeing and 69 contradicted pages, quorra's 916 agreeing and the text gate's 99.2% are all
  unchanged, which is what the narrowing above is for: no gate sets a field's value, so no gate can
  see `appearance_state` at all.
- **`RadiosInUnison` crosses and is not obeyed**, and the division is deliberate: turning on every
  button of a set that shares an on state is a decision for whatever handles the press, and this
  tree's own host has the flag rather than the behaviour. Named in `doc/todo/37` rather than left to
  be discovered.

## What this leaves open, and it is written in `doc/todo/37`

**A page rendered without its widget appearances.** A host placing native controls over the page
gets both pictures unless it can ask for one without them. That is a second decision and a larger
one: §12.5.5's appearance streams are page content, and leaving them out changes `interpret`, on
which the oracle’s 1794-page comparison rests. It is a departure a host asks for rather than one
this crate takes, and it wants its own round and its own argument.

**§12.7.5.4's list box still draws nothing.** The clause states which items are selected and states
no highlight, which is why `variable_text` refuses it and says so. A host with the items and the
selection can now draw a real list — which is the point — but the *page* is still light where one
sits, and the report is what says so.
