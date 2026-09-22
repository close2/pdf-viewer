# 1226 — A gate that says which of its figures the machine was fit to judge

Session 1194. Status: **accepted**. Adds a guard to `crates/viewer-ui/tests/launch_path.rs` beside
the ones [ADR 0903](0903-what-a-declined-figure-now-says-and-what-a-quiet-machine-still-owes.md) and
[ADR 0916](0916-a-launch-clock-is-three-quantities-and-the-kernel-counts-one-of-them.md) put there. Amends
nothing: every existing decline stands, and no band is touched.

## 1. The failure this fixes is not a false failure

`launch_path.rs` is the only instrument in this tree that can see a startup regression, and
`CLAUDE.md` principle 2 makes its four figures gates. It already declines a figure in five ways —
the profile, the sample override, the calibration probe, that probe's first pass, the disk's rate
and latency, and the thread's own wait for a processor — each paid for by a false failure somebody
read.

**What was happening instead is worse than a false failure, and no amount of declining addresses
it: rounds stopped running it.** Three rounds in one week did not take the clock figures at all,
each on a private judgement that the load average was too high to believe them. A figure nobody
takes is a figure nobody can argue with, and a regression that lands in such a week lands in
silence. The gate's own output gave a round no way to make that judgement in public, so each made
it alone and the reasoning left no trace.

So the guard added here is not another way to decline. It is the gate saying, in its own output,
**which figures this machine was fit to judge and which it was not** — so that running it is always
better than not running it, at any load.

## 2. The ceiling is the machine's physical core count, and the derivation is the whole argument

`the_load_ceiling` counts distinct `(physical_package_id, core_id)` pairs across `sysfs`'s CPU
topology. It is not a number written down, and the reason it is *this* quantity is ADR 0916's
measurement rather than an intuition about load averages.

A load average of one runnable task per **physical** core is the point at which the scheduler can no
longer give a freshly woken child a core of its own. Below it there is an idle core to place it on;
above it, somebody else is already on the other hardware thread of whatever core it lands on. ADR
0916 measured exactly what that costs and what it looks like: eight spinning processes on precisely
the eight CPUs these children are pinned to raised a warm open **43%** and the calibration probe
**74%**, while the kernel's own `sched_info.run_delay` read **exactly zero in all twenty samples**.
The neighbour was not taking turns on a processor — it was sitting inside the core, as an SMT
sibling, in a shared cache, on a boost clock four busy cores do not reach.

That is contention a clock cannot be corrected for. `corrected` removes the queueing and `WAITED_SHARE`
declines a thread that queued too much, and both read zero through the whole of the effect above. So
the only honest response to it is to decline the figure, and the physical core count is where
declining has to start.

On the machine `doc/checks/launch-path.toml`'s bands were taken on it comes to **twelve** — which is
also the number the three rounds of section 1 used by instinct. The instinct was right; what it
lacked was a derivation and a place to write itself down.

**Where the topology cannot be read the ceiling is `f64::INFINITY` and nothing is declined for
load**, and the run says so. A machine this gate cannot ask about is not a machine to decline on;
silently declining would reproduce section 1's failure with the gate's own name on it.

## 3. The protocol, and the one thing it must not do

Each figure is taken as before — the minimum of `SAMPLES` fresh children — with a load reading
immediately before and immediately after. The load that describes the attempt is the **larger** of
the two, because a figure is only as quiet as the busiest moment at either end of it.

- Above the ceiling, the whole attempt is taken again, up to `LOAD_ATTEMPTS` times.
- **The loop stops at the first attempt taken under the ceiling, and that attempt's figure is the
  one judged.** This is the part that matters and it is easy to get wrong: a `Band` has a floor as
  well as a ceiling — a figure below its floor is "either a win to record or an instrument that
  stopped measuring" — so a loop that pooled every attempt and kept the global minimum would judge
  the smallest of up to three times as many draws against a band derived from one. That biases the
  minimum downward and turns a busy afternoon into a failure out of the *bottom* of a band, which
  is a false failure wearing the disguise of an improvement. Stopping at the first quiet attempt
  keeps the population exactly `SAMPLES`, and the population is printed so that a reader can check
  that it did.
- Where no attempt was quiet, the **smallest** figure of all of them is kept and is **not judged**.
  Smallest, because a startup figure's noise is one-sided: contention adds time and never removes
  it. Not judged, because it is still an estimate of a machine nobody banded. The load it was taken
  under goes out beside it, so the run says *how* busy rather than only *too busy*.

**Why repeating helps at all, given that a one-minute average lags by a minute.** It helps in one
case and the case is common here: a neighbour's build has just finished and the average has not yet
decayed. Re-taking the figure is how that is found out. It cannot wait out a machine that is
genuinely busy, and it is not meant to — which is why the attempt count is small and the alternative
is a figure that says so rather than a figure that quietly passes.

**Why the average and not the instantaneous runnable count.** `/proc/loadavg`'s fourth field is what
actually shapes a figure of milliseconds, and it is far too noisy to band: one sample of it says
nothing about the tens of children a figure is made of. So the average is the quantity compared
against the ceiling and the runnable count is printed beside it, where a reader can see the two
disagree.

## 4. The load is asked first among the declines

`Declined::TheLoad` is tested before the wait, the probe, the probe's first pass and the disk. A
calibration probe out of band under a load average of twenty **is** the load; naming the probe would
send a reader looking for a regression in the machine that produced it, which is ADR 0903's
diagnosis being read backwards. The load is the cause and the rest are its symptoms, and the order
the declines are asked in is the answer a reader gets.

## 5. What is deliberately unchanged

- **No band is widened.** Every band in `doc/checks/launch-path.toml` is the same number it was;
  this decides *whether* a figure is judged, never *against what*.
- **A declined figure is still printed, and `gate_ratchet::band` still prints its distance to the
  nearer edge.** A figure creeping toward an edge over ten loaded rounds stays visible for all ten,
  which is the defect ADR 1075 was written for one shape along.
- **The steady figures are untouched and still judged at any load.** How many bytes an open reads,
  how many read calls it makes, how much this program allocates and how many instructions an open
  executes cannot be moved by a neighbour, so nothing here reaches them — and they remain what a
  round gets on every run in about two seconds without asking for clocks at all.
- **It is not a `gate_ratchet` bound, and the ratchets sweep asked.** That crate is for a number
  this *tree* must not exceed, printed beside the population it bounds so a creep toward it is
  visible; a ratchet has a direction. A load average is a property of the **machine**, recomputed
  every run from the kernel's topology, and "it must not grow" is not a claim anything here could
  make. What does apply is the printing half of the rule, and the ceiling goes out in the run's
  header and beside every figure it declines.
- **The default gate line pays nothing.** The attempt loop runs only when `PDFVIEWER_LAUNCH_CLOCKS`
  is set, because a run that was not asked for clocks judges only figures the machine cannot move.
