# 0884 — The launch path is a gate, and a wall-clock band needs a machine under it

Session 922. Status: **accepted**. The first of this round's two records: the instrument.
[ADR 0885](0885-what-the-launch-path-costs-and-which-of-principle-2s-claims-hold.md)
is what it measured and which of `CLAUDE.md` principle 2's claims survived it.

## Context

Principle 2 names four numbers — "cold open, time-to-first-page, page-turn latency, memory
high-water" — says a regression in any of them "fails the build", and makes cold graphics
bring-up a fifth gate of its own so that "a regression in the driver, the adapter selection or
the shader set is legible as itself rather than as a slower page". Nine hundred rounds in, **not
one of the five was printed by any command in this tree.** `doc/performance.md` records launch
timelines rounds took by hand; `examples/open_cost`, `examples/bring_up` and
`examples/first_frame` are instruments a person points at one document; no gate ran any of them,
and `tools/state.sh` — which exists precisely so that no document has to carry a number — had no
section for them. Round 921 found the sentence claiming these gates run in CI to be false and
filed it as a question.

The reason the gate did not exist is not that nobody wanted it. It is that **a wall-clock gate on
a shared machine cries wolf**, and this tree has three recorded false failures to prove it
(`doc/todo/02` section 2: a corpus document at 32.23 s against a 30 s clock, `pdf-transform`'s
floor reading 33.3 pages a second against 40 where the same tree quiet reads 198.3, and a §14.7
fault that was a *contended reference resolving less structure*). A gate that fires on a
neighbour's load gets switched off, and then the number is gone as well as the gate.

So the question this round had to answer first was not *what does the launch cost* but **what
makes a duration believable here**.

## What the machine turned out to be

Measured, before any band was written: a fixed, serial, in-memory piece of this tree's own work —
the five-page document opened from bytes already in memory and its first page interpreted —
measured **0.75 ms in some processes and 1.50 ms in others, on a machine whose load average was
under two**. A factor of two, with nothing to blame it on.

The cause is in `/sys`, and one line prints it:

```sh
for c in /sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq; do cat "$c"; done | sort -u
```

This processor is an **AMD Ryzen AI 9 HX 370**: four Zen 5 cores at 5.16 GHz and eight denser
Zen 5c cores at 3.29 GHz, twenty-four threads across the two classes. Where the scheduler puts a
process decides how fast it runs, and every wall-clock number this project has ever taken —
including every launch timeline in `doc/performance.md` — was drawn from that lottery without
anybody knowing it was one. It is not noise that averaging removes: it is a bimodal population,
and a band wide enough to hold both ends is wide enough to hold a regression.

## Decision

**`crates/viewer-ui/tests/launch_path.rs` measures principle 2's five numbers, and
`doc/checks/launch-path.toml` holds a band on each. Five things make a duration believable, and
the bands are derived from what is left over after all five.**

### 1. Each figure is the minimum of nine fresh processes

Contention adds time and never removes it, so the fastest of nine is the closest thing to a quiet
measurement a loaded machine can produce, and a run fails only if *every one* of the nine was
slow. Fresh processes rather than repetitions inside one, because that is what makes each sample
a new draw from the core lottery — and because three of the five figures are about a process's
own start.

The child is this same test binary re-executed with `--exact launch_probe`, which is
`pdf-vfs`'s `tests/confined.rs` idiom for the same underlying reason: the thing being measured is
a *process*, and a process cannot measure its own creation twice.

### 2. The children are pinned to the machine's fastest cores

`taskset -c` on a list **derived from `cpuinfo_max_freq` rather than written down**, so a machine
with one class of core is not pinned at all and a machine with three is still pinned to the
fastest. This is the fix for the factor of two above, and it is the single change that turned this
from an instrument into a gate: over twenty-eight unpinned runs at load averages from 1.8 to 40
the spread of the figures was 100% to 400%, and over sixteen consecutive pinned runs at a load of
about two it is 0.6% to 22%. The two arms are not a controlled experiment — the loads differ as
well as the pinning, and this file says so rather than implying otherwise — but the pinned arm's
*calibration* probe, which is the same work in both, spans 0.6% where the unpinned one spans
114%, and that comparison has only the pinning in it.

### 3. A calibration probe decides whether the clock figures are judged at all

The same fixed, serial, in-memory work, in the same kind of pinned child, the quickest of fifty
passes, the minimum over nine children. Its band is in the check file. **Out of band, every figure
is still printed and the clock ones are not judged, saying why.**

This is the whole answer to the false-failure problem, and it was checked against the defect
rather than assumed (trap 13). With eight busy threads pinned onto the same eight CPUs — a load
deliberately confined to this gate's own cores so that the neighbouring rounds kept theirs — the
probe read **0.955 ms against a band of 0.62 .. 0.82**, the gate said `NOT JUDGED`, and **twelve
of the twenty-eight figures were outside their bands**. Every one of those twelve would have been
a false failure. The same tree, quiet, has all twenty-eight inside.

### 4. Two of each row's figures cannot be moved by the machine, and those are judged always

How many bytes an open reads (`rchar` from `/proc/self/io`, across the open alone) and what the
open costs in memory (`VmHWM` of a process with no graphics device in it) came back **identical in
every one of the forty-four runs** the bands were derived from. They are properties of the reader
rather than of the afternoon — and they are what gates principle 2's claims about *what the launch
path does*, which are the claims that matter most and the ones a duration can only hint at. Under
the load that made twelve clock figures fail, not one of these moved.

