# 2849 — A window presents once in each of the display's refreshes, counted by the display

Session 2016, a host round on the owner's report that `quorra` on a 120 Hz macOS panel did not
feel like a well-written Mac program while scrolling and pinching. Status: implemented on macOS.
Context: `doc/todo/36` (the presenter's clock, 60 Hz the floor and 120 Hz the target), ADRs 0383
and 0384 (the cadence and where its rate comes from), 1926 (a tick that put nothing up leaves the
refresh open), 2848 (the gestures this round was measured with).

## Context

`quorra`'s clock (`cadence.rs`) allows one present per period of the rate the monitor states, the
period counted from the last present, and lets a present happen the moment a redraw arrives once
that period is spent. The *rate* was right — the trace said 120 Hz, stated by the surface — and
the *phase* was nobody's: the timer starts wherever the last present happened and an input event
arrives wherever the input device puts it, so nothing places a present at the same point of the
display's refresh twice. Measured on the GPU lane, M5 Pro, two-finger scrolling and pinching,
intervals between presents taken from `--trace=frames`:

| run | presents | interval ≤ 9.5 ms | mean |
|---|---|---|---|
| the timer | 2220 | 63 % | 11.10 ms, smeared over 10–15 ms |

Each frame took 3.8 ms on time and 5.0 ms late (host + scene + device), and one late frame in 824
took longer than a refresh, so the frames were fast enough and started at the wrong moments.
`--trace=window` measured the input: two-finger scroll events every 8.97 ms on average and pinch
events every 9.43 ms — the touchpad samples near 120 Hz **on a clock of its own**, drifting
through the display's refresh rather than locked to it.

## Decision

1. **The display says when a refresh is.** `viewer-display-link` is a count of the display's
   refreshes and a wake, behind a platform-neutral interface (`DisplayLink::on`, `beats`,
   `answered`, `want`, `listen`, `period`). macOS 14's `NSView` `CADisplayLink` is the one
   implementation; every other platform answers `Unavailable` and the window keeps the timer
   exactly as it was. Its own crate because it is Objective-C, so `viewer-ui` keeps
   `forbid(unsafe_code)`. On ProMotion it asks for 80–120 Hz with 120 preferred; a minimum of 60
   would let the system settle there.
2. **One present in each refresh, the refreshes counted by the link.** While the link runs,
   `may_present` is "a refresh has passed since the last present", whatever woke the window — an
   input, the render thread landing a frame, the link itself — and `due` is "a refresh the link
   woke the window for has not had its tick". An obligation (`owed`) asks the link for the next
   refresh instead of arming a timer tick. ADR 1926's split is kept on the link: a tick whose
   redraw put nothing up has spent the tick and not the refresh.
3. **A present is activity.** Every present tells the link (`listen`): it starts a paused link and
   restarts its count towards pausing. A link nobody wants a frame from pauses itself after
   `IDLE_BEATS` (six) refreshes, so a still window is not woken; paused, the timer answers, which
   keeps the first frame after an input immediate.

Nothing in the rule knows where in a refresh the input arrives or which platform counts: it is
one present a refresh, with the refreshes the display's. The `wake` is `Send` so that a platform
whose refresh is waited for on a thread of its own can report one.

## What was tried and measured, in order

| build | presents | ≤ 9.5 ms | mean | what it showed |
|---|---|---|---|---|
| frames only on a refresh the link woke the window for | 708 | 23 % | 15.8 ms | 183 472 "nothing to show" lines: a refresh woken for and not yet presented stayed due, and the loop asked again until the frame landed |
| the same, the tick spent by a redraw | 518 | 0 % | 17.3 ms | no spinning, every frame at 16.7 ms: an input arriving after the refresh's callback waited out the whole refresh |
| any refresh since the last present | 594, 622, 580, 492 | 73, 70, 86, 68 % | 9.0–11.1 ms | 157 of 664 inputs refused in one run: the link paused mid-gesture, because a window drawing on its input never *wants* a refresh |
| frames started and put up only on woken refreshes | 13 | — | — | the renderer drops a finished frame of a view that has moved, so a frame held for the next refresh is never shown |
| any refresh, every present activity (this decision) | 584 | 80 % | 9.24 ms | 10 of 610 inputs refused; of the intervals over 12 ms about half match the 24 gaps over 12 ms in the input itself |

The owner, on the last build: it feels good.

## Consequences

- One run is one person's gesture and the figures move between runs by several points; the
  comparison above is between builds measured the same way, not a gate. A gate would need a
  synthetic input at a stated rate and phase, which this round did not build.
- **Wayland and Windows still present on the timer.** Wayland's `wl_surface.frame` callback
  (which winit reaches through `Window::pre_present_notify`) and Windows' `DwmFlush` or
  `IDXGIOutput::WaitForVBlank` on a thread would each be a second implementation of the same
  interface; neither could be compiled or run on this round's machine. X11 has no general
  refresh event short of the Present extension and stays on the timer.
- `quorra-gtk` and `quorra-qt` are not affected: their toolkits pace frames themselves
  (`GdkFrameClock`, `QWindow::requestUpdate`). Whether both hosts draw through those was not
  checked here.
- macOS before 14 answers `TooOld` and keeps the timer; `CVDisplayLink` would serve it, is
  deprecated, and was not taken.
