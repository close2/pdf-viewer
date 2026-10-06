# Q270 — May the program's launcher leave the software Vulkan driver out, or is its cost accepted?

Asked by round 1374. It re-took the launch gate on a quiet machine and measured what the Vulkan
loader's second driver costs the device thread (ADR 1585).

## What the cost is

The Vulkan loader on this machine finds two drivers: `libvulkan_radeon.so` (RADV) for the GPU, and
`libvulkan_lvp.so` (lavapipe), a software rasteriser. The loader loads both. Fifteen bring-up
children per arm, interleaved and pinned, on 2026-10-06 at a load of 1.0, minimum / median in ms:

| arm | bring-up | instance | adapter check |
|---|---|---|---|
| both drivers, as today | 16.84 / 19.39 | 9.42 / 9.82 | 4.22 / 6.70 |
| `VK_LOADER_DRIVERS_DISABLE=*lvp*` | 13.26 / 14.89 | 8.65 / 8.89 | 1.88 / 2.87 |
| `VK_LOADER_DRIVERS_SELECT=*radeon*` | 12.83 / 16.63 | 8.69 / 8.88 | 1.75 / 3.83 |
| a loader settings file naming RADV only | 13.93 / 16.47 | 8.74 / 8.92 | 2.18 / 3.51 |

So about **3.5 ms** of a 17 to 19 ms bring-up is lavapipe's. Most of it is the adapter check, where
lavapipe's physical device is made, and not the instance. Under callgrind the child is 118.8 M
instructions with both drivers and 107.7 M with any of the three filters. That is the same 11.2 M
whichever filter is used. Every first page on every launch-gate row waits for the device.

## Why the binary cannot do it

ADR 1585 has the full argument. In brief:

- `wgpu` 30's instance options have no Vulkan driver filter.
- `VK_LUNARG_direct_driver_loading` would make the program open a named driver library itself and
  pass the loader a function pointer, and `raster-gpu` forbids `unsafe`. The loader's own
  documentation also says that `vkEnumerateInstanceExtensionProperties` still loads every system
  driver, and `wgpu` calls it first.
- Setting the variable from the binary is `std::env::set_var`. That is `unsafe` in this edition,
  and `CLAUDE.md` principle 3 forbids `unsafe` in any crate that touches PDF bytes.
- The loader settings file worked, but the loader reads only one such file, found in the user's or
  the system's configuration directories. It is vkconfig's file, and a program shipping it would
  own a setting every Vulkan application reads. It would also have to name this machine's driver.

What is left is outside the binary. **`VK_LOADER_DRIVERS_DISABLE=*lvp*` names the software driver
rather than the hardware one**, so one line works on any machine, for example in a desktop file's
`Exec=env VK_LOADER_DRIVERS_DISABLE=*lvp* quorra %f`. Its cost falls on a machine with no GPU.
There lavapipe is the only Vulkan adapter. With it filtered out, `create_launch_instance` falls back
to every backend, and the device is whatever GL offers, or the processor backend. This round did not
measure that.

## The question

May the program's launcher, a desktop file or wrapper the tree does not yet ship, set
`VK_LOADER_DRIVERS_DISABLE=*lvp*`? Or is lavapipe's 3.5 ms accepted as the driver stack's?

## Recommendation

**Accept the cost in the binary, and set the variable in the launcher when the tree ships one.**
The variable is the reader's machine configuration rather than the program's code, so ADR 0017
and principle 3 both stand. A desktop file is where a Linux package already states its
environment. The GPU-less fallback should be measured on a machine without one before the line
ships, and the launch gate should go on measuring the unfiltered launch, so that the band is a
claim about the binary and not about the launcher. Until a launcher exists nothing is owed.
