# 1985 — The archive chooser again is Ctrl+Shift+A, and the Qt list is laid out at its width

Session 1574, slot 4 of batch ninety-one. Status: **accepted, built**. Context: ADR 1966 (the archive
target and profile are chosen in the window), ADR 1954 (Shift+A), ADR 1192 (a Control row is an
operation this program performs). Code: `crates/viewer-host/src/keys.rs` (`WindowAct::ChooseArchive`,
`ctrl_meaning`), `native_window.rs` (`choose_the_archive_again`), `archiving.rs` (`remembered`),
`viewer-ui/src/bin/quorra/window.rs`, the two toolkit hosts' `input.rs`,
`viewer-qt/cpp/window.cpp` (`chooseAnArchive`). It cites no clause and moves no ledger row.

## 1. Choosing again

ADR 1966 keeps a choice for the window's life, so a second Shift+A converts without asking. A window
that had chosen had no way to choose again short of a restart.

- **The key is Ctrl+Shift+A.** Every letter Shift alone could take is bound unshifted, and the
  conversion is already on Shift+A. Ctrl+A has no row, and this program's unshifted `a` selects
  everything. So the new row is not another meaning of an existing Control row. ADR 1192's sentence
  that Shift changes no Control row still holds: Ctrl+Shift+P prints.
- **The chooser starts where it always starts**: PDF/A-2b and `refuse-any-loss`, the profile that
  loses nothing (ADR 1966). It does not start on the held choice, because the three windows'
  dialogues would each need the indices across a bridge. Enter makes what is chosen the window's.
  Escape leaves the choice the window held, and says `nothing was converted`.
- **While a presentation runs it means nothing**, for Ctrl+O's reason: a chooser is a window, and
  Table 29's `FullScreen` shows none.
- The sentence said on choosing names the key: `… and Ctrl+Shift+A chooses again`.

## 2. The Qt list's height

The list was given the sum of its rows' size hints as its least height. Those hints are asked before
the list has a width, and a wrapped sentence is as tall as the width it wraps in. At 560 px the last
profile's sentence was cut at the list's lower edge, behind a scroll bar. The dialogue now takes its
width first. The rows are laid out in it with the vertical bar off while they are measured. The
least height is then the last row's bottom, the spacing and the frame. With the six shipped
profiles, the dialogue is 432 px tall against 396 px before, and every sentence is whole.

## 3. Held by

- `keys::tests::control_and_shift_a_chooses_the_conversion_again`, and `archiving`'s choosing test
  for the sentence.
- **Driven**, drive step 88 under the round's Xvfb in `quorra`, `quorra-gtk` and `quorra-qt`, with
  two new verdicts: `88-archive-again` (Ctrl+Shift+A, Down Down Enter chooses `as-shown` and
  converts) and `88-archive-again-closed` (Ctrl+Shift+A, Escape, then Shift+A converts under
  `as-shown` with no chooser). 21 works, 0 wrong. HEAD's `quorra-qt` read `88-archive-again` wrong.
