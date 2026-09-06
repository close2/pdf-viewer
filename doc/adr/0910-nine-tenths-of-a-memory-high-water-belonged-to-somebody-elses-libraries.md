# 0910 — Nine tenths of a memory high-water belonged to somebody else's libraries

Session 935. Status: **accepted**. The first of this round's two records; the second,
[ADR 0911](0911-the-device-gate-gains-the-memory-the-device-costs.md), is what the finding leaves
the gate looking like.

## Context

`CLAUDE.md` principle 2 names four figures a perf gate owes — "cold open, time-to-first-page,
page-turn latency, memory high-water" — and
[ADR 0884](0884-the-launch-path-is-a-gate-and-a-wall-clock-band-needs-a-machine-under-it.md) built
the gate that measures them. The fourth of them has now fallen out of its band three times:

- **Session 922**, the round that built it. `peak_mib` was identical across all forty-four runs
  the bands were derived from, and an hour later — same tree, same binary, idle machine — all four
  rows had fallen together by about 12%. That round demoted the figure out of the group judged on
  any machine and widened its band to span what it had seen.
- **Sessions 926 and 931**, which found it would not sit and declined to widen it.
- **Session 933**, which ran the full sequence with every calibration between 0.712 and 0.720 ms —
  so the run *was* judging — and had all four rows fail **below** their floors by about 13%,
  reproduced within a kilobyte on a quiet re-run.
- **Sessions 932 and 934**, on a parallel branch and on the merge that took it: 932 lowered the
  four floors on readings of 98.5 to 116.4 and asked `Q32`; 934 measured nine runs of an untouched
  binary with 29 GiB free at **161 to 182 MiB**, against 99 to 116 in a run made with 19 GiB of
  swap in use, and the merge restored 931's floors. ADR 0909 read that as memory pressure — the
  right direction, one step short of the mechanism.

Four rounds in a row declined to widen the band, on an argument this project should keep: a band
is a claim about a machine, and widening one to admit whatever the machine is doing puts the
machine into the claim. That refusal was right and it was not a resolution — a gate that is red on
every run gets ignored, which is the failure mode this tree has already written down for cost
gates. The figure also has **no clock in it**, so the calibration probes that decline the other
figures cannot decline this one, and no change a coverage round makes can lower a process's
high-water by eighteen mebibytes uniformly across four documents.

So this round was sent to find out what actually moves it.

## What was measured

Everything below is `crates/viewer-ui/tests/launch_path.rs`'s own children, re-executed by hand
with `PDFVIEWER_LAUNCH_PHASE` set, `release`, pinned to the same cores the gate pins to.

### The high-water is nine tenths file-backed, and the anonymous tenth is the program

`/proc/self/smaps_rollup` answers what `VmHWM` is made of. A **bring-up** child — a process that
creates the graphics device and does nothing else at all — reads:

| | |
|---|---|
| `VmHWM` | 108.4 MiB |
| `Anonymous` | 11.2 MiB |
| `Rss - Anonymous`, that is, resident pages of mapped **files** | 97.2 MiB |

Aggregating `/proc/self/smaps` by pathname says which files:

| mapping | resident |
|---|---|
| `/usr/lib/libLLVM.so.22.1` | 51.8 MiB |
| `/usr/lib/libgallium-26.2.1-arch1.1.so` | 26.4 MiB |
| the test binary itself | 6.0 MiB |
| anonymous | 5.3 MiB |
| `/usr/lib/libvulkan_radeon.so` | 4.9 MiB |
| `/usr/lib/libvulkan_lvp.so` | 2.6 MiB |
| twenty more shared objects | under 2.1 MiB each |

`libvulkan_radeon.so` links `libLLVM.so.22.1` directly, and `libEGL_mesa.so` brings
`libgallium.so` — so this is **not** the software rasteriser being enumerated and thrown away.
Measured rather than assumed: with `VK_LOADER_DRIVERS_SELECT=*radeon*`, so that the loader opens
the AMD driver and no other, the whole figure moves by 2.5 MiB and the bring-up by about 2 ms.

### How much of a library is resident is the page cache's decision

`libLLVM.so.22.1` is **163.5 MiB on disk** and 51.8 of them were resident. That fraction is not
this program's to choose: a fault on a mapping whose pages are already in the page cache maps a
whole fault-around window, and a fault on one that has been evicted maps a single page.

The experiment is one `posix_fadvise(POSIX_FADV_DONTNEED)` on `libLLVM.so` and `libgallium.so` and
nothing else, between otherwise identical runs:

