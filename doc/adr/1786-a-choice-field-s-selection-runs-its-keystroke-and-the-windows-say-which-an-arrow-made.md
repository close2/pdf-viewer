# 1786 — A choice field's selection runs its keystroke, and the windows say which an arrow made

Session 1475. Status: **accepted** and **built**. Takes up the first of the three hunks ADR 1762
section 8 left to the windows, and reverses `doc/todo/30`'s "`quorra`'s drawn choice list takes no
keyboard". Context: ISO 32000-2 §12.6.3 Table 199 (`/K`), §12.7.5.4 (Tables 233 and 234); Adobe's
`event.change`, `changeEx` and `keyDown` as ADRs 1626 and 1762 read them; ADRs 0248, 0596, 1579.
Code: `crates/pdf-model/src/view.rs` (`set_field`'s match), `crates/pdf-model/src/view/scripts.rs`
(`Selected`, `selection_verdict`), `crates/viewer-ui/src/chrome.rs` (`ChoiceList::stepped`,
`showing`), `crates/viewer-ui/src/bin/quorra/typing.rs` (`Choosing::first`, `arrow_on_choices`),
`window.rs` (`pressed`), `crates/viewer-gtk/src/controls.rs` (`FieldChange::Arrows`, `list`),
`crates/viewer-gtk/src/host.rs` (`tell_keys_with`), `crates/viewer-qt/src/{bridge,host}.rs` (`keys`),
`crates/viewer-qt/cpp/window.{h,cpp}` (`arrowKey_`, `eventFilter`, `keys`).
Tests: `crates/pdf-script/tests/list_keystroke.rs`; `viewer-ui`'s chrome unit tests
`an_arrow_selects_the_next_option_and_holds_at_the_ends` and
`the_list_scrolls_to_keep_the_arrow_s_option_on_the_screen`; drive step 73.

## 1. The keystroke at a selection

Table 199's `/K` is performed "when the user modifies a character in a text field or combo box or
modifies the selection in a scrollable list box". `ViewState::set_field` ran it for
`Entered::Text` alone. It now runs it for `Entered::Chosen` too, through the same
`keystroke_verdict`, so a script and a one-call `AF` library function judge a selection as they
judge characters. What the event hands:

- **`event.change` is the text a person sees for the option the selection names**, the second
  string of a two-string `/Opt` entry. RFC 0008 section 6.5 item 4 reads the reference as "a
  selection change in a list box is a keystroke whose `change` is the selection". `changeEx` is that
  option's export value (ADR 1626).
- **The option is the lowest selected index**, which is the one a single selection keeps (Table 233
  bit 22). An empty selection hands an empty change. The reference states no change for several
  options at once, so the first of them is this tree's choice.
- **A rejection takes nothing**, as for characters. A rewrite to the text of an option selects that
  option. A rewrite to text no option shows is reported, and the selection stands as the person
  made it, since a list box holds no value outside `/Opt`.

A script's `currentValueIndices` does not reach this: it is the document choosing, not the user,
and `scripts::choose` applies it without `set_field`.

## 2. The windows say an arrow made it

`event.keyDown` reads `Keys::arrows` at a choice field's keystroke (ADR 1762). Each window now sets
it around the edit an arrow made, and only that edit: `Command::Keys` with the arrows down before
the `Edit::SetField`, and up after it.

- **`quorra`'s drawn list takes Up and Down.** The clause names no key, so the step is a convention,
  the toolkits' own: one option after the last selected or one before the first, held at either
  end, the first option where none is selected, and a list of several reduced to that one option.
  The list stays up and scrolls only where the option has left its rows; `/TI` still decides where
  it opens. An arrow at an end selects what is already selected, which modifies nothing and sends
  nothing. Enter is still not taken, since a pick is whole when it is made.
- **GTK** marks an arrow in the list view's capture phase and lets it go from an idle. The model's
  `selection-changed` runs inside the key press, so it reads the mark. A `GtkDropDown` changes its
  value only when a row is activated, so a GTK combo box never reports an arrow.
- **Qt** marks an arrow in the window's event filter on a `QListWidget` or a `QComboBox` and lets
  it go from a zero timer. A closed combo box takes the arrows as a new value, so Qt reports them
  there too, the editable one included.

No message was added and none changed shape. `Command::Keys` already carried the arrows.
