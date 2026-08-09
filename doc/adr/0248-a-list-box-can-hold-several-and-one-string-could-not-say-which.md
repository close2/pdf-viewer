# ADR 0248 — A list box can hold several, and one string could not say which

Status: accepted, 2026-08-09 (session 412).

## Context

Three hosts were built on `viewer-core`'s vocabulary in four rounds — GTK4 in the
four-hundred-and-eighth (ADR 0244), Qt 6 in the four-hundred-and-tenth (ADR 0246), a C ABI in the
four-hundred-and-eleventh (ADR 0247) — and between them they added **no message**. `doc/todo/30`
records that as the boundary's proof and then records the one exception, found independently by the
first two:

> **§12.7.5.4's list box is the one place the boundary genuinely limits a host**, and the second
> host established it. `viewer-qt` asks `QListWidget` for `SingleSelection` *deliberately*, because
> `Edit::SetField` carries one value while Table 233 bit 22 permits several.

Both hosts wrote the same comment beside the same line and then printed a note saying what they
were not doing. That is the shape of a vocabulary gap rather than a toolkit's: `GtkMultiSelection`
and `QAbstractItemView::ExtendedSelection` both exist, and neither was used because there was
nowhere to send the answer.

### What the standard says, read rather than assumed

This is **not** a place where ISO 32000-2 is silent. It states the shape of the answer three times.

Table 233 bit 22 makes the case real:

> (PDF 1.4) If set, more than one of the field's option items may be selected simultaneously; if
> clear, at most one item shall be selected.

§12.7.5.4 states what the value then is:

> If the field does not allow multiple selection -that is, if the MultiSelect flag ( PDF 1.4 ) is
> not set -or if multiple selection is supported but only one item is currently selected, V is a
> text string representing the selected item, as given in the field dictionary's Opt array. If
> multiple items are selected, V is an array of such strings. (For items represented in the Opt
> array by a two-element array, the name string is the second of the two array elements.)

And Table 234's `/I` states the second entry that has to move with it:

> For choice fields that allow multiple selection (MultiSelect flag set), an array of integers,
> sorted in ascending order, representing the zero-based indices in the Opt array of the currently
> selected option items. This entry shall be used when two or more elements in the Opt array have
> different names but the same export value or when the value of the choice field is an array.

**`/V` is Table 226's and `/I` is Table 234's.** Worth writing down because the round was briefed
with "Table 228's `/V`", and Table 228 is *Additional entries common to all fields containing
variable text* — `/DA`, `/Q`, `/DS`, `/RV`, and no value at all. The tree's own fifteen `Table 228`
citations were checked and every one of them is about `/DA` or `/Q`. The mis-citation was in the
question, not in the code, which is the ninth sweep working in the direction nobody expects.

### What already crossed, because this project's commonest finding is a gap that closed

Checked before any code was written, and **the reading half was already complete**:

| what §12.7.5.4 states | did it cross before this round |
|---|---|
| Table 234's `/Opt`, both element forms, in the array's order | yes — `ChoiceControl::options` |
| which items are selected | yes — `ChoiceControl::selected`, as **indices** |
| Table 233 bit 18's `Combo` | yes |
| bit 19's `Edit` | yes |
| **bit 22's `MultiSelect`** | **yes** |
| bit 27's `CommitOnSelChange` | yes |
| Table 234's `/TI` | yes |
| §12.7.5.4's rule that `/V` beats `/I` | yes — `form::selected` |
| **a way to say which items a person chose** | **no** |

So the gap was exactly one direction wide. A host could learn everything the clause states and
could not say the answer back.

## Decision

### The variant changes, and every consumer fails to compile

`Edit::SetField`'s value stops being `Option<String>`:

```rust
pub enum Entered {
    Cleared,
    Text(String),
    Chosen(Vec<usize>),
}
```

The shape ADRs 0166, 0167 and 0247 established — *where a host needs a thing a variant does not
carry, the variant changes and every consumer fails to compile* — applied for the fourth time. It
is what nothing in this vocabulary being `#[non_exhaustive]` is *for*, and this round is the first
to exercise it with five Rust consumers and a C ABI on the other side.

`Cleared` and `Text` are the old `None` and `Some`. The third is new, and two things about it were
decided rather than fallen into.

### A selection is named by index, not by label

§12.7.5.4 puts the *labels* in `/V`, so labels are what reaches the file. The message carries
indices anyway, for a reason the clause supplies: two of `/Opt`'s entries may carry the same name
string, and a host answering with labels could not say which of them a person clicked. That is the
same ambiguity `/I` exists to settle, so the message that has to be written into `/I` may as well
be what a host sends. It is also the coordinate `ChoiceControl::selected` already answers in, so a
host reads and writes in one system.

The one case a label is still right is Table 233 bit 19's editable combo box, where "the user can
type a value other than the predefined choices" — and that is `Entered::Text`, which already
existed.

### `/V` and `/I` are resolved once, in `ViewState::set_field`

Turning an index into a label needs the field's own `/Opt`. Three places would otherwise do it —
the appearance the page draws, the description `Query::Fields` answers with, and the file a save
writes — and three readings are three chances for a picture and a file to disagree about what was
chosen. So `set_field` resolves once and stores the object Table 226's `/V` will hold, beside the
indices Table 234's `/I` will hold. `FieldValue::Edited` carries both, and `Field::read` now clones
the value exactly as it already did for §12.7.8's imported one — the two arms became one, because
both are values that came from outside the document's own `/Parent` chain.

