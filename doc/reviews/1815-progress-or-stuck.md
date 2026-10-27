# 1815 — Progress, or stuck: batches 109–121 against the two denominators

Status: **review**, a record that is never rewritten. Session 1815, a review round beside batch one
hundred and twenty-one, asked for by the owner on 2026-10-27: "another round investigating if we are
making progress or if we are again (slightly) stuck in planing, doing unnecessary work,...". The
decision is ADR 2464; the owner's part of it is Q681. It builds nothing in `crates/`, `raster/` or
`hosts/`. Its instruments are `doc/reviews/1815/denominators.py` (git alone) and
`doc/reviews/1815/slots.py`, which adds up the two tables beside it. I worked in `scratchpad/r1815/`.

**What the tables are.** `records.tsv` classifies each record `doc/history/1704`–`1813` (110 of them)
by outcome, premise, the ledger moves and archive census it states, and whether it re-measured or
overturned an earlier round (`repeats`) or left part of its contract to a later batch (`carried`).
`briefs.tsv` gives each of the 107 slots of the thirteen batch briefs the earliest batch whose brief
carried the same object. Both were filled by a reader working to the column rules in their headers,
so they are **judgements, not counts**. I checked them against the merge diffs: round 1803's row
said "built" and the diff says §14.11.2.2 `inapplicable` → `implemented`, so I corrected that row.
Batch 121 is unmerged and is read from the worktree as it stood.

## 1. Measured

### 1.1 The coverage denominator: `python3 -I -B doc/reviews/1815/denominators.py .`

| batch | implemented | partial | departed | out-of-scope | inapplicable | moves (intake / finish / demotion / other) | notes rewritten, no move |
|---|---|---|---|---|---|---|---|
| 108 (base) | 729 | 12 | 18 | 84 | 34 | | |
| 109–110 | 729 | 12 | 18 | 84 | 34 | 0 | 4 |
| 111 | 731 | 9 | 19 | 84 | 34 | 3 (0 / 2 / 0 / 1) | 4 |
| 112–113 | 731 | 9 | 19 | 84 | 34 | 0 | 8 |
| 114 | 732 | 9 | 18 | 84 | 34 | 1 (0 / 1 / 0 / 0) | 5 |
| 115–116 | 732 | 9 | 18 | 84 | 34 | 0 | 8 |
| 117 | 744 | 24 | 18 | 58 | 33 | 30 (27 / 0 / 3 / 0) | 10 |
| 118 | 753 | 16 | 18 | 57 | 33 | 9 (1 / 8 / 0 / 0) | 8 |
| 119 | 754 | 16 | 18 | 56 | 33 | 1 (1 / 0 / 0 / 0) | 7 |
| 120 | 757 | 18 | 17 | 53 | 32 | 5 (4 / 1 / 0 / 0) | 8 |
| 121, worktree | 759 | 19 | 16 | 51 | 32 | 6 (2 / 3 / 1 / 0) | 3 |

There were 883 rows throughout. **Batches 109–116 (64 slots) moved four rows.** The 51 moves since
then are 35 intake (rows the owner's A631–A634 and A639 brought into scope, mostly §13.2 and Annex
K), 12 finishes and 4 demotions. **Of the twelve rows `partial` at batch 108, nine are `partial`
today**: §7.4, §7.4.9, §7.6, §7.6.6, §12.2, §12.8, §12.8.3, §12.8.3.4 and §12.8.3.4.4. Only the §11.7
family left: two rows finished and §11.7.5.3 went to `departed`. The `partial` count rose 12 → 19
because rows came into scope faster than they finished.

Every leaf row still `partial` or `reported` names its block in its own note:

| row | what holds it | whose word moves it |
|---|---|---|
| §7.4.9 | CIE Jab (19) needs CIE 131, which the owner declines to buy (A348) | the owner's |
| §7.6.5.1–3, §7.6.6 | the public-key handler: "Wait for a real trigger" (A66; ADR 1485) | the owner's |
| §12.8.3.4.4 | a signature policy no document this project could acquire decides | nobody's |
| §12.2 | the WinUI half is evidenced by CI's `window` job alone | a CI run, after the owner's push |
| §14.8.5.4.4 | five of Table 380's nine entries are a layout engine's that no platform carries | nobody's |
| §13.2.6.2, §13.2.6.3 | two owed sentences of viability | **this tree's** |
| K.2 | the form's own events are not run | **this tree's**, over many batches |

