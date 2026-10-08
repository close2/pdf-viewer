# 1706 — The second lane stays, and two walks side by side do not cost what it saves

Session 1435. Status: **accepted**. Answers the re-ask ADR 1671 section 4 set and ADR 1684 section 3
restated, on the lanes' first batch (batch sixty-seven, sessions 1426–1431). Decides nothing new in
`tools/bounded.sh`: the lane stays as ADR 1684 built it. Supersedes nothing.
Context: ADRs 1659, 1671, 1674, 1684; traps 13, 131.
Instrument: `scratchpad/r1435/` (`log67.py`, `replay.py`, `calibrate64.py`, `stretch.py`,
`breakeven.py`, `whatif.py`), kept at `/home/AI/lock-replay-1435/` beside 1417's. Input: the 128
lines of `/home/AI/heavy-walk.log` that say `batch=batch-1426-1431` — 92 rounds' lines, the open's
arms export (`round=arms`, 2 252.9 s on lane 1) and 35 of the merge's, which ran alone and queued 0 s.

## 1. The replay, and its calibration

The replay keeps every line's observed hold and its declared `kind=`. Each round's walk is asked the
observed gap after the round's most recent observed event before it — an ask, a grant or a release
of one of its own walks — so a round that fired two walks at once still does. 1417's `sim.py`
chained each walk to the one before it in ask order; batch sixty-seven's pixels and fuzzing rounds
held two of their own at once, which that chain cannot say. Under one lane every walk takes the one
lock in ask order; under the built rule a small walk takes lane 2 then 1, a large one lane 1, and a
clock run both, a waiting clock run stopping every grant.

**The one-lane arm is calibrated** (trap 13) on batch sixty-four, which ran on one lane: its own
lines at their observed asks, with ADR 1671 section 1's two holds that are on no line put back,
give **25 of 25 waits within 1 s** (worst 0.4 s), 16 594.2 s against 16 593.1 s observed.

**The two-lane arm reproduces the total and not every line.** At the observed asks it gives
12 622.7 s against the observed 13 505.5 s (−6.5%), 37 of 93 lines within 1 s. The misses are the
pollers' race, which ADR 1684 section 3 said it would not order: a walk polls twice a second, so a
wrapper started inside that half-second takes a lane from one queued for minutes. 1429's
back-to-back `seeds.sh check` lines at 03:07:49 and 03:07:51 waited 0.0 s while 1426's and 1428's
had queued 640.8 s and 319.3 s. The log's ask times are whole seconds, so the race cannot be
replayed. With the chains the built rule gives 14 229.2 s (+5.4%).

## 2. The re-ask, on its own terms

| rounds' queue, batch sixty-seven | one lane | built rule | difference |
|---|---|---|---|
| as run (observed, two lanes) | — | 13 505.5 s | — |
| chained on ask, grant and release | 37 801.9 s | 14 229.2 s | **23 572.7 s** |
| chained on ask and release | 47 500.2 s | 14 448.0 s | 33 052.2 s |

**The difference is 23 573 to 33 052 s, against ADR 1684's 3 600 s.** The lane stays.

## 3. The blind spot: two walks side by side

The replay gives a walk the hold it had beside a neighbour. On one lane it would have run alone.
74% of the rounds' 17 173 s of hold was beside another hold. The log answers how much that
stretched them only for walks an earlier, single-lane batch also ran:

- **The drive**: 1427's 1 180.9 s for 206 works, beside 1429's lane-2 walks throughout, is 5.73 s a
  work. Single-lane: 1409's 1 033.1 s for 190 works, 1415's 902.7 s for 199, 1421's 964.4 s for
  203, which is 4.54 to 5.44 s. **1.05 to 1.26×.**
- `pdf-script --features engine --test script_corpus`: 148.1 s beside 1429's `seeds.sh check`,
  against 1420's 147.0 s alone, **1.01×**. `pdf-model --test script_corpus`: 48.6 s beside, against
  47.5 to 49.7 s, **0.98 to 1.02×**.
- **The control**: the batch sixty-seven merge's 33 gates over 5 s, alone, against the same gates
  in the three single-lane merges before it: **median 1.000, 0.83 to 1.20×**. So the drive's
  stretch is inside the drift a gate shows with no neighbour at all.
- **Not answerable from the log**: `pdf-model --test corpus` (16.6 s against 6.6 to 7.7) and
  `--test dates` (15.5 s against 4.4 to 5.6). Each was its round's first run after an edit, so the
  hold includes a build the line does not separate.

**What would take the lane out.** Shrink each one-lane hold's overlapped share by a factor s, and
keep the twelve time-boxed fuzz campaigns (7 306.8 s) at their length. The difference is 16 720 s at
s = 1.2, 8 496 s at 2.0 and 475 s at 3.0. With the campaigns taken off the lock in both arms, as a
round on one lane might have done, it is 12 850 s at 1.2 and 1 964 s at 2.0. **The lane comes out
only if a neighbour slows a walk 1.5 to 3×, and nothing the log can see shows more than 1.26×.**

## 4. What the queue was, under the lane

The rounds queued 13 505.5 s. Round 1428 queued 10 213.0 s of it, and two of its waits were
declarations rather than rules:

- A `cargo build --release` left at the default `--tree 12` waited **2 239.5 s** for lane 1 behind
  the arms export, then held 1.1 s at 0.02 GiB. With it and `tools/batch.sh raster-examples`
  (2.76 GiB) declared small, the built replay is 12 294.4 s, **1 934.8 s less**. ADR 1684 section 3
  already says what an undeclared walk costs.
- `cargo test -p conformance` under the lock waited 1 820.8 s and 116.1 s. The rule line says it is
  not a walk.

## 5. Decision

- **The second lane stays**, as ADR 1684 built it. On its first batch it is worth 23 573 to
  33 052 s of the rounds' queue against one lane, and that is above ADR 1671's 3 600 s by more
  than a slowed neighbour can take back.
- **No change to `tools/bounded.sh`**, so nothing is handed to the instruments round.
- **The re-ask is answered and is not repeated each batch.** It is reopened when a batch's own
  lines replay with less than 3 600 s between the two rules: `python3 replay.py <batch>` in the
  kept directory prints both in a second. It is also reopened when a walk run beside a neighbour
  and run alone in one sitting shows more than 1.5×, which an A/B can measure and the log cannot.
