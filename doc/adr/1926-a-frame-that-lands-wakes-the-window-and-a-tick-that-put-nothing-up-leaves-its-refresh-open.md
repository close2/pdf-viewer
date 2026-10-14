# 1926 — A frame that lands wakes the window, and a tick that put nothing up leaves its refresh open

Session 1545, slot 6 of batch eighty-six. Status: **accepted, built**. Context: ADR 1911 §5 (the
question), ADR 0383 and `doc/todo/36` (the cadence), ADR 0526 (the in-flight frame's tick), ADR 0391
(the render thread). It cites no clause and moves no ledger row.

## 1. The question

ADR 1911 §5 found that a page turn's frame waits for a tick after it is drawn. WTPDF's frame was
asked at the key, drawn in 4.0 ms and collected 12 ms after the key. The render thread wakes the
event loop after the sharp pass, a look and a scene, and not after `Finished::Frame`. The guess was
that a wake there would bring a turn's page forward by up to one period.

## 2. The wake alone moves nothing

Measured under Xvfb, which presents at the 60 Hz floor. With the wake added and nothing else, AN001's
turns read 21, 21 and 22 ms from key to frame. HEAD read 17, 17 and 18 ms. In both traces the tick at
the key says `nothing to show`, and the frame is collected one period later.

The cause is the clock. `Cadence::owed` armed the tick by calling `presented` once the tick had
passed. So a tick that put nothing up was counted as a present, and `redraw_requested` deferred any
redraw before the next tick. That included the one the wake asked for. The frame was held by a
refresh that nothing had used.

## 3. What is built

- **The render thread wakes the loop after `Finished::Frame`** (`renderer/thread.rs`), as it does for
  the other three kinds of news.
- **The cadence keeps two instants.** `due` is the tick, which `about_to_wait` waits until.
  `presentable` is the first tick after the last present, and only a present moves it. `owed` moves
  the tick and nothing else. `redraw_requested` asks `may_present` rather than `due`.
- **A look put up as built is a present**, so it moves `presentable`. No frame line counts it, so
  `as_built` and its composer counterpart call `presented` themselves. One present per refresh still
  holds, because every present moves `presentable`.

The test `a_tick_that_put_nothing_up_leaves_the_refresh_open` holds the rule. With the old `owed`
put back, it fails.

## 4. What it measured

One `--clock` hold under Xvfb at the 60 Hz floor, with llvmpipe. The six launch documents were each
run three times per arm, alternating, four `Next` keys per run, timing each key to the frame that
answered it. Each key was matched to the next frame line that said `presented`. A key on the
document's last page answers nothing, and its match is the frame Escape causes about 720 ms later,
so those keys are left out.

| document | HEAD, key to frame | this ADR |
|---|---|---|
| AN001 | 21–26 ms, median 23 | 9–12 ms, median 11 |
| WTPDF | 15–21 ms, median 18 | 8–17 ms, median 10 |
| ISO 32000-2, after its first turn | 28–52 ms, median 36.5 | 23–52 ms, median 32.5 |

ISO 32000-2's frames take longer than a period. Its gain is the part of a period by which the frame
had overrun a tick. The first present moved by no more than the machine's spread: page one's
medians differed by 1–107 ms, in both directions, while the load fell from 5.4 to 1.2. That is
consistent with the at-most-one-period that the mechanism can give, and it proves nothing beyond it.

## 5. What this does not reach

- **`launch_path` and `turn_path` cannot see it.** `launch_path` measures the launch minus winit's
  loop and present. `turn_path` is `render-raster`'s and does not link `viewer-ui`. Both bands stand
  unchanged, and the window's trace is the instrument.
- **The owner's display was not measured.** This round's permissions refused the probe and the
  unlock, so the 120 Hz figure is still owed.
- **The `--cpu` window's composing thread has no waker.** Its frames still wait for the tick, which
  is the same change for a later round to make.
- **`quorra-confined` has no cadence.** Its release binary has the same digest at HEAD and here.
