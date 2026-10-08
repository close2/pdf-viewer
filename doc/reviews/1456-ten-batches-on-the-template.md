# 1456 — Ten batches on the template: review 1401's figures re-taken

Status: **review**, a record that is never rewritten. Written in slot 1 of batch seventy-two. It
answers rule 10 of `doc/reviews/1401-are-we-making-progress-and-what-a-round-reads.md` section 4:
"Re-take this review's figures in ten batches with the commands named above". That review's scratch
scripts went with its worktree, so each one is rebuilt here from its section 1's description. They
are committed beside this file under `doc/reviews/1456/` and named at every table, so the next
re-take runs them instead of rebuilding them (ADR 1748 section 2.2). The decision is ADR 1748.

Every figure was taken on 2026-10-08 against `8b8f86f0`. The period is batches sixty-three to
seventy-one (`151b7527..8b8f86f0`, 9 batch commits, 54 round records, 1402–1455). The comparison
is batches fifty-three to sixty-two (`5f1bdc55..0f9c12cf`, 10 batch commits, 62 rounds, 1340–1401),
the ten that review 1401 measured, re-taken by the same scripts so that both columns are counted
the same way. Every command runs from the worktree root with `PYTHONDONTWRITEBYTECODE=1`.

**One instrument is wrong, and it was found by calibrating it.** `tools/state.sh batches` prints
"2135 s over 6 figure(s)" for `8b8f86f0`, whose six rounds took 47 135 s. From batch sixty-four on,
the bodies write a figure with a digit group ("7 497 s"). The section's `[0-9]+ s\b` reads only
the last group, and its gate-line pattern finds no "1 755 s of gate wall time". So every batch
from sixty-four is under-summed and has no gate line. `slots.py` joins the groups, and its sum for
`8b8f86f0` matches the body read by hand. The fix belongs to whoever owns `tools/state.sh`; until it
lands, a reader reads `slots.py` (ADR 1748 section 4).

## 1. Measured

### 1.1 The batch clock

`python3 doc/reviews/1456/slots.py 151b7527 8b8f86f0`, and `… 5f1bdc55 0f9c12cf` for the column
before. It reads each batch body's `Round durations` and gate wall-time lines. Busy share is the sum
of the round walls over six times the slowest round, as review 1401 took it. Tail is the gap since
the last batch commit, minus the slowest round.

| batch | gap | rounds | sum of walls | slowest | mean round | busy | gate wall | tail |
|---|---|---|---|---|---|---|---|---|
| 63 `151b7527` | 3.09 h | 6 | 10.03 h | 2.36 h | 100 min | 71% | 1 643 s | 0.73 h |
| 64 `b9fa776a` | 4.18 h | 6 | 10.65 h | 2.49 h | 106 min | 71% | 1 876 s | 1.69 h |
| 65 `c4818cf1` | 2.33 h | 6 | 6.73 h | 1.58 h | 67 min | 71% | 1 759 s | 0.75 h |
| 66 `fe5d66eb` | 2.62 h | 6 | 6.55 h | 1.84 h | 65 min | 59% | 1 870 s | 0.79 h |
| 67 `e833db88` | 4.18 h | 6 | 10.21 h | 3.40 h | 102 min | 50% | 1 757 s | 0.79 h |
| 68 `ebe5f2e2` | 2.87 h | 6 | 6.21 h | 1.59 h | 62 min | 65% | 1 792 s | 1.28 h |
| 69 `df6cc030` | 2.20 h | 6 | 6.91 h | 1.42 h | 69 min | 81% | 1 783 s | 0.78 h |
| 70 `f0ab0216` | 2.08 h | 6 | 5.07 h | 1.34 h | 51 min | 63% | 1 855 s | 0.75 h |
| 71 `8b8f86f0` | 3.77 h | 6 | 13.09 h | 3.04 h | 131 min | 72% | 1 755 s | 0.73 h |

