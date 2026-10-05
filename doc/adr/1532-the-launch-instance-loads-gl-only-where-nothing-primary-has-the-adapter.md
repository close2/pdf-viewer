# 1532 — The launch instance loads GL only where nothing primary has the adapter

Status: accepted. Session 1348. Amends ADR 0179's reading that the backend set is not a lever on
bring-up, and raster's ADR 0014 section 3 and ADR 0017 in the same respect (raster's records are
raster's and are not edited; `create_instance_with`'s comment now points here). Keeps ADR 0221:
`WGPU_BACKEND` still names the set outright, and `QuorraWindowRenderer::instance_with` is still the
escape hatch from a driver. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("creating the device and compiling the pipelines is now part of
time-to-first-page, so it is a number to measure and to keep small"; cold bring-up its own gate);
ADRs 0179, 0182, 0185, 0221, 0910, 1531; trap 50.
Code: `raster/crates/raster-gpu/src/startup.rs` (`create_launch_instance`, `create_instance_with`'s
comment), `raster/crates/raster-gpu/src/lib.rs` (the re-export),
`crates/render-raster/src/present.rs` (`QuorraWindowRenderer::instance`, `instance_with`'s comment),
`doc/checks/launch-path.toml` (`bring_up_ms`, `bring_up_anon_mib`).

## 1. What bring-up was made of

`render-raster/examples/bring_up`, one process per sample, pinned to the four fast cores, on the
Radeon 890M through RADV, 2026-10-05:

| backends | instance creation | `request_adapter` | `request_device` | usable device |
|---|---|---|---|---|
| all (six runs) | 22.98–25.89 ms | 3.28–8.23 | 1.97–2.18 | 28.83–35.16 |
| Vulkan (six runs) | 11.90–15.54 | 2.88–5.83 | 2.87–3.15 | 18.49–21.42 |

Under callgrind the gate's bring-up child is 171.4 M instructions, 156.2 of them
`wgpu::Instance::new`: 65.9 M are `eglInitialize` — Mesa's `libEGL_mesa` creating a `radeonsi`
screen through `libgallium`, 45.6 M of that its `driCreateNewScreen3` — and
45.7 M are the Vulkan loader enumerating instance extensions. GL's adapter is enumerated and then
passed over: `request_adapter` with `HighPerformance` chose RADV through Vulkan in every run.

**ADR 0179's reading no longer holds on this machine.** It measured the same three adapters with
the Vulkan-only instance giving its saving back in `request_adapter` (34–36 ms becoming 39–43),
which it put down to enumerating the physical devices. With today's driver stack and `wgpu` 30
that enumeration costs 3 to 8 ms either way, and GL's initialisation is what is left to save.

## 2. Decision

`raster_gpu::create_launch_instance` makes an instance of `wgpu::Backends::PRIMARY` (Vulkan, Metal,
DX12), enumerates it, and keeps it if any adapter it offers is not `DeviceType::Cpu`; otherwise it
drops it and makes the all-backends instance `create_instance` always made.
`QuorraWindowRenderer::instance` — the one instance every `quorra` launch and every headless
`QuorraRasterizer` is made on — calls it where `WGPU_BACKEND` names nothing.

What it gives up is nothing a machine with a working primary backend uses: wgpu ranks a primary
adapter before a GL one wherever there is one. The two machines where it differs are both handled
by the fallback rather than by a guess — a GPU with only a GL driver (no primary adapter at all),
and a machine whose only primary adapter is a processor-emulated one (`lavapipe`) beside a GL
driver on real hardware. Each pays one enumeration more than before and draws on the adapter it
had. The Windows machine of ADR 0221 has DX12 and Vulkan, both primary, so its choice and its
escape hatch are unchanged.

## 3. What it bought

The gate's bring-up figure, three runs: 17.1, 18.5, 18.1 ms against 25.2 to 28.3 the same morning
(the A/B: the same binary with `WGPU_BACKEND=vulkan,gl`, which restores the old set, read 26.9 and
25.2 interleaved with 16.6 and 19.0). The device's allocation fell from 11.0 to 6.4 MiB and every
row's `peak_anon_mib` by 4.4 to 4.8 MiB; about 21 MiB less is resident, `libgallium` no longer
mapped. Every first frame hashes as before (ADR 1531 section 4): same adapter, same driver, same
bytes. `bring_up_ms` is banded 14.5 .. 22.2 and `bring_up_anon_mib` 5.5 .. 7.5 by the file's rule,
so that a launch which loads GL again — about 25 ms and 11 MiB — fails as itself.

## 4. Left

Instance creation is still the largest part of bring-up at about 12 ms, most of it the Vulkan
loader's own enumeration of every ICD the machine has installed; a host cannot choose which ICDs
the loader opens without an environment variable, and raster's ADR 0017 keeps the environment out
of raster. Under a window `quorra` makes the instance on its own thread beside `EventLoop::new`, so
how much of this reaches the first present is the window's to measure under `--trace`.
