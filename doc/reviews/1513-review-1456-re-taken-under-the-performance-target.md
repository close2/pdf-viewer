# 1513 — Review 1456 re-taken nine batches on, under the owner's performance target

Status: **review**, a record that is never rewritten. Written in slot 5 of batch eighty-one. It
answers ADR 1748 section 4's last item, "the next re-take is due ten batches on, at batch
eighty-one or eighty-two", by running `doc/reviews/1456/`'s commands. Where a command needed a
change it was copied into `doc/reviews/1513/` and the change is named below (ADR 1748 section 2.2).
Three scripts are new there. The decision is ADR 1863.

Every figure was taken on 2026-10-12 against `4c0c04f9`. The period is batches seventy-two to
eighty (`522408c2..4c0c04f9`, 9 batch commits, 53 round records, 1456–1508). The brief named
seventy-three to eighty. Seventy-two is included because review 1456 stopped at seventy-one, so no
batch falls between the two reviews. Seventy-two to seventy-six came before the owner's target and
seventy-seven to eighty came under it. Batch seventy-seven's body calls it "a directed batch: the
project owner asked on 2026-10-10 … that loading be made faster", and batches seventy-eight to
eighty are performance batches. Both halves are given where the target moved a figure. Every command runs
from the worktree root with `PYTHONDONTWRITEBYTECODE=1`.

**Two instruments misread, and both were found by calibrating them.** `tools/state.sh batches` now
sums digit groups, but from batch seventy-four on it prints "no gate wall-time line". The bodies
write `33 of 33 gates green, 1 836 s of gate wall time`, while `tools/batch.sh gates` prints
`… of 33 green, …` and the section's pattern wants that. `slots.py` reads them. And 1456's
`premises.py` finds no premise in 16 of the 53 records, because from about record 1472 a record
heads its paragraph with its verdict, `**Premise, held.**`. `doc/reviews/1513/premises.py` reads that
form.

## 1. Measured

### 1.1 The batch clock

`python3 doc/reviews/1456/slots.py 522408c2 4c0c04f9`, unchanged.

| batch | gap | rounds | sum of walls | slowest | mean round | busy | gate wall | tail |
|---|---|---|---|---|---|---|---|---|
| 72 `522408c2` | 2.50 h | 6 | 8.19 h | 1.76 h | 82 min | 77% | 1 709 s | 0.74 h |
| 73 `a7f5a8e0` | 2.62 h | 6 | 6.57 h | 1.71 h | 66 min | 64% | 1 804 s | 0.91 h |
| 74 `02b035a8` | 2.61 h | 6 | 7.77 h | 1.86 h | 78 min | 70% | 1 836 s | 0.74 h |
| 75 `f9d609c2` | 2.56 h | 6 | 5.49 h | 1.65 h | 55 min | 55% | 1 804 s | 0.91 h |
| 76 `06fa834b` | 2.69 h | 6 | 8.26 h | 1.94 h | 83 min | 71% | 1 582 s | 0.75 h |
| 77 `bf2a9b00` | 2.90 h | 4 | 6.93 h | 2.03 h | 104 min | 57% | 1 756 s | 0.87 h |
| 78 `75531da7` | 4.08 h | 6 | 14.52 h | 3.25 h | 145 min | 74% | 1 781 s | 0.83 h |
| 79 `849a5cfc` | 42.31 h (4.5 h worked) | 6 | 16.06 h | 3.58 h | 161 min | 75% | 1 863 s | 38.73 h (0.9 h) |
| 80 `4c0c04f9` | 5.06 h | 7 | 21.57 h | 4.00 h | 185 min | 90% | 2 156 s | 1.06 h |

Batch seventy-nine's merge gates ran from 10:54 to 11:30 on 2026-10-10. The next line on the lock's
log is the launch gate at 01:17 on 2026-10-12, and the commit came a minute after it. The 37.8 h
between them is a stop with no line saying why, so it is taken out of every figure below.