| | batches 53–62 | batches 63–71 |
|---|---|---|
| batch gap, mean (median) | 5.98 h (4.70 h) | **3.04 h (2.87 h)** |
| slowest round, mean | 3.89 h | **2.12 h** |
| rounds over 3 h / over 4 h | 18 / 10 of 62 | **3 / 0 of 54** |
| mean round (shortest, longest) | 132 min (20 min, 5.66 h) | 84 min (16 min, 3.40 h) |
| six slots busy | 43–75%, mean 60% | 50–81%, mean 67% |
| merge tail, median | 0.73 h | 0.78 h |
| gate wall | 1 304–1 789 s | 1 643–1 876 s |
| tool uses per round (mean); seconds per tool use | 29–353 (169); 47 s | 52–277 (156); 32 s |
| mean minutes by slot position 1–6 | 123, 169, 198, 207, 60, 59 | 78, 85, 99, 97, 72, 72 |

- **The batch is half as long, and the wall budget is why.** No round of the 54 passed its four
  hours. The longest build slot fell from 207 to 97 min. A round takes about as many tool uses as
  before, 156 against 169, and each takes 32 s against 47 s.
- **The slots are still a third idle**: 67% busy against 60%. The batch still waits for its
  slowest round. That round is now bounded by its budget and by the lock's queue (section 1.1.1),
  not by how big its build is.
- **The merge's tail did not shrink**: a median of 0.78 h against 0.73 h, as rule 8 meant. Gate
  wall grew to 1 755–1 876 s. Batch sixty-nine took two redundant build lines out of the list, so
  35 gates became 33. Three merges caught a red gate (64 `t3-accessibility`, 68 `t3-foreign_corpus`,
  69 one), and every one is in the batch body.

#### 1.1.1 The lock's queue, which review 1401 could not measure

Review 1401 said "between 0 and about an hour per round, unknown", and ADR 1638 asked for the
instrument. It exists: `tools/bounded.sh --lock` writes one line per hold to
`/home/AI/heavy-walk.log` (ADRs 1646, 1674, 1684), from round 1405 on. This table comes from
`python3 doc/reviews/1456/locks.py`. The queued wall is the union of each round's wait intervals,
starting at the logged ask, because a round may queue a background walk beside a foreground one
and a plain sum would count that time twice. It is an upper bound on the time a round was blocked.

