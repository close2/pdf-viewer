# 2001 — The chooser again starts on the held choice, and a page whose marks cross stays on the list

Session 1582, slot 6 of batch ninety-two, the residues of ADRs 1985 and 1986 under the project
owner's "switch to code improvements … until I say otherwise" of 2026-10-14. Status: **accepted**;
§1 **built** in `viewer-host`, `quorra`, `quorra-gtk` and `quorra-qt`; §2 **measured, nothing
built**. Amends ADR 1985 §1's third point. Code: `viewer_host::archiving::Choosing::starting_on`,
`NativeWindow::choose_the_archive_again`, `quorra`'s `choose_the_archive_again`, the Qt bridge's
`archive_target_start` and `archive_profile_start`, `viewer-qt/cpp/window.cpp`
(`chooseAnArchive`). It cites no clause and moves no ledger row.

## 1. The chooser put up again starts on the choice the window holds

ADR 1985 started Ctrl+Shift+A's chooser on PDF/A-2b and `refuse-any-loss`. It gave the reason that
the three windows' dialogues would each need the indices across a bridge. Only one of them needed
that. `quorra`'s card and the GTK dialogue draw a `Choosing` and already start on its two indices,
so they needed only a `Choosing` built on the held choice. The Qt dialogue is C++ and was handed the
lists as words. It started on row 0 whatever the Rust side held.

- **`Choosing::starting_on(held)`** starts on the held target and profile. With nothing held, or a
  profile no longer offered, that half starts on the first, as a first choice does. Ctrl+Shift+A
  builds its chooser this way in all three windows. Shift+A with nothing chosen is unchanged.
- **The Qt bridge carries the two indices** (`archive_target_start`, `archive_profile_start`), and
  `chooseAnArchive` sets the combo box and the list's current row from them. They are `i32`s, as
  `archive_answer`'s are.
- So Enter on the chooser put up again keeps what the window holds, and an arrow moves from it.
  Escape still leaves the held choice and says `nothing was converted` (ADR 1985).

Held by `archiving::tests::a_choice_made_again_starts_on_the_one_held`, which fails with
`starting_on` planted to ignore the held choice (trap 13). It is also held by drive step 88 in the
three windows. `88-archive-again` now presses Down once from the held `as-if-printed` to reach
`as-shown`; from the first profile, Down would give `as-if-printed`. The new `88-archive-again-held`
presses Enter alone and must convert under the held `as-shown`.

## 2. A page past the bound whose marks cross stays on the list

ADR 1986 §2.4 left a measurement. The question was whether a page past the bound whose marks cross,
the witness, should also cross as the worker's window. That trades the list's per-notch cost for a
processor draw at every margin a scroll leaves.

**Measured** in two `--clock` holds under an Xvfb of the round's own, `quorra-confined` on the
witness. The steps were twenty `=` notches, then twenty `Down` presses 0.5 s apart, then (second hold
only) twenty presses 0.1 s apart. Each hold alternated list, window, window, list. The window route
was a variable in an export only, which sent every page past the bound to `Marks::hold_window`; the
same binary without it is HEAD's route. Figures are per run:

| | the list (HEAD's route) | the worker's window |
|---|---|---|
| twenty notches settled | 48.4, 48.2, 47.9, 46.4 s | 41.2, 40.7, 40.7, 58.5 s |
| a press's next frame, median | 166, 168, 172, 180 ms | 9, 9, 10, 9 ms |
| a new window after the press that left the margin | — | 44–102 ms, every sixth press |
| second hold: longest wait for a press's next frame; last frame after the last fast press | 3.0, 3.2 s; 0.74, 0.92 s | 0.12, 0.15 s; none |
| the device's longest frame, asked to finished | 94, 90, 112, 94 ms | 70, 80, 84, 186 ms |
| windows the host's processor drew after the device refused its budget | 7, 6, 8, 6 | 0 |
| final photograph against the first list run | 0 pixels | 0 pixels |

**The hypothesis did not hold.** The list route is not the cheaper one on the witness. The window
route settles the notches about 7 s sooner in three runs of four. A press shifts the window already
held, and every sixth press brings a new one, drawn by the worker in 44–102 ms. On the list route
every press draws the list's marks for the new view on the device (ADR 1944), and the next
frame takes 166–180 ms.

**It is not built, and the reason is a rule rather than a figure.** On the list route every frame
the device drew was inside `WARN` (1 s). `CLAUDE.md` says the device draws every frame it is
predicted to draw inside that bound. The window route moves those frames to the processor. At a
notch, the wait the person has is the list crossing in front of the device's frame, not the frame
itself. Whether that crossing counts in a device frame's prediction is the owner's reading of ADR
1831's rule. It is asked as `doc/questions/Q477`, with the recommendation to count it and to build
a priced choice of route in the confined host. Until it is answered, a page whose marks cross
keeps crossing as its list.
