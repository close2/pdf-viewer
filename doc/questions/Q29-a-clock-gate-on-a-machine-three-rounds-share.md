# Q29 — A clock gate on a machine three rounds share

Asked by round 931, which was sent to settle the one launch figure that would not sit in its band
and found that the figure is fine, the code is fine, and the *machine* is the thing the bands are a
claim about — a machine this project has not had to itself since parallel rounds began.

`Q28` is round 929's and `Q27` was taken twice on the same day by rounds 926 and 927, which is its
own small finding; this is the next free number after those. (Session 934 settled that collision by
renumbering the font question to `Q35`; `Q27` is the cost-floor question alone.)

## What the tree does meanwhile

Nothing is blocked. `crates/viewer-ui/tests/launch_path.rs` runs, prints every figure, and judges
the eight with no clock in them on any machine. Its clock figures are judged where its calibration
probe holds and declined where it does not, and since this round a declined figure says which probe
declined it and what that probe read (ADR 0903). The gate is in `doc/todo/02` §2 and stays there.

## The situation

`CLAUDE.md` principle 2 says perf gates run in CI and a regression fails the build. ADR 0884 built
that here, and it rests on bands derived from forty-four runs "at load averages between 1.6 and
5.0" with nothing else on the box. This machine now carries **three rounds at once** by the
project's own working agreement: while this round measured, one-minute load averages ran from 3.5
to 35.7, with a neighbour walking a corpus and another linking.

The consequence, measured rather than argued (ADR 0902): over twelve consecutive runs on merged
`main`, six failed — five of them on `doc/pdf.js/test/pdfs/bug1815476.pdf`'s cold open — with **no
change to any code on the open path since the bands were derived** and with the fixed-work
calibration probe reading 0.703 to 0.749 ms against its band of 0.62 to 0.78. Every clock figure's
median sits within a few percent of its derived maximum, high for the two smallest documents and
*low* for the two largest, which is the signature of a per-operation cost rather than of a
regression.

So the gate is not wrong about the program. It is being asked a question about a machine that is
not the machine, and its guard — the quickest of fifty warm passes — cannot see that, because every
figure it guards is one first pass in a fresh process.

## The question

**Which of these does the project want, for the clock half of the launch gate?**

1. **Derive the busy-probe band and let the gate decline.** Round 931 added the first-pass
   measurement (ADR 0903) but could not band it, because banding it needs a quiet machine. Give one
   round the machine to itself for about ten minutes and the band exists; from then on a loaded run
   prints `NOT JUDGED` instead of failing. **Cost: this gate then says nothing at all on most
   rounds**, and a real regression waits for a quiet afternoon to be seen.
2. **Run the clock figures at a stated quiet moment rather than every round.** The eight figures
   with no clock in them stay on `doc/todo/02` §2 and are judged always; the clock ones move to
   `doc/verify.md` as a run a round makes when it has the machine — the same shape the quorra
   coverage lanes already have. **Cost: a launch regression is found later.**
3. **Neither: accept that a clock gate is a claim about a quiet machine and stop running three
   rounds while one measures.** That is the owner's call about how rounds are scheduled, not a
   change to any file here.

Round 931's own recommendation is **1 and then 2 together**: band the busy probe so that a loaded
run declines honestly, and keep a stated quiet run for the clock figures so that declining is not
the same as not measuring. The band derivation is about ten minutes of an otherwise idle machine
and is the only part that needs anything the project does not already have.

## Evidence session 934 added

Session 934 was sent to derive option 1's band if the machine was quiet. **It was not** — 151
samples of the one-minute load average, every thirty seconds for seventy-five minutes, read a
**minimum of 3.30, a median of 12.86 and a maximum of 61.55, with 62 % of them above 10**, and a
*fourth* round appeared while it ran. No band was derived, for the third round running.

What that round found makes recommendation 1 **necessary and not sufficient**, and the reason is
worth having before the answer is given. Its `doc/todo/02` §2 run failed the launch line on all four
`peak_mib` figures at once — a quarter below their minima, together — with the calibration probe at
**0.706 ms, inside its band**, on a machine left with ~9 GiB free by two neighbours' corpus walks.
Nine re-runs alone on the identical binary read 161 to 182 MiB, inside every band. A busy-*clock*
probe would have declined none of it, because the clock was fine: `peak_mib` is a **memory** figure
whose only guard is a clock, and what moves it is the graphics driver's allocation under memory
pressure. ADR 0909 and `doc/todo/42` have the figures.

