# 2663 — Round 1913's bands were drawn through OpenGL, and are kept as that device's

Session 1914, a single round (measure + instruments). Status: **accepted**. Context: ADR 2660 (the
server VM's bands), ADR 2662 (bands per machine by fingerprint), CLAUDE.md "The machine: the server
VM". It cites no clause and moves no ledger row.

## 1. What was found

CLAUDE.md's GPU bullet, round 1913's hold and the brief of round 1914 pin the RX 580 with
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/radeon_icd.x86_64.json`, and lavapipe with
`…/lvp_icd.x86_64.json`. **Neither file exists on the server VM**: Debian 13 installs
`radeon_icd.json` and `lvp_icd.json`. The Vulkan loader, given only a path that is not there, finds
no driver (`vulkaninfo`: "Found no drivers!"), and wgpu draws through its OpenGL backend instead.
Round 1913's launch logs name the adapter they measured: `AMD Radeon RX 580 Series (radeonsi,
polaris10, ACO, DRM 3.61, 6.12.115+deb13.1-amd64) (Other, Gl)`. So every band ADR 2660 wrote is the
card through OpenGL, not through RADV as its `machine` line says.

Naming `lvp_icd.json` alone is no better as a way to reach lavapipe: wgpu then prefers OpenGL on the
card to Vulkan on the processor, and the gate drew on the RX 580 again. Lavapipe is reached with
`WGPU_BACKEND=vulkan` beside the correct path.

It matters to the figures: in one sitting, cold bring-up read 72.3 ms through OpenGL and 38.2 ms
through RADV, its allocation 15.2 MiB against 9.4, and the toolkit windows' `device_up_ms` 0.66 and
0.74 of OpenGL's. Part of ADR 2660's `bring_up_anon_mib` finding (15.1 against 4.8 .. 7.5) is the
backend rather than the machine.

## 2. The decision

- **Round 1913's bands are kept, as the OpenGL device's**: `doc/checks/machines/epyc-7301-32-rx580-opengl/`,
  whose fingerprint names that adapter. They are what they were measured on; relabelling them as
  RADV's would be the claim that was wrong.
- **The merge's gates run unpinned** (`tools/batch.sh` sets no `VK_ICD_FILENAMES`), where wgpu takes
  the RX 580 through RADV, so RADV's bands are taken by `tools/bands.sh retake` as their own
  directory (ADR 2662), and that is the configuration the merge judges.
- **The fingerprint is read from the adapter the gate drew on** (ADR 2662), which is what would have
  shown this on the day: a mistyped path now selects another directory, or none, instead of
  silently another device.
- CLAUDE.md's two paths are the owner's file and are not edited here; the round's report asks for
  `radeon_icd.json` and `lvp_icd.json` in their place.
