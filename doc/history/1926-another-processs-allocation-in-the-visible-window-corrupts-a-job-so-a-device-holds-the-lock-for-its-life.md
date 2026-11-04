# 1926 — Another process's allocation in the visible window corrupts a job, so a device holds the lock for its life

Session 1926, a single round (pixels + instruments) on the server VM. ADR 2675, trap 320; ADR 2676
and Q778 unused. Evidence in the agent scratch's `r1926/`. **Why:** A777 allowed any number of runs
on the RX 580 to find the 14:02 hang (ADR 2665), the lock held across any reset.

**The runs (ADR 2675 §1).** `tools/hang-capture.sh` gained `--command` (a parallel `nextest` from an
archive built first), `--no-radv-debug` and `--wall`; 52 holds, the longest 46 s. (a) The clip test
alone without `RADV_DEBUG`: pass. (b) The three files in parallel: 6 of 21 wrong, no fault; serially
21 of 21. Eight copies of one test: 4–5 of 8 wrong. A 60-line C "hog" churning `DEVICE_LOCAL|
HOST_VISIBLE` memory (the 256 MiB window) beside one copy: 6 of 6 failed, twice with `VM fault … page
94741925, write from 'TC0'` — address `0x5A5A5A5000`, the hog's fill byte — then a ring timeout and a
BACO reset (reset 8, 07:57:28–29, lock 07:57:16.3–07:57:41.9; reset 9, 08:06:44–45, lock
08:06:32.3–08:06:57.6), the lock released only after the reset and a `vulkaninfo` probe. Through
OpenGL (radeonsi) at 300 MiB: 2 of 2 failed, faulting at that page. Passing: churn while the test is
stopped (4), churn only between our sections (3), a hog that holds without churning, processor
load, no shader cache. Validation (core + sync) on RADV: silent, in a failing run too.

**The cause (§2).** Below both userspace drivers: a job reads another process's fresh memory where
its own descriptors should be, when that process evicts in the window while the job is in flight.
Not ours nor wgpu's — an OpenGL context names no address. Three C victims did not reproduce it in
about 1 000 iterations, so the reproducer is our test plus the hog.

**The guard (§3).** The per-section lock serialised submissions, not allocations, which a device
makes anywhere in its life (trap 320). `raster_gpu` tells its hook `DeviceWork::Lifetime`, adapter
choice to drop; the test hook locks it and times the other sections for the hang marker itself.
After: 8 copies, 0 of 16 wrong; the three files 21 of 21 three times, and twice beside a locked hog;
the old binaries, as control, 5 of 21 wrong. Cost: one device at a time, 11 s instead of 3.3 s.

**The ban (§4).** `render-raster`'s suite on the card: 119 of 120, the one asking for lavapipe; no
fault. `raster-gpu`'s: 369 passed, all 415 failures ask for llvmpipe by name. The withheld files are
cleared; `render-gpu` and `viewer-device` wait for one scripted run. The report is §5, for `drm/amd`.
**Unfinished:** the kernel-side mechanism (debugfs unreadable, no `umr`), a wgpu-free reproducer.

**Gates.** `cargo fmt --check` of the six crates compiling the lock support: exit 0. `clippy -D
warnings` of `raster-gpu`, `render-raster`, `render-gpu`, `viewer-device`, `viewer-host`,
`viewer-ui`: exit 0. `nextest` of `raster-gpu` and `render-raster` on lavapipe: 904 passed, 8
skipped, exit 0; 933 bring-ups `type=Cpu`, no hold. `cargo test -p conformance` after `git add -N`
(`CARGO_INCREMENTAL=0`, an ICE): exit 101 on this record's length, then exit 0.
