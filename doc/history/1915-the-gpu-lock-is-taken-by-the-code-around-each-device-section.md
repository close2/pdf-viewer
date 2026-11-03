# 1915 — The GPU lock is taken by the code around each device section

Session 1915, a single round (instruments) on the server VM. ADR 2664; ADR 2665 and Q776 unused.

**Question.** Rounds 1913–1914 held the GPU lock for whole runs (167–620 s each), and the other
user starved (owner, 2026-11-03). Can it be held around device work only?

**Built (ADR 2664).** `raster_gpu::section`'s hook is told of each bring-up, frame and upload with
the adapter's type, and costs one `OnceLock` load when none is installed; test support's `gpu_lock.rs`
(the `flock`, nested, bounded, hardware only; file `PDF_VIEWER_GPU_LOCK` in `tools/places.sh`) and
`gpu_lock_hook.rs` install it in the device tests; the three clock gates hold each child's device
section from before its clock. `tools/gpu-holds.sh` sums `PDF_VIEWER_GPU_LOCK_LOG`. `tools/bands.sh`
lost both `with-gpu-lock`s; CLAUDE.md's GPU bullet says the code locks the device section.

**Holds, RADV, no shell lock** (before: one hold the length of the run):

| run | before | holds | longest | total locked |
|---|---|---|---|---|
| launch gate | 188 s | 169 | 0.72 s (first-page phase) | 43.4 s of 603 s |
| turn gate | 167 s | 168 | 2.2 s a round; 6.1 s the planted busy device | 102.2 s of 449 s |
| toolkit gate | 13 s | 18 | 0.55 s | 6.3 s of 489 s |
| `render-raster` nextest | the run | 940 | 11.5 s (a hung frame, until the reset) | 33.3 s of 64 s |

**Found.** The turn gate waited 26 s once for another holder. On RADV `render-raster`'s suite hung
the RX 580 three times (`clip_that_cuts_nothing`, `retained_frame::a_zoom_step_misses`,
`sub_pixel_coverage`, by the kernel's ring-timeout lines); each reset cancels every context on the
card, the other user's too. Code that enumerates every backend reaches the card through OpenGL under
lavapipe's two halves: vello's `headless_gpu` hung it once more, unlocked, as its hold trusted the
environment; vello's and the raw-`wgpu` holds now go by the adapter or always lock.

**Gates.** Tier 1 fmt, `clippy -D warnings`, eight crates: exit 0. `render-raster --test gpu_lock`
(a planted holder delays a declared section and a RADV bring-up ≥ 0.75 s, not a lavapipe one): 4/4,
and both seam tests fail with the hook's test inverted. `render-raster` nextest, RADV: exit 100, 61
passed, 57 failed (46 reset collateral). Turn gate: exit 101, 7 of 33 outside, all below. Launch:
exit 101, 19 outside — 8 clocks below, `cold_open_ms` above, 10 counted (ADR 2660). Toolkit: exit
101, `device_up_ms` below for both. `cargo test -p conformance`: exit 0. Lavapipe nextest, six
crates: exit 100, 25 failed (23 of the reset, a vello stripe, `function_conformance` via OpenGL).

**Unfinished.** `raster-gpu/src`'s test devices and most examples take no lock; the hangs and the
bands' low edges are the next round's.
