# Q874 — Will you install a Vulkan loader from 1.4.345 on, whose device-list race is fixed upstream?

Source: the owner's "yes, please file the vulkan issue" of 2026-11-16, on ADR 2903's last section. Nothing waits
on the answer: raster already avoids the race's trigger (ADR 2903), and one crash in 450 runs of
`render-raster`'s `marks_across_views` remains.

## What was found before filing

**The race is already reported and fixed upstream, so no issue was filed.** A new issue would duplicate a closed
one, and its maintainer has already answered it:

- KhronosGroup/Vulkan-Loader issue #2018, "Concurrent vkCreateDevice / vkDestroyDevice is a use-after-free:
  logical_device_list has a locked reader and unlocked writers" (opened and closed 2026-08-24), describes exactly
  what ADR 2903 section 2 found. The maintainer pointed to pull request #1866.
- Pull request #1866, "Use recursive mutexes to fix deadlocks in loader" (merged 2026-02-20, commit `1338b8ee`),
  fixes issues #1739 and #1863 (a `SIGSEGV`). On `main`, `loader_global_instance_list_lock` is gone:
  `loader_get_icd_and_device` takes `loader_lock`, the same recursive mutex that `vkCreateDevice` and
  `loader_layer_destroy_device` hold. The reader and the writers of `logical_device_list` therefore share one
  lock, and the race is closed.
- The first tag that contains the fix is **v1.4.345**; v1.4.344 and earlier do not (GitHub's compare of
  `1338b8ee` against each tag).
- This VM has **`libvulkan1` 1.4.309.0-1**, Debian trixie's only candidate (`apt-cache policy libvulkan1`). It
  predates the fix by a year: v1.4.309 was tagged 2025-02-21.

The sources read are kept outside the tree under `~/scratch/agent/vk-loader/`: `loader.c` and `trampoline.c` at
v1.4.309 and on `main`.

## The question

The loader is a system package, so installing it is yours. Will you give this VM a Vulkan loader from v1.4.345
on? The options:

- **(a) Recommended: a packaged loader ≥ 1.4.345**, from trixie-backports or a later Debian suite once one carries
  it. A round would then re-run `marks_across_views` and `fitted_dashes_across_views` 450 times each with
  `InstanceFlags::DEBUG` restored. If neither crashes, ADR 2903's trade is reversed by a new ADR, which gives debug
  builds their object names back.
- **(b) The LunarG SDK's loader**, unpacked in user space and selected with `LD_LIBRARY_PATH` for test runs only.
  This works today without a system change, but test runs would no longer use the loader the program ships
  against.
- **(c) Wait** for Debian to move. ADR 2903's mitigation stays, and so does the one-in-450 crash.
