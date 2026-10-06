# 1556 — A patch-mesh page's turn past its band is the processor's clock, at both ends of the load

Status: accepted. Session 1360. Answers why `personwithdog.pdf`'s turn read 11.43 to 11.5 ms in two
rounds' runs against its band's top of 10.80 while the merge's gate judged it inside. Leaves the
band where ADR 1513 put it and writes the reason beside it. Extends ADR 1519's finding from the
text row to this one, and adds the other end of the load. Builds on ADRs 0916, 1513, 1537; trap
101. Supersedes nothing.
Code: none. `doc/checks/turn-path.toml` (the row's `why`), `doc/performance.md` §3e (the rows).

## 1. Not the tree, and not the device

Every figure below is `turn_path`'s own child (`turn_probe`, five rounds, the quickest), pinned to
the eight fastest cores, on exports of HEAD and of `47ec7f10` — the tree the band was taken on — in
target directories of their own (`md5sum`-distinct, trap 50), interleaved child by child.

- **The tree.** At load 3.2 to 4.3, six children each: the banded tree 8.92–9.79 ms, HEAD
  8.96–10.36. At load 13 to 14: 9.80–10.46 against 9.56–11.26. Side by side the two read the same,
  so there is nothing to bisect. `frame_budget` on 2026-10-06 (load 2.0) read 8.81–8.98, inside the
  8.63–8.94 the band was derived from.
- **The device (ADR 1537).** A child that read 11.85 ms found the device 0% busy before it.

## 2. The clock, at both ends

- **A quiet machine.** At load 0.5, four children each, interleaved: after 1.5 s of idle 11.47–11.74
  ms, after a 30 ms spin on the pinned cores 8.85–9.28, back to back 9.09–9.79. The idle children's
  calibration probe read 0.89–0.95 ms, past the band's 0.78, but one read 0.675 with its turn at
  11.63: the probe does not always see it. ADR 1519's mechanism, on a page whose mesh rows are
  divided across the pool (ADR 1259) and whose child is short enough to live its whole run at the
  clock it started at. Three runs of the gate on this machine at load 0.3 to 1.1 found the same:
  this row, the text page and `issue19802.pdf` — the three lightest children — not judged on their
  calibration (0.94), every other row judged and inside.
- **A busy one.** At load 5 the idle and the spin moved nothing (9.60–10.21, 9.77–10.48). Sixteen
  threads spinning on the sixteen logical processors the child is *not* pinned to took the turn to
  13.33–13.93 ms on both trees and the probe to 1.01, while the one-minute load average the
  ceiling reads stood at 3.2 to 4.3: the cores share one package's clock and power, so a
  neighbour's build slows the pinned cores without sharing one with them.

So the band holds the middle state, which is where the merge's gate found it: a machine neither
idle nor saturated.

## 3. Decided

The band stays at 7.3 .. 10.8 ms. A figure past its top is the clock at one end or the other — a
slowdown nobody chose, which `turn-path.toml`'s own rule says is not a reason to move a band — and
the row's `why` says so. The gate is not changed: its calibration already declines most such
children, and making the instrument warm the processor before it measures is work done to move a
clock (ADR 1519), which is a question for the gate's method and not for this row.
