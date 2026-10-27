# Q657 — Do the toolkit windows wait one refresh before standing in?

Asked by round 1791, on ADR 2408's third lever and the owner's words of 2026-10-26: "I have the
impression, that zooming out still uses blurry outlines, even though we should be fast enough."
Answering yes re-opens ADR 1843's choice, which is why it is the owner's; nothing of it is built.

## What the windows do now

- **`quorra`** stands in only where a zoom step's sharp frame misses the refresh (rule 5 of
  `doc/todo/37`, ADRs 0384 and 0391): the frame is predicted from the last frame that built a
  picture and observed once its render is out past one period. A step whose frame lands inside the
  refresh shows no blurred picture at all.
- **`quorra-qt`, `quorra-gtk` and the Windows window** stand in at the notch itself, every notch
  (ADR 1843, `viewer_host::stand_in`): each page's last frame is scaled into the extent the page has
  now, because painting it at its own size at the new origin was the "incorrect frames during
  zooming" the owner reported on 2026-10-10. There is no prediction and no wait.

## The figures, from round 1787 (ADR 2408 §2 and §3)

Measured on the Radeon 890M, natively on Wayland in a nested headless KWin at the owner's mode
(2880 × 1800, scale 1.6), Control+wheel notches through KWin's EIS input, against the owner's
120 Hz refresh of 8.33 ms. In `quorra-qt` on ISO 32000-2 pp. 101–102 as the `OneColumn` it opens
in, the first picture at the new size landed **3 to 18 ms** after the notch and the whole page
**7 to 28 ms** after it; the first picture landed inside 8.33 ms after **9 of 12** notches. So at
its ceiling, waiting one refresh would show no stand-in on three notches in four, and a stand-in
one refresh later than now on the fourth.

## The question

**Do the toolkit windows hold the picture they have for one refresh after a zoom, and stand in
only where the zoomed frame has not landed by then, as `quorra` does?**

*Recommendation*: **yes**, built once in `viewer_host::stand_in` so that all three windows take it
from one rule: for one period after a zoom command each window keeps painting the pages it had at
the geometry it had — the whole old picture, not old pixels at the new geometry, which is ADR
1843's defect — and switches to the scaled stand-in at the first refresh the frame has not landed
by. The cost is one refresh of latency before a stand-in appears on a step that misses, which is
the price `quorra` already pays by design; what it buys is the sharp frame with no blurred refresh
before it on most notches of an ordinary page. The drive's step 82 (`zoom_stand_in`) keeps its
sentence on a page whose frame misses, and a second document whose frame lands inside the refresh
is added to it, where the sentence must not appear.

## What the tree does meanwhile

The toolkit windows stand in at every notch, as ADR 1843 decided. `quorra`'s column now stands in
from its last frame page by page rather than from the 512-pixel retained pages (ADR 2416), which
is the visible half of the complaint in the window that already waits.
