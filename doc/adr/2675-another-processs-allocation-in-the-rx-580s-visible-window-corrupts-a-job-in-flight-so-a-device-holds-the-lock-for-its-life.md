# 2675 — Another process's allocation in the RX 580's visible window corrupts a job in flight, so a test's device holds the lock for its whole life

Session 1926, a single round (pixels + instruments). Status: **accepted**; the guard is **built** in
`raster/crates/raster-gpu/src/section.rs` (`DeviceWork::Lifetime`), `device.rs` and
`device/construct.rs`, and in `tools/conformance/test-support/gpu_lock_hook.rs` and `gpu_lock.rs`;
`tools/hang-capture.sh` gained a command mode and a mode without `RADV_DEBUG`. Context: ADR 2664 (the
lock around each device section), ADR 2665 (the resets, the hang marker, the drafted upstream
report), ADR 2668 (the clip test alone under the hang mode), the owner's A777 (any number of runs,
the shared lock held across any reset). It cites no clause and moves no ledger row.

## 1. What the card does, measured

Every run below went through `tools/hang-capture.sh` on RADV with `RADV_DEBUG` unset, each its own
hold of `/run/lock/gpu.lock`, the in-process lock pointed at a private file. The "hog" is a 60-line C
Vulkan program written for this round: it allocates 1 MiB blocks of `DEVICE_LOCAL | HOST_VISIBLE`
memory — on this card the 256 MiB processor-visible window of 4 GiB of VRAM (no resizable BAR) —
writes `0x5A` into each through a mapping, and frees them, in a loop. It draws nothing.

| run | result |
|---|---|
| clip test alone, `--exact` (A777 a) | pass |
| the three withheld files in one parallel `nextest` (A777 b) | 15 passed, 6 failed — wrong pixels, no fault |
| the same, `--test-threads=1` | 21 passed |
| 8 parallel copies of `a_sub_pixel_rules_cap_carries_its_own_area`, three runs | 4, 4 and 5 of 8 failed; 2 and 4 copies pass; the window peaked at 220 MiB |
| that test alone: under 64 busy loops; with Mesa's shader cache off; beside a hog *holding* 235 MiB | pass, pass, pass |
| that test alone beside a hog *churning* 240 MiB; 100 MiB | failed 6 of 6 (one under `RADV_DEBUG=allbos`, which did not help), two of them a VM fault and a ring timeout with a BACO reset (§2); passed |
| the same through OpenGL (`WGPU_BACKEND=gl`, radeonsi) at 240 MiB / 300 MiB | pass 4 of 4 / failed 2 of 2, each with VM faults |
| churn while the test is stopped (`SIGSTOP`) and continued after, four stop times | pass 4 of 4 |
| churn only while holding the private lock, between our sections | pass 3 of 3 |
| with Khronos validation (core + synchronisation) on RADV, alone and in a failing churned run | no message either time |

## 2. The cause, and its size

Both RADV resets were preceded by `VM fault … at page 94741925, write from 'TC0'`, and both OpenGL
failures by a read from `TC0` at the same page: `VM_CONTEXT1_PROTECTION_FAULT_ADDR 0x05A5A5A5`, the
address `0x5A5A5A5000` — **the hog's fill byte, used by our process's shader as an address**. A
victim's GPU has read another process's freshly written memory where its own descriptors or
constants should be. No Vulkan or OpenGL program can do that to another: the two userspace drivers
share nothing above the kernel but `libdrm_amdgpu`, and an OpenGL context names no address at all.
So the defect is below the API — the `amdgpu` kernel driver's eviction from the visible window
(TTM), the shared library, the PCI passthrough or the card — and not in this program or in wgpu. It
needs both halves at once: a job of the victim in flight, and an allocation that evicts in the
window. Neither alone does it (stopped, interleaved and held runs above).

Two shader-free and one-shader C victims (a fill-and-copy, a compute shader through a descriptor
set, a persistently mapped block), each checking every word, did not fail beside the same churn in
about 1 000 iterations, so the trigger needs more of what a wgpu frame does than they do, and the
reproducer for the report is still our test with the hog.

This explains 14:02: three parallel test files, each process allocating outside its sections while
another's frame was in flight, until the window was full.

## 3. The guard

ADR 2664 locked each bring-up, frame and upload, because those submit. That serialises work and not
memory: a device allocates in encoding, on the warm-up thread and at its drop, outside every one of
them. So `raster_gpu` now tells its hook one more section, `DeviceWork::Lifetime`, from the
adapter's choice to the device's drop (the field holding it is declared last, so it ends after
every other field has released its memory); the test hook takes the lock for it, and times every
other section against the hang bound on its own, since a device may outlive nine seconds. Nothing
changes in a shipped host, which installs no hook.

After it, on the card: 8 copies, twice, 0 of 16 failed; the three files in parallel, three times,
21 of 21; the same with a hog churning 300 MiB under the private lock beside them, twice, 21 of 21;
the unchanged binaries in the same shape, the control, 5 of 21 failed. The whole `render-raster`
suite: 119 of 120, the one failure asking for lavapipe by name, no fault; `raster-gpu`'s: 369
passed, every one of the 415 failures asking for llvmpipe by name, no fault.

**Cost.** Device tests on the card run one device at a time: the three files take 11 s instead of
3.3 s, which is what `--test-threads=1` takes. A hold is now a device's life — 80 ms for the cap
test's devices, seconds for the longest tests — rather than a frame's.

## 4. The ban, re-argued

ADR 2665 §3 ruled out every hardware suite until the hang was understood. It is understood as far
as our side can act. `render-raster`'s suite may run on the RX 580 again under the section hook, the
hang marker and nothing else; `raster-gpu`'s may, though most of it asks for llvmpipe by name and
belongs on lavapipe; the three withheld files are cleared. `render-gpu` and `viewer-device` were
not run on the card here: vello's tests hold the lock for the whole test, which is the lifetime,
and they stay withheld until one run through the script shows it. The guard protects us from each
other and protects the other user only while their processes allocate on the card inside their own
holds, which A776's reading says they do.

## 5. The upstream report (to `drm/amd`, not Mesa)

> **amdgpu (Polaris10, RX 580, no resizable BAR, PCI passthrough): a job reads another process's
> memory while that process churns CPU-visible VRAM.** Kernel 6.12.115, Mesa 25.0.7. Process A
> renders through wgpu (Vulkan/RADV, or OpenGL/radeonsi) small frames; process B, drawing nothing,
> allocates 1 MiB `DEVICE_LOCAL|HOST_VISIBLE` blocks up to 240–300 MiB, writes 0x5A into each and
> frees them, in a loop. A's frames come back wrong, and on RADV the job writes through
> `0x5A5A5A5000` (`VM fault … page 94741925, write from 'TC0'`), times out, and the card is BACO
> reset; on radeonsi a read faults at the same page. Stopping A while B churns, or letting B churn
> only between A's submissions, removes it. A's command streams are clean under the Khronos
> validation layer. Reproducer: B is 60 lines of C; A is a wgpu test binary.