The commit messages of batches 114 and 115 say it in words: "The partial rows are all blocked
outside this machine". Two leaf families can still be finished here.

### 1.2 The PDF/A denominator: the census lines the archive records quote

The census prints converted/conforming/refused/unreadable. The records quote it HEAD → theirs.
`pdf.js as-if-printed` at part 4, converted, at each batch's HEAD: **703** (batch 109) → 723 (112) →
751 (113) → 767 → 766 → 769 (116) → 790 (117) → 784 (118) → 797 (119) → **795** (120) → **792**
(121). (117 → 118's fall is two rounds' figures that were not taken alike; ADR 2460 now prints whether the pack was present.) That is +92 over
eleven batches with one or two archive slots each, about seven documents a slot. At 2b: 757 (113) →
789 (120) → **789** (121). In batch 121 rounds 1811 and 1812 each report 789 → 791 against HEAD.
Round 1813 reports 798 → 800 with the CJK pack installed, which on its own frees nine (an
environment condition, ADR 2460).

**Batches 120 and 121 put seven slots on archive work and moved the part 4 line −3 and then up to
+4.** Batch 120's six refusals of page trees no reader can walk (1805) made the count more honest,
not smaller in fact: veraPDF had said before that it could not parse those outputs. Round 1804
ranked the refusals by documents per reason, over four census lines: `image-smoothing` 43 (a
profile's `stop`); the `as-shown` font losses 94 (a `stop`); the fence rows 41, 29, 23 and 17 (ADR
0816); part 4's `DeviceCMYK` beside an RGB intent 29 (the operator supplies the press profile, ADR
2130). Only then come the reasons a remedy carries under the existing losses: a glyph no shipped face
has (16), a CJK font with no pack (13) and the extension-schema placeholder (12). Rounds 1804, 1812
and 1813 took those three, with hypotheses of "half convert" and "ten of 13". They freed 7, 2 and 2
(nine more with the pack). **Most of what is left of the PDF/A denominator sits behind decisions,
not remedies.**

### 1.3 The robustness denominator

No slot of batches 120–121 is a corpus or oracle slot; the narrowing suspended them. Before that,
pixels and oracle slots re-took the raster golden on 26, 9, 12, 0, 0, 32, 2 and 35 pages in
batches 109–116 (`git diff --numstat` of `crates/pdf-model/tests/raster_golden.tsv` between merges,
988 tracked pages). The golden detects change and gives no verdict (ADR 1016). The oracle's per-arm
error, the verdict, lives in arms directories outside the tree, and only the last two remain, so
the robustness question cannot be answered over this span.

### 1.4 Where the hours went: `python3 -I -B doc/reviews/1815/slots.py .`

655 501 s of round wall time over batches 109–120, from the batch commits' "Round durations":

| lane | slots | share | re-measured or overturned an earlier figure | left part of the contract to a later batch |
|---|---|---|---|---|
| measure / instruments / loop | 16 | 20.1 % | **8** | 8 |
| pixels / oracle | 15 | 20.0 % | 4 | 8 |
| archive | 16 | 17.3 % | 5 (batch 121 incl.) | 6 |
| host | 15 | 16.2 % | 1 | 11 |
| clause / ledger / writer | 18 | 15.6 % | 2 | 13 |
| docs / refactor / review | 16 | 6.9 % | 0 | 1 |
| XFA | 4 (+1 in 121) | 3.2 % | 0 | **5 of 5** |

**76.5 % of the round hours went to slots that moved neither a ledger status nor freed a document.**
13.9 % went to slots that freed one and 9.6 % to slots that moved a status. Before the narrowing
much of that 76.5 % was owed elsewhere: the perf gates (principle 2), the oracle, the windows. In
batch 120, under the narrowing, it was still 39 %: the XFA slot (1800), the §12.2 slot (1801) and
the refusal slot (1805).

