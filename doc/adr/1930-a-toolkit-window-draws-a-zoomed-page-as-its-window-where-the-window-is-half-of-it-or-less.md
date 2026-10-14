# 1930 — A toolkit window draws a zoomed page as its window where the window is half of it or less

Session 1547, the next build of ADR 1892 §4, under the project owner's "switch to code improvements
… until I say otherwise" of 2026-10-14. Status: **accepted** and **built** in `viewer-host`,
`quorra-gtk` and `quorra-qt`. Amends ADR 1892 §2.1 (which pages a window draws as it shows them)
and narrows ADR 1868 §2.2 (the band is drawn only for a page drawn whole). Adds nothing to the
frozen vocabulary. Context: ADRs 1841, 1843, 1866, 1868, 1892. Code: `viewer_host::oversize::{
oversized, largest_window, Oversized::{asked, answered_whole, shows}}`, both hosts' `answered`,
`take_the_windows` and first-frame test. Tests: `viewer-host/tests/oversized_pages.rs`'s three new
ones. Instrument: `viewer-host`'s `examples/window_against_whole`. Drive: step 82's
`zoomed_window`, which replaces `band_first`. Cites no clause and moves no ledger row.

## 1. What was found

**The premise held**: both toolkit windows asked for, drew and copied the whole page at every
magnification under `viewer_core::MAX_PIXELS`, and `viewer_host::oversize` was the window path for
the pages past it.

**The hypothesis — the window path with its bound lowered to the fit — held for the path and not
for the bound.** A window is drawn from a list of its own, the marks that reach it (ADR 1841), and
that is not free: on the witness at 1200 × 900, headless and exploratory (`window_against_whole`,
the `gates` profile, four threads), a window of nine tenths of the page landed after the page drawn
whole — 740–1 077 ms against 607–858 at 1.25² of the fit, 794–858 against 723–734 at 1.25³ — and a
window of two fifths of it before — 590–607 against 770–832 at 1.25⁴ — by more the deeper the zoom:
249–404 ms against 948–1 777 at 1.25⁵ and 1.25⁸, 282–357 against 24 164–44 077 at 1.25¹⁰. The
break is at half the page, the same half ADR 1868 found for the band, for the same reason: the part
of a draw that does not scale with its pixels is about half of it.

**And two faults of ADR 1892's path, which a page past the bound reached rarely and every zoomed
page now would.** A window drawn for one request was taken as holding the view for the next request
at the same target — the same page under another ink, or interpreted again — so the new ink was
never drawn and the viewer never answered. And a first frame that was a window was not a frame:
neither host sent `Command::Report` and `Command::Presented` for it, nor said its first frame, nor
opened the command line's later documents.

## 2. Decision

1. **A page is drawn as its window where the largest window of it is half its area or less**
   (`oversized`): the viewport and a quarter of it on every side, clamped to the page, against the
   page's target — or where its whole raster is past `MAX_PIXELS`, as before. Every page at the fit
   and the first notches past it is asked for, drawn and answered exactly as at HEAD.
2. **Only the window drawn for the request held answers it**; one drawn for an earlier request
   stands in until the new one lands, as a page's last frame does (ADR 1843).
3. **A notch back to a page drawn whole keeps the last window standing in**, scaled, over the
   retained picture, until the page's own frame answers (`answered_whole`) — the viewer holds no
   frame of a page it was answered `Listed` for.
4. **The first window to answer a request is a frame**: the report, the open sequence, Table 147's
   placing and the transition's face follow it as they follow a whole frame (`answered`), and a
   window drawn for the request in front counts as the first frame on the screen (`shows`).
5. **ADR 1868's band stays, for the pages it now reaches**: a page drawn whole of which the window
   shows under half the rows — a tall page a few notches into a wide window, or a neighbour scrolled
   mostly out of a continuous arrangement.

## 3. Measured

The witness, one `--clock` hold on Xvfb (`scratchpad/r1547/hold.sh`), HEAD (`eabb91be`) against
HEAD plus this round's paths, each its own export and target (digests in `scratchpad/r1547/*.export`):
ten `=` notches from the fit, each waited for until its frame or window lands.

The window's viewport is about 610 × 1 019 under Xvfb's 1400 × 1100, so the first four notches are
drawn whole in both arms, at the same sizes, in 329–425 ms each in both; from the fifth this round
draws the window. Milliseconds from the request handed over to its frame landing ("waited"), two
runs, and the bytes copied into the toolkit after it:

| notch | `quorra-gtk` HEAD | this | `quorra-qt` HEAD | this |
|---|---|---|---|---|
| 5 | 393, 394 (9.8 MB) | **339, 407** (window 914 × 1313) | 378, 366 (10.1 MB) | **354, 357** (928 × 1335) |
| 6 | 394, 410 (15.3 MB) | **300, 314** (6.3 MB) | 374, 378 (15.8 MB) | **305, 310** (6.3 MB) |
| 7 | 772, 740 (23.8 MB) | **232, 238** | 745, 713 (24.6 MB) | **223, 244** |
| 8 | 766, 764 (37.2 MB) | **161, 175** | 772, 782 (38.5 MB) | **160, 177** |
| 9 | 802, 888 (58.2 MB) | **148, 143** | 800, 843 (60.1 MB) | **151, 157** |
| 10 | 948, 945 (90.9 MB) | **126, 130** (6.3 MB) | 925, 930 (93.9 MB) | **139, 132** (6.3 MB) |

The copy into the toolkit at an update went from 5.2–5.6 ms at the tenth notch to 0.3–0.4 ms;
on the owner's larger window and deeper zoom it is ADR 1892's 65–250 ms, which this bounds to the
window's. The owner's display was not taken (optional; the Xvfb hold is the contract's).

The window's pixels against the same rectangle of the page drawn whole (`window_against_whole`):
from 1.25² to 1.25⁸ of the fit, 83–1 920 of 2.4 million pixels differ, by at most 2 levels, at the
edges of marks, which is ADR 1817's seam; at 1.25¹⁰, 6 889 differ, 914 by more than 8 and at most
by 56, at the antialiased edges of strokes on a page 11 176 pixels wide, as often lighter as darker.
Photographed side by side the two are the same drawing. Which of the two is the nearer to §10.7.4's
coverage at that size is not settled here (section 4).

## 4. What this does not do

- The window's cull is the price a window pays and is `render-cpu`'s and `pdf-render`'s: a culled
  list costs about what the whole one does for a window that is most of the page.
- The difference at 1.25¹⁰ is measured and not explained; a round that reads it against §10.7.4
  owns it, and the whole page at that size took 24–44 s to draw where its window took a third of
  a second.
- The confined window keeps the bound (ADR 1921 §3).
