# 1831 — `quorra`'s processor page is shown as it is built, and the processor is asked for it where it is the faster lane

Session 1497. Status: **accepted** and **built**; the part that puts page one on the processor is
put to the owner as `doc/questions/Q393`. Asked by the project owner on 2026-10-10: "improve
performance — loading and display time, zooming in and out especially", the first frame on his
drawing `/tmp/demo_heavy_engineering_A1.pdf`. Context: ADRs 0443, 0657, 0699, 1811, 1817, 1819,
1821, 1823, 1825; trap 143. Code: `Costs`, `Prediction`, `Partial`, `draw`, `processor_half`,
`retain` and `retain_one` in `crates/viewer-ui/src/bin/quorra/renderer.rs`; `as_built`,
`put_up_as_built`, `say_the_lane` and `predicted` in `surface.rs`; `App::mind_the_share` in
`app.rs`; `compose_pages_watched`, `Assembly`, `Seen` and `Looks` in `crates/viewer-ui/src/software.rs`.
Drive: `--step page_as_built_heavy` now offered in `quorra`. Cites no clause.

## 1. What the frame cost after ADR 1823, measured first

ADR 1823 made the device draw the witness rather than refuse it, and the device's frame is host
encode, about 1.8 µs a command whatever the command's size. Measured on the witness at HEAD
`75531da7` before anything was built, last notch of ten `plus` notches 300 ms apart to the first
real frame after it (scripts under `scratchpad/r1497/`):

- **On the owner's display** (XWayland, Radeon 890M, one clock hold) the device draws every frame:
  the first in 12.5 and 19.0 s (scene 2.3–4.7 s, device 6.5–9.5 s), and a zoom gesture's last
  notch waited 15.2 s in one run and had no real frame 20.3 s later, when the run ended, in the
  other — every notch's frame is uninterruptible and each one is drawn to its end. The brief's
  hypothesis held there.
- **Under Xvfb** llvmpipe binds 128 MiB of storage and the device still refuses (ADR 1823 section 3),
  so HEAD pays the attempt, 2.2 s, and then the processor's 1.9–2.2 s: first frame 6.8–10.8 s,
  gesture 3.5–4.7 s. The hypothesis did not hold there — the figure is the refusal path's.

## 2. Decision

1. **The lane is chosen from a figure the frame states.** `Costs` keeps, per arrangement of pages,
   what each lane last charged per command; a frame's prediction on each lane is that price times
   the commands the arrangement holds, and a lane that has not drawn these pages is priced by a seed
   measured on this machine: the device at 1 788 ns a command (ADR 1823 section 4, 7 884 ms over
   4 408 405), the processor at 667 ns (ADR 1819 section 3, 2.94 s on nine strips). **A frame the
   device is predicted to draw inside `viewer_host::drawing::WARN` stays there**, whatever the
   processor would take — about 560 000 commands at the seed — so no ordinary page leaves the device;
   past it the smaller prediction draws. A page whose processor draw turned out slow (large fills)
   goes back to the device the next frame, and the trace says each prediction and whether it was
   measured or seeded (`processor chosen: …`). The device's price is read only from a frame it built
   rather than replayed, and a frame taken back measures nothing (ADR 0657's rule).
2. **The processor's draw is shown as it is built** (ADR 1825's hook). `compose_pages_watched` runs
   `compose_pages` on a scoped thread with `CpuRasterizer::showing` on every page, asks for a look at
   `GLIMPSE_AFTER` and every `GLIMPSE_EVERY`, and folds the rows into an `Assembly` — a later page's
   rows composed over the pages already finished, so a column is shown as it is too. The render
   thread uploads each look into a pair of its own and sends it as `Finished::Partial`; the window
   puts it up only where it holds no picture of those pages (the first frame, a page turn) and
   otherwise stands in with the page it has, reprojected, which is the choice `doc/pdf.js`'s viewer
   makes (`web/base_pdf_page_view.js`), taken as evidence about the approach. ADR 1811's bar in the
   title carries the share drawn; `first frame on the screen` stays the finished page.
3. **A whole-page picture of a page the processor draws is the processor's too**, and so is one
   the device refused: the device would refuse it or take as long over it as over the frame, and
   its draw cannot be taken back. It is drawn while the render thread is idle under an interrupt
   `Window::ask` raises after sending a job — after, so that the flag left under the same lock or the
   job is always there to be found.
4. A frame the processor drew is not sharpened by the device and predicts no sharp pass: its cost
   is an upload's (trap 143).

## 3. What it measured

Witness, ten notches, both arms alternating twice in one hold each; HEAD's binary
`f9b240c2…` against this change's `728bb691…`, seconds:

| | first frame | first picture part-way | last notch to real frame |
|---|---|---|---|
| owner's display, before | 18.98, 12.51 | — | > 20.3, 15.22 |
| owner's display, after | **6.73, 5.20** | 4.19, 3.30 | **1.43, 2.11** |
| Xvfb, before | 10.79, 7.58 | — | 4.66, 3.47 |
| Xvfb, after | **4.92, 6.32** | 3.31, 4.48 | **1.50, 1.48** |

The read is 2.7–4.8 s of each first frame in both arms and is slot 3's; the first picture lands
about half a second after it, which is `GLIMPSE_AFTER`.

## 4. What it does not do

- `quorra --cpu`'s composing thread still draws a frame unobserved; `compose_pages_watched` is
  the road it would take.
- The device is never measured on a page the seeds gave the processor. On this machine that is the
  right lane by both seeds; a machine whose host encode is several times faster would keep the
  processor until a round prices the device there.
- A look is uploaded into a fresh pair of textures, two window-sized allocations four times a
  second during a draw of seconds; a pool would save them and was not measured as needed.
