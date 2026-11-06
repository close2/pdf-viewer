# 2730 — The RX 580's launch bands are taken unpinned, because the gate measures the unfiltered launch

Status: accepted. Session 1958, a single measure round. Context: `doc/questions/A270` (the launch gate
measures the unfiltered launch), ADR 2662 (bands per machine, by fingerprint), ADR 2663 (both halves
of a pinning), ADR 2667 (a first page's memory beyond the device's own), ADR 2675 (the lifetime lock).
It cites no clause and moves no ledger row.

## What failed

Round 1954 ran `launch_path` on its tree and it exited 101: `bring_up_ms` 60.8 against 27..40, the
57-page row's first page 110.0 and cold 107.7 against 67..100.3, and four `peak_anon_mib` figures under
their floors. The first suspects were ADR 2675's lock and ADR 2712's grey rule.

## What it was

Neither. The RX 580's bands were taken on 2026-11-03 by `WGPU_BACKEND=vulkan
VK_ICD_FILENAMES=…/radeon_icd.json tools/bands.sh retake` (round 1917's record), and the gate is run
with neither variable. The two launches are different launches on the same adapter, and the
fingerprint, which names the adapter, cannot tell them apart:

- **Unpinned**, `raster_gpu::create_launch_instance` runs: the loader reads all seven ICDs this machine
  has and Mesa's device-selection layer, and the adapter check enumerates them. The device thread read
  render-node wake 3.3 + instance 37.0 + adapter check 10.4 + `request_device` 3.6 ms.
- **Pinned**, `WGPU_BACKEND` names the set, no adapter check is made (ADR 1569), and only RADV is
  loaded: adapter 3.9 + `request_device` 2.8, the rest 25.7 ms.

**Measured in one sitting, one binary, HEAD unchanged, the arms alternating** (round 1958's record
has the loads): every figure round 1954 named is out of band unpinned and inside it pinned. The
counted figures — instructions, bytes, read calls, `open_anon_mib` — are the same in both arms. ADR
2675's lock is not in the bring-up either: the gate's child takes the lock before its clock starts
(`Hold::unless_software`, ADR 2664) and installs no section hook, so the lifetime section ADR 2675
added is told to nobody there.

**The memory figures moved for the same reason, and only through the subtrahend.** Each row's
`peak_anon_mib` is its first page's allocation less the bring-up child's (ADR 2667). Unpinned, the
bring-up child allocated 12.6 to 13.5 MiB against 8.9 pinned, while each first page allocated within
about half a mebibyte of its pinned figure: the other ICDs' enumeration allocates in a process that
does nothing else, and what of it is freed before a first page's high-water is not there to subtract.

## Decision

1. **The figures with the device in them are re-taken unpinned**: `bring_up_ms` and five rows'
   `first_page_ms` (and so `cold_first_page_ms`, ADR 2158), by ADR 1969's rule over the three quiet
   unpinned runs of round 1958 (one-minute loads 4.9 to 6.8), from `tools/bands.sh write` over them.
   A270 is the rule: the band is a claim about the binary a person launches, with every driver the
   loader finds, and not about a launcher that filters them. The opens, the turns, the probes and the
   script stage hold no device and keep the 2026-11-03 bands, which more runs drew; so does the
   font-walk row's first page, whose unpinned readings sit inside its band and whose own two kept
   readings would have narrowed it under a reading of 134 ms that band already holds. The fourth
   unpinned run, at loads 27 to 36 with fourteen figures declined, is in no band.
2. **Five rows' `peak_anon_mib` bands are re-derived unpinned** by this file's allocation rule (the
   observed range widened by a tenth of the value, rounded outwards to half a mebibyte) over round
   1954's run and round 1958's four, because what they subtract is the machine's driver stack and not
   this program, and the program's first page did not move. The font-walk row's readings sit inside
   its band and it is kept. **The cost**: the bring-up child's own allocation varies by 0.9 MiB from
   run to run unpinned, and every row's figure carries that spread; `xfa_filled_imm1344e`'s band is
   1.0 .. 3.0 MiB, half a mebibyte above its highest reading.
3. **`tools/bands.sh retake` refuses a pinning that pins nothing**: under any of `WGPU_BACKEND`,
   `VK_ICD_FILENAMES`, `VK_DRIVER_FILES`, `VK_ADD_DRIVER_FILES`, `VK_LOADER_DRIVERS_SELECT` or
   `VK_LOADER_DRIVERS_DISABLE` it asks the fingerprint again without them, and where the device is the
   same one it stops and says so. A device the loader does not take by itself (lavapipe, the card
   through OpenGL) is still reached pinned, which is the only way to reach it.

## Cost, and what is not done

The bring-up band roughly doubles, and every row's first page carries the 25 to 30 ms of the other
drivers. That is the launch this machine's user meets, and A270 already priced the lever
(`VK_LOADER_DRIVERS_DISABLE=*lvp*` in a launcher's `Exec=` line, the day the tree ships one). The
`toolkit-launch` bands were taken in the same pinned retake and were not re-measured here.
The gate was not run again under the new bands: the turn gate's run after them left the hang marker
(ADR 2665) — a turn section held 9.4 s at a one-minute load of 28 to 31, with no fault, ring timeout
or reset in the kernel's log — and no hardware section runs until a person has removed it.