| batch | rounds' holds | queued round wall | share of the batch's round wall | longest single wait |
|---|---|---|---|---|
| 64 | 24 | 16 481 s | 43% | 3 929 s (1411, behind round 1410's arms export) |
| 65 | 18 | 10 079 s | 42% | 1 430 s (1416, behind the batch's arms) |
| 66 | 14 | 5 912 s | 25% | 1 411 s (1422, behind the batch's arms) |
| 67 | 92 | 10 700 s | 29% | 2 240 s (1428, behind the batch's arms) |
| 68 | 44 | 2 332 s | 10% | 428 s |
| 69 | 23 | 4 801 s | 19% | 1 134 s (1441, behind a campaign and the arms) |
| 70 | 32 | 1 470 s | 8% | 865 s |
| 71 | 31 | 24 463 s | **52%** | 4 420 s (1453, behind round 1455's campaign holds) |

- **Over batches 64–71, 76 238 s of 235 512 s of round wall, 32%, was a round with a walk
  queued.** It is the largest single term this review measured. It moves with the lanes: two lanes
  came in batch sixty-six (ADR 1684, kept by ADR 1706 and round 1435), and the share fell to 8–29%.
  Batch seventy-one's two long campaign holds took both lanes, and the share rose to 52%. Its
  slowest round, 1453, had a walk queued for 8 318 s of its 10 938 s. Slot 5 of this batch builds
  the `--long` kind for exactly this case.
- **The batch's arms export is queued behind once per batch.** It takes lane 1 for 1 694–2 474 s
  at `open`, while the rounds start. Three batches' longest round wait was behind it. Moving it
  ahead of the launch would delay all six rounds instead of the ones whose first walk comes early,
  so this review proposes no change.
- **The declined second lock was re-asked on a count, as ADR 1638 section 3 said it would be**,
  and the count carried it: ADR 1684 for the lanes, and round 1435's replay for keeping them.

### 1.2 What the batches produced

`python3 doc/reviews/1456/batches.py 151b7527 8b8f86f0` (and `5f1bdc55 0f9c12cf`). It reads ledger
statuses through `tomllib` at each batch commit and compares each row with the previous batch
commit: a status move, or a note/code/test-only edit. It counts ADRs and records added with
`--diff-filter=A`, and bins `--numstat` as code (`.rs`, `.sh`, `.py`, `.toml` outside `doc/`) or
doc (under `doc/`, or `.md`). The bins are stated here, and the 53–62 column is re-binned the same
way, so it differs from review 1401's table by a few hundred lines a batch.

| | batches 53–62 | batches 63–71 |
|---|---|---|
| ledger status moves | 45 (39 in the two re-reading batches 54 and 61) | **4, all in batch 63** (§12.5.6.2, §12.7.8, §12.7.8.3, §12.7.8.3.2 `partial` → `implemented`) |
| note-only row edits | 685 | **29** |
| `implemented` / `partial` / `departed` at the end | 707 / 23 / 20 | 711 / 19 / 20 |
| ADRs (lines), per round | 108 (7 290), 1.74 | 84 (5 479), 1.56 |
| records, mean lines | 62, 38.1 | 54, 38.2 (33 of them at 39 or 40) |
| code added / removed | +60 339 / −4 202 | +36 814 / −4 036 |
| code per batch; per hour of batch gap | 6 034; 1 008 lines/h | 4 090; **1 347 lines/h** |
| doc added / removed | +16 958 / −10 982 | +9 330 / −1 244 |

- **The ledger has stopped moving, and the reason is written down.** Every `partial` leaf waits on
  an owner's answer or a purchase: Q308, Q271, Q348, A66's trigger, and a policy syntax no
  signature names. The briefs of batches seventy-one and seventy-two say so, batch sixty-eight's
  said so for every row but §12.7.4.3's language systems, and batch seventy-one's body says so
  too. Note-only churn fell from 685 to 29 row edits. So the review 1401 judgement, "the ledger is
  the wrong instrument for this period", is stronger now. The ledger is blocked rather than
  slow, and the next ledger move is the owner's.
- **The instruments that moved**: the drive counted 199 verdicts at batch sixty-five and 239 at
  seventy-one, with 0 wrong in every batch (batch bodies). The script wire went from version 6 to
  version 11. Code per hour of batch rose by a third, and code per batch fell, because a batch
  lasts half as long.
- **Paperwork is where it was.** There are 1.56 ADRs a round, and records average 38.2 lines
  against the 40 cap, with 33 of 54 at 39 or 40. The cap is still the target, as review 1401
  section 2.2 said, and none of the ten rules was aimed at it. `doc/HANDOVER.md` was edited in 2 of
  9 batches (9 of 9 before), `doc/state-of-play.md` and `doc/todo/65` in 9 of 9, and
  `doc/traps/README.md` in 7 of 9. The diff over `0f9c12cf..8b8f86f0` for each file gives those
  counts.

### 1.3 What a round reads before it writes

`bash doc/reviews/1456/reading.sh`. It sums `tools/round.sh --lines [kind]`, which prints no total,
for every kind. It also counts each brief from `/home/AI/batch-1402-brief.md` to
`batch-1456-brief.md`: its common part, which is everything before the first `## Slot`, its longest
slot part, and the non-blank lines it shares with the brief before it.

| kind | review 1401's "after" | now |
|---|---|---|
| every round alone | 567 | 574 (`CLAUDE.md` 439 of it) |
| pixels | 1 280 | 1 308 |
| oracle | 2 856 | 2 878 |
| parsers | 2 575 | 2 608 |
| loop | 1 974 | 2 065 |
| instruments | 2 239 | 2 435 |
| clause | 1 832 | 1 874 |
| measure | 4 010 | 4 235 |
| host | 2 174 | 2 282 |
| dependency | 1 816 | 1 830 |
| docs | 1 097 | 1 121 |
| script, writer, archive, fuzz | 3 884, 1 273, 5 347, 3 319 | 4 087, 1 278, 5 361, 3 469 |

- **The lists held.** Every kind grew by 0.3–9%, as its files grew. None gained an entry. The
  every-round five grew by 7 lines.
- **The briefs held to the template**: whole briefs of 160–182 lines (240 before), common parts of
  36–42 lines (cap 60), and slot parts of 22–25 lines (cap 25). 24–29 non-blank lines repeat from
  one brief to the next (149 of 240 before), and they are the common part's standing pointers. So a
  round reads about 574 + 41 + 25 lines, plus its kind's list, before it writes.
- **What the trap index is used for**: `tools/state.sh traps` counts 771 citations (739 at review
  1401), and the same ten traps carry 77% of them. The 54 records cite a trap 35 times in 24
  records: trap 1 12 times, 13 eight times, 130 four, 10 three, 109 two, and 9, 110, 122, 127, 132
  and 134 once each. So the ten carry 24 of the period's 35 citations. Trap 130, an in-place edit
  of a script another process runs, is cited only by instruments rounds, and it belongs to their
  group file rather than to the ten. The index grew from 123 to 134 rows (`grep -cE '^\| [0-9]+ \|'`
  at `0f9c12cf` and `8b8f86f0`). Habits are cited by number in no record.

### 1.4 Rework, premises, siblings, duplicated walks

`python3 doc/reviews/1456/premises.py 1402 1455 -v` prints each record's `**Premise**` paragraph
with a first classing by phrase, which was then read paragraph by paragraph. The hand reading is
the script's `BY_HAND` table, and the script prints the seven records where the phrases disagree
with it (1402, 1407, 1409, 1412, 1413, 1422, 1430).
`python3 doc/reviews/1456/siblings.py 1402 1455 -v` prints the sentences behind the sibling,
HEAD-export and section 3e counts.

- **Premises: 47 of 54 records state one (10 of 49 before), and 29 of the 47 say part of it did
  not hold (4 of 10 before).** The rest are 12 that held and 6 that held but needed more than
  stated. Rule 3 made the premise visible. It did not make it right, and there is no trend across
  the batches: 2 of 5, 3 of 6, 5 of 6, 3 of 6, 4 of 6, 4 of 5, 3 of 4, 2 of 4 and 3 of 5 missed,
  batch sixty-three to seventy-one. Read one by one, the 29 fall into four classes:
  - **a figure (5)**: 1407, 1419, 1431, 1437, 1446. A grep's count or a census's population was
    wrong. A command the orchestrator runs finds these.
  - **a place (3)**: 1412, 1418, 1443. A file or a binary was named where it is not. A `grep -rn`
    finds these.
  - **a mechanism (7)**: 1405, 1416, 1417, 1422, 1428, 1434, 1453. The brief assumed where the time
    went or what a digest followed. The round found it by profiling, and no orchestrator's grep
    could have. These are hypotheses written as premises.
  - **what a feature reaches (14)**: 1408, 1411, 1414, 1423, 1424, 1426, 1429, 1432, 1433, 1438,
    1439, 1444, 1450, 1452. For example, the window's history (1450), `popupOpen`'s owner (1432),
    and Table 197 events the column never walks (1426). Most were found only by building the
    feature's next step.

  The six "held but more than stated" (1403, 1409, 1415, 1421, 1427 and 1410) are mostly host
  rounds that needed a hunk in a model file owned by a sibling slot, `popup/rich.rs` or
  `rich_text.rs`. The pairing of host and model slots has made that the shape of the work.
- **Siblings: 51 of 54 records mention one, and 17 name a gate that failed on a sibling's
  in-flight edit (4 of 49 before).** From batch sixty-four the six rounds share one worktree, so a
  sibling's half-written file is in every round's build. The cost is a re-run. No record names a
  defect that came from it.
- **Duplicated walks: none.** One HEAD arms export per batch is under `/home/AI/arms-<session>/`
  for every batch from 64 on. No round exported HEAD as its "before". Three rounds (1422, 1428,
  1453) exported HEAD with their own patch as their "after", because the shared worktree carries
  siblings' edits. That is a walk the shared tree requires, not a repeated one. Two rounds (1410,
  1417) exported HEAD for a callgrind A/B. Batch sixty-three's own arms were exported twice, since
  the first export held no per-page digest (record 1404).
- **Re-measuring: one record (1404) re-took `doc/performance.md` section 3e rows, against five in
  the nine batches before.**

## 2. Judged

1. **The clock is the gain.** The batch is half as long (5.98 → 3.04 h mean) and no round passed
   its budget. That is rule 4 and nothing else: the slowest round was the batch's length before,
   and the budget bounds it now.
2. **The lock's queue is the cost that is left**, at 32% of round wall over eight batches and 52%
   in the last one. It is measurable now and assigned: slot 5 of this batch builds the `--long`
   kind. A review that did not have the log would have called this "slots a third idle" and gone
   no further.
3. **A premise is now written down and is still wrong 29 times in 47.** Eight of the 29, the
   figures and the places, are what an orchestrator's command finds before the brief. The
   mechanisms, 7 of 29, are hypotheses, and habit 59 already says a lever is profiled before it is
   built. Briefing them as premises makes a round first argue with its brief. ADR 1748 section 2.1
   splits the slot's line in two.
4. **The ledger cannot move without the owner.** Every `partial` leaf waits on a question or a
   purchase. The rounds have turned to the instruments and the JavaScript stream, which moved every
   batch.
5. **Paperwork was not addressed, and it shows**: records at the cap, and 1.56 ADRs a round.
   Review 1401 declined records as a table, and that still stands.

## 3. The ten rules of review 1401 section 4, each by its figure

| rule | figure, then and now | verdict |
|---|---|---|
| 1. Brief from the template | 240-line brief → 160–182; common 36–42 (cap 60); slot 22–25 (cap 25) | **kept** |
| 2. Lessons are made once in the tree | 149 of 240 lines repeated before; now 24–29 non-blank lines repeat, the standing pointers | **kept** |
| 3. The orchestrator checks every premise | premise paragraphs 10 → 47 of the records; did not hold 4 of 10 → 29 of 47, no trend | **changed** (ADR 1748 section 2.1): checked premises carry their command and its output, and a mechanism or a feature's reach is briefed as a hypothesis the round tests first |
| 4. A wall budget per slot | rounds over 4 h 10 of 62 → 0 of 54; batch gap 5.98 → 3.04 h | **kept** |
| 5. A gate's figure is read, not re-taken | section 3e re-takes 5 in nine batches → 1 (1404) | **kept** |
| 6. The batch's HEAD arms once | HEAD exported as a "before" by 5 rounds → 0; one arms hold per batch at 1 694–2 474 s | **kept** |
| 7. The instruments slot's standing items rotate | navigational re-read every fourth batch: due at 66 and 70, done at 66 (1423) and not briefed at 70; instruments ADRs about 1.3 a round | **kept**; the re-read skipped at batch 70 is owed to the next brief with an instruments slot |
| 8. The merge keeps all three tiers | merge tail median 0.73 → 0.78 h; three merge-time reds caught in nine batches | **kept** |
| 9. Every round is told its kind | 60 of 60 slots in ten briefs carry `a "<kind>" round` | **kept** |
| 10. Re-take in ten batches | the scripts were lost with the worktree, and `tools/state.sh batches` misreads every body since batch 64 | **changed** (ADR 1748 section 2.2): a review's scripts are committed beside it; the next re-take is at batch eighty-one or eighty-two |

## 4. For the next re-take

Run, from the worktree root, with `PYTHONDONTWRITEBYTECODE=1`:

- `python3 doc/reviews/1456/slots.py <first batch commit> <last batch commit>`: section 1.1.
- `python3 doc/reviews/1456/locks.py [/home/AI/heavy-walk.log]`: section 1.1.1.
- `python3 doc/reviews/1456/batches.py <first> <last>`: section 1.2.
- `bash doc/reviews/1456/reading.sh`: section 1.3. Change its brief range if the briefs move.
- `tools/state.sh --round <session> traps`: section 1.3's citations.
- `python3 doc/reviews/1456/premises.py <first session> <last session> -v` and
  `python3 doc/reviews/1456/siblings.py <first> <last> -v`: section 1.4. Then read each premise
  paragraph, because the phrase classing is only a first pass (trap 13).

The scripts are a record of how this review counted. A re-take that needs a change copies a script
into its own `doc/reviews/<session>/` and says what it changed. It never edits this one.
