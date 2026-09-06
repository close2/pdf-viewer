# 938 — A launch clock is three quantities, and one of them was already counted

Date: 2026-09-06 (begun 2026-09-04; a model limit ended the round mid-experiment and it resumed
two days later, which turned out to be a measurement of its own).
ADRs: [0916](../adr/0916-a-launch-clock-is-three-quantities-and-the-kernel-counts-one-of-them.md),
[0917](../adr/0917-the-cold-open-gets-a-figure-with-no-clock-in-it.md).
Answered: [`A29`](../questions/A29-a-clock-gate-on-a-machine-three-rounds-share.md) — "combining
option 1 and 2 sounds good".
Touched: `crates/viewer-ui/tests/launch_path.rs`, `doc/checks/launch-path.toml`, `doc/todo/02`,
`doc/todo/42`, `doc/verify.md`, `doc/traps/instruments-and-reports.md` (trap 36), `doc/HANDOVER.md`,
`doc/questions/Q29`, two ADRs, this file. **No pixel moves**: no crate that draws changed.

The round was sent to do for the launch gate's clock half what session 935 did for its memory
half — find out what the number is made of, and band a quantity that does not contain the
mechanism.

## What a launch clock is made of

Three quantities, and only the first is this program's.

**The work.** Now counted: `open_kinstructions`, under callgrind, in a child that opens the
document and stops. Five consecutive runs of each of the four documents spread **0.00007% to
0.004%**, and `read_calls` — `/proc/self/io`'s `syscr` — was identical on every run: 25, 31, 89,
1077.

**The rate this machine executes it at.** The controlled arm: eight spinning processes pinned to
*exactly* the eight CPUs this gate pins its children to. A warm open rose **43%**, the fifty-pass
calibration probe rose **74%**, and the kernel's wait counter read **exactly zero in all twenty
samples**. The neighbour never took a turn on a processor; it sat inside the core. Nothing in a
wall clock can be subtracted for that.

**The time the thread had no processor.** `sched_info.run_delay`, counted in nanoseconds at every
wakeup. It is zero in the ordinary case and it is the whole of an excursion: of fifteen consecutive
warm opens, fourteen read 0.97 to 1.08 ms with a wait of zero and one read **3.947 ms with a wait
of 2.825**. That is session 931's "the probe moved 1.3% while the figures moved by factors of two",
with an exact number on it.

Two things were measured and thrown away, which is why they are here. The first version of the
instrument read `/proc/self/schedstat` and reported zeros under saturating load, because that is
the thread *group leader* and libtest runs a test on a thread of its own — trap 36. And the
first-pass probe session 931 asked for a band on **does not discriminate**: its ratio to the
fifty-pass reading has a median of 1.64 idle, 1.50 at load 9, 1.40 with spinners on the SMT
siblings and 3.23 with spinners on the pinned cores, the quiet machine in the middle of the
contended ones.

## What the gate says now

- **Every duration is elapsed time less the wait for a processor**, and a figure that lost more
  than a tenth of itself to it is declined rather than corrected. On the machine the bands describe
  the correction is zero, so **no band moved — the sixth round running.**
- **Two counted figures per document**, judged on any machine under any load: `open_kinstructions`
  and `read_calls`, bands one per cent wide against a measured spread of a hundredth of that.
- **A disk probe made of the same stuff as the figure**: `io_latency_ms`, a 128 KiB single-extent
  file evicted beside every cold sample, where `io_ms` reads eight mebibytes and measures
  throughput. A small document's cold open is round trips, and this gate could not see one.
- **The clock figures are asked for rather than assumed** — `PDFVIEWER_LAUNCH_CLOCKS`, an
  environment variable as `A28` prefers. `doc/todo/02` §2 runs the gate without it;
  `doc/verify.md` runs it with, when a round has the machine.

## Green at load and green quiet

| | figures judged | verdict |
|---|---|---|
| §2's line, load average 6.14 | 21, all counted or steady | **green in 3.1 s** |
| the clock run, load average 4.12 | 37 printed, the counted ones judged | **exit 0**, `NOT JUDGED`, naming the probe |

The clock run declined because the calibration probe read 0.896 ms against a band of 0.62 .. 0.78 —
and it was right to.

## The machine moved under the round

On 2026-09-04 the fifty-pass probe read **0.703 to 0.724 ms** in this gate's own runs. On
2026-09-06, on an idle machine, the **same binary** read **0.849 to 0.951 ms** over 300 consecutive
samples — and one busy thread pinned to a performance core reached **3.74 GHz against a rated
5.16**, at 63 °C, on the `performance` platform profile with `amd_pstate` active.

So the idle window four rounds had waited for arrived and could not be used for what `Q29`'s option
1 asked. The machine was not busy; it was a different one. A band taken that day would have put a
27% slower processor into a claim five rounds have refused to put a busy one into. Option 1's
*purpose* is delivered instead by the count, which needs no particular machine at all.

## What is left, with an instrument under it at last

`bug1815476.pdf`'s cold open has been a per cent or two over its ceiling since session 933, on quiet
machines, with every probe in band. Two explanations remained and neither could be tested
backwards: the program's open grew, or this machine's small-read latency grew. `open_kinstructions`
settles the first exactly from the next round on. The second already has a figure beside it — the
cold read session 931 measured at 0.109 ms reads 0.125 to 0.199 today.
