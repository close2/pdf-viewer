# 1629 — A mode is waited on by its own line, and an editable combo box's keyboard is its text box

Status: accepted and **built**. Session 1396. Amends ADR 1605 (the drive waits for the window's
word) and ADR 1617 section 5 (the editable combo box commits like a text field). Trap 120.
Code: `tools/drive-windows.sh` (`mode_on`, `mode_last`, `mode_quiet`, `combo_commit`,
`drive-combo.pdf`), `crates/viewer-gtk/src/controls.rs` (`give_the_keyboard`),
`crates/viewer-ui/src/bin/quorra-confined/typing.rs` (`focused_text_field`).

## 1. A toggle is pressed until the window says the mode is on

`38-located` read wrong once in a full drive because `quorra`'s `m` landed twice — "measuring",
then "measuring off", then the click. A key press under `xdotool` is a key down and a key up the X
server may see apart under load, and its autorepeat (660 ms by default) turns a held key into more
presses. So a step that toggles a mode presses with `mode_on KEY ON OFF`: it waits for the window's
ON line, then for the window's lines about the mode to be quiet for 0.8 s — past the autorepeat
delay, so that a second landing has had time to show — and reads the last of them; a mode found off
is pressed again, three times at most. The quiet period is the one fixed time, and its reason is the
server's delay, not the window's. `38-located` uses it in all four windows; `r`'s card and `f`'s bar
are not toggles a click follows (`f` only opens in `quorra`; `r`'s step reads the level it set).

## 2. The editable combo box's commit is driven, and two windows were wrong

`45-combo-commit`, in all four windows, on two Table 233 bit 18 and 19 combo boxes under the
currency `/K` and `/F`: `12.5` typed and committed by Tab is saved `12.5` and drawn `$12.50`, and
`-` committed by Enter is refused at the commit and said. Driven, it found two defects:

- **GTK**: `follow_focus` and `aim` gave the keyboard with `grab_focus` on the placed widget, and
  an editable combo box's is a composed `GtkBox`, which takes no focus. The first box worked by
  GTK's own Tab move; the walk out of it left the keyboard on its drop-down button, and the next
  character reached the page as a zoom. A move *into* the box is no fix either — GTK's Tab may
  already have put the keyboard in the entry, and a move forward from there leaves it, which is a
  commit. So `controls::give_the_keyboard` names the entry.
- **`quorra-confined`** typed into single-line text fields only and said so of a combo box; its
  text box takes characters until the commit exactly as an entry does, so it is typed into now.

## 3. What stays

`quorra` and `quorra-qt` passed the step as they were. A multiline field's `GtkScrolledWindow` is
given the keyboard by `grab_focus` as before, and no step drives a multiline field's keyboard: that
is the next composed control to drive, not one found wrong.
