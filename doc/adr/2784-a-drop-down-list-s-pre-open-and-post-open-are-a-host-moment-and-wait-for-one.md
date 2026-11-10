# 2784 — A drop-down list's `preOpen` and `postOpen` are a host's moment, and wait for a round of their own

Session 1986, a ledger round (K.2). Status: accepted, not built. Beside ADR 2783, which built the
two of round 1983's four remaining events that need no host; this one records why the other two
are effort of a different size, and the construction the round that takes them follows.

## Context

XFA 3.3 page 392: `preOpen` and `postOpen` "apply only to drop-down choice lists", and are "intended
to trigger scripts that add choices to and remove choices from the choice list". `preOpen` fires when
a person clicks the symbol that drops the list down, when a person presses the key sequence that
drops it down while it has the focus, and when a script calls `$host.openList()`, and "does not fire
in response to the user pressing the Enter key while the combo box has the focus"; `postOpen` "fires
after the combo box is displayed. It also fires when the display of the combo box fails, to ensure
that any cleanup code is executed". Page 400 makes the two a pre- and post-event pair, so a
`preOpen` script's `cancelAction` cancels the opening and page 400's cleanup fires the `postOpen`.

The moment is the host's. `viewer-core` sees a choice only when a person commits one
(`Edit::SetField` with `Entered::Chosen`); the list is drawn by each host: `quorra`'s own open list
(`viewer-ui`'s `typing/choices.rs`, `App::open_choices`), the confined window's (`quorra-confined`'s
`typing.rs`), `viewer-gtk`'s `GtkDropDown` (`controls.rs`) and `viewer-qt`'s `QComboBox`
(`window.cpp`). No command says that a list is about to open or has opened. This is effort, not an
owner's question: the static half (a choice list of a field the AcroForm holds), `off` by default for
scripts, and nothing leaves the document.

## Decision — the construction, for the round that builds it

1. **Two commands, because the scripts run between them.** `Command::OpeningList { field }` before a
   host shows the list, answered by the activation's verdict (open, or cancelled by `cancelAction`),
   and `Command::OpenedList { field, shown: bool }` once it is shown or has failed, which raises
   `postOpen`; a cancelled opening raises `postOpen` itself (page 400). One command after the list is
   shown would run `preOpen` after the display it exists to precede.
2. **The options are read after `preOpen`, not before.** A host reads `Query::Fields`' options once
   the opening's events have run, so a script that changes the list is what the person sees.
   `addItem()`, the method page 392 names for this, "is described in the Adobe LiveCycle Designer
   ES3 Scripting Reference" and not in XFA 3.3; this program's scripts have no method that changes a
   field's options, so until one exists a `preOpen` script can change values and properties only.
3. **Every host that draws a drop-down reports both moments**: `quorra` in `open_choices`, the
   confined window through a new frame of its protocol (a frame number and its reader in
   `confined-transport`), `viewer-gtk` where its `GtkDropDown`'s popover is shown (GTK 4 names no
   signal for it; the popover's `visible` notification is the one to try first), `viewer-qt` in a
   `QComboBox` whose `showPopup` is overridden, and the C ABI as two entries, whose counts move in
   `unsafe_position.rs` and `header_and_library_agree.rs`. A list box (Table 233 bit 18 clear) is no
   drop-down and raises neither.
4. **`$host.openList()`** is the third trigger and is a script asking the window to open the list:
   it goes through the same two commands, the host being asked to show the list, and is refused by
   name where the host draws no list (a window with no forms interface).

## Why not this round

ADR 2783's two events needed only `pdf-model`; this needs four hosts, a confined frame and two C
ABI entries, each built and tested, under a shared machine whose load stood at seven hundred while
this round's first test build ran. Doing half of it — the core commands without the hosts — would
be a stimulus nothing raises, which is the placeholder principle 1 forbids.

## Cost

Recorded so that the next round can price it: two `Command` variants, two `Stimulus` variants, one
confined frame, two C ABI entries, and a host change in each of four windows.
