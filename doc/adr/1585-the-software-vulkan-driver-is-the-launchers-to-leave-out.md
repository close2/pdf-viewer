# 1585 — The software Vulkan driver costs 3.5 ms of bring-up, and only a launcher can leave it out

Session 1374. Status: **accepted**. Amends ADR 1569 section 2's last bullet, which priced the
lever at about 2 ms and put it in the instance. Leaves raster's ADR 0017 standing. Context:
`CLAUDE.md` principles 2 and 3; ADR 1569; `doc/questions/Q270`; `doc/checks/launch-path.toml`.
Code: none. Every arm below was measured on unchanged code.

## 1. What the second driver costs, re-measured

The bring-up child of `crates/viewer-ui/tests/launch_path.rs` (phase `bring-up`) was run fifteen
times per arm. The arms were interleaved and pinned to cores 0–3 and 12–15. The run was at
13:16 on 2026-10-06: load 1.04, 27 GiB free, Tctl 49 °C. Minimum / median in ms:

| arm | bring-up | instance | adapter check | `request_device` |
|---|---|---|---|---|
| both drivers | 16.84 / 19.39 | 9.42 / 9.82 | 4.22 / 6.70 | 1.00 / 1.87 |
| `VK_LOADER_DRIVERS_DISABLE=*lvp*` | 13.26 / 14.89 | 8.65 / 8.89 | 1.88 / 2.87 | 1.02 / 1.13 |
| `VK_LOADER_DRIVERS_SELECT=*radeon*` | 12.83 / 16.63 | 8.69 / 8.88 | 1.75 / 3.83 | 1.00 / 1.18 |
| `vk_loader_settings.json`, RADV exclusively | 13.93 / 16.47 | 8.74 / 8.92 | 2.18 / 3.51 | 1.00 / 1.48 |

Under callgrind the child is 118.83 M instructions with both drivers. It is 107.66, 107.66 and
107.70 M under the three filters. `wgpu::Instance::new` goes from 77.54 to 68.11 M.
`vkCreateInstance` goes from 21.74 to 13.73 M. **`vkEnumerateInstanceExtensionProperties` goes
only from 45.72 to 44.29 M.** ADR 1569 attributed those 45.7 M to the loader opening both
libraries, and the brief for this round repeated it. That holds of the call, but lavapipe's share
of it is 1.4 M. The rest is RADV's own. Lavapipe's clock is mostly in the adapter check, about
2.4 ms at the minimum, where its physical device is made. The resident mapped libraries are
35.5 → 32.9 MiB.

## 2. The library-side routes, and why none is admissible

- **`wgpu`'s instance options.** `wgpu` 30's `InstanceDescriptor` carries backends, flags, memory
  thresholds and `BackendOptions`. The last holds GL, DX12 and noop options and nothing for
  Vulkan. `wgpu-hal`'s Vulkan `Instance::init` loads `libvulkan.so` and enumerates the instance's
  extensions and layers with no filter. There is nothing to set.
- **`VK_LUNARG_direct_driver_loading`, exclusive mode.** The application passes the loader a
  driver's `vkGetInstanceProcAddr` in `VkInstanceCreateInfo`'s chain. That means opening
  `libvulkan_radeon.so` by name and calling through a raw function pointer. `raster-gpu` is
  `#![forbid(unsafe_code)]`, and the program would be choosing the machine's driver by file name.
  The loader's own documentation (`LoaderDriverInterface.md`, its section on that extension's
  limitations) also says that `vkEnumerateInstanceExtensionProperties` has no chain to carry the
  list. That call still loads every system driver, and `wgpu-hal` makes it before the instance.
- **`VK_LOADER_DRIVERS_SELECT` or `_DISABLE` set by the binary.** That is `std::env::set_var`,
  which is `unsafe` in this edition. Principle 3 forbids `unsafe` in every crate that touches PDF
  bytes, and raster's ADR 0017 keeps the environment out of raster.
- **The loader settings file.** Loader 1.4.357 honours `additional_drivers` with
  `additional_drivers_use_exclusively`, scoped by `app_keys` to an executable path. Measured above,
  it saves what the variables save. But the loader reads only the one `vk_loader_settings.json` it
  finds first, under `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `~/.config`, `~/.local/share`, the XDG
  config directories or `/etc`, never beside the program. It is the file vkconfig writes. A
  package that installed it would own a setting every Vulkan application on the machine reads. It
  must also name the driver's manifest, which differs from machine to machine.

## 3. Decision

The binary is unchanged. Lavapipe's 3.5 ms belongs to the driver stack the machine installed.
The one admissible lever is the launcher's environment. `VK_LOADER_DRIVERS_DISABLE=*lvp*` names
the software driver rather than the hardware one, so it is the same line on every machine. The
tree ships no launcher today, so whether it ever sets that line is the owner's to say
(`doc/questions/Q270`, with a recommendation). The launch gate keeps measuring the unfiltered
process, so its bands stay a claim about the binary.
