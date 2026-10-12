# 1853 — A page transition's last frame is the page it moves to, and nothing else

Session 1508. Status: **accepted** and **built** in `quorra`. Reported by the project owner on
2026-10-12: "when having animations between pages, the last frame shows for a fraction of a second
something incorrect … I think it's graphical elements of the next page already", on the device and
under `--cpu` alike. Context: ADRs 0230, 0378, 0391, 0443, 0457, 1216, 1299, 1831. Code:
`crates/viewer-ui/src/bin/quorra/stale.rs` (`Depicts`, `Stale::settled`, `Proxies::wanted`),
`presentation.rs` (`End`, `App::transition_frame`), `surface.rs` (`App::present`). Tests:
`stale.rs::a_transition_frame_settles_no_view_and_is_never_retained`,
`transition.rs::the_last_frame_is_the_arriving_page_byte_for_byte`. Drive: step 83
`transition_frames`.

## 1. What was found

Reproduced under Xvfb on `examples/presentation_fixture`'s slides, every picture of the window
photographed with `xwd` through a `Wipe` (`scratchpad/r1508/repro.sh`, `check.py`): two consecutive
photographs at the transition's end, up for 17–32 ms on the device and 12–23 ms under `--cpu`, held
**379 585 pixels** (47 % of the 800 × 1000 window) that were neither the page being left nor the page
arrived at — the arriving slide drawn 1.25 times too large and moved up, its white square 124
pixels wide where the page's is 100. 1.25 is the slide's own magnification (800 / 640). The trace
named it: `approximated: … the real frame is being drawn`.

The cause is not this week's. A transition frame went to the presenter as a `Placed` carrying the
arriving page's `Picture` at the identity target, because the frame is already in window pixels.
Since ADR 0391 a frame lands on a later tick than the one that asked for it, and the landing calls
`Stale::settled` with what it drew — so the last transition frame was recorded as *the arriving
page, settled at the identity*. `App::present` called `Stale::forget` on each transition tick, but
before the adopt, which re-recorded it. The first tick after the transition then saw the page
asked for at its real placement against a page settled at the identity, a magnification change,
and where rule 5 predicted a miss it put up ADR 0378's reprojection: the picture of two pages
moved by one page's transform. Read in the code and not photographed: the retained layer (ADR
0443) could also keep a transition frame as the page's low-resolution picture, by the same
identity. The brief's hypothesis — ADR 1831's looks or ADR 1832's stand-in — did not hold; whether
the fault showed depended on the prediction, which is why it was a fraction of a second and not
every time.

## 2. Decision

1. **A placed list says whether its pixels are its page.** `stale::Depicts::{Page, Transition}`
   on `Placed`. A transition frame keeps the arriving page's identity, because the window does
   hold that page once the frame at progress 1 is up — so a page still being built behind it is
   not put up over it part-way (`Window::shows_as_built` reads `of`).
2. **A landed transition frame settles nothing** (`Stale::settled`): the window then holds no view
   to carry from, so the frame after it is the page's own or nothing, and the frame's cost does
   not feed the page's prediction. **The retained layer never keeps one** (`Proxies::wanted`).
3. **The tick that finds `/D` run out draws the frame at progress 1** (`presentation::End`), once.
   `viewer_host::Clock::frame` ends a transition by drawing nothing, so the window otherwise left
   the last frame it happened to draw — a sliver of the old page — up until the page landed.
   Table 164 states `/D` as "[t]he duration of the transition effect", so at `/D` the effect is
   over and the screen is the page moved to; `viewer_core::transition` already draws that frame as
   the arriving face alone, and the new test holds it byte for byte over every shaped style.
   Kept in quorra and not in the clock: the two toolkit windows rasterise a frame and put the page
   up in the same turn, and a clock that returned one more frame would leave Qt's `playing`
   raster up for good, because its timer stops when `animating()` turns false.

## 3. What it costs, and what stays owed

Two `Arc` clones per transition and one extra frame at its end. The device arm of step 83 caught
the planted fault in neither of two runs here and the `--cpu` arm in both, because the
reprojection needed a predicted miss; the step runs `quorra` in both. `quorra-gtk` fails step 83
on a different and smaller seam — 2 970 pixels along the page's own edge, in mid-transition
frames, where a face's antialiased edge row differs from the page's — which is the toolkit host's
and is left to it. The owner's display was not needed: the fault is in which picture is presented,
and both lanes reproduced it headless.
