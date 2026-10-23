# 2275 — A zoom gesture holds the point it was pointed at, and the deep-zoom drift was the centring

Status: implemented. Session 1720, 2026-10-23.

## Context

Rounds 1708 and 1716 left the deep-zoom drift unattributed: driven deep with Control and the wheel,
the page point under the pointer moved. The brief's hypothesis was a rounding — the anchor computed
in logical pixels and rounded to device pixels at each step. It did not hold. On a nested KWin at
1.6, the pointer put on a red square and ten Control notches sent in and ten out through KWin's own
EIS input (`tools/zoom-probe-wayland.py --drift`), each window's `zoom-geometry` receipt — where the
core put the page after the step — separates the two candidates:

- **the window's placement** of the frame stayed within 1.1 device pixels of the core's prediction
  at every step, in both windows, and did not grow: that is the whole-pixel origin of ADR 2214;
- **the core's prediction carried the whole drift**: 212.8 device pixels in GTK's window and 348.2
  in Qt's at 9.3 times, 20.5 and 38.1 user space units of the page, and none of it in the axis the
  fitted page already filled. A headless drive of `viewer-core` alone read 0.00 pixels of error in
  that axis over twenty steps.

The cause is the arrangement, not an arithmetic: while the page is narrower than the viewport it is
centred and its scroll is zero, so no step can keep the pointer's point under the pointer
(`Open::hold` says so); each step read the point under the pointer afresh, found the one the
centring had left, and every later step held *that* point exactly, so the offset grew by the step's
1.25 each notch after. In a window 2560 pixels wide a Letter page's point moved from 200 to 251.8
units across the first three steps, 926 device pixels at 17.9 times. Ten steps back out retraced it.

## Decision

1. **A gesture holds the point it was pointed at.** `Open::pointed` keeps, from the first step
   about a pointer, the page point under it, and a further step about the same pointer from exactly
   the magnification, viewport, page and settled scroll the last step left puts that point under
   the pointer as far as the clamp lets it. Anything else that moves the page — a scroll, a key, a
   turn, a resize, a view restored — leaves a scroll or a magnification the next step does not find,
   and the next step starts a new gesture from the point under the pointer then. The first step of
   every gesture is computed exactly as before.
2. **GTK reads the pointer on `enter`** as well as on motion: a pointer that arrived without moving
   anchored the first Control+wheel step at the viewport's centre, which a receipt showed as
   `at=None` after the pointer had been put over the page.

After, by the same probe: the square's offset from the pointer at 9.3 times is 0.57 units in GTK's
window and 0.78 in Qt's, each under what one device pixel is worth on the fitted page (0.90, 1.02)
— the distance a pointer on a whole logical pixel can be from the square's centre — and ten steps
out land where the gesture began. `a_zoom_gesture_holds_the_point_it_was_pointed_at` in
`viewer-core`'s headless tests is the case without a window: 234.6 units on HEAD, the pointed 100.

## Costs

Forty lines in `Open` and a field. A gesture whose page is centred for some of its steps shows the
pointed point away from the pointer during those steps, as before, and brings it back once the page
is large enough — the alternative, holding whatever the centring left, is the drift.
