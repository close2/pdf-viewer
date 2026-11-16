# 2911 — raster names its objects to the Vulkan loader again, from 1.4.345 on

Session 2047. Status: **accepted, built**. It supersedes ADR 2903's trade; ADR 2903 stays as the record
of why the trade was made. It cites no clause and moves no ledger row. Code:
`raster/crates/raster-gpu/src/startup.rs` (`instance_flags`). Context: ADR 2675 (a device holds the lock
for its life), Q874.

## 1. What changed on the machine

ADR 2903 left `InstanceFlags::DEBUG` out of debug builds. With it set, a test binary that brought
devices up and down on parallel threads died with `SIGSEGV` inside `libvulkan` on the RX 580. The Khronos
loader at 1.4.309 walked its list of logical devices to answer `vkSetDebugUtilsObjectNameEXT` under one
mutex, while `vkCreateDevice` and `vkDestroyDevice` changed that list under another. Q874 traced the
upstream fix to Vulkan-Loader pull request #1866, first tagged in v1.4.345. In its source on `main`,
`loader_get_icd_and_device` takes `loader_lock`, the same lock creation and destruction hold, and
`loader_global_instance_list_lock` no longer exists (`loader/loader.c`).

On 2026-11-16 the owner updated the VM: `libvulkan1` is now `1.4.363.0-1`, and
`/usr/lib/x86_64-linux-gnu/libvulkan.so.1` points to `libvulkan.so.1.4.363`, the only loader on the machine
(`LD_LIBRARY_PATH` empty). Mesa did not change (`mesa-vulkan-drivers` `25.0.7-2+deb13u1`), so the driver
side of ADR 2903's timing is the same.

## 2. The test

`instance_flags` was given `DEBUG` back for debug builds. Each test binary was then run 450 times on the RX
580, with `ulimit -c unlimited` and `PDF_VIEWER_GPU_LOCK_LOG` set:

| binary | before, loader 1.4.309, `DEBUG` set (ADR 2903) | now, loader 1.4.363, `DEBUG` set |
|---|---|---|
| `render-raster` `marks_across_views` (three tests) | 31 `SIGSEGV` in 150 runs | 0 in 450; 450 passed |
| `render-raster` `fitted_dashes_across_views` (two tests) | 10 `SIGSEGV` in 200 runs | 0 in 450; 450 passed |

No run left a core. Each bring-up's `type=` line in the lock log reads `DiscreteGpu`: 6300 lines for
`marks_across_views` and 3150 for `fitted_dashes_across_views`. The two loops ran at the same time, so
the parallel load on the loader was greater than in ADR 2903's runs. The binaries were debug builds from
the 1.100.0 toolchain prepared by ADR 2910. The race is in `libvulkan`, and raster's own code at
those call sites is the same under both compilers.

At ADR 2903's crash rate, 0 crashes in 450 runs of `marks_across_views` has a probability of about
(1 − 31/150)⁴⁵⁰, or below 10⁻⁴⁵. The race is closed on this machine.

## 3. The decision

`instance_flags` drops only `VALIDATION_INDIRECT_CALL`. A debug build again names every object
it makes, and the validation layer's messages again carry those names and go through `wgpu`'s log.
The function's comment states the loader version this needs. An older loader still crashes, so a
machine with a Khronos loader older than 1.4.345 that runs these suites will hit ADR 2903's
`SIGSEGV` again. The remedy there is the loader, not this flag.
