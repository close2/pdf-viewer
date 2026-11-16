# 2903 — raster names no object to the Vulkan loader, whose device list races

Session 2043, a single finding of round 2034 (`findings-queue.md`). Status: **accepted, built**. It
cites no clause and moves no ledger row. Code: `raster/crates/raster-gpu/src/startup.rs`
(`instance_flags`). Context: ADR 2675 (a device holds the lock for its life), raster's ADR 0018.

## 1. The question

Round 2034 saw a test binary of `render-raster` die with `SIGSEGV` on lavapipe after one of its
tests panicked while others held devices, and asked whether a panic during a device's life leaves
shared state broken — the lock hook's drop, or `wgpu`'s instance.

## 2. What was found

**The panic was not the cause.** On the RX 580, `fitted_dashes_across_views` (two tests, seven
devices) died with `SIGSEGV` in 10 of 200 runs and `marks_across_views` (three tests) in 31 of 150,
every test passing. Lavapipe, pinned with both halves (ADR 2663), did not crash in 270 runs: the
race wants the hardware driver's timing. Four cores, read with `gdb` (three before any change, one
with `DISCARD_HAL_LABELS`), show the same two threads:
one inside `vkSetDebugUtilsObjectNameEXT` (naming a bind group, a pipeline, a command buffer) and
crashed in `libvulkan`; the other inside `vkCreateDevice` or `vkDestroyDevice`, waiting on a loader
mutex.

The Khronos loader's source at the installed version (1.4.309, `loader/loader.c`; its `main` reads
the same) says why. `loader_get_icd_and_device`, which the naming terminator calls, walks every
instance's `logical_device_list` under `loader_global_instance_list_lock`; `loader_add_logical_device`
(from `terminator_CreateDevice`) and `loader_remove_logical_device` (from `vkDestroyDevice`) change
that list holding only `loader_lock`, a different mutex. Creation and destruction are serialised
with each other by `loader_lock`; a reader outside it is not.

## 3. What is built

`instance_flags` leaves out `InstanceFlags::DEBUG`, as a release build already does.
`DEBUG` is what makes `wgpu-hal` enable `VK_EXT_debug_utils` and name every object it makes; without
it no naming call is made, and the validation layer still runs. `DISCARD_HAL_LABELS` was tried first
and is not enough: `wgpu-hal` names each command buffer it opens whatever label it is given (8 of
150 still crashed).

Afterwards: `marks_across_views` 1 crash in 450 runs, `fitted_dashes_across_views` none in 150. The
one remaining crash left no core to read; the loader's other outside-lock readers
(`vkGetDeviceProcAddr`'s terminator, `vkSetDeviceDispatch`) are the candidates, and they are the
loader's to fix.

**Cost**: a debug build's validation messages lose `wgpu`'s object names and their route through
`wgpu`'s log (the layer reports on its own default output instead). Serialising every naming
call against device creation inside raster was the alternative, and it cannot be done: the calls are
inside `wgpu`, on every thread that makes an object.

## 4. What is left

The loader's race is upstream's (an issue against KhronosGroup/Vulkan-Loader is the remedy; this
round filed none). The lock hook's drop and `wgpu`'s instance were not implicated.
