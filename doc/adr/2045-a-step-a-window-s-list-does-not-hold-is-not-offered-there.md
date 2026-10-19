# 2045 — A step a window's list does not hold is not offered there

Session 1604, slot 6 of batch ninety-five. Status: **accepted** and **built** in
`tools/drive-windows.sh`. Context: round 1596's report (`--step accessibility` judged `wrong` in
`quorra-confined`, which the full drive never asks it of); ADRs 1815 and 1825 (the steps outside the
list); trap 25 (a hand-written list can name what is not there).

## 1. What held

`--step X` ran X in every window asked for, and the full drive's two lists were calls written out in
the loop, which `--step` did not read. Under HEAD, `--step accessibility` in `quorra-confined` said
`wrong — no DocumentFrame reached the bus (0)` after 20.2 s. **The premise that a step outside a
window's list cannot work there did not hold for every step**: `--step rendition_click`, in the
confined list alone, said `works` in `quorra`, `quorra-gtk` and `quorra-qt` under HEAD, because the
three hand a clip over through the same `viewer_host::handed_media` the confined window does.

## 2. Decision

- **The lists are two arrays**, `CONFINED_STEPS` and `WINDOW_STEPS`, run in order by the full drive
  and read by `--step`, with `OUTSIDE_STEPS` (`heavy_open`, `page_as_built_heavy`,
  `zoom_past_the_bound_heavy`) beside them. The 65 names are HEAD's, in HEAD's order, proved by a
  diff of the loop's names against the arrays.
- **Under `--step`, a step that window's list does not hold says `not offered`**, naming the window
  and that the full drive does not run it there, and is not run. A step outside every list runs in
  every window and says itself where it is offered, as it did.
- **A name in no list is refused** before anything starts, on one line with exit 2: a part of a
  step (`archive_chosen`) is run by its step, and a misspelt name was otherwise a `command not
  found` in every window. A listed name that is no function stops the drive before its first window.
- **`rendition_click` joins the three windows' list**, so the full drive drives §12.6.4.14's click
  in all four; it costs under a second a window.
- The summary line counts `not offered` beside `works` and `wrong`.

## 3. Driven

On this tree's release binaries, `--step confined --step rendition_click --step accessibility
--step zoomed_window_processor` in the four windows: 10 `works`, 0 `wrong`, 7 `not offered` — the
two confined-only steps in each of the three windows and `accessibility` in the confined one.
