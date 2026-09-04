# 0903 — What a declined figure now says, and what a quiet machine still owes

Session 931. Status: **accepted**. The change that follows from
[ADR 0902](0902-the-guard-is-the-best-of-fifty-and-every-figure-is-a-first-pass.md), which is the
reading.

## Context

0902 found that the launch gate's calibration probe is the quickest of fifty passes inside one
warmed process while every figure it guards is one first pass in a fresh one, and that this is why
session 926 could not separate its three hypotheses. Two things about the instrument follow, and
they are different in kind: one is a *measurement it was not taking*, the other is a *sentence it
was not printing*.

## Decision 1 — every child reports its first pass beside its best of fifty

`calibration_pass` already ran the same fixed work fifty times and threw forty-nine of the
readings away. It now keeps the **first** one as well, and both go out on the child's `measured`
line as `calibration_ms` and `calibration_first_ms`. The parent carries the second into `Judged`
beside the first and prints the minimum over children in the run's header:

```text
launch-path: calibration 0.709 ms, band 0.620 .. 0.780
launch-path: the same work as one first pass 1.032 ms, no band — the quantity every figure below
             is made of
```

It costs nothing: the passes were already being run. What it buys is a number in the same units,
on the same fixed work, in the same process, measured the way the figures are measured — which is
the only thing that can say *this child was not in the state the bands are a claim about* when the
fifty-pass minimum says the opposite.

**Printed and not judged**, which is `doc/todo/05`'s standing rule for a figure whose band has not
been derived, and this round could not derive one: the machine carried two other rounds throughout,
at one-minute load averages of 3.5 to 35. A band for it is what would make the gate decline
correctly instead of failing, and deriving it needs the quiet machine
[`Q29`](../questions/Q29-a-clock-gate-on-a-machine-three-rounds-share.md) is about.

## Decision 2 — a figure that is declined says which probe declined it, and what that probe read

The gate printed `(not judged)` and nothing else. Session 926 therefore had four runs, one figure
outside its band, and no way to tell a figure the processor declined from one the disk declined —
which is most of why `doc/todo/42` carried three hypotheses instead of a finding. One line answers
it:

```text
launch-path:   (doc/PDF20_AN001-BPC.pdf: the cold open) not judged: the child that produced it was
               not on the machine's own clock — this child's calibration 1.682 ms, its first pass
               2.719 ms, the disk 4.100 ms
```

`why_not` says which of the four conditions failed — the run is not judging at all, the child's
probe was out of band, its *first pass* was, or the disk was — and prints all three readings after
it, because the reason alone does not say by how much. The condition is a [`Declined`] value
computed where the decision is made rather than three `bool` parameters, which is a signature to
pass the wrong argument to and which `clippy::fn_params_excessive_bools` says so about. The readings are the child's own, paired with its figure
the way `calibration_field` already paired the first of them.

**A complaint carries the same readings**, and that half was added after the first twenty-six
runs, because only the *declined* figures were printing any: eight figures failed across those
runs and the output could not answer "would a tighter guard have declined this instead of failing
it", which is precisely the question the first-pass band exists to settle. A figure with no clock
in it says *no clock in this figure* rather than three dashes.

**The general rule, and it is why this is a decision rather than a print statement: a gate that
declines has to say what declined it.** A refusal with no reason is a measurement nobody can act
on, and it costs the next round the whole diagnosis. It is `doc/traps/instruments-and-reports.md`'s
trap 11 seen from the other side — a report is only as good as the condition it fires on, and a
*non*-report is only as good as the condition it is silent on.

## What this does not do

- **It does not move a band**, and 0902 says why: a band is a claim about a machine and these
  figures were not taken on it.
- **It does not stop the gate failing on a loaded machine.** With the first-pass probe printed and
  not judged, the guard is exactly what it was, and on merged `main` under two parallel rounds it
  fails in about half of twelve runs — five of those six on
  `doc/pdf.js/test/pdfs/bug1815476.pdf`'s cold open. The failure is now *legible* rather than
  mysterious, which is this round's whole delivery on that figure; making it stop is the band
  derivation above.
- **It does not touch the disk probe.** 0902 refuted the hypothesis that a latency probe was what
  was missing, so adding one would be a second guard against something that has not been shown to
  move a figure. The evidence is written down; a round that finds a disk-shaped excursion has the
  measurement to hand.
