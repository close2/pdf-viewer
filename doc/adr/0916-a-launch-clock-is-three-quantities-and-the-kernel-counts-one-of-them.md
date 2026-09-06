# 0916 — A launch clock is three quantities, and the kernel counts one of them exactly

Session 938. Status: **accepted**. The first of this round's two records; the second,
[ADR 0917](0917-the-cold-open-gets-a-figure-with-no-clock-in-it.md), is the figure that lets the
gate say something on a machine it cannot make a claim about.

## Context

[ADR 0910](0910-nine-tenths-of-a-memory-high-water-belonged-to-somebody-elses-libraries.md)
resolved the half of the launch gate that is a memory high-water, and it did so by finding out
what the number was *made of*: nine tenths of it was resident pages of shared libraries the Vulkan
loader had mapped, proved with one `posix_fadvise` that moved the figure a quarter while the
memory this program had asked for moved thirty kilobytes. The gate then banded a quantity that
does not contain the mechanism, and claims less and better.

The other half is a wall clock, and it has been the subject of five rounds:

- **Session 922** derived the bands from forty-four runs at load averages between 1.6 and 5.0.
- **Session 926** found `doc/PDF20_AN001-BPC.pdf`'s cold open outside its band in three runs of
  four and wrote three hypotheses rather than moving anything.
- **Session 931** refuted all three — the gate's copies are reflinks and have never had an extent
  of their own; the disk is not what moves the figure; there is no regression on the open path —
  and named a fourth thing: the guard is the quickest of fifty passes inside one warmed process
  while every figure it guards is one first pass in a fresh one (trap 34).
- **Session 934** was sent to derive a band for that first pass and found no quiet machine in
  seventy-five minutes of sampling.
- **Session 935** left one clock figure a percent over its ceiling with every probe in band.

`doc/questions/Q29` put three options to the owner and `A29` answers: *"combining option 1 and 2
sounds good"* — derive the busy probe so a loaded run declines, **and** take the clock figures off
every round's sequence and run them when a round has the machine.

## What a launch clock is made of, measured

An elapsed time on a shared machine is three quantities, and only the first is this program's:

1. **the work**, which is instructions executed — [ADR 0917](0917-the-cold-open-gets-a-figure-with-no-clock-in-it.md);
2. **the rate this machine executes them at today**, which a neighbour changes without ever taking
   a turn on the processor;
3. **the time the thread was ready to run and had no processor**, which the kernel accumulates per
   thread, in nanoseconds, at every wakeup, as `sched_info.run_delay`.

The experiment separates 2 from 3, and it is a controlled arm rather than an inference. Eight
spinning processes were pinned to **exactly the eight CPUs this gate pins its children to** — four
Zen 5 cores and both SMT threads of each — and twenty warm opens of the five-page document were
measured in each arm:

| | warm open, min of 20 | the fifty-pass probe | `run_delay` over the phase |
|---|---|---|---|
| the machine as found (load 9) | 0.546 ms | 0.798 ms | 0.000 ms in 18 of 20 |
| eight spinners on those same CPUs | 0.779 ms (**+43%**) | 1.385 ms (**+74%**) | **0.000 ms in 20 of 20** |

**The figure rose by nearly half and the thread waited for nothing at all.** A short, freshly woken
task is what this scheduler runs first, so a process of a millisecond is essentially never
preempted — even when every processor it may use is saturated. What the neighbour did was sit
*inside* the core: an SMT sibling, a shared cache, and a boost clock four busy cores do not reach.
Nothing in a wall-clock figure can be subtracted for that.

Quantity 3 is real all the same, and it is the whole of an *excursion*. Fifteen consecutive warm
opens on a loaded machine:

```text
1.065 1.083 1.040 1.016 1.021 1.012 0.987 0.991 1.037 1.082  3.947  0.967 1.039 1.036 1.044
                                                             ^^^^^
run_delay:  0.000 on thirteen of them, 0.019 on one, and 2.825 ms on the eleventh
```

1.04 + 2.83 = 3.87 against 3.95 measured. That is session 931's observation — "the probe moved
1.3% while the figures moved by factors of two" — with a name and an exact number on it. The
fifty-pass probe could not see it because the probe's *child* was not the child that waited.

Two smaller findings, both measured and both used:

