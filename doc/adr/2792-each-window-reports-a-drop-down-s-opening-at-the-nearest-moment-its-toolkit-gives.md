# 2792 — Each window reports a drop-down's opening at the nearest moment its toolkit gives

Session 1990, a ledger round (K.2). Status: implemented. Corrects ADR 2784 decision 3 in two places
and records where each window's moment falls against XFA 3.3 page 392; ADR 2791 is the core half.

## Context

Page 392 places `preOpen` at a click on "the symbol that causes the choice list to drop down" or the
key sequence that does, never at Enter, and before the list is displayed. ADR 2784 assumed four
windows draw a drop-down. Read against the code, three of its five do, and two of those three have
no moment before the display.

## Decision

1. **`quorra`** draws its own list (`App::open_choices`): a press on a combo box (Table 233 bit 18
   set) now sends `Command::OpeningList` and keeps the press as `list_asked`; the list is displayed
   at `Event::ListOpens`, and `OpenedList` is queued behind it. A list box (bit 18 clear) is
   displayed whenever the field is and drops nothing down, so it raises neither. The exact moment.
2. **`viewer-qt`**: `DropDown`, a `QComboBox` whose `showPopup` — the toolkit's documented moment
   before the list shows, reached by the arrow and by Alt + Down or F4 and not by Enter — asks and
   returns; `QtUpdate::list_opens` names the control at the grant and the list is shown then. The
   exact moment.
3. **`viewer-gtk` and the WinUI host have no moment before the display.** GTK 4 gives a popover's
   `visible` becoming true (`GtkDropDown`'s own and the editable combo's `GtkMenuButton`'s) and
   WinUI a `ComboBox`'s `DropDownOpened`, both after the list shows within the same press. Each takes
   the list straight down there, asks, and shows it again at the grant, so the list a person reads
   is the one the `preOpen` scripts left: the nearest moment to page 392's, a frame late. Neither
   toolkit tells a window which gesture showed the list, so page 392's Enter exclusion is not kept
   where the toolkit's own Enter or space opens a focused drop-down.
4. **The confined window draws no list** (ADR 2784 assumed it did): a non-editable combo box is "a
   choice of rows, which this window does not offer", an editable one a text box. It sends no
   `OpeningList`, and a grant that reached it would be said and not drawn. The confined *wire*
   carries the two commands and the grant all the same, since the confined worker holds the view.

## Consequences

- No new confined frame: the existing command and event frames carry the new kinds.
- The WinUI host is not built on this machine (no .NET SDK); `header_and_interop_agree` checks its
  interop declarations.
