# 2322 — A toolkit notch draws the window it shows first, on the compute lane, and the page after

Status: implemented. Session 1744, 2026-10-25, a "measure" round under the project owner's switch
to code improvements and bug fixes of 2026-10-14 and his ask of 2026-08-25 that a zoom happen on
the device without re-encoding the page (ADR 0368). Context: ADRs 2314 and 2315 (the split and the
lever this prices), 2070 (the read-back lane), 1843 and 1930 (the stand-ins), 2132 (a window on the
page's grid), 0700 (`lane_for`), raster's ADRs 0082 and 0087. Code: `viewer_device::renderer::
read_back` (`ReadBack::ahead_of`, `Queued`, `window_drawn`), `viewer_device::lane::same_transform`,
`viewer_host::drawing::{PageLane::offer, Handed::windows}`, `viewer_host::oversize::Oversized::
{ahead, place}`, `NativeWindow::{shown_of, take_the_lane}`; both toolkits' `host/stand_in.rs`.
Tests: `read_back`'s two new, `tests/oversized_pages.rs`'s two new.

## 1. The premise, checked

- **The window's size.** The brief priced a 1238 × 1994 window; that is Qt's first Wayland size at
  the whole scale 2 before the compositor states 1.6 (ADR 2267's held record, 619 × 997 logical).
  At 1.6 round 1740's three runs show a page area of 990 × 1554 device pixels, and that is what is
  priced: at the notches that reach the device it is 38, 49 and 31 % of the pixels drawn whole
  (the first notch draws pages 20 and 21), not a quarter to a third throughout.
- **Why the read-back lane takes `coverage_for`.** ADR 2070 §2.2 states it as the launch rule per
  page and argues nothing more; its own module says a page is drawn in the lane `quorra` would draw
  it in, and `quorra` takes `lane_for`'s compute lane for a moved view.

## 2. The ceiling, before anything was built (habit 164)

Headless, one `--clock` hold at a load of 0.8, `zoom_frame` at 1.6 on page 20, 1740's sequence,
minimum of five, two passes; the device frame at the three notches, ms:

| lane | the page | the window 990 × 1554 | the window with its margin, 1485 × 2331 |
|---|---|---|---|
| `Cpu` | 8.9–10.6, 10.9–12.2, 12.5–15.1 | 8.8–9.0, 7.6–8.3, 6.5–7.9 | 8.7–9.9, 10.0–10.3, 11.7 |
| `Compute`, replayed | 3.2–3.3, 3.9–4.3, 5.1–5.8 | 2.7–2.9, 2.6–2.8, 2.4 | 4.8–4.9, 4.4–4.5, 5.1–5.2 |

On `Cpu` a window saves little: the glyph atlas rasterises each glyph *shape* at the new linear
part, and a window keeps most shapes. Put in place of the owner's device frames, with copies
scaled by the window's share and everything else of the settled notch kept (1740's three runs,
3.1 to 4.7 ms outside the device job), only the margin-less window on `Compute` passes: 6.0 to 8.3
ms at the second notch, 5.1 to 6.3 at the third, 7.9 to 9.6 at the first. A window with
`oversize`'s margin is nearly the page at these zooms, and `Compute` on the page leaves one notch
of six under. So the lever is built in the one form the ceiling admits.

## 3. Decision

1. **Where a page's view moved, the read-back lane draws the part the viewport shows first**
   (`viewer_host::oversize::shown`, the window on the page's grid, ADR 2132): its target is the page's with the
   window's corner moved to the origin, its lane is `lane_for`'s for a moved view — `Compute` on a
   device worth the name, the launch rule on a software adapter — and it is handed back as a
   window, not an answer. The page follows on the launch rule's lane exactly as before, so what a
   window shows at rest is HEAD's frame byte for byte. A launch, a view that did not move and a
   page the viewport holds whole are drawn as before, with nothing ahead.
2. **The window stands in at its own size until the page lands** (`Oversized::ahead`): held only
   for the newest unanswered request of its page, so one that lands late is dropped; where it holds
   all the viewport shows of the page, what stood in under it — the scaled last frame, the retained
   picture — is not placed, so it is neither copied into a `QImage` nor painted.
3. **A window in flight when its page is asked again is let go when it lands**, and a window
   refused is remembered as a refusal of that lane for the page, the page drawn as before. Neither
   is priced into the lane's forecast, which is the page's.

## 4. What it changes on the screen

The window is on the device's compute lane and the page after it on `Cpu`. Against the page's own
pixels at the window, `examples/window_against_page` (round 1744's scratch, deleted) read 8.07 to
8.45 % of the window's pixels different, the worst by 14 to 16 of 255, all on glyph edges (the
picture was looked at, trap 1); the same window drawn on `Cpu` differs from the page by 0 to 4
pixels by 1 — the window's translation is exact. So for the 10 to 15 ms between the window and the
page the reader sees what `quorra` shows on every moved view (raster's ADR 0082's tolerance), and
then HEAD's page.

## Costs

A second device job per notch (the window, 4.7 to 8.9 ms headless, ADR 2323) before the page's; in a fast spin the next
notch's window waits for the page in flight, as HEAD's next page did. Two windows' worth of memory
per page shown while the page is coming. The window's lane differs from the page's for one frame.
