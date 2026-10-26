# 2408 — A zoom step shows the blurry frame where its sharp frame misses the refresh, and a column's always does

Status: accepted — a verdict on the owner's question and three levers priced; nothing is built.
Session 1787, 2026-10-26, a "loop" round added to batch 118 at the project owner's word. Context:
ADR 0378 and 0391 (rule 5: a stand-in only once the frame misses a refresh), ADR 0442 (a column is
one texture under one placement), ADR 0443 (the retained low-resolution pages), ADR 0702 (a scene
in page space is one page), ADR 1843 (a toolkit window stands in at every notch), ADR 1930 (a page
drawn as its window), ADR 2315 (a toolkit notch's floor), traps 262, 271 and 274, habit 164.

## 1. The question

The owner, 2026-10-26: "I have the impression, that zooming out still uses blurry outlines, even
though we should be fast enough. (Zooming in is at least in some documents, always crisp, but
zooming out, always shows the blurry frame). Please investigate in a round, if this is necessary,
because we really aren't fast enough, or if this is a bug."

**The answer, in the owner's terms: necessary, not a bug — but not for the reason "we aren't fast
enough" suggests everywhere.** On a simple page quorra is fast enough and shows no blurry frame in
either direction. On a column of two text pages — the arrangement ISO 32000-2 opens in — every step
misses a 120 Hz refresh by a few milliseconds, and the stand-in there is the blurriest one this
program has. On a heavy page every step misses by two to six refreshes. In the toolkit windows a
stand-in goes up at every notch by design. And whatever a zoom-out *reveals* has no picture sharper
than the 512-pixel retained page, which is the one true asymmetry between in and out.

## 2. How it was measured

quorra on the real Radeon 890M (`rendering with AMD Radeon 890M Graphics (RADV STRIX1)`), natively
on Wayland in a nested headless KWin of the agent's own at the owner's mode (2880 × 1800, scale
1.6), the window 800 × 1600 device pixels, Control+wheel notches sent through KWin's EIS input 1.5 s
apart — four in, six out, two in — HEAD's build, one `--clock --nice 0` hold, every arm started at a
one-minute load of 0.95 to 2.98, the render node held by no process of this account between arms,
`pp_dpm_sclk` at its middle level before and after each arm. Each step's notch-to-sharp time is read
from the window's own trace: the `zoom` command's line to the first frame line that presents a
device draw (whole milliseconds), against the owner's 120 Hz refresh of 8.33 ms. The nested
compositor runs at 60 Hz, so its own stand-in decisions are at 16.7 ms; at 120 Hz rule 5 stands in on
every step whose sharp frame takes longer than one refresh, which is the column counted below.

| page, arrangement | direction | inside 8.33 ms | notch to sharp | device |
|---|---|---|---|---|
| `issue19802.pdf` p. 1, one page | in | 8 of 12 | 2–27 ms | 2.1–26.5 ms |
| | out | 12 of 12 | 2–6 ms | 1.8–4.9 ms |
| ISO 32000-2 p. 101, `SinglePage` | in | 8 of 12 | 5–9 ms | 3.5–6.3 ms |
| | out | 6 of 12 | 6–10 ms | 3.2–7.4 ms |
| ISO 32000-2 pp. 101–102, `OneColumn` | in | 1 of 12 | 8–24 ms | 5.3–8.0 ms |
| | out | 1 of 12 | 8–15 ms | 4.8–9.9 ms |
| `22060_A1_01_Plans.pdf` p. 1 | in | 0 of 12 | 12–51 ms | 11.4–36.5 ms |
| | out | 0 of 12 | 12–29 ms | 11.1–20.8 ms |

The simple page's four misses are its first two steps in each run: the compute lane's first frame
(19 and 27 ms, a first use read from the frame's wall time, trap 271) and the step predicted from it.

## 3. Why each row is what it is

- **One page** builds its scene in page space (ADR 0702), so a step replays the compute lane's
  records (`re-placed` on every step) and lands in 2 to 10 ms. Rule 5's prediction follows the last
  built frame and was under the refresh on every step whose frame was, so nothing stood in.
- **A column** keeps the baked path — `render_raster::present::page_space` admits one page only — so
  every step rebuilds its scene and walks it whole (`building its scene`, `encoded` on all 24): the
  sharp frame is its device time plus 2 to 6 ms of rebuild, and misses. **And its stand-in is the
  512-pixel retained pages alone**: the arrangement's gap is `viewer_core::layout`'s 8 logical
  pixels, which a zoom does not scale, so under one placement the second page lands 1.5 to 4 device
  pixels from where the new view puts it, past `AGREEMENT`'s half pixel, and the last frame is
  refused as "not one affine of itself" (ADR 0442). A one-page view's stand-in is the last frame
  resampled; a column's is the low-resolution layer.
- **A heavy page** draws 11 to 37 ms on the device whatever the view; no step can land inside.
- **The toolkit windows** (`quorra-qt` measured in the same hold, `quorra-gtk` and the Windows window
  through the same `viewer_host::stand_in`) stand in at the notch itself (ADR 1843), each page from
  its own last frame; on the ISO column the first picture at the new size landed 3 to 18 ms after
  the notch and the whole page 7 to 28 ms after it.
- **The asymmetry is geometry.** A zoom-in's new view lies inside the old one, so the last frame
  covers all of it, a little soft. A zoom-out's new view contains the old one: the band it reveals —
  and a page entering the view, `page 102 stands in … its retained 362x512 page` in the Qt trace —
  has never been drawn at this size, and the only picture of it is the retained page
  (`viewer_device::stale::PROXY_EDGE`, `viewer_host::stand_in::RETAINED_EDGE`, both 512).

**No defect was found on the in/out paths.** The lane is the same in both directions (the compute
lane on a moved view); `re-placed` against `encoded` follows the page count, not the direction; the
prediction is the last built frame in both; every sharp frame was presented on the tick it landed
(the frame line follows the render thread's landing within the same millisecond or two, because
the render thread wakes the loop, ADR 1926); no notch was dropped.

## 4. The levers, priced at their ceilings (habit 164), none built

1. **A column in page space, one encode per page** (ADR 0702's exclusion lifted). Its ceiling is the
   column costing what its first page costs alone, the `SinglePage` row: 14 of 24 steps inside
   8.33 ms against the column's 2 of 24. It brings a column to the edge of a 120 Hz refresh and not
   inside it; it needs raster to draw several viewports in one frame.
2. **A column's stand-in carries the last frame page by page**, each page under its own placement
   and the gap kept, as the retained layer is already composed. No frame gets faster; the cost is
   one textured quad per page, inside the 0.4 to 4.5 ms a stand-in's present took in this round's runs. What it buys
   is the visible half of the complaint: a column's stand-in becomes the last frame resampled, as a
   one-page view's is, instead of the 512-pixel pages. It needs the presenter to draw the base under
   a source rectangle per page.
3. **The toolkit windows wait one refresh before standing in**, as quorra's rule 5 does. In the Qt
   trace the first picture at the new size landed inside 8.33 ms after 9 of 12 notches, so at its
   ceiling three notches in four would show no stand-in; it re-opens ADR 1843's choice, so it is the
   owner's to take.

The band a zoom-out reveals stays blurry until its frame lands under all three: no picture of it
sharper than the retained page exists, and a larger retained page costs a quadrupled idle draw and
quadrupled memory per retained page for every edge doubling.
