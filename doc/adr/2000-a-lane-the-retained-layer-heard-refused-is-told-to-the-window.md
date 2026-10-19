# 2000 — A lane the retained layer heard refused is told to the window, and answers a job already asked

Session 1582, slot 6 of batch ninety-two, ADR 1984's residue under the project owner's "switch to
code improvements … until I say otherwise" of 2026-10-14. Status: **accepted, built**. Context: ADR
1968 (a lane the adapter refused is remembered for the device's life), ADR 1984 (a retained picture
of a refused lane is not asked). Code: `crates/viewer-ui/src/bin/quorra/renderer/lanes.rs`
(`RefusedLanes::{learn, said}`), `renderer/retained.rs` (`retain`), `renderer/thread.rs`,
`renderer.rs` (`Finished::Lane`), `renderer/window.rs` (`collect`). It cites no clause and moves no
ledger row.

## 1. What was left

Two memories hold the lanes the adapter refused. The window's (`Refusals`, the event thread's) is
read at the ask and reaches a frame as the job's `Known`. The render thread's (`RefusedLanes`) keeps
a retained picture off a refused lane. The render thread learned every lane a frame heard, but the
window never heard a lane the retained layer's own ask heard first. So after a retained picture's
refusal, the next frame of that lane was sent to the device and refused again.

**The premise held.** Driven with ADR 1984's three plants (the count pipeline broken, the compute
assist forced on, the atlas reporting no room), `tracemonkey.pdf#zoom=1500` under `--coverage` auto
draws its first frames on the `Gpu` lane. That lane never reaches the compute pipelines. The idle
retained picture asks under `Cpu` and is refused. Fourteen notches out, the first `Cpu` frame was
asked of the device and refused, in both runs at HEAD's routing.

## 2. The decision

- **The render thread tells the window once** (`Finished::Lane`, on the channel every other result
  takes). It sends this when the retained layer's ask teaches it a lane it did not hold. The window
  keeps that lane as it keeps one a frame heard. The message goes before any frame behind it, so
  the next ask after the window collects carries it as `Known`.
- **The render thread's record answers a job asked before the window read the message.** A job can
  be in the channel while the retained picture is refused, because the window asks without waiting
  for idle work. Such a job carries no `Known`. If the device would be asked for it (not predicted
  the processor's, not finished from the page drawn while it was read), the render thread gives it
  the lane's refusal, in the adapter's words, and the device is not asked.
- **One memory was the other choice**: keep the lanes on the render thread alone, since the device
  is that thread's. It was not taken because the window decides more at the ask than whether to ask
  the device. A lane-refused frame skips the forecast, is marked as the processor's from the ask,
  and is said on the trace as not asked. Moving that decision would change what the window
  predicts, warns of and says for every refused frame. The message keeps all of that as it was, and
  the record covers the one gap.

## 3. Held by

- `lanes::tests::the_render_thread_keeps_a_lane_s_refusal_and_not_a_frame_s`: a lane is new, and
  told, once. `lanes::tests::a_lane_the_retained_layer_heard_answers_the_next_frame_on_either_side`:
  the render thread's record answers a frame of the lane, the window answers it once told, and the
  two answers are equal. With `RefusedLanes::said` planted to answer nothing, the second test fails
  (trap 13).
- **Driven** under an Xvfb of the round's own, with the three plants in an export only. A fourth
  variable restored HEAD's routing (no message, no answer from the record) as the control in the
  same binary. Two runs each, eight page turns at fit, or fourteen notches out from 1500 % then four
  turns:

  | scenario | routing | frames asked under `Cpu` | retained pictures asked | refused asks |
  |---|---|---|---|---|
  | at fit (a frame hears it first) | HEAD | 1, 1 | 0, 0 | 1, 1 |
  | at fit | this | 1, 1 | 0, 0 | 1, 1 |
  | zoomed out (the retained layer hears it first) | HEAD | 1, 1 | 1, 1 | 2, 2 |
  | zoomed out | this | 0, 0 | 1, 1 | 1, 1 |

  So the device is asked for a refused lane once per window, whichever path hears it first.
