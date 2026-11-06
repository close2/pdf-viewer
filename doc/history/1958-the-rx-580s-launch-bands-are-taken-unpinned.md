# 1958 — The RX 580's launch bands are taken unpinned, as the gate runs

Session 1958, a single measure round on round 1954's red `launch_path`. ADR 2730.

**The premise.** Round 1954 read `bring_up_ms` 60.8 (27..40), WTPDF's first page 110.0 and cold
107.7 (67..100.3) and four `peak_anon_mib` figures under their floors; ADR 2675's lock and ADR
2712's grey rule were the suspects. Neither: the RX 580's bands were taken on 2026-11-03 under
`WGPU_BACKEND=vulkan VK_ICD_FILENAMES=…/radeon_icd.json` (round 1917's record), and the gate runs
with neither, so `create_launch_instance` loads the machine's seven ICDs and makes the adapter
check. The fingerprint names the adapter and cannot tell the two launches apart.

**HEAD, one binary, arms alternating, each its own `--clock` hold**, the test binary run directly.

| run | loads (start, end) | bring-up | device thread | exit |
|---|---|---|---|---|
| unpinned 1 | 4.9 at its header | 53.0 | wake 3.3 + instance 37.0 + check 10.4 + request 3.6 | 101 |
| pinned 1 | 18.2 / 25.5 | 32.2 | adapter 3.9 + request 2.8 + rest 25.7 | 101 (warm open 1.29, ISO turn 16.3) |
| unpinned 2 | 25.5 / 36.0 | 53.2 | check 21.7; fourteen figures declined | 101 |
| pinned 2 | 36.0 / 19.0 | 32.3 | adapter 3.6 + request 2.5 | **0** |
| unpinned 3, 4 | 5.7, 6.8 | 53.9, 52.6 | instance 39.0, 38.2 | 101 |

Unpinned, every figure round 1954 named is out; pinned, every one is in. The counted figures agree
across arms to 0.06 %, in band: no bisect owed. The lock is taken before the child's clock.

**Memory.** The bring-up child read 12.6 to 13.5 MiB unpinned, 8.9 pinned; first pages moved 0.5.

**Decided (ADR 2730).** `bring_up_ms` 44.7 .. 64.8 and five rows' first pages re-taken by ADR
1969's rule over unpinned runs 1, 3, 4 (`bands.sh write`; the open, turn, probe and script bands
kept, as is the font-walk row); five rows' `peak_anon_mib` by the allocation rule over five
unpinned runs; `tools/bands.sh retake` refuses a pinning that selects the device an unpinned
launch takes (a stub test: unpinned allowed, RADV pinned refused, GL pinned allowed).

**Unfinished.** `turn_path` (unpinned, load 5 rising to 31) exited 101: two pages' turns past
their bands under the load, then `bug1721218_reduced` p1 held a turn section 9.4 s and left the
hang marker. `journalctl -k` from 07:30 has no entry: no fault, timeout or reset. No hardware
section ran after it, so `launch_path` was not re-run under the new bands; the marker is a
person's to remove. `toolkit-launch` was taken in the same pinned retake and is not re-measured.

**Gates.** `bash -n tools/bands.sh` 0; the guard's stub test as above; `cargo test -p conformance`
0 (471 passed); `tools/state.sh --round 1958 docs` 0; `launch_path` 101 and `turn_path` 101 as above.
