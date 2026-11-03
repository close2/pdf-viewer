# 2665 — The shared card's resets are read from the kernel, a lavapipe run stays off the card, and a hang stops every later device test

Session 1916, a single round (pixels + instruments). Status: **accepted**; the pinning and the guard are
**built** in `tools/conformance/test-support/backends.rs`, `gpu_lock.rs`, `gpu_lock_hook.rs`,
`tools/gpu-holds.sh`, `crates/render-gpu/src/lib.rs` and the device tests named below; **the cause of
the RADV hangs is not found**, and Q776 asks the owner for the one run that could find it. Context: ADR
2663 (both halves of device selection), ADR 2664 (the lock around each device section), raster's ADR
0017 (the backend set is the host's to name). It cites no clause and moves no ledger row.

## 1. What the kernel says about the five resets

`journalctl -k` names a process for every timeout ("Process information: process … thread …"), which
round 1915 did not read. The five hard resets of 2026-11-03, with the lines around each:

| time | process the kernel names | what precedes the timeout | route |
|---|---|---|---|
| 09:38:57 | `headless_quorra`, thread `headless_q:cs0` | 09:38:47 `GPU fault … VM fault … at page 3076, write from 'TC3'` | OpenGL (round 1913's run named an ICD file this machine does not have, and set no `WGPU_BACKEND`) |
| 14:02:33 | `clip_that_cuts_nothing`, thread `a_clip_that_contains…` | nothing; no soft recovery is reported, so no wave could be killed — the command processor itself was stuck | RADV |
| 14:02:35 | `retained_frame`, thread `a_zoom_step_misses` | 14:02:35 `gfx_v8_0_priv_reg_irq: Illegal register access in command stream`, one second after reset 2 completed | RADV |
| 14:02:49 | `sub_pixel_coverage`, thread `a_sub_pixel_rule_at_the_rasters_edge…` | 14:02:48 `ring gfx timeout, but soft recovered` (a wave killed after ten seconds), then `Illegal register access in command stream` | RADV |
| 14:50:50 | `headless_gpu` (vello), thread `headless_g:cs0` | 14:50:40 `soft recovered` | OpenGL (radeonsi's submission thread), under an environment meant for lavapipe |

Three readings follow, and each is the evidence's size and no larger.

- **Two of the five came through OpenGL**, which no run meant: one because the ICD file did not
  exist, one because vello's instance read no environment and loaded every backend. Section 2 closes
  that route.
- **Two of the three RADV resets are the next process's first frame after a recovery.** Round 1915's
  lock log shows `a_zoom_step_misses` waiting 562 ms for the lock while the clip test's frame hung,
  bringing a device up as reset 2 completed, and faulting within 1.4 s; the rasters-edge test waited
  10.1 s behind the frame that was soft-recovered at 14:02:48 and faulted 1 s later. A privileged
  register written by a command stream is not something a valid Vulkan command stream can ask for —
  RADV writes the stream, the application cannot — so these two are read as the card or its driver
  failing after a recovery, not as those tests' own faults. Run alone on RADV in this round, the
  eight of round 1915's "failed on their own" that are not in a hanging test's file passed, seven of
  them, and the eighth asks for lavapipe by name.
- **The clip test's hang and the soft-recovered wave are not explained.** The clip test draws one
  triangle on a 16 × 8 target under `Coverage::Cpu`. Every loop in the shaders that path reaches is
  bounded by a constant (`line.wgsl`'s three at eight, `image.wgsl`'s two at eight, `onto.wgsl`'s
  three channels) or by a binary search whose bounds close (`shading.wgsl`),
  and naga bounds every loop besides. On lavapipe, with the Khronos validation layer's core checks,
  synchronisation validation and GPU-assisted validation, the three named test files (21 tests) pass
  with no error of ours; the one error reported, `VUID-vkCmdCopyBufferToImage-pRegions-00173` in
  wgpu's internal pending-writes buffer, is in two other tests of `retained_frame` and is wgpu's
  staging copy, not a command this program records. raster-gpu takes no path by adapter except
  timestamp queries (both drivers offer them) and limits. So the command stream RADV received was the
  one lavapipe validated, and a hang on a valid stream is the driver's or the card's — but "show it"
  needs `RADV_DEBUG=hang`'s dump of the stuck stream, and that dump is only taken by running the clip
  test on the card, which this round's conduct rule forbids without a fix. Q776 asks for that run.
- **Every reset happened inside a parallel suite**; the clock gates, one device process at a time,
  took 355 holds on RADV in round 1915 and reset nothing. That is a correlation, recorded because the
  next round's question is whether concurrency on this passthrough Polaris is the trigger.

## 2. A run meant for lavapipe stays off the card

wgpu loads every backend an instance is made with, and `VK_ICD_FILENAMES` restricts only Vulkan's
drivers: an instance over every backend under a lavapipe pin still loads OpenGL, whose driver is
radeonsi on the RX 580, and a high-performance request or an enumeration of every adapter ranks that
adapter above lavapipe's (trap 317). Raster's ADR 0017 keeps the library out of the environment and
leaves `WGPU_BACKEND` to hosts in one line; a test is a host, so the tests say that line:

- `tools/conformance/test-support/backends.rs` gives `from_environment()` (the set `WGPU_BACKEND`
  names, every backend where it names none) and `instance()` over it. raster-gpu's
  `compute_coverage_determinism`, `function_support::compute` (`function_conformance` and its
  siblings), `backend_choice`, `m1`, `perf_gate`, `readback_cost`, `device_lifecycle`, `compute_lane`,
  `lane_crossover`, `line_lane` and the three opaque-paint and ramp tests enumerate or request through
  it.
- `render-gpu`'s headless context takes its backend set from `WGPU_BACKEND`, as render-raster's
  instance already does; the adapter is still the constructor's choice. Its old comment argued that a
  comparison should not depend on the environment, but the loader's own variables already decide which
  Vulkan devices exist, and the variable only narrows. vello's holds then follow the pin
  (`Hold::unless_software`) instead of always locking.
- **The adapter got is recorded.** The section hook appends `gpu-adapter pid=… type=… exe=…` to
  `PDF_VIEWER_GPU_LOCK_LOG` at every bring-up, locked or not, and `tools/gpu-holds.sh` counts the
  bring-ups by type. Before the change a lavapipe-pinned `nextest` of the three crates brought up 927
  devices on `Cpu` and 28 on `Other` — the card through OpenGL, 8 241 holds; after it, raster-gpu's
  suite brought up 720 on `Cpu`, none elsewhere, and took no hold.

What stays: `Device::headless` over `create_instance()` still initialises OpenGL's EGL display, so a
sampler of `/proc/*/fd` sees raster-gpu tests open `/dev/dri/renderD128` under the pin; they choose
lavapipe by name and bring no device up on the card. Routing the hundred `Device::headless` calls
through the environment's instance is the mechanical follow-up, not done here.

## 3. A hang stops every later device test

A hardware section held past `HUNG_AFTER` (9 s: amdgpu's ring timeout is ten seconds, so a job the
kernel times out holds its section at least that long, and the longest working hold measured is a
planted busy device's 6.1 s) leaves `<lock file>.pdf-viewer-hung`, naming the process, the section and
the time. Every later hardware section, in any process, panics while the marker stands, naming it and
`journalctl -k`; a person removes it. Section 1's second reading is the reason: a suite that goes on
after a hang feeds a card that is still recovering, round 1915's one hang became three resets that
way, and each reset cancels the other user's contexts too. A lavapipe section never takes the lock and
never reads the marker. `render-raster --test gpu_lock`'s
`a_section_held_past_the_hang_bound_stops_every_later_section` plants a hold past a shortened bound
and shows the next section refused and, once the marker is removed, allowed.

**Cost.** A frame that is merely slow past nine seconds on the card stops the suite as a hang would;
that is the safe failure, and its message says what to read. The marker stops gates and suites alike
until a person acts, which is the point.

## 4. The upstream report, drafted

To be filed against Mesa (RADV, Polaris) only once `RADV_DEBUG=hang` has produced a dump (Q776):

> **RADV/Polaris10 (RX 580): ring gfx timeout on a small render pass, then "Illegal register access in
> command stream" from the next context after recovery.** Mesa 25.0.7 (Debian 25.0.7-2+deb13u1),
> kernel 6.12.115, RX 580 passed through to a KVM guest, two users' contexts on the card. A wgpu 30
> application draws one 16 × 8 render pass (instanced quads, a compute pass, a texture-to-buffer copy,
> timestamp queries resolved with `VK_QUERY_RESULT_WAIT_BIT`); the stream is clean under the Khronos
> validation layer's core, synchronisation and GPU-assisted checks on lavapipe. On the card the job
> times out with no soft recovery; after the BACO reset the next process's first submission raises
> `gfx_v8_0_priv_reg_irq: Illegal register access in command stream` and a second reset. Dump attached.
