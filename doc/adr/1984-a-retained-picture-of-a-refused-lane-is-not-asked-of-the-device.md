# 1984 — A retained picture of a refused lane is not asked of the device

Session 1574, slot 4 of batch ninety-one. Status: **accepted, built**. Context: ADR 1968 (a lane the
adapter refused is remembered for the device's life), ADR 1831 (the retained layer draws on the lane
its frame is drawn on), ADR 0090 (raster's compute assist). Code:
`crates/viewer-ui/src/bin/quorra/renderer/retained.rs` (`LANE`, `retain`, `draw_whole_page`),
`renderer/lanes.rs` (`RefusedLanes`), `renderer/thread.rs`. It cites no clause and moves no ledger
row.

## 1. What was left

The retained layer draws a whole page at 512 px while the render thread is idle, under
`Coverage::Cpu`. Where the page's frame was the device's, it asked the device first and took the
processor where the device refused. ADR 1968 keeps a refused lane for the window's frames, but the
memory is the event thread's (`Refusals`) and reaches a frame only as its job's `Known`. A retained
picture is drawn while no job is in hand, so it asked again for every page.

**The premise held in part.** Under `Cpu` the compute assist sends to the compute lane only the
tiles the atlas will not hold. A retained picture is small, so it reaches the lane only when the
atlas has no room. Driven with ADR 1962's plant alone (the count pipeline broken, the assist forced
on), all 9 pictures of `tracemonkey.pdf` were asked and all 9 drawn. The device refused them only
with the atlas planted full as well.

## 2. The decision

- **The render thread keeps the refused lanes itself** (`RefusedLanes`). It learns them from every
  frame that carried a lane's refusal or heard one, and from the retained layer's own ask. Both are
  sorted by the frames' rule (`lanes::remembered`). A frame's refusal is not a fact about another
  page, so it is not kept. Nothing ages out, for ADR 1968's reason: the device is this thread's.
- **A retained picture of a refused lane goes to the processor unasked**, as a frame of that lane
  does (ADR 1968). The lane is still set first, so the processor's picture is uploaded under the
  lane it used after a refusal. The upload is one image with no chrome, and the compute assist
  takes only solid fills.

## 3. The cost, written down

After a `Cpu` frame is refused, every retained picture goes to the processor, including one the
device could have drawn because its tiles fitted the atlas. This is ADR 1968 §3's cost for frames,
now paid by retained pictures too. They are drawn in idle time, and the processor's draw can be
interrupted when a job arrives (`Window::ask`). A device draw cannot be interrupted. The
alternative was to keep only the retained layer's own refusal. That pays one encode per window and
leaves on the device a picture whose frame was refused under the same lane.

## 4. Held by

- `lanes::tests::the_render_thread_keeps_a_lane_s_refusal_and_not_a_frame_s`. With `learn` planted
  to keep nothing, it fails at its second assertion (trap 13).
- **Driven** under an Xvfb of the round's own, `--coverage gpu`, eight page turns, with three plants
  in one export only: the count pipeline broken, the assist forced on, and the atlas reporting no
  room. A fourth variable restored HEAD's routing as the control. Asks of the device for a retained
  picture: `tracemonkey.pdf` 9 with the control, 1 with this change; `PDF20_AN001-BPC.pdf` 5 and 6
  with the control in two runs, 1 with this change. Under `--coverage` auto, both builds asked 0
  times: the `Cpu` frames were refused, so their pages were the processor's already. Without
  plants, all 9 pictures were the device's. Settled captures were identical, 0 pixels differing. A
  capture taken 2 s after the last turn differed by 931 pixels, and the same build differed from
  itself by as much against one taken 8 s after. That capture was of a view still settling.
