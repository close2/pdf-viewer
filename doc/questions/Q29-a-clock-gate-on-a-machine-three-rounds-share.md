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

## What is *not* being asked

Whether to widen a band. Two rounds in a row have declined to, on the same argument, and this one
has the measurement behind it: a band widened to admit a loaded machine has put the loaded machine
into the claim.
