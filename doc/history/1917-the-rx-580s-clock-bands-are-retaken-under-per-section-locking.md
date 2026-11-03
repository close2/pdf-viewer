# 1917 — The RX 580's clock bands are re-taken under per-section locking

Session 1917, a single round run by the orchestrator (measure), no round agent: the command round 1914
built (`tools/bands.sh retake`) was the whole of the work. ADRs, traps and habits: none new.

**Why.** Round 1915 moved the GPU lock from the shell into the harness's device sections (ADR 2664),
and a gate's clock now starts inside the lock: the RX 580 through RADV gates read faster than the
bands round 1914 took under its whole-run holds, and fell red low.

**Taken.** `WGPU_BACKEND=vulkan VK_ICD_FILENAMES=…/radeon_icd.json tools/bands.sh retake --again --runs 3
--round 1917`, exit 0, fingerprint `… RX 580 Series (RADV POLARIS10) (DiscreteGpu, Vulkan)`. The other
user's load ran 7.7 – 47.3 (one-minute) through it, so the launch gate's warm and cold opens kept one or
two readings of three on some rows. The launch gate was re-taken with five more runs (`--gates launch
--runs 5`, load 15 – 25); `retake` writes only when every gate ran, so the second run's five launch logs were copied, as
`launch-4.log` … `launch-8.log`, into a copy of the first run's log directory and written with
`tools/bands.sh write <that directory>`; each band file's comments carry every kept reading. Warm opens now rest on 5 – 7
kept readings. Bring-up's ceiling fell from 65.7 to 40.0 ms: the wait for the lock is out of the clock.

**Holds.** Over the first re-take, `tools/gpu-holds.sh`: first-page phase 603 holds, longest 1.51 s;
page-turn phase 153, longest 0.87 s; toolkit launch 54, longest 0.71 s; bring-up 11, longest 4.9 ms;
busy-device 3, longest 6.08 s. One wait for the other user's hold lasted 38.1 s. No amdgpu reset
(`journalctl -k | grep -i 'gpu reset'`, 0 after every run).

**Gates.** On the new bands, RADV: turn exit 0, 33 judged, 0 outside. Toolkit exit 0. Launch exit 101,
twice: every red a counted figure ADR 2660 records (`open_kinstructions`, `peak_anon_mib`,
`bring_up_anon_mib`) except one warm open each time, faster than its floor — `PDF20_AN001-BPC` 1.471
against 1.70 before the merge, `bug1815476` 1.018 against 1.08 after it. `cargo test -p conformance`: exit 0.

**Unfinished.** The warm open's floor: a sub-2-ms processor figure read under another user's varying
load falls under ADR 1969's floor by a few percent on a different row each run; whether a warm open
under the load ceiling should be judged on its ceiling only is the next measuring round's question.
`retake --gates` cannot write a part of a machine's bands, which the merge above worked around. The
busy-device test holds the lock 6.1 s, two thirds of the hang bound (ADR 2665). ADR 2660's findings
and the turn gate's `*_lowest_ms` / `*_raised_mhz` stand as they were.
