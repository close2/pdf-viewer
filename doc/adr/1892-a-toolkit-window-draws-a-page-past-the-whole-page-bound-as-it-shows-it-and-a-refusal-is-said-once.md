# 1892 — A toolkit window draws a page past the whole-page bound as it shows it, and a refusal is said once

Session 1528, on the project owner's report of 2026-10-14: "quorra-qt (and gtk) go into an endless
rejection loop when zooming in into the heavy demo pdf, as the CPU refuses and quorra-qt still tries
to show every frame. (The actual bug is the rejection, but it doesn't make sense to try again every
frame)". Status: **accepted** and **built** in `viewer-core`, `viewer-host`, `quorra-gtk` and
`quorra-qt`. Adds one `Command` to the frozen vocabulary (`doc/ui-boundary.md`). Context: ADRs
0640, 0657, 1821, 1841, 1843, 1866, 1868. Code: `viewer_core::{Command::Oversize, Oversize}`,
`Viewer::raster_budget`, `schedule`'s refusal and `OnScreen::refused`; `viewer_host::oversize`;
`Drawing::{ask_window, windows}`, `draw_window`, `Partial::window`; both hosts' `NeedsRender` arm,
`ask_for_windows`, `take_the_windows` and frame placement; `viewer_host::trace::described`. Tests:
`viewer-core/tests/headless.rs`'s two new ones, `viewer-host/tests/oversized_pages.rs`. Drive: step
82's `zoom_past_the_bound`. Cites no clause and moves no ledger row.

## 1. What was found

Reproduced on the owner's display before anything was built (`tmp/r1528/repro.sh`): `quorra-qt` at
1594×1718 opened the witness, fourteen `=` notches crossed `viewer_core::MAX_PIXELS` (2²⁸) at the
twelfth, and **51 refusal sentences were said in ten seconds of no input but the pointer**. The
brief's premise named the wrong refusal: `viewer.rs`'s `Event::Refused` is a document restriction.
The one said was `schedule`'s `Event::Reported` — "this page cannot be drawn at this size" — which
is pushed at every settle, and every command settles, a pointer move included. So the loop was a
mouse over the window: at each motion event the viewer said the refusal again, the window rewrote
its status bar and copied its frames into `QImage`s again. Under Xvfb with no pointer it was said
once a notch in Qt and twice in GTK.

Two more costs were found on the way. **The `events` trace formatted a whole `Command` with `{:?}`
to keep 120 characters of it**: `RenderReady` carries the raster, so a 718 MB frame was written out
as seven hundred million numbers, and the toolkit's thread stood still 8–11 s at every landed frame
past twelve thousand pixels on a side. And a toolkit window copies every frame it shows into the
toolkit's image at every update (65–250 ms a pump for a 0.5–1 GB page), which this decision bounds
past the bound only (section 4).

## 2. Decision

1. **The bound stays, and applies where the allocation is.** `Command::Oversize(Oversize::Listed)`
   says the host draws a page whose whole raster would be past `MAX_PIXELS` from its list, as much
   of it as its window shows. The viewer then asks for such a page at its whole target, and the
   host answers `Rendered::Listed` — the outcome ADR 0640 made for a host that takes one page's
   list and not the next one's. Every page under the bound is asked for, drawn and answered exactly
   as before, so no frame a gate digests moves. `Oversize::Refused` is the default; the confined
   worker refuses as before and the command does not cross to it (`Uncarried`).
2. **The window is the viewport and a quarter of it on every side**, inside the page, drawn as a
   target of its own (the page's transform with the window's corner at the origin), which
   `CpuRasterizer` draws from the marks that reach it (ADR 1841). `viewer_host::oversize::Oversized`
   holds the request, asks for a window where neither the one drawn nor the one asked holds what the
   viewport shows, and answers the viewer `Listed` when the first window lands. A scroll inside the
   margin repaints from the window; one past it asks again. A window taken back before it lands is
   declined until the view leaves it, as an abandoned page is (ADR 1821's rule).
3. **What stands in is what stood in before**: the viewer keeps its last frame while the request is
   outstanding (ADR 1843), the retained picture under a window once it is answered (ADR 1866), and a
   window of the page at the last magnification, scaled, while the next one is drawn.
4. **A refusal is said once for the view it is about.** `OnScreen::refused` holds the magnification
   and interpretation a page was refused at; `schedule` says the refusal when that pair changes and
   not otherwise, for every host.
5. **The trace formats at most what it prints** (`viewer_host::trace::described`).

## 3. What it measured

Before and after on Xvfb and the owner's display are the round's record (`doc/history/1528`).

## 4. What this does not do

- A page under the bound is still drawn whole, and its frame copied whole into the toolkit at
  every update. Drawing every zoomed page as a window — `quorra`'s shape — would bound both, and is
  the next build; it moves the band of ADR 1868 and every frame past the fit, so it owes arms.
- A window is not glanced at band-first; it is small, and is glimpsed in painting order.
