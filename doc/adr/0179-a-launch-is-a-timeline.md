# ADR 0179 — A launch is a timeline, and most of it had never been timed

Status: accepted, 2026-08-04 (session 274).

## Context

The project owner decided in the two-hundred-and-seventy-third session that **page one goes to the
graphics device**, and `CLAUDE.md` records what follows from that as an obligation rather than a
licence:

> creating the device and compiling the pipelines is now part of time-to-first-page, so it is a
> number to measure and to keep small

with a third requirement beside it — "**cold bring-up is its own gate**, separate from
time-to-first-page, so that a regression in the driver, the adapter selection or the shader set is
legible as itself rather than as a slower page".

What existed was `--trace`, which prints a duration for each step it knows about, and quorra's
`StartupTimings`, which splits bring-up three ways. Neither answers the question a person asks by
waiting: **how long from the process starting to the first frame on the screen?** A per-step
duration cannot be added up, because the sum of the steps somebody thought to time is not the
launch — it is the launch minus everything nobody thought to time.

## Decision

**One `Instant`, taken as the first statement of `main`, and one mark per milestone**, printed once
under `--trace` when the first frame reaches the window. Two columns: when the step finished, which
is what a person waiting sees, and what it cost, which is where a regression appears. Neither is
derivable from the other once a step is added or reordered.

```
trace: launch path, process start to first present:
trace:   arguments                 0.014 ms  (+0.014)
trace:   document read             8.079 ms  (+8.065)
trace:   chrome fonts              9.457 ms  (+1.378)
trace:   document open            37.225 ms  (+27.768)
trace:   event loop               45.236 ms  (+8.011)
trace:   window                   45.392 ms  (+0.156)
trace:   graphics device          90.519 ms  (+45.127)
trace:   first present           144.609 ms  (+54.090)
```

Beside it, two examples that take the two largest steps apart, because the timeline says *which*
step is expensive and nothing about why:

- `cargo run --release -p pdf-model --example open_cost -- <file>` — every step
  `viewer_core::Open::around` and `notes::about` take before a window exists, each measured on its
  own.
- `cargo run --release -p render-quorra --example bring_up -- [all|vulkan|gl]` — instance creation,
  `request_adapter` and `request_device` separately, which a host cannot see because quorra
  measures all three of the first as one figure (its `adapter_enumeration` starts before
  `wgpu::Instance::new`).

## What it found

Under `Xvfb` with `lavapipe`, release, this machine (ADR 0126's recipe):

| | 5 pages, 173 KB | ISO 32000-2, 1023 pages, 101 318 objects |
|---|---|---|
| document open | **0.84 ms** | **27.8 ms** |
| graphics device | 40.6 ms | 45.1 ms |
| first present | 65.4 ms | 54.1 ms |
| **process start → first frame** | **145 ms** | **145 ms** |

`open_cost` on the same two documents:

| step | ISO 32000-2 | 5-page |
|---|---|---|
| `Document::open` (§7.5) | **12.5 to 22.6 ms** | 0.202 ms |
| `Pages::new` (§7.7.3) | 0.23 to 0.47 | 0.031 |
| `PageLabels::read` (§12.4.2) | 0.17 to 0.34 | 0.018 |
| `Outline::read` (§12.3.3) | **3.35 to 6.61 ms**, 988 items | 0.112, 5 items |
| `signature::signatures` (§12.8) | **1.55 to 3.70 ms, for none** | 0.146 |
| everything else | under 0.02 each | under 0.01 |

**Five runs, and the spread is a fact about the machine rather than about the code**: every column
moves by up to a factor of two together, so a single run is a ranking and not a measurement. The
ranking is stable across all five.

**And the first version of this table said "6.716 ms for 38 items"**, which is `items.len()` — the
top level of a book's table of contents, its chapters. There are 988, at 3.4 to 6.7 µs each, and
the example prints both now. A per-item cost divided by the wrong count is how a proportionate
number becomes a scandal.

**`CLAUDE.md`'s "a 500-page document must open no slower than a 5-page one" is false today, by a
factor of thirty-three**, and it was stated as a rule rather than measured as one. The three costs
are named and priced in [todo 42](../todo/42-the-launch-path.md); none of them is fixed here,
because a round that measures and fixes in one sitting cannot say which of the two the numbers
came from.

`bring_up`, on the real adapters (this machine has three: RADV, `llvmpipe`, `radeonsi`), three
processes apiece:

| backends | instance | `request_adapter` | `request_device` | usable device |
|---|---|---|---|---|
| all | 21–32 ms | 34–36 ms | 1.7 ms | 57–70 ms |
| Vulkan only | 9–16 ms | 39–43 ms | 1.7–2.8 ms | 55–57 ms |
| GL only | 22–25 ms | 0.8–1.1 ms | 3.2–5.3 ms | 26–31 ms |

**The backend set is not the lever.** Restricting the instance to Vulkan halves instance creation
and gives all of it back in `request_adapter`: the cost is enumerating the physical devices and
querying their properties, and it is paid wherever it is first asked for. (The GL row is not an
option — it is a different renderer's device, and it is here because a row that only shows two
configurations cannot show that the *total* is the invariant.)

**What is a lever is overlap, and it is not this tree's to pull.** An instance needs no window, so
it can be created on a thread started at `main`'s first line while the document is read and the
window made. `bring_up overlap` measures exactly that, four processes: opening ISO 32000-2 and
creating an instance one after the other costs 44.4 to 50.0 ms, and both at once 22.9 to 28.9 —
**about 20 ms of a 145 ms launch**. quorra's `Device::for_surface` creates the instance itself, so
this needs an entry point that takes one, which is `doc/QUORRA_FEEDBACK.md` §8.2 along with the
field split §8.1 asks for. Told to the team rather than worked around: a host that made its own
instance and handed it to a library that ignores it would be measuring nothing.

## Two lessons, both about the instrument

**Measure each configuration in its own process.** The first version of `bring_up` created two
instances in one process and reported 26.0 ms for `Backends::all()` against 4.4 ms for
`Backends::VULKAN` — a 6× "finding" that is entirely the driver loader being warm the second time.
One measurement per process, and compare processes.

**A step nobody timed is a step nobody has an opinion about.** `EventLoop::new` costs 8–37 ms on
this machine and appears in no design document, no gate and no previous session's notes. It is the
X connection, and it is a quarter of the launch on a small document. It is not a defect; it is a
thing that was invisible.

## What this is not

It is not yet the gate `CLAUDE.md` asks for. A gate needs a number to fail against, and the
numbers above are one machine's software adapter under a virtual X server; the same build on the
real device is the user's to run. What ships here is the *instrument* and the baseline, which is
the order the same file prescribes for every other perf gate: "targets are set once a spike gives
a real baseline, rather than invented now".
