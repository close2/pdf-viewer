# 1658 — Bring-up's idle excursion is a power-up the render node's open starts, and the device thread opens the node first

Session 1411. Status: **accepted** and **built**. Answers ADR 1647 section 4 (the call and the kernel
state behind the 4.4 ms an idle-born launch pays). Supersedes nothing.
Context: `CLAUDE.md` principle 2 (cold bring-up its own gate); ADRs 1569, 1585, 1647;
`doc/questions/Q270`; traps 13 and 110.
Code: `raster/crates/raster-gpu/src/startup.rs` (`wake_render_nodes`, `render_nodes_awake`,
`LaunchSteps::render_node_wake` and `render_nodes_opened`, `create_launch_instance_timed`),
`crates/viewer-ui/tests/launch_path.rs` (`device_steps`, `device_thread_instants`,
`print_timeline`, `print_device_thread`).
Tests: `raster-gpu`'s `startup::tests::only_a_render_node_whose_device_is_awake_is_woken`.

Every figure is the Radeon 890M through `amdgpu` and RADV on Linux 7.2.6, from children spawned as
`run_phase` spawns the gate's bring-up child (pinned to CPUs 0–3 and 12–15, no display), behind the
heavy-walk lock, at a one-minute load of 1.3 to 15. Instruments: `strace -T`, an `LD_PRELOAD` shim
that times every `ioctl` and reads `AMDGPU_INFO`'s query number, and three C probes that open
`/dev/dri/renderD128` and ask single queries. Medians unless said.

## 1. The call

Twenty interleaved pairs, one child born after 1.5 s of idle and the next at once:

| | idle-born | back to back |
|---|---|---|
| bring-up | 26.57 | 21.64 |
| adapter check | 9.10 | 5.60 |
| the first `AMDGPU_INFO_DEV_INFO` | 4.96 | 0.46 |
| every `ioctl` the child made, summed | 12.18 | 8.92 |
| its 36 `GEM_CREATE`s, summed | 4.42 | 4.60 |

The excess is one call: **`DRM_IOCTL_AMDGPU_INFO` with `AMDGPU_INFO_DEV_INFO` (0x16)**, the second
query the process sends on the render node, which RADV asks while `vkEnumeratePhysicalDevices` makes
the physical device in the adapter check. `strace -T` read it at 5.3 to 7.6 ms in five idle-born
children. No other call moves.

## 2. What it waits for

- **Not the processor.** `AMDGPU_INFO_VRAM_GTT`, which the driver answers from its own memory,
  reads 0.007 ms after 500 ms of idle. A probe pinned to a core whose SMT sibling spun throughout
  read 4.58 ms, and 4.34 without the spin. This is ADR 1647's finding that a spin moves nothing.
- **Not the number of firmware messages.** DEV_INFO's four clock queries each ask the power
  firmware twice whether a feature is enabled (`smu_cmn_get_enabled_mask`, mainline
  `amdgpu_kms.c` and `smu_cmn.c`). That is eight messages, and it reads 4.95 ms. A single sensor
  query, one message, reads 4.99 ms.
- **Not runtime power management.** The device's `power/control` is `on`, its
  `runtime_suspended_time` is 0, and it is in D0.
- **Something the open starts.** On one descriptor, a query every 1 to 100 ms after the first
  reads 0.06 to 0.51 ms, but the first query after each open reads 3.4 to 5.0. Waiting after the
  open before asking: 0 ms gives 5.89, 2 ms gives 3.35, 5 ms 0.47, and 10 to 50 ms 0.5 to 0.9. So
  the open starts about 5 ms of work, and a query that arrives during it waits for the rest.
- **After about 50 ms without work.** A new process's first query reads 0.78 ms when the previous
  one ended 0 ms before, 0.97 at 20 ms, 4.38 at 50 ms, and 4.9 to 5.7 ms from 100 ms to 3 s.

