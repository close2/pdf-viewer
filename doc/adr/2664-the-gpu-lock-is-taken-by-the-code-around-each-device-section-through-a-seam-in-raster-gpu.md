# 2664 — The GPU lock is taken by the code around each device section, through a seam in `raster-gpu`

Session 1915, a single round (instruments). Status: **accepted** and **built** in
`raster/crates/raster-gpu/src/section.rs`, `tools/conformance/test-support/gpu_lock.rs` and
`gpu_lock_hook.rs`, the three clock gates and the device tests. Context: CLAUDE.md's GPU bullet, ADR
2656 (machine places), ADR 2663 (both halves of device selection). It cites no clause and moves no
ledger row. The owner, 2026-11-03, after rounds 1913 and 1914 held the shared adapter for an hour
and more: "yes! fix this. the other process was starved and only locked the GPU when it really
needed it!"

## 1. The decision

- **The span is a device section, and the code that does the work is the one that knows it.** A
  shell's `with-gpu-lock` around a gate held the lock through the build, every processor-only
  measurement and every child's start-up: 167–188 s a run through RADV, 517–620 s through OpenGL, back
  to back (round 1914's hold), and round 1913 held it around a whole `cargo nextest run`, compile
  included. Now the lock is taken around a bring-up, a frame or a gate child's device phase, and
  released before anything else.
- **The seam is in `raster-gpu`, the hook is installed by tests.** `raster_gpu::section` defines a
  [`SectionHook`] with `begin`/`end`, called around `Device::build` from the adapter's choice to the
  return, around each public frame call (`render`, `render_retained`, `build_whole`, `submit`,
  `collect`, `Presenter::present`) and around each resident upload. The hook is told the work and the
  adapter's `DeviceType`. With none installed a section is one load of an empty `OnceLock`, so the
  launch path pays nothing; no shipped host installs one. The `flock` is test support, because the
  file is a machine place and has no business in a library (ADR 2656).
- **Not at device creation.** A device lives as long as a window, so a lock tied to it would hold the
  adapter for the program's lifetime — the very starvation this replaces.
- **Not per `queue.submit`.** A frame's sections are its public calls, not the dozen submits inside
  one: a frame is the unit a waiting neighbour can interleave with, and a hook called inside the
  passes would put a lock acquisition between a frame's own passes.
- **The warm-up thread's compiles are not a section**: they are the driver's compiler on a processor
  and submit nothing.
- **Nested and process-wide.** A gate holds an outer section around a child's whole device phase and
  starts its clock inside it, so no figure contains a wait; each frame inside opens a section of its
  own, which counts a holder rather than locking again (a second `flock` on a fresh descriptor in the
  same process would deadlock).
- **Hardware only.** The hook locks a section whose adapter is not `DeviceType::Cpu`. A gate section
  taken before any adapter exists asks the environment, and counts a run as software only where both
  `VK_ICD_FILENAMES` names lavapipe and `WGPU_BACKEND=vulkan` (ADR 2663: one half alone is OpenGL on
  the card).
- **The place.** `PDF_VIEWER_GPU_LOCK` in `tools/places.sh`: `/run/lock/gpu.lock` where it exists,
  empty — no lock — where it does not (CI, a machine with an adapter of its own). It is not derived
  from `$HOME` because it is not this account's place but the machine's. The wait is bounded by
  `GPU_LOCK_TIMEOUT`, `with-gpu-lock`'s own name, 600 s by default as there.
- **Measured by itself.** `PDF_VIEWER_GPU_LOCK_LOG=<file>` appends one line per hold (process,
  outermost section, wait, hold); `tools/gpu-holds.sh <file>` prints the count, the longest hold and
  the total.

## 2. What it reaches, and what it does not

The clock gates: `launch_path`'s `bring-up`, `first-page` and `page-turn` phases each hold from before
their clock to their return, and its `open`, `count-open`, `calibrate` and cold-cache phases take
nothing; `turn_path`'s round holds from its first bring-up to its last frame, after the page is read
and before its calibration; the toolkit gate holds around each device-lane launch and not the
processor lane's. The parents' fingerprint bring-ups take it through the hook. The device tests of
`render-raster`, `raster-gpu`, `viewer-device` and `render-gpu` install the hook or, for vello's
device and the raw-`wgpu` helpers the seam does not see, hold a section for the device's life — by
the adapter's own type where the helper has one, and always for vello, whose instance enumerates
every backend and reaches the card through OpenGL whatever `WGPU_BACKEND` says. `raster-gpu`'s tests include the support files from outside `raster/`, a cost to the
sub-project's separability that is accepted while it is a member of this workspace.

Not reached: a `#[cfg(test)]` module inside `raster-gpu/src` that brings a device up, and the
examples other than `frame_budget`; each is run by hand, and `with-gpu-lock` around one such command
is what CLAUDE.md now reserves the shell lock for.

## 3. Cost

Every device section takes and releases a lock: two system calls and a mutex, microseconds beside a
frame. A neighbour's device work can now land between two of a gate's children, which is the point;
a clock never includes the wait because the clock starts inside the hold.
