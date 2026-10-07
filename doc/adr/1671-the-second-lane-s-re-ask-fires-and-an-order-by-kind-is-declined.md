# 1671 — The second lane's re-ask fires, and an order by kind is declined

Session 1417. Status: **accepted**; the lane is **owed**, not built. Answers ADR 1659 section 3's
re-ask on the first batch whose rounds all took the lock through the wrapper (batch sixty-four,
sessions 1408–1413). Supersedes nothing: ADR 1659's decision stood on batch sixty-three's log, and
this one is the next batch's log read against ADR 1659's own threshold.
Context: ADRs 1646, 1650, 1659, 1662; traps 110, 116, 131.
Instrument: `scratchpad/r1417/lock.py`, `sim.py` and `lanes.py` (scratch, not kept), over
`/home/AI/heavy-walk.log`'s 64 lines that say `batch=batch-1408-1413`.

## 1. What batch sixty-four's rounds queued, and behind whom

Five rounds wrote 24 lines (1413 took no walk); the merge wrote 40, queued 0 s and held 2 299.1 s.
Two holds are on no line, and both are recovered from the records: the open's arms export, a bare
`flock` until ADR 1662, held 1 659 s up to 17:52:24 (record 1413), and an `sccache` server held the
lock's descriptor from 1412's release at 19:18:39 to 1408's grant at 19:37:14, **1 115.3 s**
(trap 131). Holders are every hold whose interval overlaps a line's queue, not only `behind=`:

| round | lines | queued | held | queued behind |
|---|---|---|---|---|
| 1408 | 9 | 5 742.9 s | 892.1 s | 1409 2 038 s, 1410 1 521 s, sccache 1 115 s, export 842 s, 1408 107 s, 1412 68 s, 1411 52 s |
| 1411 | 10 | 5 098.1 s | 864.8 s | 1409 2 041 s, 1410 1 629 s, sccache 1 115 s, 1408 245 s, 1412 68 s |
| 1412 | 1 | 2 626.6 s | 67.5 s | 1410 1 594 s, 1409 1 033 s |
| 1409 | 2 | 2 091.0 s | 2 090.0 s | 1410 1 655 s, 1408 371 s, 1411 65 s |
| 1410 | 2 | 1 034.5 s | 1 794.0 s | 1409 604 s, 1408 378 s, 1411 52 s |

**16 593.1 s of queue**, against batch sixty-three's 7 410 s known. Three holds made most of it: the
drive's two (1 056.9 s, which a script edited while it ran ended, trap 130, and 1 033.1 s) and the
pixels round's own six arms with `turn_path` twice (1 655.4 s). 1408's 107 s behind itself is two of
its own requests in flight at once.

## 2. The replay, and what each ordering would have saved

`sim.py` replays the log as a single non-preemptive lock: each round's walks in its own order, each
asked the observed gap after the round's previous one ended, each held its observed time. **First
come first served reproduces every one of the 24 observed waits to within 1 s**, which is the
calibration (trap 13); every row below changes one thing on that replay.

| ordering | queue, five rounds |
|---|---|
| as run | 16 593.9 s |
| the `sccache` hold gone (ADR 1674's marker) | 14 363.3 s, −2 230.6 |
| and the open's export not on the lock | 11 520.6 s, −2 842.7 more |
| counts first, clock runs last (`turn_path` split off the arms) | 15 273.2 s, +909.9 |
| the same, the arms' `turn_path` left inside them | 14 676.8 s, +313.5 |
| a declared hold under 600 s first | 12 687.2 s, −1 676.1 |
| shortest hold first (a bound, not a rule) | 10 087.9 s, −4 275.4 |
| two lanes, a clock run taking both | 5 443.7 to 7 978.6 s, −8 919.6 to −6 384.7 |

**Counts first and clock runs last would have cost, not saved.** Batch sixty-four's clock runs were
its short holds — round 1411's A/B children, 1.1 to 232.4 s — and its counts its long ones; an order
by kind put the shortest work last, which is the opposite of what shortens a queue. **The export at
open costs the rounds about 2 840 s on one lane** by its knock-on, 842 s of it directly, and nothing
on two. Every row after the second is without the `sccache` hold, and only the third takes the
export off the lock.

## 3. ADR 1659's re-ask, on its own terms

ADR 1659 section 3: sum the queue of runs that peaked under 6 GiB, were not clock runs, and queued
behind a holder that was not one either; over 3 600 s a batch, a second lane is worth building.
Batch sixty-four's sum is **10 759 s**: 1408's four queued walks 5 503 s (two before `peak=` existed, taken
as under 6 GiB), the drive's 1 837 s, 1412's 2 438 s and the arms' 982 s. The largest pair of peaks
that could have run side by side is 5.30 + 4.57 GiB.

## 4. Decision

- **The re-ask fires, so the second lane is worth building**, as ADR 1659 section 3 describes it: a
  separate lock under `--tree 6`, a walk that peaks under 6 GiB and is not a clock run taking
  either, a clock run taking both. On the replay that is 6 385 to 8 920 s of batch sixty-four's
  14 363 s. It is owed to `tools/bounded.sh`; this batch's owner of that file is building ADR
  1674's marker, and the lane is the next instruments round's.
- **A clock run that waits must stop new grants on both lanes.** The replay assumed it; `flock`
  does not give it, and without it a clock run waits until two holds happen to end together.
- **No order by kind.** A rule that the short go first saves 1 676 to 4 275 s on one lane but needs a
  hold declared before the walk, and two lanes save more without it.
- **Not modelled**: two walks side by side stretch each other's holds, so section 2's two-lane
  figures are the best case; a lane's first batch reads its own lines against them.

**Re-ask**: after the second lane's first batch, the same replay over its log. If the two-lane queue
is not below one lane's replayed for the same lines by at least 3 600 s, the lane is taken out.
