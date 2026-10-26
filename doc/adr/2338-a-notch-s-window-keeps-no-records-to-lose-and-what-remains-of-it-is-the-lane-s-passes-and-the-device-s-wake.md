# 2338 — A notch's window keeps no records to lose, and what remains of it is the lane's passes and the device's wake

Status: rejected — ADR 2323's second remainder, priced at its ceiling (habit 164) and not built.
Session 1752, 2026-10-26, a "measure" round under the project owner's switch to code improvements
of 2026-10-14. Context: ADRs 2322, 2323, 0700; raster's ADRs 0087 (records), 0095 (the lane's one
submission). Code read: `viewer_device::renderer::read_back::draw_until_told_to_stop`,
`render_raster::QuorraRasterizer::{rasterize, render_whole, rasterize_frame}`,
`raster_gpu::RetainedScene::prepare`, `raster_gpu::compute::dispatch_chain`.

## 1. The premise, checked

ADR 2323 read the per-window remainder of the device call (1.8 to 5.8 ms, against `zoom_frame`'s
1.3 to 1.5) as the compute lane walking again rather than replaying its records, because the page
is drawn on `Cpu` between two notches' windows. **The mechanism is not in the code.** The read-back
thread draws every job through `Rasterizer::rasterize`, whose `render_whole` discards the frame slot
and builds the scene fresh, so no window is ever record-replayed, with a `Cpu` draw between or
without one. Record replay (raster's ADR 0087) lives only in `rasterize_frame`'s retained slot,
which is what `zoom_frame` draws through.

## 2. The ceiling, before anything was built

A scratch example in an export of HEAD, deleted with it, drew the read-back thread's sequence on
one device: the page at scale 1.6 on `Cpu`, then three notches 400 ms apart (1.25, 1.5625,
1.953125), each the 990 × 1554 window on `Compute` and then the page on `Cpu`; ISO 32000-2 page 20,
the Radeon 890M through RADV headless, as the instrument's adapter line printed. Two `--clock`
holds, three interleaved passes, three devices an arm. The second hold's arms started once the
one-minute load was under 3 (2.0 to 3.0); the first hold's waited out the 60 s cap at loads of 4
to 26 and is used here only for the two speeds below. The window's device job, ms, gated arms:

| arm | window 2 | window 3 |
|---|---|---|
| as shipped | 4.7–5.9 | 4.4–5.2 |
| no `Cpu` page between the windows | 4.2–6.2 | 4.6–5.8 |
| record-replayed through the retained slot, no page between | 4.1–6.3 | 4.6–5.7 |
| no page and no idle between the windows | 2.9–4.9 | 2.6–3.8 |

Records and the lane switch are worth nothing measurable: the replay arm replayed (its windows 2
and 3 report `RecordReplayed`, 18 of 18) and its jobs are the shipped ones. What the job holds, in
the first three arms' 32 windows: the walk 0.2 to 0.7, the count pass 0.35 to 0.72, emit and
deposit 0.76 to 1.42, the read back 0.97 to 1.91, and 1.3 to 2.0 ms that the last row removes —
the device coming back from the 400 ms between two notches, which nothing this tree submits can
shorten without submitting work that draws no picture.

**The device has two speeds.** In runs of seconds, emit and deposit took 3.2 to 4.2 ms instead of
0.8 to 1.6, in every arm alike, and the window's job was then 9.3 to 11.4 ms: most of the first
hold, once in the second at a load of 2.64, and six of six bare runs while a sibling's corpus walk
held the render node. amdgpu's reported shader clock read 600 to 840 MHz in both. ADR 2323's spread
of 1.8 to 5.8 ms is this; its cause is the device's state, not the lane's choice.

## 3. Decision

1. **Nothing is built.** A `Cpu` draw drops no records, because the read-back lane keeps none, and
   keeping them moves no window.
2. **The read-back module no longer says the window replays records**; it says why keeping them
   would buy nothing, citing this ADR. No behaviour changed.
3. **What is left between notch 3 and one 120 Hz frame** is the lane's own passes in the device's
   slow state, the read back, and the device's wake after an idle notch interval. None is raster's
   record or the lane rule; the passes are the coverage shaders'.

## Costs

None in code. The figures are headless on the owner's adapter; the owner's loop was not run,
because nothing was built for a display to show.
