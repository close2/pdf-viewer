# ADR 0338 — An object reference has two answers, and a `Form` is a control

Status: accepted, 2026-08-14. Session 503. Amends §12.5.2's, §14.7.5.3's, §14.8.4.7.2's and
§14.8.5.4.3's ledger rows. Extends ADR 0214's bridge and ADR 0301's placement; changes nothing
either decided.

## The question

`doc/todo/31` carried two of its remaining entries in two places, and they turn out to be one
missing link.

- **"An element that marks no text and states no `/BBox` still has no place."** ADR 0301 read Table
  379's rectangle and it answered for the elements that state one; the rest reach an assistive
  technology with no extent at all, and a magnifier with nothing to point at. The entry named the
  strongest remaining route itself: "576 of them reach the page only through §14.7.5.3's `/OBJR`,
  and … an object reference names an annotation, §12.5.2's `/Rect` says where an annotation is,
  and `AccessibilityNode` does not carry the object".
- **"A `Form` element's control role."** ADR 0214 mapped §14.8.4.7.2's `Form` to
  `accesskit::Role::Group` and wrote down why that was wrong: the type is one widget annotation, so
  a screen reader is told there is something on the page rather than that it is a check box. The
  route it named is the same one — "the annotation behind §14.7.5.3's `/OBJR`".

Both were blocked on one sentence: `viewer_core::AccessibilityNode` did not carry the object an
element's content item *is*.

## What the standard states

**§14.7.5.3** makes an object reference the whole of an element's content:

> When a structure element's content consists of an entire PDF object, such as an XObject directly
> or indirectly referenced by a page description or an annotation, the object shall be identified
> in the structure element's K entry by an object reference dictionary

That sentence names two kinds of object and only one of them has a place of its own. **§12.5.2**,
Table 166, makes `/Rect` *(Required)* and

> defining the location of the annotation on the page in default user space units

so an element whose content is an annotation has been placed by the document. An `XObject`'s place
is the transformation matrix in force at the `Do` that painted it, which is in the content stream
and not in the object — and Table 358's NOTE 2 says the same thing from the producer's side: an
object rendered "multiple times on the same page" needs only "a single object reference", so the
reference cannot be naming one of the places. That half is left unanswered rather than guessed.

**§14.8.4.7.2**, Table 368, says what a `Form` is:

> Either an association between content enclosed by the Form structure element and a corresponding
> widget annotation or a mechanism to include a widget annotation in the structure tree. In a
> tagged PDF, Form shall be used for each PDF widget annotation that belongs to the real content of
> the document.

(**The first sentence of that quotation is struck out**, and this round did not know it: Errata
Collection 3's Issue #437 replaces it with "Encloses a PDF widget annotation and associated content,
if any". Nothing here changes — the reading is the same one, and the replacement states it more
plainly — but the quotation above is the *pre-errata* text and is left as this session wrote it.
Found and corrected everywhere else in the five-hundred-and-ninetieth session; ADR 0425.)

One widget annotation, one per widget. The neighbouring `Annot` row is the same sentence in the
negative — "Annot shall not be used for link annotations (see the Link structure element) or widget
annotations (see the Form structure element)" — so the standard has divided the annotations between
three structure types and `Form` is the one that means *control*. What control it is, is §12.7.5's
question, and this tree has answered it since ADR 0245 for a host building native widgets.

## The population, counted before anything was built

`crates/pdf-model/examples/element_bounds_census.rs` gained two counts:

```sh
cargo run --release -p pdf-model --example element_bounds_census -- \
  $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') $(find doc/corpora -name '*.pdf') doc/*.pdf
```

| | |
|---|---|
| documents read / with a structure tree | 1245 / 151 |
| structure elements | 166 115 |
| elements whose content items produced **no text** | 2079 |
| of those, stating a Table 379 `/BBox` | 404 |
| of the remaining 1675, **placed by §12.5.2's annotation rectangle** | **333** |
| `Form` elements | **272** |
| of those, naming a widget whose §12.7.5 field type is readable | **272** |
| by field type | `Tx` 218, `Btn` check box 43, `Btn` radio 6, `Ch` 5 |

Two things in that table decided the shape of the change. **Every `Form` element in the corpus
names a widget the field tree reaches** — not most of them, all 272 — so the entry ADR 0214 left
open was not a long tail. And **all 272 of them are in the placeless population**, which is the
whole of `Form` in the by-role list: a widget annotation marks no text by nature, so the two todo
entries were describing the same elements from opposite ends.

