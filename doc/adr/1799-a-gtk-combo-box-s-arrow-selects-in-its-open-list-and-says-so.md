# 1799 — A GTK combo box's arrow selects in its open list, and says so

Session 1481. Status: **accepted** and **built**. Revises the GTK sentence of ADR 1786 §2 ("A
`GtkDropDown` changes its value only when a row is activated, so a GTK combo box never reports an
arrow"). Context: ISO 32000-2 §12.6.3 Table 199 (`/K`), §12.7.5.4 Table 233 bits 18 and 19; ADRs
0508, 0596, 1762, 1786; trap 17. Code: `crates/viewer-gtk/src/controls.rs` (`marks_arrows`,
`list_view_under`, `arrows_choose`, `arrows_choose_rows`, `combo`, `editable_combo`). Drive step
`74-combo-arrows`.

## 1. The sentence was true of the widget and not of the control

Table 199's `/K` runs when a person "modifies a character in a text field or combo box or modifies
the selection in a scrollable list box". Adobe's `event.keyDown` is true when the arrows made a
choice field's selection. GTK 4.22's `gtkdropdown.c` is the evidence for how its widget behaves. The
drop-down's own `selected` changes only in `row_activated`, so only after a click or Enter. Its
popover holds a `GtkListView` over a second selection model, `popup_selection`, which the list's
move-cursor bindings (`gtklistbase.c`, `select` true for a plain arrow) move inside the key press.
So the arrows already were selecting, in a model this program did not listen to. It is trap 17's
shape again: the capability was one composition away, behind a widget that names no accessor for it.

## 2. What the control does now

- **Plain drop-down (bit 19 clear).** `list_view_under` finds the popover's list, a child of the
  drop-down like any other, and `marks_arrows` marks an arrow on it in the capture phase, as ADR
  1786 does for the list box. When the list's selection moves under a marked arrow, the drop-down
  gets the same position at once. Its `notify::selected` then runs inside the key press, reads the
  mark, and sends `Edit::SetField` between `Command::Keys` with the arrows down and with them up.
  The Enter that closes the list activates the option already selected, which GTK does not notify,
  so the edit is sent once. The template sets `single-click-activate`, so the list's selection
  also follows the pointer. A move with no arrow down selects nothing.
- **Editable combo box (bit 19 set).** The popover's `GtkListBox` selects the row its cursor moves
  to inside the key press (`gtk_list_box_move_cursor`), so `row-selected` under a marked arrow sends
  that row's position with the arrows held. The row sent is kept, so the Enter that closes the list
  sends nothing more. Typing in the entry clears it, since the value is characters again.

Both behave the way ADR 1786 describes `quorra`'s drawn list and Qt's closed combo box: an arrow is
a selection, made where the person is looking. No message was added and none changed shape.
