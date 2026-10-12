# 1891 — `bring_up_ms` and `first_page_ms` are judged only where the carve-out had room for the device

Session 1527, an instruments round of batch eighty-three. Status: **accepted** and **built**.
Follows ADR 1872, which found the cause and kept the band. Context: ADRs 0884, 0903, 0909; traps
11, 34, 39. Code: `crates/viewer-ui/tests/launch_path.rs` (`CarveOut`, `carve_out_now`,
`drm_client`, `usage_stat_kib`, `print_carve_out`, `Declined::TheCarveOut`). Check file:
`doc/checks/launch-path.toml`, the `bring_up_ms` comment. Cites no clause.

## 1. The question

ADR 1872 found why `bring_up_ms` reads 21–33 ms against a ceiling of 20.6, and why four
`first_page_ms` rows fail beside it. The desktop holds 409–420 MiB of the adapter's 512 MiB
carve-out, and `wgpu`'s first 128 MiB device-local block is placed in system memory, whose pages
the kernel clears inside `request_device`. No probe in the gate could see that. The calibration
probe measures a processor and the disk probes measure a disk, so each read in band and the figure
failed as a regression. That is trap 34, the guard not made of what moves the figure. Since then
every round has reported the failure as "the environment's" by hand, from an ADR rather than from
the gate.

## 2. Decision

1. **The bring-up child and each first-page child read their own placement** once the device is
   up (bring-up) or once the frame is drawn (first page), and print four fields. From the DRM
   clients in `/proc/self/fdinfo`, de-duplicated by `drm-client-id`: `amd-requested-vram` (what was
   asked of the carve-out) and `drm-memory-vram`, falling back to `drm-resident-vram` (what is
   there). From the adapter, named by `drm-pdev`: `mem_info_vram_total` and `mem_info_vram_used`.
   None of these needs root.
2. **The rule is placement, not occupancy.** The carve-out had room for the device when every
   kibibyte it asked of the carve-out was placed there. A threshold on the others' occupancy would
   have to guess the device's block size, and `wgpu`'s memory hints choose that size: under
   `MemoryHints::MemoryUsage` a device asks 8 MiB blocks, and those fit where 128 MiB does not. The
   placement answers for whatever the hints are.
3. **Where it had no room, the figure is printed with the reading and not judged**, named
   `Declined::TheCarveOut`. That comes after the load and the run's own verdict and before the
   processor's guards, because no processor probe can see it. The summary counts it: `(n of them
   for load, m for the carve-out)`. The reading goes beneath both figures on every run, judged or
   not. A figure whose child reports no placement (another driver, a software adapter, no `/proc`)
   is judged as before.
4. **The bands do not move.** The bands claim what the program costs on this machine when the
   device has its memory. This rule says when a run can test that claim.

## 3. Calibration

On this machine, release build, three bring-up children: the carve-out 524 288 KiB, 397 932–397 936
held, asked 197 472–197 476 and placed 864–868 KiB. That is ADR 1872's 197 476 / 868 again, and
`request_device` read 8.9–10.2 ms. So the guard declines every run while the desktop holds the
carve-out, and the summary says so. For the other direction, a process that opens the render node
and allocates nothing asks for 12 KiB and is placed 12 KiB, which the rule judges. The unit test
plants both readings (`the_carve_out_is_read_from_fdinfo_and_declines_a_device_it_had_no_room_for`).

## 4. The gate run

One `--clock` hold, HEAD's export and this round's export, each with its own target directory. The
desktop held 423–428 MiB of the carve-out, and every device asked 192.8–193.5 MiB and was placed
0.4–1.5. HEAD's run started at a load average of 15.17, so 9 of its figures were declined for load.
`bring_up_ms` read 27.798 and `first_page_ms` 35.7–38.2 on the rows it judged. This round's run
started at 11.32. It declined 7 for the carve-out and 0 for load: `bring_up_ms` at 27.110 and six
`first_page_ms` at 34.4–39.8, each printed with its reading. The merge's run at 16:45, on HEAD with
the carve-out full, had failed `bring_up_ms` at 24.039 and one `first_page_ms` as regressions. This
run's four complaints are not the device's. Two are `peak_anon_mib` floors, which HEAD's children
read too (the record has the A/B). The other two are a cold open and a warm open judged at a load of
9.1–9.6 while siblings built (trap 149).

## 5. What it costs, and the re-ask

A run on a full carve-out judges neither figure, so a regression in bring-up that lands while the
desktop holds the memory is not seen until a run with room. It is seen then. The counted figures
(`bring_up_anon_mib`, `peak_anon_mib`) are judged on every run, as before. **Re-ask**: when
`raster-gpu` takes `MemoryHints::MemoryUsage` (ADR 1872's lever), the gate's carve-out lines should
show the device placed in full on this same desktop, and the two figures judged again.