Over all 110 records the outcomes are: built 41, built with a named part deferred 39, priced and not
built 7, premise failed with nothing built 2, refactor 12, review 7, other 2. 17 records say their
premise or hypothesis failed. **52 of 110 left part of their contract to a later batch.** 76 of the
107 briefed slots took an object an earlier brief had carried. The objects briefed most often were
§10.7.4 (six batches, 109–116, still `departed`; its note is now 78 706 characters, about 20k
tokens, and grew 2 930 characters while the status stayed), §12.2 (five: 112, 113, 114, 120, 121),
the zoom-notch latency (eight, 109–116, by `briefs.tsv`'s own grouping) and XFA (five, 117–121,
with K.1 moved once and K.2's note rewritten four times).

### 1.5 What was written: `denominators.py`, the lines and bytes sections

Over 109–120 the added lines carried 2.44 MB of code, 0.56 MB of tests and 0.17 MB of tools,
against 0.65 MB of ADRs, 0.43 MB of ledger notes, 0.31 MB of records, 0.24 MB of other documents,
0.10 MB of reviews, 0.09 MB of traps and habits, 0.05 MB of questions and 0.03 MB of `doc/todo`.
**That is 1.9 MB of prose to 3.2 MB of code, about 0.6 bytes per byte.** 156 ADRs were added, about
thirteen a batch, against 49 status moves. The ledger's 240 added lines carry 434 kB because a note
is one line, rewritten whole; the file grew only 62 kB. The ledger rewrote 62 notes without a status
move. Outside the tree each batch writes 58–72 kB of brief, reports, commit message and placement
script.

## 2. Findings, and where we are stuck

1. **Not stuck in planning; stuck at two tails that only decisions move.** The rounds build: 80 of
   110 records built something. What they build no longer moves the counts because the counts'
   remainders are blocked. Five of the seven partial or reported leaf families are held by the
   owner's word (A348, A66), by an unacquirable document, by a platform limit or by a CI run.
   About 280 of the PDF/A refusal instances in round 1804's ranking are profile `stop`s, fence rows
   or the press profile. The briefs keep filling eight slots with what is left, and the yield per
   slot shows it: seven documents a slot before the narrowing, under one after.
2. **The narrowing was followed in label, and only in part in substance.** All 16 slots of batches
   120 and 121 were clause or archive slots. Batch 120 finished one row (§7.4.2), took in four and
   finished no `partial` one. Its documents moved net −3 at part 4, and the six it refused were honest
   corrections. Batch 121 as it stands finishes three (§12.7.4.1, §13.2.4.3 and §13.2.4 derived),
   takes in two, demotes one, and frees up to four documents, plus nine with the pack. The XFA and
   §12.2 slots work rows and finish none.
3. **A row evidenced only by CI turns once per batch.** §12.2 was briefed in five batches. Batches
   120 and 121 each spent a slot on a fix whose verdict comes from the next CI run, which comes after
   the merge and the owner's push. The cycle is the batch, not the work.
4. **A row worked without moving grows its note.** §10.7.4 was briefed in six batches, stayed
   `departed`, and its note reached 78.7k characters. The ledger rewrote 62 notes against 49 moves.
   A note is read whole by every round that touches the row (ADR 2372 rule 2 cannot cut one line).
5. **The measure lane overturns itself.** Eight of sixteen measure, instruments and loop slots
   re-measured or overturned an earlier round's figure (1712, 1724, 1752, 1777, 1784, 1791, 1792,
   1793), mostly by finding that the earlier figure was the machine's load. The lane is suspended
   now. The rule in ADR 2464 is for when it resumes.
6. **Carrying is the norm, and it converges only where the finish line is a status.** §13.2's
   intake in batch 117 finished eight rows in batch 118 because each slot named the rows. XFA's five
   slots each named a step of RFC 0011 and no row, and K.2's note was rewritten four times.

## 3. Rules, in ADR 2464

R1, a frontier line opening every narrowed brief, and slots no more than its objects. R2, a
CI-evidenced row is briefed only on a CI run read. R3, a row is briefed a fourth time only with a
named owed sentence, and its note does not grow without a move. R4, a continuing slot states its
finish line as a status or a census count. R5, a measure slot re-takes the figure it questions under
the same load record before overturning it. Q681 puts to the owner what only the owner can move.

## 4. Rejected

- **Fewer slots per batch as the remedy.** The hours are not idle. A smaller batch would carry the
  same remainders more slowly. The remedy is to put the frontier and the decisions in front of the
  owner.
- **Counting `briefs.tsv`'s continuations as waste.** §13.2 shows continuation that converges.
- **Lowering ADR output by quota.** 156 ADRs against 49 moves reads as excess, but most record a
  refusal's or a measurement's argument, which is the project's memory (CLAUDE.md, "Where knowledge
  lives"). Token cost is review 1814's subject.
- **Reading the oracle's per-arm error over the span.** The arms directories of batches 109–119 are
  gone, and a corpus walk is outside this round's contract.