The denominator is larger than ADR 0301's (1080 documents, 1700 placeless, 61 rescued) because
`doc/corpora`'s four submodules are checked out here and were partly not there; the ratio it
matters for — placeless against placed — is taken within one run.

## Decision 1 — the element carries the objects its own content items name

`viewer_core::accessibility::Gathered` gains `objects`, filled in the `Child::Object` arm of the
walk from **the element's own** object references and not its descendants', which is the division
`own` already makes for marked-content identifiers and is sharper here: both answers are statements
about the object that *is* this element's content item. An ancestor's extent is a different
question and the standard states no union for it.

The page's answers are read **once per page and only where an element states an object reference**,
which is nearly no page: `pdf_model::structure::annotation_rectangles` walks `/Annots` and
`pdf_model::form::fields` walks §12.7.4.1's field tree, and neither is worth paying for an answer
nothing will look at. `Viewer::referenced_objects` is that gate.

`annotation_rectangles` answers only for the objects the page's own `/Annots` array names, and that
is a reading rather than a convenience. Table 166 makes `/Type` *optional*, so a `/Rect` read off
whatever dictionary an `/Obj` happened to point at would place an element from a rectangle the
standard never promised — which is exactly the shape ADR 0215 paid for, when a signature dictionary
stating no `/Type` was read as not being one. Membership in `/Annots` is the one available check
that the object is an annotation, and of *this* page.

## Decision 2 — `bounds` widens rather than a second field appearing beside it

`AccessibilityNode::bounds` already meant "where the document says the element is, in the
viewport's pixels", and both clauses state their rectangle in default user space, so both go
through `Viewer::device_rect` — §7.7.3.3's `/Rotate`, the crop box's origin, the y flip, and
ADR 0301's intersection with the page. A host needs no change to benefit, which is the point: the
333 elements got a place on the bus without `viewer-accessibility` moving.

**Table 379's rectangle wins where both exist**, because it is a statement about the *element* and
the annotation's is a statement about its content. Where an element names several annotations —
which Table 368's `Annot` permits, requiring only that "they shall be of the same annotation type"
— the union is what crosses, because a magnifier pointed at one of several would be pointed at the
wrong one as often as not.

## Decision 3 — the control crosses as `pdf_model::form::Control`, and the platform mapping stays a host's

`AccessibilityNode::control` is the same type `Answer::Form` already carries, read with the same
view state, so a check box a person has just ticked answers `on` in the accessibility tree exactly
as it does in the form panel. One type rather than a second taxonomy: a copy would be a second
statement of one fact that could disagree with the first.

It is filled for **any** element whose own object reference names a widget of a field on the page,
and read by `viewer_accessibility::role::map` for **`Form` and nothing else**. That split is
deliberate. Which structure type *should* name a widget is §14.8.4.7.2's question and the mapping
is where it is answered; the fact that a file put the reference somewhere else is a fact about the
file, and withholding it at the boundary would be `viewer-core` deciding a platform question.

The mapping itself, with the two arms that are decisions rather than lookups:

| §12.7.5 | becomes |
|---|---|
| push button | `Role::Button` |
| check box | `Role::CheckBox`, `toggled` from the field's value |
| radio button | `Role::RadioButton`, likewise |
| text field | `PasswordInput` under Table 231 bit 14, else `MultilineTextInput` under bit 13, else `TextInput` |
| choice field | `EditableComboBox` under Table 233 bits 18 and 19, `ComboBox` under bit 18, else `ListBox` |
| signature field | `Role::Group`, described |
| a field stating no `/FT` | `Role::Group`, described |

**Password before multiline**, and it is the only ordering in the table that could do harm. Table
231 bit 14 makes the field "intended for entering a secure password that should not be echoed
visibly to the screen"; bit 13 makes it one that "may contain multiple lines of text". A file
setting both has said something the table does not contemplate, and a control that echoed a
password because of the second flag is the one mistake here that cannot be taken back.

**A signature field keeps the group and says so.** §12.7.5.5 makes it "a form field that contains a
digital signature", and neither AccessKit nor AT-SPI has a role for one. A button or a text input
would each assert something about what a person may do with it that this program does not
implement, so the loss goes in the node's description — the rule this module has followed for every
distinction the platform cannot carry, since ADR 0214's table.

**And the state crosses, which is half of what a check box means.** `Mapping::toggled` becomes
`accesskit::Toggled`, which `accesskit_atspi_common` turns into AT-SPI's `Checked` state. A box
announced without it says there is a box and not whether it is ticked.

## How it was verified, and it is the bus

