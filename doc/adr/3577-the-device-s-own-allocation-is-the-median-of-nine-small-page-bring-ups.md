# 3577 — The device's own allocation is the median of nine small-page bring-ups

Session 2219. Status: accepted. Context: ADRs 0910, 1902, 2244, 2667, 2730; `doc/checks/launch-path.toml`;
`CLAUDE.md` principle 2 ("never move a band to fit").

## The question

Round 2215 left `xfa_filled_imm1344e.pdf`'s `peak_anon_mib` at 3.469 MiB against its band of 1.0 to
3.0, and 3.082 on the commit before. The figure is the first page's anonymous high-water in a child
with its huge pages off, less the device's own allocation (ADR 2667). The brief asked which commit
raised it and what allocation is new.

## What was measured

**No commit raised it, and no allocation is new.** Every kept gate log that prints the row — 45
runs, rounds 1954 to 2218, twelve of them the post-merge train's from fcd1d05b to 294976f4 — splits
the figure into its two terms, which each run prints on the row's line "allocated by the same first
page":

| term | 45 runs |
|---|---|
| the first page, small pages | 15.0 to 15.6 MiB, 15.2 in most |
| the device's own (one bring-up child) | 11.8 to 13.7 MiB |
| the difference, judged | 1.5 to 3.5 MiB |

Round 2215's 3.469 is 15.3 less 11.8, the lowest device reading of the 45; its 3.082 on the commit
before is 15.2 less 12.1. Ten pairs of children taken directly on this tree, 2026-11-24, on the RX
580: the first page 15.25 to 15.54 MiB, a bring-up child with small pages 12.32 to 13.17, one with
the machine's huge pages 12.07 to 13.10. The program's own term holds within 0.3 MiB; the term
subtracted from it moves by 0.9 between two identical processes and by 1.9 over the record. A band
two mebibytes wide on the difference was being judged against the driver's draw, so a heap profile
of the open would have found nothing to find, and none was taken.

Two defects in how the subtraction was taken, both in `crates/viewer-ui/tests/launch_path.rs`:

1. **One child.** The device's figure was the fields of whichever bring-up child was quickest — one
   child without clocks, the fastest of nine with them — so a memory figure was chosen by a clock.
2. **Counted another way.** The bring-up phase ran with the machine's transparent huge pages
   (`always` on this VM), and the first page it was subtracted from without them (ADR 1902): the two
   terms were not counted alike, which ADR 2667's own comparison happened not to show at 8.9 MiB.

## Decision

The device's own allocation is the **median of nine bring-up children with their huge pages off**
(`DEVICE_SAMPLES`, `device_allocation`), with or without clocks, and the run prints it with its
least and most. The bring-up clock and its printed allocation are unchanged; only the figure every
row is judged beyond is taken this way. The median of nine moves by about a third of one draw's
spread; it does not change what is subtracted, so no row's band moves.

## Cost

Nine bring-up children a run, about two seconds of device time, each inside its own lock hold
(ADR 2664). A real growth in the device's allocation is still invisible here — ADR 2667's choice,
whose band belongs with the driver's figures.

## Figures after

`tools/state.sh --round 2219 launch`: see `doc/history/2219-*.md`.
