# 3062 — `viewer-ui`'s window is made by one constructor, and driven without a display

Session 2090. Status: **implemented.** Context: ADR 3041, record 2086 ("`viewer-ui`'s window has no headless
constructor, so its read is counted only through `commit_of`"), ADR 2936. Clauses: ISO 32000-2 §12.7.5.3.

The GTK, Qt and C windows each have a test that drives the window's own handlers without a toolkit loop. `quorra`'s
`App` was a 140-line struct literal inside `main`, so a test could make one only by copying it, and a copy is a
second window that drifts from the first.

## Decision

1. **`App::new(Launching)` is the window's one constructor.** `Launching` holds what the launch path gathers before
   the window exists (the command line's answers, the reader's policy, and the threads already started: the
   document's opening, page one's drawing ahead, the driver's instance), and `main` hands it over unchanged. The
   literal moved whole, comments included; the launch order in `main` is as it was.
2. **A test drives a real `App`, not a mock** (`window/headless.rs`): made by `App::new`, its document opened by
   the launch path's own `open_document` on a `viewer_host::Loading` thread and joined by `take_the_opening`, as
   `resumed` joins it, then given a size with the same `Command::Resize`. Keys go to `pressed` and presses to
   `click_page`, the handlers winit's events reach. What is absent is the window and the device, which every
   handler already treats as not yet made (`App::window_now`, `App::redraw`). No display is needed.
3. Its first two tests: §12.7.5.3's file-select read counted at the window (one `chosen_reads()` for a typed path
   and its Enter, none per keystroke), and Shift + T on a signed document (ADR 3063).

## Cost

`Launching` is a second list of twenty-one names beside `App`'s fields. A field added to `App` that the launch path
sets is added to both, and the compiler names the omission in each.
