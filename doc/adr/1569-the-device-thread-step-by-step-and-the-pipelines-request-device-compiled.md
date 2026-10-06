# 1569 — The device thread, step by step, and the two pipelines `request_device` compiled

Status: accepted. Amends ADR 1558 section 3 in one respect: `raster-gpu`'s `startup.rs` did hold a
lever, in the instance's flags rather than in its backends. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("[t]he graphics library must return a usable device before it is
warm"; cold bring-up its own gate); raster's ADR 0017; ADRs 1532, 1544, 1558; trap 50;
`doc/todo/42` item 4.
Code: `raster/crates/raster-gpu/src/startup.rs` (`instance_flags`, `create_launch_instance_timed`,
`LaunchSteps`, the test), `raster/crates/raster-gpu/src/lib.rs` (the re-export),
`crates/render-raster/src/present.rs` (`QuorraWindowRenderer::instance_timed`),
`crates/render-raster/src/lib.rs` (`QuorraRasterizer::startup`),
`crates/viewer-ui/tests/launch_path.rs` (`device_steps`, `device_thread_instants`,
`print_device_thread`), `doc/checks/launch-path.toml`.
Tests: `raster-gpu`'s `startup::tests::raster_records_no_indirect_call_so_its_validation_is_not_asked_for`.

## 1. What the device thread does, named

The gate printed "graphics device up" and nothing of the thread before it. It now prints each
step's own duration, and on the first-page rows lays them end to end on the timeline. The Radeon
890M through RADV, `bug1815476.pdf`'s first-page child (instants from the process's spawn):

```text
 15.7 ms  graphics instance made (device thread)
 21.4 ms  adapters enumerated, one found on hardware (device thread)
 21.5 ms  adapter chosen (device thread)
 22.9 ms  page one interpreted (document thread)
 23.5 ms  device and queue returned by request_device (device thread)
 24.7 ms  graphics device up
```

Thirty interleaved bring-up children per arm, pinned, minimum / median in ms:

| step | what it is | before | after |
|---|---|---|---|
| instance | `wgpu::Instance::new` — the Vulkan loader and both installed drivers initialising | 11.09 / 12.31 | 11.16 / 12.53 |
| adapter check | `create_launch_instance`'s enumeration — the first `vkEnumeratePhysicalDevices` | 3.04 / 5.00 | 2.98 / 5.37 |
| adapter | `request_adapter`, the second enumeration | 0.15 / 0.17 | 0.16 / 0.17 |
| `request_device` | the device and its queue, one call | 2.54 / 2.96 | 1.67 / 2.07 |
| assembly and host | pipeline store, warm-up spawn, sampler, query sets, `QuorraRasterizer`'s caches | about 1.2 | about 1.2 |
| bring-up | the gate's figure | 17.94 / 21.71 | 17.60 / 20.94 |

Where each goes, under callgrind on the gate's bring-up child: the instance is 77.5 M of the
child's 90.2 M instructions — 45.7 M of them `vkEnumerateInstanceExtensionProperties`, the loader
opening every installed driver library (`libvulkan_radeon.so` and `libvulkan_lvp.so`, each linking
`libLLVM`), 21.7 M `vkCreateInstance`, 15.6 M of `libexpat` parsing the drivers' configuration
files inside those two, 9.1 M the layer list. The adapter check is 2.5 M instructions and 3 to 9 ms:
under `strace` one `DRM_IOCTL_AMDGPU_INFO` took 6.5 ms. It is the kernel driver answering for the
GPU, and the step that varies. The second enumeration costs 0.2 ms because the physical devices are
made once per instance.

## 2. The levers `wgpu` offers, each measured

`scratchpad` probe on `wgpu` 30 directly, one process per sample, eight per arm interleaved, and
the gate's child for the one that held:

