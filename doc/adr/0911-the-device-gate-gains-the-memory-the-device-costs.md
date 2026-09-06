# 0911 — The device gate gains the memory the device costs

Session 935. Status: **accepted**. The second of this round's two records; the first,
[ADR 0910](0910-nine-tenths-of-a-memory-high-water-belonged-to-somebody-elses-libraries.md), is
the measurement this one follows from.

## Context

`CLAUDE.md` principle 2 makes cold graphics bring-up a gate of its own, and says why:

> **Cold bring-up is its own gate**, separate from time-to-first-page, so that a regression in the
> driver, the adapter selection or the shader set is legible as itself rather than as a slower
> page.

Since [ADR 0884](0884-the-launch-path-is-a-gate-and-a-wall-clock-band-needs-a-machine-under-it.md)
that gate has been one duration and one witness: `bring_up_ms`, and the adapter's own name printed
beside it so that a machine which quietly fell back to a software rasteriser reports a different
measurement rather than the same number.

ADR 0910 measured what a device costs in memory and found that the sentence above was being asked
of a figure that could not answer it. Two thirds of what a first page's process allocates is the
device's — 11 MiB of 27 on the five-page document — and until this round that share was inside
each *document* row, where a driver's change reads as four documents changing at once and a
document's change reads as one row moving by a third of what the driver contributes.

## Decision

**`bring_up_anon_mib` joins `bring_up_ms` in the check file**: what the graphics device allocates,
measured in the process that has done nothing else, banded `9.5 .. 12.5` from eleven consecutive
samples spanning 10.89 to 11.05 MiB, and judged on any machine like the other memory figures.

**The bring-up line prints the whole-process figure and the split.** It now reads

```text
launch-path: cold graphics bring-up 33.6 ms on AMD_Radeon_890M_… , 108 MiB resident
             of which 10.9 MiB is allocated and the rest is mapped libraries
```

which is the one place in this tree where the two quantities ADR 0910 separated appear side by
side. A reader who wants to know what a launch costs a machine reads the first; a reader asking
whether *we* regressed reads the second.

## Why this rather than the alternatives

Three resolutions were priced for the failing figure. The others are recorded because the argument
against them is what makes this one worth keeping.

- **Widen the band.** Declined by sessions 926, 931, 933 and 934 and declined again: a band widened to
  admit whatever the machine is doing has put the machine into the claim. It is also useless here —
  the whole-process figure was seen at 92 MiB and at 180 MiB on one afternoon, and a band spanning
  that is not a guard against anything.
- **Derive the band per driver version and fail only on an increase.** It fails on the measurement:
  the driver version did not change between session 922's forty-four identical runs and the fall an
  hour later, and Mesa has been at 26.2.1 since six days before the first of these recurrences. What
  moves the figure is the page cache, which has no version.
- **Keep the figure and take its floor off, leaving a one-sided ceiling.** This is round 932's
  `Q32`, asked from the same red lines on a parallel branch, and it is the closest of the
  alternatives: the floor is indeed a claim nothing in this program controls. It stops one step
  short, because the ceiling is not a claim about this program either — the same binary read 92 MiB
  and 180 MiB in one afternoon, so a ceiling at 231 admits an eighty-mebibyte leak on a cold page
  cache and fires on a warm one. `Q37` records what happens to that question and to the floors
  round 932 lowered.
- **Say the figure is not gateable on this machine.** This is what would have gone to the owner as
  a question, and the measurement makes it unnecessary: the figure is not gateable *as a whole
  process*, and the part of it this program owns is gateable more tightly than the whole ever was.

## Consequences

- **A driver change is legible as itself.** A Mesa release that allocates two mebibytes more now
  fails one line naming the graphics device, rather than four document rows at once.
- **The device's share is subtracted by reading, not by arithmetic.** `bring_up_anon_mib` and a
  row's `peak_anon_mib` are two separate processes' figures and the gate does not subtract one from
  the other: a difference between two processes measured minutes apart is a worse number than
  either. What the pair gives a reader is the proportion — about eleven mebibytes of a first page's
  twenty-seven on the smallest document, and of forty-four on the one that substitutes a font.
- **What is still nobody's figure is the launch's whole cost on a user's machine.** That is
  `doc/todo/42`'s item 5 and `Q29`'s neighbour: this gate is headless and its bands are this
  machine's. The whole-process number is printed on every run so that the question stays visible.
