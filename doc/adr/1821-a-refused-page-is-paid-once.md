# 1821 — A refused page is paid once

Session 1492. Status: **accepted** and **built**. Asked by the project owner on 2026-10-10 as part of
"improve performance — loading and display time, zooming in and out especially". Context: ADRs
0125, 0391, 0442, 0657, 0707, 0725, 1289, 1466, 1811. Code: `Refusals`, `Flight`, `draw` and the
render thread's loop in `crates/viewer-ui/src/bin/quorra/renderer.rs`; `on_the_device` and `adopt`
in `surface.rs`; `mind_a_long_draw` and `stop_the_long_draw` in `app.rs`. Cites no clause.

## 1. What a refused frame cost, and the premise

`CLAUDE.md` keeps the processor for the frame the graphics device refuses. On the owner's drawing
every frame is refused — "frame needs 423206976 scene-derived bytes, over the stated budget of
268435456" — and the brief's premise held in the code: `draw` asked the device, raster built the
scene and priced it before saying no (`ComputePersist::price` runs after the edges are counted),
and then `compose_pages` drew the whole arrangement, uninterruptibly, on the render thread. Three
further costs were not in the brief and are measured by the trace line this round adds
(`refused frame: the device refused in … ms; the processor drew it in … ms`):

- **the device was asked again at every frame**, a pan included: 1.43–1.45 s an attempt on Xvfb's
  llvmpipe (2.26 s cold), before the processor began;
- **the sharp pass of ADR 0699 asked it again** after every fallback frame, read from the code: the
  frame's `FrameCost` is the upload of the processor's raster — 3.2 to 5.2 ms in the frame lines —
  so `sharp_pass_affordable` predicted a cheap pass and `draw_sharp` built and priced the scene at
  twice the size, to be refused in silence;
- **the retained page of ADR 0443 asked it again** after every frame (`draw_whole_page`'s silent
  refusal "retried once per frame at most"); "0 of 0 retained" pages in every stand-in line says it
  never once succeeded on this page.

And a zoom gesture paid all of it twice: the frame of the first notch ran to the end — 10.0 s on
Xvfb — and only then was the last notch's frame asked for.

## 2. The decision

**A frame the device refused is not asked of it again; the processor's half of a refused frame can
be taken back; and it is taken back at the first notch that leaves its magnification.**

1. `Window::refusals` remembers a refusal by the arrangement's pictures and the linear part of its
   transform — the pages and the magnification — and a job that matches carries the device's
   sentence, so `draw` goes straight to the processor. Only a refusal that is a fact about the frame
   is kept (`remembered`: the budgets and limits, and what the display list carries); a device lost
   or an allocation refused is asked again, as before. Thirty-two entries, newest first.
   **Priced against keying the whole transform**: raster culls what reaches no pixel, so a pan at a
   magnification well past the fit could bring a refused frame under the budget, and remembering by
   magnification draws that pan on the processor — the oracle, the same pixels, slower. Keying the
   whole view would pay the attempt at every pan. On the witness under llvmpipe the refusal states
   the same bytes at the fit and ten notches in, so the case was not met there.
2. The job carries a `pdf_render::Interrupt`, and `compose_pages_interruptibly` honours it. A raise
   while the device is still being asked is honoured where it refuses: the processor is not started.
   An abandoned frame lands with no pixels and settles nothing (`adopt_composed`'s rule, ADR 0657).
3. `Window::superseded` raises it where the view's magnification differs from the frame in flight,
   or where the frame could not stand in at all. **This departs from ADR 0657's rule on purpose**:
   the composer lets a zoom of one page run on because an ordinary frame costs hundreds of
   milliseconds; a refusal states that this page is past what a frame was budgeted to be, so
   finishing a magnification the gesture has left puts the rest of a processor draw in front of
   the frame the person stops at. The raise is harmless to a frame the device accepts. A pan is not
   a reason: the frame in flight is of the view's magnification and carries onto it.
4. A view the device refused is not sharpened by it (the 2× pass is the same view at four times the
   pixels), and a page whose retained picture it refused is not asked again for that ink.
5. **The warning reaches it.** `Window::overlong` is true while the processor draws a refused frame
   past `viewer_host::drawing::WARN`, so `mind_a_long_draw` says `still_drawing` and Escape stops it
   (`Window::abandon`, with `declined` so the next tick does not ask again). A frame on the device
   is never overlong: nothing reaches it (ADR 0725). The terminal's note is said when the title's
   sentence changes rather than at every frame (ADR 1466's rule, now on both).

## 3. What it measured

Ten `plus` notches 300 ms apart on `/tmp/demo_heavy_engineering_A1.pdf`, `--clock`, the time from
the last notch to the first real frame after it, HEAD against HEAD with this change exported apart
(`scratchpad/r1492/`, md5s in the record). **Xvfb, llvmpipe: 10.94, 18.91 and 11.08 s before; 3.15
and 3.91 s after.** The owner's display is in the record.

## 4. What this does not do

- The device is still asked once per new magnification — 1.4 s on llvmpipe, uninterruptible. A
  prediction of the refusal before the scene is built is raster's to offer (slot 4's budget work).
- A refused page's retained picture is not drawn on the processor; a zoom out still reveals the
  window's medium at the margin, as before.
- `tools/drive-windows.sh` has no step for it: a refusal needs a page past 256 MiB of scene, which
  none of its fixtures reaches inside its time.