| | `VmHWM` | `Anonymous` |
|---|---|---|
| page cache warm, three runs | 108.37, 108.46, 108.47 MiB | 11.152, 11.172, 11.148 MiB |
| those two libraries evicted, three runs | 81.25, 81.21, 80.95 MiB | 11.180, 11.152, 11.148 MiB |

**The figure the gate was banding fell 25% and the figure this program actually allocates moved by
30 KiB.** Over the afternoon the whole-process figure was seen at 92 MiB and at 180 MiB on the same
binary — a factor of two — while the anonymous one stayed inside 4%.

That is the whole explanation of three years' worth of this file's puzzlement in one afternoon: a
neighbouring round walking a corpus is exactly what evicts a shared library, the eviction costs
almost no time so no clock probe can see it, and the resulting fall is uniform across the four
documents because it is the *same libraries* in all four processes. It is also why
`open_peak_mib` — the same measurement in a process with no graphics device in it — has never
moved by a kilobyte: that process maps six mebibytes of binary and a libc, and they are always hot.

### The anonymous high-water discriminates, and the whole-process one barely did

Eleven consecutive samples per row, at one-minute load averages between 4.0 and 11.2:

| | anonymous high-water | spread | whole-process, same runs |
|---|---|---|---|
| bring-up, no document | 10.89 .. 11.05 MiB | 0.8% | 92 .. 144 MiB |
| `PDF20_AN001-BPC.pdf` | 26.79 .. 27.66 MiB | 2.2% | 111 .. 135 MiB |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 31.25 .. 31.80 MiB | 1.8% | 116 .. 139 MiB |
| `ISO_32000-2_sponsored_EC3.pdf` | 31.64 .. 32.32 MiB | 2.1% | 117 .. 140 MiB |
| `bug1815476.pdf` | 43.56 .. 45.32 MiB | 4.0% | 129 .. 153 MiB |

The four documents are 27, 31, 32 and 44 MiB apart in what they allocate — the font-substituting
page costs 60% more than the five-page one — where the whole-process figure separated them by
about a tenth against a band 82 MiB wide.

The residual spread is a **transparent huge page**: this machine has `enabled=always`, a huge page
is 2 MiB, and the samples that read high are the ones whose `AnonHugePages` is 2048 KiB rather
than 0. That is a step the program did not take, and the band has to have room for it.

## Decision

**The launch gate bands the anonymous high-water and prints the whole-process one.**

- Every child now reports `mapped_kib` — `Rss - Anonymous` off `/proc/self/smaps_rollup`, read a
  moment *before* `VmHWM` so the subtraction cannot go negative in the ordinary case.
- Each row's `peak_mib` becomes **`peak_anon_mib`**, `VmHWM - mapped_kib`: what the allocator asked
  the kernel for at the high-water. Bands are the observed range widened by a tenth of the value on
  each side and rounded outwards to half a mebibyte — one huge page's headroom on the smallest of
  them.
- It joins `read_kib` and `open_peak_mib` in the group **judged on any machine**, which is where a
  figure the machine cannot move belongs and where the whole-process figure never should have been.
- `VmHWM` is still measured and printed on every run, beside the driver's share of it, and banded
  nowhere.

**What the gate no longer claims**, stated plainly because a gate that quietly narrows is worse
than one that fails: it no longer says anything about how much memory the *process* occupies on
this machine, so a regression that mapped a large file rather than allocating it — a document
`mmap`ped whole instead of read incrementally — would not fail this line. `read_kib` gates the read
side and the printed `MiB resident` is what a person reads. What it claims instead is sharper:
`PDF20_AN001-BPC.pdf`'s old band was 82 MiB wide, three times the whole quantity now judged, so a
page one that allocated three mebibytes more than it does passed then and fails now.

## Consequences

- **Principle 2's fourth number is gated again, and on every run rather than on a quiet one.** It
  was the one figure of the four that could not be declined by a probe and therefore the one that
  had to be either red or ignored.
- **`doc/questions/Q29` is unchanged.** It is about the clock half of this instrument and this
  round touched none of it. The memory half is no longer part of that question.
- **The rule generalises past this gate**, and it is worth a sentence wherever a resident-set
  figure is quoted: `examples/open_cost` and `examples/confined_peak` quote `VmHWM` too. Neither
  brings up a graphics device, so neither is exposed the way this was — but "the process's
  high-water" is a measurement of the machine's page cache as much as of the program whenever the
  program maps something large, and the anonymous total is the half that is the program's.