A side effect worth naming: the encoding to §7.9.2.2's text string used to happen **twice**, once
in `Field::read` for the drawing and once in `ViewState::save` for the file. It happens once now.

### The file is written the way the clause states, and `/I` is removed where it would lie

- one item selected → `/V` is a string
- several → `/V` is an array of strings
- none → `/V` is removed, which is "[t]he default value of V is null , indicating that no item is
  currently selected"
- `/I` is written whenever the edit named options, ascending
- **`/I` is removed whenever it did not** — text typed in, a button's state, a clear

That last one is the decision with an argument behind it. §12.7.5.4 says the value wins where the
two disagree, so a stale `/I` beside a new `/V` is not *unreadable* — it is a file relying on the
reader's tie-break to hide a contradiction this program wrote. Session 411 found `ViewState::save`
storing what Table 231 bit 14's NOTE forbids; this is the same class of defect caught before it
shipped.

### Table 233 bit 22 is obeyed, not carried

"if clear, at most one item shall be selected" is a `shall` and this program is what selects, so a
selection sent to a single-select field is **cut to its first index in `/Opt` order**. That is the
shape ADR 0197 gave Table 231 bit 24's `DoNotScroll` — take the part that is permitted rather than
refuse the whole edit — and for the same reason: refusing would leave a control that does nothing.

`Entered::Chosen` on a field that is *not* §12.7.5.4's is refused outright and `set_field` returns
zero widgets. Table 230's `/Opt` is a **button's** export values under the same key, so resolving an
index against it would write an export value where §12.7.5.2.3 wants an appearance-state name.

## Consequences

- **Both native hosts obey the flag now instead of printing a note about it.** `viewer-gtk` builds
  a `GtkMultiSelection` where bit 22 is set and a `GtkSingleSelection` where it is clear;
  `viewer-qt` sets `ExtendedSelection` against `SingleSelection`. Both read the selection *out of
  the model* rather than accumulating it from the signal, because the signal says which positions
  changed and the edit needs what is selected now.
- **Both combo boxes send an index rather than a label** where the toolkit's control is not
  editable, which removes the duplicate-label ambiguity from that path too. Qt's editable combo box
  still sends characters, because bit 19 says its value need not be one of the options at all.
- **The C ABI did not change and that is the finding, not the omission.** `Command::Edit` is one of
  the verbs `doc/todo/30` records as absent from the 39 entry points, so a variant's shape changed
  under it and cost it nothing: `PDFV_EVENT_KIND_COUNT` is **15** before and after, and
  `pdfv_abi_check` would have said so at startup if it were not. This is the first round to
  demonstrate what that section predicted rather than to assert it.
- **The confined transport carries all three shapes**, and its round-trip test names each; the
  `confined_wire` fuzzer ran **13 632 129 executions in 181 s** over the changed decoder with no
  crash.
- **Nothing about what gets drawn changed.** No gate moved: the corpus's 974 with 65 incomplete,
  the oracle's 905/68/786 verdicts line for line, quorra's 912/36/9/17, the text gate's 99.2%. A
  list box still draws nothing on the page, for the reason ADR 0240 gave and this round did not
  revisit — the clause states which items are selected and states no highlight.
- **`ViewState::edits()` answers with `FieldValue` rather than `Option<&str>`**, because an edit is
  no longer describable as one optional string. Its one caller outside `pdf-model` is a dirty check.

## What the corpus says, and what one file said back

`examples/field_flag_census` over the 964 openable documents: Table 233 bit 22 is set on **4 widgets
over 4 documents** — `annotation-choice-widget.pdf`, `issue15096.pdf`, `issue16500.pdf`,
`issue17492.pdf`. Bit 18's combo box is 26 widgets over 17, bit 19's editable one 3 over 3, and bit
20's `Sort` **none at all**.

The round was driven on `issue17492.pdf`'s `databases`, and reading it found something worth
recording as a *fact about files* rather than a defect. Its `/Opt` is Table 234's two-element form
throughout, so the export values (`oracle`, `db2`) and the labels (`Oracle`, `DB2`) are different
strings — and the file's own `/V` holds **the export values**, which §12.7.5.4 says it does not:
"the name string is the second of the two array elements". So this tree opens it with nothing
selected, which is the clause read correctly against a file that was written wrongly. Both other
readers agree with the file rather than the clause. It is left as it is: principle 5 says
disagreement is a question for the standard, and the standard answered.

## The evidence, driven rather than argued

Under `Xvfb` with the release binaries, `issue17492.pdf`, both hosts:

| | what happened |
|---|---|
| Qt, click *Oracle* | `databases: option(s) [0] of Table 234's /Opt selected` |
| Qt, ctrl-click *DB2* | `databases: option(s) [0, 2] ... selected`, and **two rows highlighted at once** in `#3daee9` |
| Qt, `s` | `saved 71524 bytes` |
| GTK, the same two clicks | `option(s) [0]` then `option(s) [0, 2]`, two rows highlighted |
| GTK, `s` | `saved 71524 bytes` |

The two files are **byte-identical**, which is what one vocabulary under two toolkits should
produce and is not something either host could have said alone.

What the saved file holds, read back with `mutool show`:

```
/I [ 0 2 ]
/V [ (Oracle) (DB2) ]
```

The producer's 70 166 bytes are byte-for-byte intact underneath the 1 358 bytes §7.5.6 appended —
checked by comparing the prefix — `qpdf --check` finds no syntax error, and re-opening the written
file in the same host shows *Oracle* and *DB2* selected.