| | batches 63–71 (review 1456) | 72–80 | 72–76 | 77–80, the target |
|---|---|---|---|---|
| batch gap, mean (median) | 3.04 h (2.87 h) | 3.28 h (2.69 h) | 2.60 h | **4.14 h** |
| mean round | 84 min | 108 min | 73 min | **154 min** |
| slowest round, mean | 2.12 h | 2.42 h | 1.78 h | 3.22 h |
| rounds over 3 h / over 4 h | 3 / 0 of 54 | 10 / 1 of 53 | 0 / 0 | 10 / 1 |
| six slots busy | 50–81%, mean 67% | 55–90%, mean 70% | | |
| merge tail, median | 0.78 h | 0.87 h | | |
| gate wall | 1 643–1 876 s | 1 582–2 156 s | | |
| tool uses per round (mean); s per use | 52–277 (156); 32 s | 66–247 (156); 41 s | | |

- **Under the target the round doubled and the batch grew by half.** The mean round went from 73 to
  154 min and the mean gap from 2.60 to 4.14 h. Tool uses per round stayed at 156, and each one
  took 41 s against 32 s. The extra time went into the waits inside those calls (section 1.1.1).
- **The merge is where it was**: a median tail of 0.87 h. Gate wall rose to 2 156 s in batch
  eighty, which is the launch gate run whole (ADR 1851).

#### 1.1.1 The lock's queue, and what it waited behind

`python3 doc/reviews/1456/locks.py` (unchanged) gives each round's queued wall, which is the union
of its waits. `python3 doc/reviews/1513/lanes.py` is new. It charges each second of that wall to
the highest of the holds running at that moment: clock, the batch's arms export, long, large or
small. Apart from that, it gives the share of the wall during which another round's clock run had
asked or was holding, and the seconds lane 2 stood free while a clock run waited at the gate.
`tools/bounded.sh` grants no walk either lane while a clock run waits, "the ones queued before it
included". Both figures were calibrated on planted logs in `scratchpad/r1513/` (trap 13): a clock
run waiting 90 s behind a large walk gives 90 s idle on lane 2, and a control with the clock line
taken out gives 0 s.