`doc/verify.md`'s recipe, unchanged: `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd`
with a `DISPLAY` of its own, `Xvfb`, `IsEnabled` set on the session bus, and a client walking
`org.a11y.atspi.Accessible` from the registry root — asking `GetRole`, the `Name` and `Description`
properties, `Component.GetExtents` and `GetState` at every node. **The same binary twice, two
fields of difference**: the A/B was taken by making `finish` answer `None` for both the annotation
rectangle and the control, and rebuilding.

`doc/pdf.js/test/pdfs/annotation-button-widget.pdf` is the witness worth having, because **the
document states the right answer in its own paragraphs**. Nine `Form` elements, each beside a
label saying what it is:

```text
[paragraph] 'Check box, unchecked'
  before  [panel]     '' no Component
  after   [check_box] '' (39, 27, 12, 12) state=checkable
[paragraph] 'Check box, checked'
  before  [panel]     '' no Component
  after   [check_box] '' (39, 61, 12, 12) state=checked,checkable
[paragraph] 'Radio button, unselected'
  before  [panel]        '' no Component
  after   [radio_button] '' (39, 163, 12, 12) state=checkable
[paragraph] 'Radio buton, selected'
  before  [panel]        '' no Component
  after   [radio_button] '' (39, 197, 12, 12) state=checked,checkable
```

`panel` is AT-SPI's name for `Role::Group`, and `no Component` is what "this element has no place"
looks like from a client — the call errors rather than answering a zero rectangle. Three check
boxes and six radio buttons, and `checked` arrives on exactly the three the document's own labels
call checked or selected. The extents are each widget's `/Rect` through the viewport's transform.

And at scale, `doc/pdf.js/test/pdfs/prefilled_f1040.pdf` page 1, which states 242 `Form` elements
across its two pages:

| | before | after |
|---|---|---|
| nodes reaching the bus as a control | **0** | **104** — 93 `entry`, 11 `check_box` |
| nodes implementing no `Component` | **204** | **100** |

The hundred that remain implement none for a reason this change does not touch: 92 `section`, 4
`table_row`, 2 `paragraph`, the `application` and the outer `document_frame` — containers whose
extent this program does not compute and the two nodes AccessKit does not place.

**One honest note about the A/B's left-hand column.** The `before` binary has the *new* mapping
compiled in with the control removed, so each `panel` also carried the description this round adds
for a `Form` naming no widget. The real "before" said nothing at all. The role and the extents are
the difference the table is about.

## What it cost

Nothing on any page that states no object reference, by construction: the two readings are behind
a check of the gathered elements, and 1094 of the corpus's 1245 documents have no structure tree at
all. On a page that does state one, it is one walk of `/Annots` and one walk of §12.7.4.1's field
tree — the same `pdf_model::form::fields` a host already calls when it asks `Query::Form` for the
same page.

**No wall clock is quoted**, deliberately, and for the reason ADR 0312 established: this round ran
beside nine others and a stopwatch would have measured the machine. `viewer-core --example
accessibility_cost` and the callgrind method are what the next round should use; `doc/todo/31`'s
cost item is unchanged and now has one more thing in it.

## What this does not do

- **1342 placeless elements still state nothing.** They are `P`, `Div`, `Span`, `TD` and `Figure`
  elements whose content items produced no text and which name no annotation — a bound for them
  would have to come from the *marks* rather than from the document, and the display list records
  no `/MCID`, so nothing today can say which commands an element's content items made. That is
  unchanged and stays on `doc/todo/31`.
- **An `XObject` object reference is still unplaced**, on the argument above.
- **Whether a stated rectangle should win over the shapes that were drawn** is still unmeasured, so
  `tree::place` still prefers the quads. It does not bite here: an element placed by an annotation
  has no quads by definition.
- **AT-SPI's `Table` and `TableCell` are still not implemented by `accesskit_atspi_common`**, and
  neither is a relation set beyond `ControllerFor`. Unchanged.
- **Nothing acts on the control.** A screen reader that asked to tick the box would reach
  `Bridge::requested` and be printed by name: this tree declares no actions, which is
  `doc/todo/31`'s own entry and is now the sharpest one on it — a check box announced as a check
  box invites the question. (Taken in the five-hundred-and-ninetieth session; the box can be
  ticked from the bus. ADR 0425.)

## What it corrected on the way

§12.5.2's ledger row listed `/StructParent` among the entries that are "[n]ot read, none of which
marks the page". It has been read since ADR 0325, which asks each of a page's annotations for it as
one of §14.7.5.4's three keyings — thirteen sessions of a row describing the code as it was. The
list's own justification still holds for the rest of it, and the entry is now named with its
consumer.
