# 1868 — A zoomed toolkit window draws the rows it shows first, as a look, and the page whole after

Session 1515, on the project owner's target of 2026-10-10, "improve performance … zooming in and
out". Status: **accepted** and **built** in `quorra-gtk` and `quorra-qt`. Replaces ADR 1859 §4's
design, which was priced and not built; amends ADR 1843 §2.4 (a part-way picture is drawn over the
stand-in rather than in its place). Context: ADRs 0139, 0219, 1758, 1819, 1825, 1837, 1841, 1843,
1859. Code: `viewer_host::drawing::{seen, Drawing::ask_showing, Drawing::seen_drawn, band_of,
draw_band, over_frames}`, `Partial::{take, finished_over}`, both hosts' `seen_of` and `NeedsRender`
arm. Tests: `drawing.rs`'s four new ones. Instrument: `viewer-host`'s `examples/band_first`. Drive:
step 82's `band_first`. Cites no clause and moves no ledger row.

## 1. What was found

**The premise held**: `Viewer::schedule` asks a toolkit window for the whole page at every
magnification (`TargetSpec::for_page`), so a notch deep into a page costs the whole page, and ADR
1859 §4 priced the band at four times the fit at 444 ms against the page's 1 303 (pool of four,
pinned).

**The hypothesis — that the band is the protocol change ADR 1859 §4 lists — did not hold, twice.**
First, nothing in the protocol has to change: the window knows the rows it shows (its own
`Query::PageGeometry` and viewport), the drawing thread is the window's, and the order in which
that thread draws a requested page's rows is its own business. The request stays a whole page and
its answer a whole page, so `quorra`, `quorra-confined`, `viewer-ffi` and the worker read both as
before, `FrameView` needs no extent of another size, and `stand_in::placed` and the retained pages
see the page rasters they always saw. Second, **the band as part of the frame lost, measured**: its
edges have to be rows the processor's planner would cut at, or a curve crossing an edge is drawn
twice at two origins (ADRs 0139, 1819, 1837); asking the planner for them reads every command's
extent on one thread — 230–490 ms on the owner's drawing at four to seven times the fit, beside
whole draws of 533–783 ms in the same process on a pool of twenty-four (`examples/band_first`) —
and the band composed from three draws reached the screen no sooner than the page drawn whole
(546–615 ms against 536–644 ms headless; 0.63–0.71 s against 0.57–0.70 s in the windows under Xvfb,
exploratory, outside a clock hold) and landed the frame about twice as late. That build is not kept.

## 2. Decision

1. **A host hands the drawing the rows of the request's target it shows** (`Drawing::ask_showing`,
   `drawing::seen` from the page's geometry and the viewport's height), and a page of a document
   behind the one in front is asked for whole.
2. **Where those rows are less than half the page, they are drawn first, as a target of their own,
   and sent to the window the moment they are finished** — a glimpse of the page drawn through every
   command, placed where the rows belong. **Then the page is drawn whole, exactly as before**, and
   that raster is the frame the viewer is answered with: byte for byte HEAD's frame. The band is a
   look, not part of the frame, because its edges are the viewport's rows and not the planner's.
   A page whose band was drawn is not glanced at again: the rows on the screen are finished, and a
   part-way glimpse of the page would put them back part-way.
3. **Half, because the band is paid for twice**: what it costs is added to the frame's landing and
   bought back only in how soon the rows a person looks at are finished; at half the page or more
   it buys almost nothing, and every page at its fit is there, so no launch draws page one twice.
4. **A part-way picture is drawn over a frame of another size, not in its place** (`over_frames`):
   the page at its last magnification, scaled by ADR 1843's stand-in, shows wherever the partial has
   no rows — the band's off-screen rest, or the strips of a page drawn in painting order that have
   not answered — so a look is a picture only of the rows it has drawn. A frame at the partial's own
   size is still replaced.
5. **A window says when the rows it shows are finished**, once a draw: `page N shown finished where
   the window shows it, T after it was asked` (`Drawing::seen_drawn`), the figure this decision
   moves; `Partial::take` counts a run of rows once when a band answers in its strips and then
   whole.

## 3. Measured

The witness, one clock hold, HEAD (`c6fb18fb`) against HEAD plus this round's hunks in their own
target, two runs an arm: eight `=` notches from the fit, each waited for until its frame lands,
seconds from the notch's command to the rows the window shows finished on it (HEAD: its frame) and,
after the slash, to the frame. The window shows less than half the page from the seventh notch; the
owner's display was its own clock hold, the loop's (trap 142).

| | notch 7, HEAD | notch 7, this | notch 8, HEAD | notch 8, this |
|---|---|---|---|---|
| Xvfb, `quorra-gtk` (2904 × 2052, 3630 × 2565) | 0.572, 0.613 | **0.358**/0.927, **0.384**/0.962 | 0.633, 0.628 | **0.353**/0.986, **0.356**/0.971 |
| Xvfb, `quorra-qt` (2952 × 2085, 3690 × 2607) | 0.589, 0.590 | **0.361**/0.972, **0.378**/0.975 | 0.614, 0.628 | **0.333**/0.973, **0.355**/0.993 |
| owner's display, `quorra-gtk` (2952 × 2085, 3690 × 2607) | 0.576, 0.559 | **0.346**/0.908, **0.360**/0.942 | 0.620, 0.607 | **0.303**/0.909, **0.305**/0.922 |
| owner's display, `quorra-qt` at 1.59 (4707 × 3325, 5883 × 4156) | 0.696, 0.693 | **0.388**/1.076, **0.381**/1.115 | 0.807, 0.815 | **0.356**/1.119, **0.358**/1.169 |

Notches one to six, where the window shows half the page or more, are both arms' whole draw,
0.48–0.57 s on Xvfb. Headless in the same hold (`examples/band_first`, the pool unset, a window of
609 × 1019 centred on the page, three rounds): at 5.86 times the fit the page drawn whole lands in
574–602 ms, the band in 313–358 and the page after it in 889–928; at 7.33 times, 634–699 against
339–345 and 971–992; the frame is HEAD's byte for byte in every round, and the planner alone, asked
as the whole draw asks it, takes 236–331 ms of the whole. Drive step 82's `band_first` on the
drive's own page: **works** in both windows (the red block 284 × 284 at the band's photograph and
the landed frame's alike in GTK, 288 × 288 in Qt); HEAD's windows never say it (the control, two
`wrong`).

## 4. What it does not do

- The whole draw is not shortened, and the frame lands later by the band's cost; a scroll during
  that draw shows the stand-in, scaled, in the rows the band did not draw.
- A band draws its rows at its own origin, so where a curve crosses its edge the look's rows are
  the page's only very nearly; the frame that replaces it within the draw is exact.
- **The planner is about half of a zoomed page's processor draw on the owner's drawing** (236–331
  of 574–699 ms), on one thread, for every request of every window — the next lever on the zoom,
  and `render-cpu`'s, not this crate's.
- A page never drawn before that a notch out brings on screen still has no picture until it is
  drawn (ADR 1866 §4).
