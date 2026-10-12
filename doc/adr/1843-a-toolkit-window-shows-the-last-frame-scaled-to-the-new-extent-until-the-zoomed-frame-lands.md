# 1843 — A toolkit window shows the last frame scaled to the new extent until the zoomed frame lands

Session 1503. Status: **accepted** and **built** in `quorra-qt` and `quorra-gtk`. Reported by the
project owner on 2026-10-10: "quorra-qt … would show incorrect frames during zooming (quorra was
fine)". Context: ADRs 0378, 0461, 0668, 1825 (§3), 1832. Code: `crates/viewer-host/src/stand_in.rs`
(`placed`, `Placed`), `viewer-qt`'s `Host::frames` and `PageArea::paintEvent`, `viewer-gtk`'s
`Host::placements` and `refresh`. Tests: `stand_in.rs`'s four. Drive: step 82 `zoom_stand_in`.

## 1. What was found

Reproduced under Xvfb in **both** toolkit windows, not only Qt (`scratchpad/r1503/zoom-shots.sh`):
on a page whose frame takes most of a second, a photograph 0.15 s after a Control + wheel notch in
held the page's red block at **202 × 201** pixels where the landed frame holds it at **251 × 252**,
and moved up and left by the change of origin; a notch out held it at 251 × 252 against 202 × 201.
The cause is the brief's premise, read in the code: `viewer_core::Viewer::arrange` gives every page
still on the screen its new origin and extent at once (`OnScreen::origin`, `OnScreen::raster`) and
keeps its pixels (`OnScreen::frame`) until `RenderReady` replaces them, and both windows painted
those pixels at the new origin at their own size — Qt's `drawImage(QPointF, …)`, GTK's
`GtkPicture` sized by its texture. The old page at the old size anchored at the new corner is the
incorrect frame. `quorra` was fine because `stale.rs` reprojects.

## 2. Decision

1. **Between a zoom and its frame, a page's last frame is shown scaled to the extent the page has
   now, at the origin it has now.** `viewer_host::stand_in::placed` pairs each frame with the
   extent `Query::PageGeometry` answers; both windows paint into that rectangle.
2. **Only a uniform scale stands in.** A zoom multiplies both sides by one factor and each side is
   rounded up, so the old and new extents agree in shape to within the four sides' rounding; a
   change of shape (a rotation of anything but a square) keeps the pixels' own size, as before,
   because stretching them would show the page distorted.
3. **The toolkit scales, not a resample of ours.** Qt's raster engine draws the image into the
   rectangle with `SmoothPixmapTransform` — bilinear and clipped to the region repainted, so its
   cost is the window's and not the page's; GTK lays the picture out at its texture's size and,
   for a stand-in only, `GtkFixed`'s child transform scales it — the translation `GtkFixed::move`
   always was with a scale after it, drawn by GSK linearly; every other frame keeps `move`, so its
   drawing path is the one it had. A size request cannot do GTK's half: `GtkFixed` gives a
   child its natural size, and a picture's is its texture's, so a notch out stayed large (measured,
   the first build of this round). ADR 1832's banded resample was the brief's shape; it is not
   needed where the toolkit draws the scale itself, and ADR 1832 §4 named the toolkit's scaled
   copy as where a cheaper stand-in would be found.
4. **ADR 1825's part-way picture replaces the stand-in**, which answers ADR 1825 §3's "revisit":
   the part-way picture is the true page at the new size, the stand-in a wrong picture shown
   deliberately, and a wrong picture is shown only while nothing true at that size exists.
   `over_frames` already puts the part-way picture in the frame's place before `placed` sees it.

`doc/todo/37`'s rules, as a toolkit window holds them. Rule 1: the stand-in is the viewer's own
frame with an extent beside it, and the `RenderReady` that sets the new pixels makes the extents
agree, so nothing has to be cleared. Rule 2 is kept by construction rather than by a private
module: `placed` returns a rectangle and no pixels, the viewer's `Query::Frame` still answers the
true raster, and the scaled pixels exist only in the toolkit's backing store, which no gate, oracle
or harness reads. Rule 3: the window says `page N stands in at W×H, its w×h frame scaled until the
frame for this size lands` on its `frames` topic. Rule 4: the scale is drawn on the toolkit thread
beside the render on the drawing thread, and costs 1.9 ms for a notch either way on a 1000 × 1000
window, 2.9 and 4.2 ms from a 5000 × 7000 page (`scratchpad/r1503/paintcost.cpp`, one core, Qt's
raster engine, best of fifteen), against a 0.1–0.2 ms copy at the image's own size. Rule 5 has no
second answer here: where `quorra` refuses a stand-in it shows the previous picture unmoved, but a
toolkit window repaints from the viewer's state on every pump, and its only other picture is the
old frame misplaced.

## 3. Measured

One `plus` notch on the owner's drawing after its first frame, both arms alternating, two runs each,
one clock hold per display (`scratchpad/r1503/witness.sh`, `owner-display.sh`); seconds from the
notch to the first picture of the page at its new size, by each window's own trace. Before, that
picture was ADR 1825's first part-way picture, `GLIMPSE_AFTER` past the draw's start, and until
then the page sat misplaced at its old size. **Xvfb**: `quorra-qt` 0.900, 0.592 → **0.000, 0.001**;
`quorra-gtk` 1.192, 0.626 → **0.006, 0.007**. **Owner's display** (XWayland, 890M): `quorra-qt`
1.212, 1.458 → **0.002, 0.001**; `quorra-gtk` 1.104, 1.320 → **0.000, 0.000**. The notch to the
landed frame did not move beyond its spread (Xvfb 1.45–2.22 s before, 1.54–2.60 s after; owner's
display 2.24–2.74 s before, 2.27–3.33 s after, at load 6–9 with the batch's other rounds building),
and no frame line's wait exceeds its draw by more than 2 ms on the small fixture in either arm. The
photographs taken 0.2 s after the notch hold the page where the landed frame puts it, on both
displays; before, the old picture sat at the new corner, cut off on the left.

## 4. What it does not do

A page that arrives on the screen with the zoom — a notch out under a column — has no frame and
shows the ground until it is drawn; `quorra`'s retained low-resolution page (ADR 0443) is what
would fill it, and these hosts hold none. A rotation still shows the old frame at its own size until
the turned one lands. A scroll needs nothing: the extent does not change and the origin always
moved.
