# Q777 — May two more supervised runs on the RX 580 separate the hang mode's serialising from concurrency?

Source: round 1919 (single round), ADR 2668, after A776.

## What A776's run found

The clip test (`clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it`) ran once on
the RX 580, `--exact`, with `RADV_DEBUG=hang`, under the shared lock held from before the test until a
device probe succeeded afterwards (`tools/hang-capture.sh`). It **passed**: the lock was held for 1.0 s
(23:56:55.921 to 23:56:56.946 UTC on 2026-11-03), the kernel logged nothing, and RADV wrote no dump,
because it writes one only on a hang. The hang of 14:02:33 is therefore still unexplained.

The pass cannot say why, because two things differed from 14:02:33: the test ran **alone** rather than
in a parallel suite, and RADV's hang mode **serialises** (it turns on `syncshaders`, which waits out
every draw, dispatch and submission). Each run below changes only one of them.

## The question

1. **Allowed, each once, by the same script, in this order:**
   - **a.** the same test alone, **without** `RADV_DEBUG`. A hang here lives in the one process's own
     work, and the serialising mode hid it.
   - **b.** if (a) passes: the shape of 14:02 — `clip_that_cuts_nothing`, `retained_frame` and
     `sub_pixel_coverage` in one parallel `nextest` on RADV, inside the script's lock, every process's
     own section lock pointed at one shared private file so that their device sections take turns as
     they did at 14:02. A hang here makes concurrency among our own processes the trigger; ADR 2665's
     hang marker stops the rest of that run after the first reset, so it is at most one reset, and the
     lock is held until the card is back, as in A776.
2. **Only (a).**
3. **Neither**: the guards of ADR 2665 stay as the answer, the three tests stay withheld on this card,
   and the upstream report stays unfiled for want of a dump.

The recommendation is 1: (a) costs about a second of the card if it passes and one reset inside a held
lock if it does not, and (b) is the only run that reproduces the condition every reset of 2026-11-03
shared.

## What is built, and what is not

Built: `tools/hang-capture.sh`, which holds `/run/lock/gpu.lock` from before the test until the
kernel's reset has completed and `vulkaninfo --summary` succeeds on RADV. Not built: the mode for (b),
running a command rather than one test binary, and a way to run (a) without `RADV_DEBUG=hang` — both
small and made only if allowed.
