# 2388 — The compute lane's first frame keeps the guess as its floor

Status: implemented. Session 1777, 2026-10-26, a "measure" round under the project owner's switch to
code improvements of 2026-10-14. Context: ADR 2355 (the first frame sized by its own count), raster's
ADR 0095 (the persistent capacity, its quarter of headroom, and the re-run of a frame that outgrows
it), ADR 1969 (how a clock band is re-taken), ADR 1537 (a turn is judged only on a quiet device),
traps 262 and 271. Code: `raster_gpu::compute::{ComputePersist::first_size, dispatch_chain}`. Tests:
`compute::persist_tests::the_first_frame_is_sized_by_its_count_above_the_floor_and_by_the_floor_below_it`,
and `compute::refusal_tests::the_first_frame_is_sized_by_its_own_count_and_runs_once` re-aimed at the
floor. It cites no clause and moves no ledger row.

## 1. What the gate said

`doc/checks/turn-path.toml` bands `issue19802.pdf` page 1's zoom step at 1.1 .. 1.8 ms (observed 1.33 ..
1.44 on 2026-10-05). Batch 116's merge read 2.92 ms and re-took it at 3.17 on the batch and on the
tree before it alike, and called it the machine's. It was two things, and one of them was the code.

## 2. The code: the step ran twice

The step's device is warmed by `doc/PDF20_AN001-BPC.pdf` page 1 on the compute lane, draws the page
at 1×, and is timed at 2×. Since ADR 2355 the lane's first frame — the warm-up — sizes the capacity
by its own count plus a quarter, where it used to take the guess of 32 edges a tile, at least 65 536.
The warm-up's count is far under the guess, so the page at 1× outgrew it and ran again (untimed), and
the step outgrew the 1× size and ran again whole: upload, passes and the end-wait twice. The step's
phase list shows one attempt (trap 271); the frame's wall shows two. A scratch probe drew the gate's
sequence and then the same 2× frame again on one device: on this tree the step's budget was 9.0 to
10.2 ms against 4.7 to 5.0 for the repeat, on six trees from 2026-10-07 to ecb6bea0 (0f9c12cf, 77fa741b,
020a86f3, e87f86ad, 2d37fcad, ecb6bea0, each built on its own) within 0.5 to 1.1 ms of it.

**The construction**: `ComputePersist::first_size` sizes the first frame by `max(count, floor)`, the
floor being the old guess clamped to what `max_frame_bytes` prices, so the floor alone never refuses.
A first frame lighter than the guess gets exactly the capacity it had before ADR 2355; a denser one
— ADR 2355's ISO 32000-2 page 20, 246 217 edges against a 99 200 guess — is sized by its count and
still runs once. A frame that scanned nothing keeps no capacity, as before. Capacity decides only
whether a frame re-runs; `tests/capacity_growth.rs` holds a re-run to the steady road's bytes.

## 3. The device: the 890M's clock

`pp_dpm_sclk` sampled every 2 ms beside each gate child says the Radeon runs this row at one of two
clocks. With nothing else on the device it stays at its lowest level, 600 MHz, through the whole
child, and the step reads 5.10 to 6.02 ms on ecb6bea0 and on this change, 9.07 to 9.59 on HEAD. While
another process — the owner's desktop session, which this account cannot list — keeps the device 30 to
49 % busy, the clock sits at 1 100 MHz or above and the step reads as below, but the gate declines to
judge any of it (ADR 1537's 20 %). Both are the device's state, not the tree's.

## 4. Measured

One `--clock --nice 0` hold, the gate's own child for this page (five rounds, the quickest), HEAD's
export, this change's and ecb6bea0's interleaved six times, pinned, one-minute load 2.70 to 4.63 (the
60 s cap expired on five arms), no process of this account holding the render node:

| arm | step at ≥ 1 100 MHz, ms | at 611–1 099 MHz |
|---|---|---|
| HEAD (004f8e73) | 2.74, 2.89, 2.96, 3.09, 3.09, 3.28 | — |
| this change | 1.71, 1.71, 1.72, 1.88, 1.89 | 2.56 |
| ecb6bea0 | 1.70, 1.70, 1.72, 1.72, 1.76 | 2.71 |

The turn and seventh rows moved in neither direction (6.45 to 9.22 ms on every arm). Every page's
frame digest is HEAD's on all six arms, against HEAD's six arms.

## 5. The band

Kept at 1.1 .. 1.8 ms: on the device's faster clock the change reads 1.71 at its quickest, inside it, as
ecb6bea0 did. At 600 MHz the row reads about 5 ms on every tree and is outside it; that is the device
state section 3 names, judged as the machine's by ADR 2242 until the gate reads the clock itself. The
cause is written above the band in `doc/checks/turn-path.toml`.

## Costs

The first frame's capacity is never below 1.25 MiB of edges (65 536 × 1.25 × 16 bytes), or the guess
for its tiles — what every first frame allocated before ADR 2355.
