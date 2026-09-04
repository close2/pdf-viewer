# 935 — Nine tenths of a memory gate was a library

Date: 2026-09-04.
ADRs: [0910](../adr/0910-nine-tenths-of-a-memory-high-water-belonged-to-somebody-elses-libraries.md),
[0911](../adr/0911-the-device-gate-gains-the-memory-the-device-costs.md).
Question: [`Q37`](../questions/Q37-the-part-of-a-launchs-memory-that-is-not-ours.md).
Touched: `crates/viewer-ui/tests/launch_path.rs`, `doc/checks/launch-path.toml`, `tools/state.sh`,
`doc/todo/42`, `doc/traps/instruments-and-reports.md` (trap 35), `doc/HANDOVER.md`, two ADRs, one
question, this file. **No pixel moves**: no crate that draws changed.

The round was sent to resolve the launch gate's memory high-water, which had been red on every run
for three sessions and which no probe could decline. It is resolved by finding out what the figure
was made of.

## What moved, measured

The gate banded `VmHWM` — the resident high-water of the process that draws page one. Asking
`/proc/self/smaps_rollup` what that is made of, in the gate's own bring-up child:

| | |
|---|---|
| `VmHWM` | 108.4 MiB |
| `Anonymous` — memory this program asked for | 11.2 MiB |
| resident pages of mapped **files** | 97.2 MiB |

and `/proc/self/smaps` aggregated by pathname says which files: `libLLVM.so.22.1` at 51.8 MiB,
`libgallium.so` at 26.4, the test binary at 6.0, `libvulkan_radeon.so` at 4.9. `libLLVM` is **163.5
MiB on disk**, so 51.8 is not a property of the program at all — it is how much of it the kernel
happened to keep, which a fault on a cached mapping raises by a whole fault-around window and a
fault on an evicted one raises by one page.

The experiment is one `posix_fadvise(POSIX_FADV_DONTNEED)` on those two libraries between otherwise
identical runs:

| | `VmHWM` | `Anonymous` |
|---|---|---|
| page cache warm | 108.37, 108.46, 108.47 MiB | 11.152, 11.172, 11.148 MiB |
| those two evicted | 81.25, 81.21, 80.95 MiB | 11.180, 11.152, 11.148 MiB |

**The figure the gate was banding fell a quarter and the figure this program allocates moved by 30
KiB.** Over the afternoon the whole-process figure was seen at 92 MiB and at 180 MiB on the same
binary while the anonymous one stayed inside 4%. A neighbouring round walking a corpus is exactly
what evicts a shared library; the eviction costs almost no time, so no clock probe can see it; and
the fall is uniform across four documents because it is the same libraries in all four processes.
That is the whole of what sessions 922, 926, 931 and 933 were chasing.

It also explains, at last, why `open_peak_mib` — the same measurement in a process with no device
in it — has never moved by a kilobyte: that process maps six mebibytes of binary and a libc, and
they are always hot.

## What the gate says now

`peak_anon_mib` per row — `VmHWM` less the resident pages of mapped files — in the group **judged
on any machine**, beside `read_kib` and `open_peak_mib`. It separates the four documents at 26.8,
31.3, 31.6 and 43.6 MiB, where the whole-process figure separated them by about a tenth against a
band 82 MiB wide, and its spread over eleven consecutive samples was 0.8% to 4.0%. The residual
spread is a transparent huge page — 2 MiB, present or not — so the bands are the observed range
widened by a tenth of the value on each side.

`bring_up_anon_mib` at the top: what the graphics device costs in allocated memory, in the process
that has done nothing else, which is where principle 2 says a driver's regression belongs. Eleven
mebibytes of a five-page document's twenty-seven.

`VmHWM` itself is still measured and printed on every run, beside the driver's share of it, and
banded nowhere. **What the gate no longer claims** is written into the check file's header and into
`Q37`: it says nothing about how much memory the *process* occupies, so a document `mmap`ped whole
rather than read incrementally would not fail this line — `read_kib` is what gates that side.

**No band was widened**, for the fifth round running. Four rounds refused on the argument that a
band is a claim about a machine; this one agrees and found that the figure was a claim about the
machine's page cache.

## The same lines, met on a parallel branch

Round 932 hit these four rows on its own branch and asked `Q32` — should the high-water be a
ceiling rather than a band? — after lowering each floor under the lowest reading it saw. Its
measurements corroborate this round's mechanism rather than competing with it: **two runs of one
binary ten minutes apart read 108.9 and 99.3 MiB**, and seven consecutive runs then sat within a
megabyte of each other. Stable within a plateau, stepping between them, is what a page cache that
holds and then changes produces.

The merge into `main` had already declined those floors on session 934's evidence: nine runs with 29
GiB free read 161 to 182 MiB where the failing run, with 19 GiB of swap in use, read 99 to 116. ADR
0909 called that memory pressure and was right about the direction — pressure is what makes the
kernel reclaim a mapped library's pages, which is what this round did on purpose to two named files
and watched the anonymous total not move. What 0909 said was owed, a third probe so that a pressed
machine declines the figure, is answered by removal: a figure that is a property of this program
needs no probe, and one that is a property of the page cache should not be banded for a probe to
rescue.

`Q37` says what becomes of `Q32`: superseded, because the ceiling is no more a claim about this
program than the floor was, and because `peak_mib` no longer exists in the check file — the harness
rejects an unknown row key, so a `peak_mib` line reaching it again fails loudly on its first run
rather than leaving two answers standing. `doc/todo/42` carried four accounts of this figure after
the merge and carries one now, which is what that section itself asked the resolving round to do.

## Two things measured and not used

- **The software Vulkan driver is not where the mapped pages come from**, which was the first
  guess: `libvulkan_radeon.so` links `libLLVM` directly and `libgallium` arrives with
  `libEGL_mesa`. With `VK_LOADER_DRIVERS_SELECT=*radeon*`, so that the loader opens the AMD driver
  and no other, the whole figure moves 2.5 MiB and the bring-up about 2 ms. It is `Q37`'s third
  option and not a switch anybody can throw.
- **Transparent huge pages are `always` on this machine**, and the samples that read high are the
  ones whose `AnonHugePages` is 2048 KiB rather than 0. That is a step the program did not take,
  and it is why the new bands are a tenth wide rather than two per cent.