| batch | queued round wall | share of round wall | under a clock hold | another's clock asked or held | lane 2 idle behind the gate | clock runs by rounds | longest single wait |
|---|---|---|---|---|---|---|---|
| 72 | 5 764 s | 20% | 2% | 21% | 2 087 s | 2 | 1 244 s (1461, behind 1459's arms) |
| 73 | 1 868 s | 8% | 0% | 0% | 0 s | 0 | 921 s (1465, behind the export) |
| 74 | 9 760 s | 35% | 0% | 0% | 0 s | 0 | 4 340 s (1470, behind 1471's census) |
| 75 | 2 774 s | 14% | 0% | 0% | 0 s | 0 | 931 s (1475, behind 1478's arms) |
| 76 | 14 047 s | 47% | 42% | 83% | 3 328 s | 7 | 3 118 s (1482, behind 1483's oracle) |
| 77 | 9 670 s | 39% | 9% | 20% | 1 130 s | 8 | 1 002 s (1488, behind 1487's drive) |
| 78 | 34 212 s | **65%** | 32% | 72% | 4 629 s | 21 | 5 693 s (1491, behind 1492's `launch_path`) |
| 79 | 32 613 s | **56%** | 22% | 78% | 5 365 s | 18 | 4 096 s (1498, behind 1499's clock) |
| 80 | 48 646 s | **63%** | 42% | 75% | 3 545 s | 25 | 7 005 s (1505, behind 1507's clock) |

- **The queue is again the largest cost.** It is 46% of round wall over batches 72–80 (159 354 of
  343 265 s), against 32% over 64–71. Before the target it was 26%. Under the target it was 59%,
  and 62% over 78–80. In batch eighty, round 1505 had a walk queued for 12 110 s of its 13 100 s,
  and a 42 s small walk of its own waited 7 005 s.
- **The queue now follows the clock runs.** Rounds ran 9 clock runs over 72–76 and 72 over 77–80.
  The median hold went from 94 to 114 s. In batches 78–80, three-quarters of the queued wall
  passed while another round's clock run had asked or was holding. Only 22–42% of it fell under a
  running clock hold. The rest is the gate. A clock run that waits for lane 1 behind a 1 000–1 700 s
  large walk freezes lane 2 for that long, so 1.0–1.5 h of lane 2 a batch stood free while small
  walks queued for it. `lanes.py` charges that time to the large walk. The cause is the clock run
  that waited behind it.
- **Rounds' six-arm runs**: 15 over the nine batches, 9 of them in 78–80. A whole run held lane 1
  for 1 258–2 187 s and queued for 0–5 693 s first. Of the 15 records that report one, 12 moved no
  digest. In 1478 and 1499 the run moved the pages the change was meant to move. In 1502 it moved
  one gpu page, and that took the offscreen cull back out of the change. That is the arms' one
  finding in the period, and all three of those changes altered what is drawn. Round 1486
  skipped the arms on evidence instead: "`display_list_census` before and after over the corpus
  gate's 958 first pages differs on no page's commands or refusal, so no digest can move by this
  change". Round 1506's change, a cut in the operator loop, left the witness's list digest where it was,
  and the round ran the arms anyway, for 1 672 s. The same digest over the corpus gate's first
  pages would have been the evidence.
- **The batch's own arms export** held lane 1 for 1 573–2 224 s a batch. It is not the cost it was
  in 64–67, where it accounted for the longest wait three times. Here it did once (73).
- **ADR 1790's ask order stands** on `retake.py`'s replay of batches 78–80, with 0 defect pairs.
  Batch eighty's lines replay worse than any earlier batch's: 20 of 53 within 1 s and max |d|
  2 509 s, because four waits were stopped before a grant. So that batch's replayed rules are not
  read here.

### 1.2 What the batches produced

`python3 doc/reviews/1456/batches.py 522408c2 4c0c04f9`, unchanged.

| | batches 63–71 | 72–80 |
|---|---|---|
| ledger status moves | 4 | **0** |
| note-only row edits | 29 | 50 (36 in 74–76, the rows' claims read against their lines) |
| `implemented` / `partial` / `departed` at the end | 711 / 19 / 20 | 711 / 19 / 20 |
| ADRs (lines), per round | 84 (5 479), 1.56 | 63 (4 796), **1.19** |
| records, mean lines | 54, 38.2 | 53, 39.4 (**40 of them at exactly 40**, 47 at 39 or 40) |
| code added / removed; per batch; per worked hour of gap | +36 814 / −4 036; 4 090; 1 347 | +28 796 / −2 769; 3 200; 975 |
| doc added / removed | +9 330 / −1 244 | +8 551 / −317 |

- **The ledger did not move, and two sentences say why.** Every `partial` leaf waits on the owner, as
  review 1456 found. From batch seventy-eight the briefs say that "the partial-row word yields to
  the owner's current target".
- **What the target moved in the product**, from the records rather than re-taken (rule 5). On the
  owner's display, the witness's first frame went from 12.5–19.0 s (record 1497, HEAD before
  batch seventy-nine) to 5.2–6.7 s (1497) and then 3.8–4.8 s (1504). The zoom gesture went from
  over 15 s to 1.4–2.1 s (1497). A notch to the real frame takes 0.17–0.21 s (1502).
- **Paperwork**: fewer ADRs per round, 1.19 against 1.56. The record cap is now the length of a
  record, at 488 words on average against 476. Three conformance runs failed on a sibling's record
  at 41 lines (1477, 1494, 1499).

### 1.3 What a round reads before it writes

`bash doc/reviews/1513/reading.sh`. It is 1456's script with the brief range moved to 1456 onwards.
Citations come from `bash doc/reviews/1513/cites.sh 1456 1508`, which is new. It uses `tools/state.sh
traps`'s own pattern over one range of records, since that section prints no range.

- Every-round reading is 591 lines (574 at review 1456). `measure` is 4 439 (4 235), `loop` 2 161
  (2 065) and `pixels` 1 359 (1 308). No list gained an entry.
- **The briefs left the template's caps.** Whole briefs are 155–251 lines. Common parts are 41–62
  lines, and three of the last four are over 60 (1490, 1502, 1509). The longest slot part is 24–35
  lines: in five of the ten briefs it is over 25, and in 1502's it is 35. Batch eighty-one's "what changed" list has seven
  pointers where the template says at most five. 22–37 non-blank lines repeat from one brief to
  the next. The common part's new lines are the owner's display loop, about eleven lines in section 1,
  which `doc/environment.md`'s *measurement loop* entry already holds, and the witness's figures.
- **Traps**: the index went from 134 to 157 rows. The period's records cite a trap 32 times in 27
  records: trap 13 22 times, trap 1 five, trap 130 twice, and 10, 122 and 142 once each. Habits
  are cited by number once (habit 74). By the same pattern, review 1456's period has 32 citations
  in 22 records. Its own count was 35 in 24, by a pattern it did not keep.

### 1.4 Premises, hypotheses, siblings, exports

`python3 doc/reviews/1513/premises.py 1456 1508 -v`. It is 1456's script changed to find
`**Premise, …**` and `**Hypothesis, …**`, the heading's verdict, and both paragraphs. Every
paragraph was then read by hand, which is the script's `BY_HAND`, and the script prints the 31
places where its phrases disagree with that reading. `python3 doc/reviews/1456/siblings.py 1456 1508
-v` is unchanged.

| | review 1456 (1402–1455) | 1456–1508 |
|---|---|---|
| premise stated | 47 of 54 | 52 of 53 |
| premise did not hold | 29 of 47 | **23 of 52**: 11 a figure, 6 a place, 3 a mechanism, 3 a feature's reach |
| batch by batch, premises missed | 2/5 … 3/5, no trend | 5/6, 4/6, 5/6, then 2/6, 1/6, 1/4, 1/6, 2/6, 2/6 |
| hypothesis stated | (one line, from ADR 1748) | 41 of 53 |
| hypothesis did not hold | | 21 of 41: 15 a mechanism, 3 a reach, 2 a figure, 1 a clause reading |

- **ADR 1748's split worked after a lag of three batches.** Premise misses fell from 14 of 18 over
  72–74 to 9 of 34 over 75–80. Mechanism and reach misses fell from 21 to 6 as premises, because
  they now arrive as hypotheses, which a round tests first by profiling. Under the target, 13 of
  22 hypotheses missed, 11 of them as mechanisms. That is habit 59 doing its work, not a defect of
  the brief.
- **What still misses as a premise is figures and places, 17 of 23.** These are exactly what a
  command finds. Five of the eleven figures were quoted from an earlier record after the tree had
  moved: 1467 (ADR 1758's table), 1490 ("2.27–2.55 s on HEAD (not 7 s; ADR 1813 cut it)"), 1496
  ("2.9 s now, not the 7.3 s quoted"), 1505 (round 1502's "2 169 of 3 509 rows") and 1507 ("About
  450 s held"). Two more were lines that did not exist yet when the brief was written (1466, 1472).
- **Siblings**: all 53 records name one. 19 name a gate that failed on a sibling's in-flight edit
  (17 of 54 before), and none names a defect that came from one. **11 records ran their gates
  from a private export of HEAD plus their patch**, against 5 before, and four of them say it was
  because a sibling's mid-edit broke the shared build (1467, 1490, 1497, 1502).
- **Exports' build time is on no line.** No record and no log line gives one, so the brief's 10–20
  minutes cannot be read. One was measured this batch: round 1512's export built one `bench`
  example of 168 crates in 249 s at load 20–45, through `sccache`. The batch's arms export built in
  149 s (`/home/AI/arms-1509/build.log`).
- **Section 3e re-takes: 0.** Rule 5 held.

### 1.5 Wall budgets

`python3 doc/reviews/1513/budgets.py 522408c2 4c0c04f9` is new. It joins each brief's
`**Wall budget**` for a slot to that session's figure in the body.

- **One round of 53 passed its budget: 1503, at 14 405 s of 14 400 s.** The rounds the brief names
  as "over 10 000 s against 4 h budgets" are 12, not 7: add 1493, 1495, 1497, 1503 and 1508. All
  of them are under four hours.
- Under the target the median round used 72% of its budget, against 36% before, and 10 of 23 used
  more than 75%. In those ten, the queued wall was 51–92% of the round (1491 73%, 1493 84%,
  1505 92%). **The budget now mostly measures the queue.**

## 2. Judged

1. **The lock's queue is the cost, and under the target it is the clock runs and the gate behind
   them.** It is 59% of round wall, three-quarters of it while another round's clock run waited or
   held. 72 clock runs over four batches means about three a round, taken one by one during the
   round's work. Trap 149 adds that a round's clock hold keeps nothing else off the machine, so a
   clock figure taken while siblings build is "not judged" anyway. A round's clock runs cost five
   rounds their lanes, for a figure the merge's hold reads better.
2. **The second cost is six-arm runs that could not have moved a digest.** 12 of 15 moved nothing.
   The one finding came from a change to what is drawn, and round 1486 showed the evidence that
   makes a run unnecessary when nothing drawn can change.
3. **Premises improved, and what is left is stale figures.** A record's figure belongs to that
   record's HEAD. A brief that quotes it as a premise has skipped the command ADR 1748 asks for.
4. **The template's caps drifted** when a new standing instruction, the owner's display loop, was
   written into the brief rather than pointed at. This is a fix the orchestrator makes at the next
   brief, and it needs no rule: the template already says to point.
5. **The rest held**: budgets (one round over, by 5 s), one arms export per batch, no section 3e
   re-take, the merge's three tiers, and every slot told its kind.

## 3. Review 1456's two changed rules, each by its figure

| rule | figure then → now | verdict |
|---|---|---|
| ADR 1748 2.1: a premise is a command, a hypothesis is briefed as one | premise misses 29/47 → 23/52, and 9/34 over 75–80; mechanism and reach as premises 21 → 6 | **kept**; figures quoted from records are the residue (change 3) |
| ADR 1748 2.2: a review keeps its scripts | four of six ran unchanged; two copied with their change stated; three added | **kept** |

## 4. The three things to change next

Each is one line, in the place `doc/todo/_brief-template.md` names for a lesson. ADR 1863 is the
argument.

1. **A round's clock figures are one hold.** It explores by instruction count. It takes every arm
   and every figure its verdict needs in one `--clock` hold, interleaved (habit 87), and the
   launch gate rides in the same hold. This is a line in `doc/todo/02` section 0 item 3.
2. **A change shown byte-identical in its display list owes no six arms.** The evidence is a
   digest of each of the corpus gate's first pages' lists before and after
   (`callgrind_interpret --digest`). Round 1486 made the same argument from `display_list_census`'s
   counts, for a change that could move only a bound. This is a line in the template's *What the
   orchestrator does not write*, and it applies to *Gates owed*.
3. **A figure quoted from a record is re-run at the brief's HEAD, or briefed as a hypothesis.**
   This is a line in the same list. Beside it, the template's premise line now says that what
   still misses is a figure or a place, and it points at `doc/reviews/1513/premises.py`.

Owed, not changed here, because neither file is this slot's. `tools/state.sh batches` should read
`N of M gates green` as well as the log's `N of M green`, or the merge should paste the log's line
as `doc/todo/02` section 8 item 4 says. The gate's freezing of lane 2 behind a large walk is a
question for an instruments slot, which can now measure it with `lanes.py`.

## 5. For the next re-take

From the worktree root, with `PYTHONDONTWRITEBYTECODE=1`:

- `python3 doc/reviews/1456/slots.py <first> <last>`: section 1.1. Take out a stop the log shows.
- `python3 doc/reviews/1456/locks.py` and `python3 doc/reviews/1513/lanes.py`: section 1.1.1.
- `cd /home/AI/lock-replay-1485 && python3 retake.py batch-<first>-<last>`: the replay.
- `python3 doc/reviews/1456/batches.py <first> <last>`: section 1.2.
- `bash doc/reviews/1513/reading.sh` and `bash doc/reviews/1513/cites.sh <lo> <hi>`: section 1.3.
- `python3 doc/reviews/1513/premises.py <lo> <hi> -v`, then read each paragraph and replace
  `BY_HAND` in a copy; and `python3 doc/reviews/1456/siblings.py <lo> <hi> -v`: section 1.4.
- `python3 doc/reviews/1513/budgets.py <first> <last>`: section 1.5.
