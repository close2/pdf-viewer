# 1872 — The bring-up band's excess is the desktop's carve-out, and the lever is `wgpu`'s memory hints

Session 1517, asked by the batch's brief: the launch gate's `bring_up_ms` read 21.4–24.3 ms against
a ceiling of 20.6 on HEAD and on batch 81's tree alike, with every page turn under its floor. Status:
**accepted**; the band is kept and the lever is priced, not taken (it is `raster-gpu`'s). Context:
ADRs 1532, 1569, 1585, 1647, 1658; `doc/checks/launch-path.toml`'s `bring_up_ms`. Instrument:
`render-raster`'s `examples/bring_up`, whose second argument now chooses the hints. Cites no clause.

## 1. What moved, and what did not

**Nothing installed moved.** `pacman.log` has no driver, kernel, firmware or loader upgrade since
2026-09-19 (Mesa and RADV 26.2.3, `linux` 7.2.6, booted 2026-09-20); ADR 1585 derived the band on
2026-10-06 on the same stack. **No code on the path moved**: `request_device` in `raster-gpu`'s
`construct.rs` and the `wgpu` versions are unchanged since that ADR's commit. The adapter is the
same, RADV STRIX1, named on every run.

**One step moved: `request_device`, 1.00–1.87 ms then, 7.0–10.9 ms now**; the instance and the
adapter check are within their old ranges. Under `strace` the excess is three `GEM_CREATE` ioctls of
2.5–6.5 ms, and `gdb` on the ioctl names them: a 128 MiB block asked of VRAM-or-GTT, and two of
64 MiB — `wgpu-hal`'s first blocks under `MemoryHints::Performance`, 128 MiB device-local and
64 MiB host-visible. The process's `fdinfo` at the end of bring-up says where they went:
**`amd-requested-vram` 197 476 KiB, `drm-memory-vram` 868 KiB, `drm-memory-gtt` 265 508 KiB.** The
adapter's carve-out is 512 MiB and the session's other clients held 409–420 MiB of it
(`mem_info_vram_used`), so the blocks the device asks VRAM for are placed in system memory, whose
pages the kernel allocates and clears inside the call.

**So the cause is the environment's: the desktop's use of the carve-out**, which this account can
read but not change, and which no gate run controls. The four `first_page_ms` rows outside their
bands in the same runs are the same cause: the device is the longer thread of a first page.

## 2. Decision

**The band stays 13.6 .. 20.6**, and its comment says why it fails while the carve-out is full. The
file's rule over today's runs would put the ceiling near 30 ms, past the 25.2–28.3 an instance that
loaded GL read (ADR 1532), which is the regression the band exists to name as itself.

**The lever, priced** (one clock hold, ten interleaved runs each, the carve-out at 409 MiB held by
others): under `MemoryHints::MemoryUsage` — 8 MiB device blocks, 4 MiB host — `request_device`
reads **1.80–4.01 ms** against 7.61–9.42 under the default, and the usable device **15.2–24.3 ms**
against 20.3–31.1. That is the band's own range again, on this machine as it is. It is a change to
every frame's allocation pattern, not only to bring-up: blocks of 8 to 64 MiB where they were 128 to
256 means more allocations for the scenes the zoom and turn gates measure, so the round that takes it
measures those gates beside the launch gate. It is `raster-gpu`'s, and was another slot's this
batch.