The source fits all five, though this round could not trace inside the kernel (no root, no
debugfs). The open, `amdgpu_driver_open_kms`, creates the file's GPU address space, whose page
tables are cleared by a job on the device. DEV_INFO and the sensor query take the
power-management lock (`adev->pm.mutex`), and VRAM_GTT takes none. So the reading is this: on a
device idle for 50 ms, the open's first job powers part of the GPU up through the power firmware,
which holds the lock for about 5 ms, and the query waits. **Which block and which lock is the
reading; the timings are the measurement.**

## 3. What on this side holds it

- **A process that keeps asking does not.** A probe that queried every 10 ms beside the children
  left them at 10.55 ms of adapter check, against 11.25 without it, and cost 7 to 9% of a core: the
  kernel busy-polls the firmware's mailbox.
- **A process that keeps opening the node does**: opened and closed every 10 ms, it brings the
  first query to 0.55 ms, against 6.04. But that keeps the GPU powered for good, a cost the owner's
  display and battery would pay for every second the viewer is not starting, so it is not taken.
  Nor is a launcher setting in Q270's shape: none was found that holds the state. The kernel's own
  parameters (`amdgpu.ppfeaturemask`) are the machine's, need a reboot, and were not measured.
- **Opening the node once, early, does**, at no standing cost. The power-up is going to happen
  when RADV opens the node; made at the device thread's first instant, it runs beside
  `wgpu::Instance::new`'s 10 to 13 ms of loader work instead of in front of the query. Emulated
  first, as a wrapper that opened and closed the node and then ran the child (sixteen pairs, both
  arms idle-born): bring-up 22.91 against 27.21 ms, adapter check 6.53 against 10.07.

## 4. Decision

`create_launch_instance_timed` calls `wake_render_nodes` first: every `/dev/dri/renderD*` whose
`/sys/class/drm/<node>/device/power/runtime_status` reads `active` or `unsupported` is opened, and
the files are closed after the adapter check. **Closing them at once was the first build, and it was
wrong**: a close made while the power-up runs waits for it. The step then read 1.7 to 2.4 ms in the
gate's first-page children, and it gave back about half of what it took off. A suspended node is
left alone, so a laptop's sleeping discrete GPU is woken only by the adapter check, as before. An
open that fails is not an error, because the adapter check opens the same nodes and reports what it
cannot. `LaunchSteps` carries the step's duration and the count. The gate prints them as
`device_wake_ms` and `device_nodes_woken`, and the duration also appears on the device thread's
line and timeline. On other platforms the step is a no-op.

**Measured, the tree with and without the hunk**: `--release` builds of the same tree, md5
`f029a50b…` without it, `8d808f28…` closing at once, `f59d84ab…` closing after the check. The gate's
bring-up child was run in rotating order at nice 0, load 0.2 to 0.8, over two sittings:

| | without | closed at once (12 an arm) | closed after the check |
|---|---|---|---|
| born after 1.5 s of idle, bring-up | 26.32 (22) | 23.94 | 22.59 (22) |
| the same, adapter check | 9.83 | 6.38 | 6.19 |
| born back to back, bring-up | 19.99 (52) | 23.91 | 20.69 (52) |
| the same, adapter check | 6.04 | 7.76 | 6.01 |
| the step itself | — | not printed by that build | 0.08 to 0.19 |

So a launch after an idle second, which is what a person makes, is 3.7 ms quicker (median; 3.3 on
the mean). A launch straight after another reads 0.7 ms more on the median and 0.5 on the mean, and
its adapter check does not move. That difference is in no named step, and the forty pairs of the
larger sitting alone read 0.4. The gate's minimum is the back-to-back end, so its band does not
move.

## 5. What is not answered

Whether other drivers pay anything on an open that an early open could take: Intel's and NVIDIA's
render nodes were not available here, and on them the step costs one open. Which engine the power-up
is, and whether a kernel parameter would remove it for every program on the machine, need root.