So whichever of the three options is chosen, the clock half is not the whole of it: this gate's
memory figures need a memory probe, on the same idle ten minutes option 1 already asks for.

## Measured again in session 935, on the quietest machine this question has had

That round resolved the *memory* half of this instrument (`Q37`) and, in verifying it, ran the
launch line fifteen times alone. The clock half's state, at one-minute load averages between 1.3
and 5.4 with the calibration probe reading **0.704 to 0.769 ms** — inside `0.62 .. 0.78` on every
one of them, so every run judged:

| load average | runs | runs with every clock figure in band | the figures that failed |
|---|---|---|---|
| 1.3 – 2.0 | 3 | 2 | `bug1815476.pdf`'s cold open, 0.506 against a ceiling of 0.500 |
| 3.3 – 5.4 | 6 | 1 | `PDF20`'s warm open (0.762, 0.936, 0.832 against 0.570); `bug1815476.pdf`'s cold open (0.537, 0.530) and warm open (0.656); `WTPDF`'s cold open (3.063 against 3.060) |

**No memory figure was outside in any of the fifteen**, which is what the other half of the
instrument now looks like. And the failures are the shape option 1 predicts: a probe reading dead
centre while the figure it guards is half as fast again, because the probe is fifty warm passes and
the figure is one cold one. The round did not move a band and did not derive the first-pass one —
it had no idle ten minutes either, with two neighbours running throughout.

## What is *not* being asked

Whether to widen a band. Two rounds in a row have declined to, on the same argument, and this one
has the measurement behind it: a band widened to admit a loaded machine has put the loaded machine
into the claim.

## Answered, and what session 938 built on the answer

`A29` is the owner's: *"combining option 1 and 2 sounds good"*. Both are built, and the
measurement changed the shape of the first one — so what follows is what the gate does now and
what it found while getting there. ADRs 0916 and 0917; `doc/todo/42` carries the tables.

**Option 2, as asked.** `doc/todo/02` §2 runs this gate with no `PDFVIEWER_LAUNCH_CLOCKS` in the
environment and judges **twenty-one figures no machine can move** — the bytes an open reads, the
read calls it makes, the instructions it executes, what it costs in memory, what page one
allocates, what the device allocates — in **three seconds at a load average of 6**, green.
`doc/verify.md` says to set the variable and run the nine-sample clock figures when a round has the
machine. An environment variable rather than a flag, following `A28`.

**Option 1's purpose, delivered as a count.** The busy probe this question asked for needed ten
minutes of an idle machine that four rounds could not get. The kernel had been counting the same
thing exactly all along: `sched_info.run_delay`, per thread, in nanoseconds, at every wakeup. Every
duration is now elapsed time less that wait, and a figure whose thread waited more than a tenth of
its own elapsed time is declined. On a quiet machine the correction is zero, so **no band moved,
for the sixth round running**.

**Why the band this question asked for is still not derived, and it is a third reason.** Round 938
was given an idle machine and went to take it. On 2026-09-06 the fifty-pass probe read **0.849 to
0.951 ms over 300 consecutive idle samples** where the *same binary* read 0.703 to 0.724 in this
gate's own runs on 2026-09-04 — and a single busy thread pinned to a performance core reached
**3.74 GHz against a rated 5.16**, at 63 °C, on the `performance` platform profile. The machine was
not busy. It was **a different machine**, and a band taken that day would have put it into the
claim. The gate says so out loud: the clock run prints `NOT JUDGED`, names the probe, prints all
thirty-seven figures and exits 0.

**And the first-pass probe does not discriminate**, which is worth recording because this question
recommended banding it. Its ratio to the fifty-pass reading, per child, has a median of 1.64 on an
idle machine, 1.50 at load 9, 1.40 with eight spinners on the SMT siblings of the pinned cores and
3.23 with eight on the pinned cores themselves — the quiet machine sitting *in the middle* of the
contended ones. It stays printed and unbanded, now with a measurement saying why rather than a
promise that a quiet afternoon would fix it.

**What is left for a later round**, and it now has an instrument: `bug1815476.pdf`'s cold open has
been a per cent or two over its ceiling since session 933 on quiet machines with every probe in
band. Its two candidate explanations — the program's open grew, or this machine's small-read
latency grew — were not separable backwards. `open_kinstructions` makes the first exact from now
on, and the second already has a number beside it: the cold read session 931 measured at 0.109 ms
reads 0.125 to 0.199 today.
