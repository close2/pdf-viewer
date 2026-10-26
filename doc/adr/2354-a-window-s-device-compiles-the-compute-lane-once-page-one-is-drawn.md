# 2354 — A window's device compiles the compute lane once page one is drawn

Status: implemented. Session 1760, 2026-10-26, a "measure" round under the project owner's switch to
code improvements of 2026-10-14. Context: ADR 2339 (the lever, priced and left for a round whose bar
it clears), ADRs 0700 (`lane_for`), 2298 and 2322; raster's ADRs 0018 (a warm-up thread may not
outlive its device), 0042 (the warm-up's one notifier), 0080 (the lane compiled on first use);
CLAUDE.md principle 2. Code: `raster_gpu::Options::warm_compute_lane`,
`raster_gpu::pipeline::{PipelineStore::{compute_lane, warm_compute_lane}, warm::LaneCue}`,
`render_raster::window_options`, `viewer_device::renderer::read_back`. Tests: three in
`raster_gpu::pipeline::tests`.

## 1. The first use, and who pays it

A window takes `Coverage::Compute` on every moved view of a hardware adapter (`lane_for`), and the
lane's four compute pipelines were compiled inside the first frame that took it: 1.99 to 6.12 ms of
a zoom's first window over 54 devices (ADR 2339). Raster's warm-up thread, which nobody waits on,
compiles the warm set; the lane was left out of it because a device whose host never takes the
lane would compile four pipelines for nothing.

## 2. The construction

- **An option, off by default.** `Options::warm_compute_lane` asks raster's warm-up thread to compile
  the lane. `render_raster::window_options` is `options()` with it on; the three
  `QuorraWindowRenderer` constructors (`quorra`, the toolkits' presenting renderer) and the toolkit
  windows' read-back device use it, and every corpus, oracle and gate device keeps `options()`. The
  launch gate's first-page phase builds `quorra`'s device the same way, so it measures what `quorra`
  runs.
- **After page one, not beside it.** The thread compiles the warm set and records it as before —
  `warm_up`, `wait_until_warm` and the startup figure answer for the warm set alone — and then waits
  for the device's first completed frame, cued at the end of `Device::complete` after the frame's
  readback. A first page therefore shares no processor and no driver with the lane's compile, which
  is principle 2's "nothing eager" taken literally rather than argued from a core count. A device
  that asked for nothing is never cued, so its frames pay a lock and a comparison.
- **The device's drop releases a waiting thread** before it joins it (ADR 0018), so a device dropped
  before any frame compiles nothing and does not hang. Where no thread can be spawned the lane is
  left to its first frame, as before.
- **The pipelines live in the store**, in a cell of their own beside the render pipelines: a frame
  that takes the lane while the warm-up compiles it waits in the cell instead of compiling twice,
  and a refusal is kept there and heard by the lane's first frame by name (ADR 1962). The frame that
  paid a compile, or waited for one, names it as `compute pipeline compile (first use)`.

The first version built in this round started the lane's compile right after the warm set, so it
ran beside page one; the hold's launch arms measured that version (section 3), and the cue replaced
it because the figures could not show a difference smaller than their own spread.

## 3. Measured

Two `--clock` holds, HEAD against this tree interleaved, headless on the Radeon 890M through RADV as
every instrument's adapter line printed, no process of this account holding the render node before
any arm. The first window, both levers together, is ADR 2355's table; in the second hold, the cued
version, no window of this tree's 18 named a compute compile.

**The launch path.** The second hold (loads 1.53 to 2.25, every arm started under 3) ran the launch
gate twice a tree and the toolkit gate twice a tree:

- `t2-launch_path`, time to first page by document over the two runs, ms: HEAD 25.8–31.0, this tree
  25.4–30.5, each document's two runs within 1.5 ms of the other tree's — two read lower in both of
  this tree's runs, one higher, three either way; the six first frames' digests are HEAD's. Three runs report 0 figures outside
  their bands; one of this tree's reports one, `PDF20_AN001-BPC.pdf`'s warm open, which opens the
  document and draws nothing. The bring-up 17.8–19.0 against 18.8–19.4.
- `t2-toolkit_launch`, every figure judged and inside its band in all four runs. Page one on the
  device: GTK 6.7–7.0 against 6.8–6.9, Qt 5.9–6.2 against 5.9–6.3; the first frame GTK 83.1–88.0
  against 83.9–84.6, Qt 84.6–84.7 against 81.4–81.5, where Qt's window and its device's bring-up —
  both before page one, and neither on a path this change reaches but by the option's one store —
  read 2 to 3 ms later in this tree's two runs. In the first hold, at loads near 5 and with the lane
  compiled beside page one, Qt's first frame read 95.5 against HEAD's 100.0, the other way.

So the launch is unmoved within what these gates can see, and the cue makes "unmoved" a property
of the construction rather than of the spread: before page one is handed back, a window's device
has run a lock and a store more than HEAD's.

## Costs

One option, one cue and a second wait on the warm-up thread, and a window's device compiles four
pipelines it uses only if the reader zooms. On a software adapter, where `lane_for` never takes the
lane, that compile is spent for nothing on the thread nobody waits on, after page one.
