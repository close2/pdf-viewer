# 1289 — A zoom step is not drawn behind a pass nobody asked for

Status: accepted and **built**.
Context: `crates/viewer-ui/src/bin/quorra/renderer.rs` (`draw_until_told_to_stop`,
`sharp_pass_cost`, `Window::collect`, `Landed::drew`, the `gesture` test module),
`crates/viewer-ui/src/bin/quorra/surface.rs` (`adopt`, one line),
`crates/render-raster/src/present.rs` (`QuorraWindowRenderer::new_headless`).
Answers: the project owner's report — *"we always have the blurred borders, even if the GPU should
have no problems rendering the document in time. It's as if we always showed the reprojected
document even if we should be fast enough."*
Amends: ADR 0699 (when the sharp pass runs), ADR 0704 (what rule 5's prediction is measured over).
ADR 0761's budget is unchanged. The standard says nothing about a gesture's intermediate frames, so
no ledger row is touched.

## What the tree did

Nothing in the zoom path defers the render to the end of a gesture, and nothing holds a stand-in for
a minimum time: `Surface::on_the_device` asks for the new view's frame at the first tick of the
step, and `Stale::plan` answers `Render` the tick that frame lands. Two things stood between a step
and its frame, both on the render thread:

1. **The sharp pass began the moment the thread was idle.** After every landed frame the thread
   found the job channel empty and started ADR 0699's 2× pass of the view just drawn. It is one
   submission and cannot be taken back, so the next step's job waited for all of it.
2. **Rule 5's prediction was measured from the ask** (ADR 0704), so a job that waited behind that
   pass was charged the pass as well as itself, and the next step was predicted to miss.

## What was measured

`renderer::gesture::a_zoom_gesture_out_counts_what_each_refresh_put_up`, on the real render thread
and the owner's Radeon 890M, a 120 Hz clock, ten zoom-out notches. Before, `tmp/Entwurf.pdf` with a
notch every 100 ms: each step drew in 55–62 ms and waited 108–121 ms, **nine notches of ten never
had a frame of their own**, and 125 of 150 refreshes were stand-ins. `--supersample 1` on the same
run: six of ten did. A text page (`doc/PDF20_AN001-BPC.pdf`) drew in 5.0 ms and was charged 9.0 against 8.3.

## Decision

- **The sharp pass waits for the view to be still for as long as it is predicted to take**
  (`sharp_pass_cost`, ADR 0761's own four-times prediction) before it begins, on
  `recv_timeout`, and a job arriving in that time is drawn instead. This is the rent-or-buy bound:
  the pass delays a view change only after the person has already been still at least as long
  as the delay. No new constant.
- **`Landed::drew` replaces `Landed::waited`**: the render's own span, from the thread taking the job
  up to finishing it. A render that is late because the thread was busy is still known to be late
  by rule 5's observation of how long the ask has been out.

After: supersample 2 matches supersample 1 on the same gesture (a notch's own frame 7–9 refreshes
after it, on seven of ten notches in one run and five in another; the rest are steps whose frame genuinely takes longer than the
gap between notches, or the first compute-lane frame at 276–286 ms). Every sharp pass comes after
the gesture.

## Held by

`a_zoom_step_is_drawn_before_the_sharp_pass_of_the_view_it_replaced` (fails three times of three with
the wait removed) and `a_frame_that_waited_behind_another_is_charged_only_its_own_drawing`.

## Not changed

A retained page (`draw_whole_page`) is still drawn the moment the thread is idle. It is drawn once
per page, not once per view, so it cannot stand in front of every step. The processor's window
(`crate::composer`) has no sharp pass and keeps its ask-to-finish span.
