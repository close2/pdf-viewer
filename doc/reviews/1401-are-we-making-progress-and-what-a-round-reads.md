# 1401 — Are we making progress, and what does a round really need to read?

Status: **review** — a record, never rewritten. Written in the extra slot of batch sixty-two at
the owner's request of 2026-10-07: "evaluate, if we are making progress, how we can improve
turnout and reduce unnecessary work … go through the documents and clean them up or possibly
split them, so that not every agent needs to read everything." The last review of this genre is
`doc/reviews/984-direction-and-boundaries.md` with ADR 1005, which measured a stall (one ledger
row of 875 moved in 58 sessions) and priced the reading cost; ADRs 0281, 1023 and 1499 are the
rules this one measures against. The decisions are ADR 1638 (how a round is briefed and run) and
ADR 1639 (what a round reads); the document work is done in the same commit.

Every figure below was taken by a command on 2026-10-07 against `main` at `b87299eb`, batches
fifty-three to sixty-two (`5f1bdc55..HEAD`, 8 batch commits, 49 round records 1346–1394). The
commands are named so that the next review re-takes them rather than quoting these.

## 1. Measured

### 1.1 The batch clock

`git log --format=%B 5f1bdc55~1..HEAD | grep -E '^Gates:|^Round durations'`, parsed per round
(`scratchpad/r1401/slots.py`'s reading; `tools/state.sh batches` prints the sums).

| batch commit | gap since last commit | rounds | sum of walls | slowest round | mean round | six slots busy | gate wall |
|---|---|---|---|---|---|---|---|
| 5f1bdc55 (53) | — | 6 | 13.96 h | 3.35 h | 140 min | 69% | 1304 s, 32 gates |
| 57da5459 (54) | 3.65 h | 6 | 9.59 h | 2.91 h | 96 min | 55% | 1384 s |
| 2e5bfcbe (55) | 13.33 h (overnight) | 7 | 13.09 h | 4.49 h | 112 min | 49% | 1396 s |
| 8e1a7205 (56) | 5.96 h | 6 | 21.72 h | 5.25 h | 217 min | 69% | 1360 s |
| c6221445 (57) | 6.32 h | 6 | 14.61 h | 5.66 h | 146 min | 43% | 1381 s |
| fbd2b1b5 (58) | 4.11 h | 6 | 9.74 h | 2.78 h | 97 min | 58% | 1538 s |
| 5d8f154f (59) | 11.08 h (OOM kill, trap 116) | 6 | 15.90 h | 5.00 h | 159 min | 53% | 1459 s |
| b7e868c5 (60) | 2.57 h | 6 | 10.14 h | 2.26 h | 101 min | 75% | 1459 s |
| b87299eb (61) | 3.51 h | 6 | 10.83 h | 2.80 h | 108 min | 64% | 1576 s, 35 gates |

- **A round is 130 minutes on average** (55 rounds), 25 minutes at the shortest and 5.7 hours at
  the longest; tool uses per round 77–206, so 31–124 s per tool use.
- **The batch's wall is its slowest round plus the merge.** The gap between commits minus the
  slowest round is 0.31–1.33 h, typically 0.7 h: the merge (tier 1, the 35 gates at 1 300–1 580 s,
  `install`) runs serially with nothing else on the machine. That is about 15% of a four-hour
  batch — and the *rest* of the slack is bigger: "six slots busy" is the sum of walls over six
  times the slowest, and it is **43–75%**. A quarter to a half of the batch's slot-hours are five
  rounds finished and the orchestrator waiting for the sixth.
- **By slot** (position in the `Round durations` line, 8 batches): slot 1 98 min, slot 2 167,
  slot 3 171, slot 4 197, slot 5 127, **slot 6 (instruments) 64 min**. The batch waits for slots
  2–4, which are the build slots; the instruments slot is done in a third of their time.
- **Gate wall rose 1 304 → 1 576 s (+21%) over nine batches** while three gates were added
  (`turn_path`, `raster_examples`, `script_corpus_engine`; `tools/state.sh gates-cost`). Every
  merge's gate log shows `wait 0s` on the lock: at the merge nothing else runs.
- **Lock waiting inside rounds cannot be measured from the tree.** `flock` and `tools/bounded.sh`
  log nothing about the queue; the records are the only witness, and three of 49 give a figure
  (1355: "3873 s with the lock wait"; 1379: "589 s under the bound (939 s with the lock's queue)";
  1396: a round whose "only process was an A/B waiting on the lock, having written nothing"). The
  honest statement is: between 0 and about an hour per round, unknown, and ADR 1638 asks for the
  instrument before any second lock is argued.

### 1.2 What the batches produced

`scratchpad/r1401/batches.py` (ledger status counts from `git show <commit>:doc/conformance/ledger.toml`,
`git diff --shortstat`, `--numstat` binned by path, ADR and record counts by `--diff-filter=A`);
`scratchpad/r1401/ledger_moves.py` (per row, status change versus note-only change).

| batch | status moves | note-only edits | ADRs (lines) | code +/− | doc +/− |
|---|---|---|---|---|---|
| 54 | 14 (inapplicable → implemented, re-read) | 36 | 10 (622) | +2 756/−247 | +1 287/−208 |
| 55 | 1 | 217 | 7 (519) | +2 563/−309 | +2 067/−229 |
| 56 | 1 | 84 | 10 (629) | +1 908/−250 | +1 139/−178 |
| 57 | 1 | 86 | 12 (786) | +1 168/−123 | +2 679/−8 453 |
| 58 | 0 | 6 | 12 (762) | +10 391/−315 | +1 810/−498 |
| 59 | 1 | 103 | 12 (713) | +7 884/−572 | +1 378/−358 |
| 60 | 0 | 123 | 10 (728) | +9 180/−865 | +1 381/−253 |
| 61 | 25 (departed → partial/implemented, ADR 1622) | 24 | 12 (931) | +5 943/−459 | +1 704/−301 |

- **The ledger moved by re-reading, not by building**: 43 status changes in eight batches, 39 of
  them in the two reclassification batches; the six building batches moved 0–1 row each. Against
  that, 679 note-only edits. The counts at the end (`implemented` 705, `partial` 25, `departed` 20)
  say the same thing as at the start (685/11/39) once the two re-readings are subtracted.
- **The code did move**: +41 800 lines of Rust and tools in eight batches, 10 000 a batch in the
  three JavaScript batches (58–60). RFC 0008's Tier 0 and Tier 1 (a)–(e), the confined script
  worker, the turn-path gate, the raster-examples gate, the outline beside page one, the drive
  under Xvfb in every window — none of it is a ledger row (§12.6.4.17 stays `out-of-scope` by the
  owner's word, `Q286`). **So the ledger is the wrong instrument for this period**, and a review
  that read only it would call eight batches a stall. The right instruments are `tools/state.sh
  scripts`, `drive`, `turn`, `launch` and `gates-cost`, and they moved every batch.
- **Paperwork**: 85 ADRs (1.7 per round, 5 690 lines) and 49 records averaging **37.9 lines
  against a budget of 40** — the cap has become the target. The four navigational files were
  edited in every batch (`doc/HANDOVER.md` 9 of 9, `doc/state-of-play.md` 9 of 9, `doc/todo/65`
  9 of 9, `doc/traps/README.md` 9 of 9).
- **Doc lines per batch 1 100–2 700 added**, about a fifth of the code's; batch 57's −8 453 is
  round 1364 deleting the capitalised session ordinals (ADR 1023's rule, applied).

### 1.3 What a round reads before it writes

`wc -l` on the files the old `tools/round.sh` and `doc/HANDOVER.md` named as every-round reading,
plus the brief.

| every round, before | lines |
|---|---|
| `CLAUDE.md` | 439 |
| `doc/todo/README.md` | 184 |
| `doc/todo/02-every-round.md` | 926 |
| `doc/environment.md` | 646 |
| `doc/HANDOVER.md` | 138 |
| `doc/traps/README.md` | 175 |
| the batch brief (`/home/AI/batch-1395-brief.md`) | 240 |
| **total** | **2 748** |

Per kind, on top of that (the old `kind_reading` lists, whole files): pixels 2 682, oracle 3 902,
parsers 3 189, loop 1 407, instruments 1 672, clause 4 272, measure 3 443, host 1 607, dependency
2 006, docs 577. A clause round was told to read about 7 000 lines; a pixels round 5 400.

**How much of it changes**: the brief grew 202 → 240 lines over sixteen batches and **149 of its
lines are identical in all sixteen**; 30–61 lines are new per batch. Of batch sixty-two's 240
lines, 155 are lessons, owner-answer summaries and incident retellings (the sections "Three
things the last batch got wrong", "Things batches thirty-four and thirty-five taught", "Lessons of
batches thirty-eight to sixty-one", the two owner-answer blocks, the OOM paragraph). `doc/todo/02`
changed in 4 of 9 batches and `doc/environment.md` in 5; `CLAUDE.md` in none.

**What the trap index is used for**: `tools/state.sh traps` counts 739 citations across all of
`doc/history/`, and **the ten most cited traps carry 78% of them** (13, 8, 9, 1, 11, 5, 10, 15,
25, 2). The 49 records of this period cite a trap **five times** (110 three times, 101, 100).
Traps 53–121 — sixty-nine rows added since round 1250 — are cited by seven records in the whole
history, once each. Habits are cited by number twice (habit 63). The index is read by every round
and used by almost none, which is what a list of 126 rows read at speed looks like.

### 1.4 Rework, premises, siblings, duplicated walks

- **Premises**: 10 of the 49 records carry a paragraph headed "premise"; four say it did not hold
  (1378: `/MK` is Table 192 not 189; 1390: a spawn figure read in seconds was milliseconds; 1372:
  the audience observable the brief named does not exist; 1352: hosts do not go through
  `viewer-ffi`). The briefs' own lessons paragraphs confirm it from the other side: batch
  sixty-one's brief lists four wrong premises in its six contracts. Each costs a round the minutes
  of re-establishing what the tree says before it can start — and the orchestrator could have
  spent those minutes once.
- **Siblings**: 25 of 49 records mention a sibling or neighbour; four name a gate that failed on a
  sibling's in-flight edit (`clippy` on `pdf-model` mid-edit, `records` on an unfinished record,
  `state_sections` on an untracked test, `names.rs` on a sibling's doc comment). The cost is a
  re-run, not a defect; `cargo test -p conformance` reading the whole tree is the design.
- **Duplicated walks**: five of 49 records (1347, 1351, 1360, 1366, 1375) exported HEAD and ran
  the corpus arms on both trees; none names a figure for the walk, so "about 2 600 s each" is the
  orchestrator's figure and the records neither confirm nor deny it. No record of this period
  says "fifth round", although `tools/round.sh` printed "a fifth round: section 2 runs whole" to
  every fifth session: the rounds ignored the rule, and the merge ran the walks anyway.
- **Re-measuring**: the performance slot re-took `doc/performance.md` section 3e rows in 1342,
  1351, 1366, 1374 and 1385 — five of nine batches — where `turn_path` holds every row to a band
  since ADR 1513 (batch 54); after that batch the re-take is the gate's job.

## 2. Judged

Where work is produced that nothing uses, concretely:

1. **Reading that no one acts on.** 2 748 lines before any work, of which the trap index (175) and
   the brief's lessons (155) are read and, by the citation count, not used; `doc/todo/02`'s 926
   lines are read for a six-line tier-1 list and a twelve-line record rule. At six rounds a batch
   this is 16 500 lines of instruction a batch, re-read almost unchanged.
2. **Paperwork at the cap.** A 40-line record written to 38 lines on average is a record written
   to the budget rather than to the finding; 1.7 ADRs a round when the contract says "only for a
   decision a later round must not re-litigate". The instruments slot's records each open "ADRs
   <n> and <m>; no row, no question" — two ADRs a batch for a slot that is finished in an hour.
3. **Waiting that is structural.** Six slots busy 43–75%: the batch is as long as its slowest
   round, and nothing bounds that round. The merge's 0.7 h serial tail is the second term.
4. **Rework the orchestrator could have done once.** Four wrong premises in one batch's six
   contracts; a lessons paragraph re-read by six rounds for sixteen batches instead of one line
   in section 0.
5. **Walks the merge repeats.** The fifth-round rule (dead in practice); HEAD exports in five
   rounds; the arms re-run per round where one HEAD baseline per batch would serve every sibling
   and the merge.
6. **Figures re-taken where a gate holds them.** Five re-takes of the frame table after the band
   existed.

And where progress is real and the instruments say so: the code grew 41 800 lines in eight
batches with 34–35 gates green at every merge; the JavaScript stream went from nothing to Tier 1
(e) in three batches; the drive runs every window; a turn is banded; a frame is measured. The
ledger's flat line is the ledger counting the wrong thing for this period, and `CLAUDE.md`'s
"work is chosen from both" is being honoured by the slots that are not ledger slots.

## 3. Proposed, decided, done

Each candidate the owner's request named, with the cost it removes, its risk, and the decision
(ADR 1638 for the running of a round, ADR 1639 for what it reads).

| candidate | removes | risk | decision |
|---|---|---|---|
| a SHORT per-slot reading list | 2 748 → 567 lines every round (`tools/round.sh --lines`), and a kind's list cut to its trap group, its habit file and the one document it changes | a round misses a file it needed — `doc/HANDOVER.md` stays the index, one hop away | **done** (ADR 1639): `doc/todo/02` section 0, `doc/environment.md`'s rule block, `doc/traps/every-round.md`, `doc/habits/every-round.md`, `tools/round.sh` rewritten |
| traps split: ten every round, the rest on demand | 175 → 37 lines; the ten carry 78% of the citations | a trap outside the ten springs unread — it did before too, at 5 citations in 49 records; a contract that touches one names it | **done**; `doc/traps/README.md` keeps every row and `tests/traps.rs` is unchanged and green |
| habits likewise | six files (2 598 lines) → 26 lines up front, the kind's file by `tools/round.sh` | as above | **done** |
| a brief template with a fixed shape, lessons as a pointer | 240 → at most 60 + 25 lines; the lessons paragraph (155 lines) becomes one line where it belongs | a lesson is lost — no: it is made once in the tree instead of retold | **done**: `doc/todo/_brief-template.md`; section 8 step 2 points at it |
| read a gate's held figure instead of re-measuring | five re-takes of the frame table in nine batches, each a release build plus a walk | a band hides a drift — it is the band's job to fail then | **decided** (section 0 item 3, ADR 1638) |
| a corpus-arm cache keyed by the tree's hash | the HEAD arms exported per round (five rounds, "about 2 600 s each") become one export per batch at `tools/batch.sh open` | a sibling's in-flight change makes HEAD the wrong "before" — it is the right before, that is what a baseline is; the cache is keyed by commit so it cannot serve a stale tree | **owed**, not built here: the next instruments slot adds `tools/batch.sh arms` (digests per page per arm, under the batch directory, keyed by `git rev-parse HEAD`), and `open` runs it after the warm build |
| two heavy-walk locks | at most the unmeasured lock wait (three records, 350–3 900 s) | 2026-09-02 took the machine down at 61 GB with one 32 GiB walk beside builds; two 12 GiB walks plus six 8 GiB builds is 72 GiB nominal on a 63 GB machine, and trap 116's kill was a task count no memory bound saw | **declined**; `tools/bounded.sh` is first to gain a `--lock` that logs the wait per command to `scratchpad/r<round>/locks.log` (owed to the instruments slot, not built here), and the question is re-asked on a count |
| a shorter merge: change-reaches gates per batch, the full set weekly | about 20 of the 1 576 s gate minutes a batch | tier 3's catches happened at merges (section 2's own count: fourteen false alarms and one catch in rounds, the real catches at merges); a weekly full set moves a catch a week later across six batches | **declined**; the merge keeps rule 5. What is cut instead is the fifth-round duplicate (below) and the batch's tail (below) |
| records as a table | the writing, not the reading — no round reads a record | the `records` gate reads `**Gates.**`; a table changes the gate | **declined**; the budget stays and the record is written to the finding, not to the cap |
| the instruments slot's standing items rotated | two ADRs and a navigational re-read a batch | an instrument drifts a batch longer | **decided**: the navigational re-read every fourth batch; the frontier map is a gate and needs no re-derivation; an ADR only for a decision |
| the fifth-round rule in a batch | one round in five running tier 3 (about 25 min of walks) that the merge repeats | the map decays unseen — the merge checks it every batch, five times as often | **done**: section 2 rule 4 amended, `tools/round.sh` no longer prints it |
| a wall budget per contract | the 25–57% of slot-hours spent waiting for the slowest round | a round stops mid-build — it reports what is done with every file compiling, which the brief already asks of a killed round | **decided** (ADR 1638): every slot's part carries a budget, 3 h by default, 4 h for a build slot the orchestrator names |

### 3.1 The reading list, before and after

| kind of round | before (every-round + kind) | after (`tools/round.sh --lines <kind>`) |
|---|---|---|
| every round alone | 2 748 (with the 240-line brief) | 567 (+ a brief of at most 85) |
| pixels | 5 430 | 1 280 |
| oracle | 6 650 | 2 856 |
| parsers | 5 937 | 2 575 |
| loop | 4 155 | 1 974 |
| instruments | 4 420 | 2 239 |
| clause | 7 020 | 1 832 |
| measure | 6 191 | 4 010 |
| host | 4 355 | 2 174 |
| dependency | 4 754 | 1 816 |
| docs | 3 325 | 1 097 |

The "after" kind figures count whole files where the list names a section (`doc/verify.md`'s fuzz
block, `doc/performance.md` section 3e), so they are upper bounds; four kinds the old script did
not know — `script`, `writer`, `archive`, `fuzz` — are 3 884, 1 273, 5 347 and 3 319 for the same
reason, and their lists are three files each.

## 4. What the orchestrator stops and starts, next batch

1. **Stop writing the brief from the last brief.** Write it from `doc/todo/_brief-template.md`:
   a common part of at most sixty lines that points, and a slot part of at most twenty-five that
   contracts. `wc -l` both before launching.
2. **Stop retelling lessons.** When a batch teaches something, put it in the tree once — a line in
   `doc/todo/02` section 0, in `doc/environment.md`'s rule block, or a trap or habit from a
   report — and let the brief name where it went, in one line, in section 3 of the common part.
3. **Start checking every contract's premise before writing it**: the table number in `doc/md/`,
   the refusal's call sites in the code, the figure with its unit in the record. Write the evidence
   into the slot's "Premise and its evidence" line. A premise the orchestrator did not check is a
   premise six rounds will.
4. **Start giving every slot a wall budget** and the rule that at the budget the round leaves every
   file compiling and reports. Three hours by default; four for a build slot you name as such.
5. **Stop asking a round to re-take a figure a gate holds.** Brief a performance round against
   `doc/checks/turn-path.toml`'s band and `tools/state.sh turn`; a round moves a band only with a
   reason beside it.
6. **Start the batch's HEAD arms once** — when `tools/batch.sh arms` exists, in `open`; until then,
   export HEAD once yourself into the batch directory after the warm build and tell every pixel
   slot the path, so no round exports it again.
7. **Rotate the instruments slot's standing items**: the navigational re-read every fourth batch,
   otherwise an instrument the records of the last batch asked for (this batch: `bounded.sh --lock`
   logging and `batch.sh arms`).
8. **Keep the merge as it is** — all three tiers, every batch — and run `tools/batch.sh check`
   before `commit`; the batch's clock stays in the commit body as `doc/todo/02` section 8 says.
9. **Tell every round its kind** (`tools/round.sh --list`), so `tools/round.sh <kind>` is the
   reading list and the brief need not be.
10. **Re-take this review's figures in ten batches** with the commands named above, and read the
    reading-list table from `tools/round.sh --lines` rather than from here.