- **`InstanceFlags`: the one that held.** A release build's default is `VALIDATION_INDIRECT_CALL`
  alone (validation and debug are a debug build's, and no validation layer is installed here). With
  it, `wgpu-core` parses two WGSL modules and builds two compute pipelines inside `request_device`,
  to check indirect calls' arguments on the device. That is a pipeline compiled before the device
  is usable. Raster records no indirect call. Callgrind: `Device::build` 10.05 → 6.55 M,
  `create_device_and_queue_from_hal` 4.30 → 0.78 M, the whole bring-up 90.16 → 86.65 M. Clock:
  `request_device` 2.54 / 2.96 → 1.67 / 2.07 in the gate's child, 2.13 / 2.63 → 1.30 / 1.69 in
  the probe. The process's resident anonymous memory at the end of the probe is 6.11 → 6.00 MiB.
  The gate's `bring_up_anon_mib` (high-water less mapped pages) read 6.36–6.53 → 6.69–7.09, inside
  its band. This round did not separate why the two figures move in opposite directions.
- **`Backends::VULKAN` alone against `PRIMARY`**: the same set on Linux, since Metal and DX12 are
  not built for this target. The instance read 11.64 / 12.17 against 11.49 / 12.00. Nothing to take.
- **`request_adapter`'s options**: 0.2 ms after the check, whatever they say. Nothing to take.
- **Features**: `TIMESTAMP_QUERY` dropped read 2.00 / 2.43 against 2.13 / 2.63. Inside the spread.
  It is kept: raster's ADR 0031 measures frames with it.
- **Limits**: WebGPU's defaults in place of the adapter's read 2.17 / 2.42. Nothing to take, and the
  adapter's limits are what `Device::limits` reports.
- **`wgpu`'s trace**: the `trace` feature is not enabled in this graph, and `DeviceDescriptor`'s
  trace is off by default. **Logging**: nothing installs a logger on the gate's path, and `quorra`'s
  is at `warn`.
- **Environment, measured and not a lever**: the loader restricted to RADV
  (`VK_LOADER_DRIVERS_SELECT`) read about 1 ms off the instance and 1 to 2 ms off the check. That is
  `lavapipe` loaded for an adapter `request_adapter` passes over. Raster's ADR 0017 keeps the
  environment out of raster. A host setting it would call `std::env::set_var`, which is `unsafe` in
  this edition, and `render-raster`, `raster-gpu` and `viewer-ui`'s library forbid `unsafe`. The
  cost is written down here rather than taken.

## 3. Decision

`create_instance_with`, the one place every instance raster makes is built, passes
`InstanceFlags::from_build_config()` less `VALIDATION_INDIRECT_CALL`. The test reads the crate's
own sources for an indirect call and fails if it finds one, because the day raster records one, the
flag must come back with it. `create_launch_instance_timed` returns `LaunchSteps` beside the
instance. `QuorraRasterizer::startup` hands that over with the device's `StartupTimings`, and the
gate prints both.

Nothing waits for warmth after this change, and nothing is moved later: the two pipelines are not
compiled at all.

**The bands did not move.** The same morning, six gate runs with A and B interleaved read 18.7 to
25.8 ms of bring-up on both arms, and `request_device` read 4 to 8 ms in each. Thirty-two more
children per arm read 7.7 to 10.8 ms (minimum to median) on both. Under `strace` the excess sat in
the kernel driver's ioctls, `AMDGPU_GEM_CREATE` among them, while the machine had about 1 GiB free
and 43 GiB of page cache. That points at the kernel reclaiming memory, though this round did not
prove it. Either way it is the machine's state: a 0.9 ms saving inside a 6 ms swing.
`bring_up_ms` and every `first_page_ms` stay where they were, and a quiet machine owes the re-take.
Every run now prints each step, so the re-take will show which one moved.

## 4. What is left, and whose

Everything else on the thread belongs to the loader, the drivers or the kernel. On this machine the
device stays the longer thread on the rows where it was. The first present in a window on the real
adapter is the owner's to take (`Xvfb` has no DRI3). The command is in record 1367.
