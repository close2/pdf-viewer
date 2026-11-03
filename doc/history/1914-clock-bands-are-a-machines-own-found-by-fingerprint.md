# 1914 — Clock bands are a machine's own, found by its fingerprint and taken by a command

Session 1914, a single round (measure + instruments) on the server VM. ADRs 2662, 2663; Q776 unused.

**Question.** Could the clock bands be factors of baseline probes (owner, 2026-11-03)? No, for every
class. A factor holds to a fifth only where a class's largest ratio over its smallest is under 1.5,
whatever the probe; the points were notebook → VM and, in one sitting, the RX 580 through OpenGL,
through RADV, OpenGL on two CPUs, and lavapipe:

| class | probe | the class's figures | spread |
|---|---|---|---|
| processor, notebook → VM | calibration 2.71 | warm open 3.32 – 5.22, script 3.39 – 3.72, toolkit processor page one 8.29 – 9.06 | 2.7 |
| processor, 32 CPUs → 2 | 1.00 | turn rows 0.70 – 6.42 | 9.2 |
| disk, notebook → VM | io 5.10, latency 6.30 | cold open 2.85 – 4.21 | 1.5 |
| bring-up, notebook → VM / OpenGL → RADV | — | 3.28, 2.84, 1.97 / 0.53, 0.66, 0.74 | 1.7 / 1.4 |
| device work, RADV → lavapipe | bring-up 0.97 – 1.15 | step 2.4 – 12.4, device page one 3.2 – 3.5 | 14 |
| page one and turns, notebook → VM | — | first page 4.49 – 5.51, device page one 6.1 – 6.7, turns 0.85 – 5.00 | 5.9 |

**Built (ADR 2662).** Band files keep counted figures and prose, and `clock_bands = machine` where
clocks go; `doc/checks/machines/<name>/` holds a `fingerprint` and the clock bands with their runs;
`machine_bands.rs` merges them. The fingerprint is processor model, processors, and the adapter the
harness drew on. A machine with none fails by name; `tools/bands.sh retake` takes it. Machines:
`epyc-7301-32-rx580-opengl` (1913's, moved), `-rx580-vulkan` (three runs, 1413 s), `-llvmpipe-vulkan`
(two runs, 924 s). The toolkit gate line gained `--no-fail-fast`.

**Found (ADR 2663).** CLAUDE.md's `*_icd.x86_64.json` do not exist (`radeon_icd.json`, `lvp_icd.json`):
every 1913 band is the card through OpenGL (bring-up 72.3 ms, 15.2 MiB; RADV 38.2 ms, 9.4 MiB; RADV's
`peak_anon_mib` 21.5 against OpenGL's 38.4 on the five-page row). Lavapipe needs `WGPU_BACKEND=vulkan`.
One first-hold run meant for lavapipe drew on the RX 580 without the lock, about five minutes.

**Gates.** Lavapipe's retake figures fall 37 of 79 outside RADV's bands, 28 outside OpenGL's, 0 outside
their own; RADV's 21 of 79 outside OpenGL's, 0 outside their own. On the new scheme, each its own hold. RADV: launch exit 101, every clock inside, 11 outside
all counted (ADR 2660); turn exit 0, 33 judged, 0 outside; toolkit exit 0. Lavapipe: launch exit 101,
warm open 1.361 and `opt_demo` cold open 0.698 under two-run floors 1.49 and 0.72, the rest counted;
turn exit 101, one seventh 106.03 under 106.80; toolkit exit 0. Tier 1, five crates: fmt and clippy
exit 0; their band tests pass. `cargo test -p conformance`: 471 passed.

**Unfinished.** `*_lowest_ms` / `*_raised_mhz` are not retaken; the driver's version is not in the
fingerprint; lavapipe's bands rested on two runs, and the orchestrator re-took them with three (`retake --again --runs 3`) before the commit, after which lavapipe's turn gate judged 33 of 33 inside and its launch gate every clock inside; the counted findings stay ADR 2660's.