So the gate always tests something. A perf gate whose only mode is "not judged" would be trap 25
with a clock on it.

**A third figure was in this group for most of the round and was demoted by its own evidence.**
`peak_mib` — the high-water mark of the process that brings a device up and draws page one — was
identical across all forty-four runs, so it was banded as tightly as the other two. An hour later,
on the same tree and the same binary and an idle machine, **all four rows had fallen together by
about 12%** and the gate failed on all four. What moved is the driver's allocation; the memory
figure with no device in it did not move by a kilobyte, in the same runs. So the high-water mark
is judged like a duration — only where the calibration says this is the machine — with a band
spanning what has been observed rather than one afternoon's value.

The general shape is worth more than the instance: **"deterministic" is a claim about a
population of runs, and forty-four consecutive ones is not a wide population.** A figure that
comes out of another program — a driver, a kernel, an allocator — is that program's determinism
and not ours, and the way to tell is to find the version of the figure that has none of that
program in it. Here that pair is `open_peak_mib` beside `peak_mib`, an order of magnitude apart,
and only the smaller one is ours.

### 5. It judges under `release`, and says so under anything else

`[profile.gates]` costs `Document::open` between 4.06% and 12.30% against `[profile.release]` —
`Cargo.toml`'s own table, ADR 0666 — which is wider than these bands. A launch figure is a claim
about the program a person runs, so this is the one line of `doc/todo/02` section 2 that is not
`--profile gates`, and the harness derives the profile from the directory its binary sits in and
prints-without-judging under any other. The cost is a `release` link of `viewer-ui` where the
sequence would otherwise have none, measured at 2m24s cold in a fresh worktree and nothing when
warm; section 5 of that file already builds the release binaries every fifth round.

### How the bands were derived

**Forty-four runs on this machine, pinned, `release`, at load averages between 1.6 and 5.0** —
thirty for the three `doc/` documents and fourteen for the fourth row — with, for each figure,
`low = the smallest observed × 0.85` and `high = the largest observed × 1.20`, rounded outwards;
and for the two deterministic figures, the observed value ± max(2%, 2 units). The margins are
about one observed spread wide (the clock figures' own spread over those runs was 1.9% to 22%),
so a run at the edge of the measured distribution passes and a regression of about a third fires.

Nothing here was chosen and then defended. The bands are what the machine did.

### The cold page cache, which principle 2 asks for by name

"Cold-start and time-to-first-page are CI gates with numbers attached, **measured with a cold page
cache**." Root's `/proc/sys/vm/drop_caches` is not available to this user — and would be the wrong
instrument anyway, since it empties the *machine's* cache and would evict every neighbouring
round's working set. What an unprivileged user may do is `posix_fadvise(POSIX_FADV_DONTNEED)` on a
file they can open for writing, which is what `dd of=<file> oflag=nocache conv=notrunc,fdatasync
count=0` is.

It works, and was checked before it was believed: nineteen megabytes read in **16 to 26 ms** after
the drop against **3 to 7 ms** with the pages in place. The gate drops the cache of **a copy** it
makes under the build directory, never of a file in `doc/` — dropping a file's cache means opening
it for writing, and a gate that opens a document of the repository for writing is one bad flag
away from changing it. The copy also has to be outside `/tmp`, which is a `tmpfs` here whose pages
cannot be dropped at all.

The cold and warm arms are both measured and both banded, which is what keeps the cold one honest:
if eviction ever silently stopped working, the cold figure would fall to the warm one and out of
the bottom of its band. On the largest document those two are 23.0 ms and 13.2 ms, so the check
has 10 ms of daylight in it.

## Consequences

- `tools/state.sh launch` prints the five numbers, and no document carries them.
- The gate is in `doc/todo/02` section 2's sequence and costs about six seconds after its build.
- **A round that makes the launch faster moves the number in the check file and says why**, the
  same discipline `fixed-documents.toml` asks for. A figure below its band is either that win or
  an instrument that stopped measuring, and the witnesses printed beside it — page count, command
  count, bytes read, the adapter's name — are what tell those apart.
- **The adapter is named beside the bring-up figure**, because a machine that quietly fell back to
  a software rasteriser would otherwise report a different measurement wearing the same number.
  On this machine, headless, that is `AMD Radeon 890M Graphics (RADV STRIX1) (IntegratedGpu,
  Vulkan)` — the real adapter, which `doc/environment.md` has said since session 552 is reachable
  without the owner's session and which no gate had used until now.
- Children are spawned with `DISPLAY` and `WAYLAND_DISPLAY` removed. Every figure here is
  headless, and a graphics stack that finds a display it has no authority cookie for spends the
  difference failing an X handshake — which user `AI` does on every run.
- **What is not in the time-to-first-page figure is the window**: `EventLoop::new`, the window,
  the surface and the present need a display server, and a gate that skipped silently without one
  would be worse than none (`doc/environment.md` says the same of `Xvfb`). `pdf-viewer --trace`
  under `Xvfb` remains the instrument for the whole launch, and ADR 0885 records what it says
  today.

## What this does not decide

**Whether these run in CI, on what hardware, and what a failing build means are the owner's**, and
they are the substance of round 921's question. Everything above is a claim about *this* machine:
the bands name it, the calibration probe enforces it, and a CI runner would print every figure and
judge none of them. That is the honest default, and moving it is a decision rather than a
configuration change — a band is a machine.
