# 1226 — The zoom step waited behind the sharp pass

Contract: the owner's report that a zoom-out always shows the reprojected, blurred picture even on
a device quick enough to draw every step. Not a clause round: the standard says nothing about a
gesture's intermediate frames, so no ledger row is touched.

## The mechanism as it was

- The stand-in is chosen in `Stale::plan` (`viewer-ui/src/bin/quorra/stale.rs`, rule 5 in
  `Stale::missed`): stand in when the last *built* frame's cost is over one refresh, or when the
  render asked for has been out longer than one refresh. It ends the tick the new view's frame
  lands. There is no timer, no frame count and no wait for the gesture to end, and the render is
  asked for at the first tick of the step (`Surface::on_the_device`).
- What made it permanent was the render thread (`renderer.rs`, `draw_until_told_to_stop`): after
  every landed frame it began ADR 0699's 2× sharp pass at once, one submission that cannot be taken
  back, so the next step's job queued behind it. And `Landed::waited` ran from the ask, so the
  queue was charged to the next prediction as well.

## Measured (real render thread, Radeon 890M, 120 Hz clock, 10 zoom-out notches)

`renderer::gesture::a_zoom_gesture_out_counts_what_each_refresh_put_up`, an ignored test.

| run | before | after |
|---|---|---|
| Entwurf, notch every 100 ms, supersample 2 | 1 of 10 notches got its own frame; 125/150 stood in; drew 55–62, waited 108–121 ms | 7 of 10 (5 in a slower run), 8–9 refreshes after the notch; 93/150 stood in |
| same, supersample 1 | 6 of 10 | 7 of 10 |
| AN001 text page, every 33 ms | fresh at the next refresh, 0–3 stand-ins | same |
| ISO 14289-1, every 33 ms | 10–11 stand-ins; drew 7.5 charged 11.4 | 10–11 (frames 7.5–9.4 ms, genuinely at the refresh) |

The remaining stand-ins are genuine: steps that draw longer than the gap between notches, and the
first compute-lane frame (276–286 ms). The owner's observation is explained by the sharp pass.

## Changed (ADR 1289)

- The sharp pass waits for the view to be still for its own predicted cost before it begins.
- `Landed::waited` became `Landed::drew`, measured from the render thread taking the job up.
- `QuorraWindowRenderer::new_headless` so a host can drive its render thread with no display.

No windowed run: Xvfb has no DRI3, so RADV cannot present there, and llvmpipe is no fast device.