- **`/proc/self/schedstat` reads zero however busy the machine is**, because it is the thread
  *group leader* and libtest runs a test on a thread of its own. `/proc/thread-self/schedstat` is
  the measurement. The first version of this instrument reported zeros for an hour.
- **The first-pass probe session 931 asked for a band on does not discriminate.** Its ratio to the
  fifty-pass reading, per child, has a median of 1.64 on an idle machine, 1.50 at load 9, 1.40 with
  spinners on the SMT siblings and 3.23 with spinners on the pinned cores — the quiet machine in
  the *middle* of the contended ones. 300 samples idle, 15 to 20 an arm otherwise. A probe whose
  reading does not order the conditions it is meant to separate cannot decline on them.

## And the machine itself moved between two days of this round

The round was interrupted and resumed two days later, and the same binary reads differently:

| | the fifty-pass probe | one busy thread on a performance core |
|---|---|---|
| 2026-09-04, in this gate's own runs | 0.703 to 0.724 ms | — |
| 2026-09-06, idle, 300 consecutive samples | **0.849 to 0.951 ms** | **3.74 GHz** against a rated 5.16 |

At 63 °C, on the `performance` platform profile, with `amd_pstate` active and `scaling_max_freq`
reading 5 157 895. The bands in `doc/checks/launch-path.toml` are a claim about a processor doing
5.16 GHz, and on the one idle window this round was given the processor was not doing it. So the
first-pass band `Q29`'s option 1 asks for is **still** not derived, for a reason no previous round
met: not a busy machine, a *different* one. A band taken that day would have put the slower machine
into the claim, which five rounds have refused to do.

## Decision

**Every duration this gate judges is elapsed time less the time its thread spent waiting for a
processor**, and a figure whose thread waited for more than a tenth of its own elapsed time is
**declined** rather than corrected.

That is ADR 0910's subtraction in another unit: band a quantity that does not contain the
mechanism. On a machine with nothing else on it the wait is zero and every band derived before this
change means exactly what it meant; under a neighbour it is the whole of an excursion. The decline
above a tenth is there because a thread preempted that heavily also came back to caches and a
branch predictor somebody else had used, and *that* part is quantity 2 and cannot be subtracted.

**Option 1 is delivered as a count rather than as a band**, which is what the measurement supports:
the busy probe `Q29` asked for needed ten minutes of an idle machine that four rounds could not
get, and the kernel has been counting the same thing exactly, per thread, all along. A tenth is
dimensionless, needs no derivation on any particular machine, and separates a population with
nothing between its two halves — fourteen samples at zero and one at 72%.

**The disk's guard gains the half it was missing.** `io_ms` reads eight mebibytes, so it measures
throughput; a small document's cold open is a handful of seeks. `io_latency_ms` is a 128 KiB
single-extent file, evicted and read beside every cold sample: 0.086 min and 0.098 median over
forty idle samples, where the eight-mebibyte probe sat at 2.8 ms — 2.6 GB/s, at which rate a
hundred kibibytes would be 0.04 ms. Trap 34 in one more unit. It also sizes the term that has been
putting the smallest rows over their ceilings: this gate's own copies answer a cold read in 0.125
ms (one extent) and 0.199 (two), where session 931 recorded 0.109 and 0.127 for the same
measurement.

**And the clock figures are asked for rather than assumed** — `Q29`'s option 2, and
`PDFVIEWER_LAUNCH_CLOCKS` is how, an environment variable being the owner's stated preference for a
switch of this shape (`A28`). Without it the gate takes one sample of each phase and judges
twenty-one figures no machine can move, in three seconds; `doc/verify.md` says to set it when a
round has the machine.

## What it cost, and what it does not do

- **A clock figure is now a slightly different quantity from the one its band was derived on.** The
  check file says so where a reader will meet it. The correction is zero on the machine the bands
  describe, so no band moved and none needed to — **no band has been widened for six rounds
  running**.
- **Quantity 2 has no instrument but the calibration probe, which over-reads it** — 74% against the
  figure's 43% in the arm above. That is a conservative guard, which is the right direction for a
  guard and the wrong direction for a gate anybody wants a verdict from. It is why ADR 0917 exists.
- **The first-pass probe stays printed and unbanded**, now with the measurement saying why rather
  than with a promise that a quiet afternoon would fix it.
