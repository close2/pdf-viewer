# 1513 — Review 1456 re-taken under the owner's target, and a round's clock is one hold

Slot 5 of batch eighty-one, 2026-10-12, a measure round on the process. ADR 1863; 1864 not used; no
Q. Ledger: no row (a review; the batch is the owner's performance target).

**Premise, half held.** `ls doc/reviews/1456/` lists six scripts, and its section 4 names their
commands. Four ran unchanged. `premises.py` finds no paragraph in 16 of 53 records, because records
now head it `**Premise, held.**`. `tools/state.sh batches` prints "no gate wall-time line" from batch
seventy-four on: the bodies write `33 of 33 gates green` where the log prints `33 of 33 green`.
`retake.py` runs. Seven rounds are over 10 000 s, as the brief says, but the full list is twelve
(add 1493, 1495, 1497, 1503 and 1508), and only 1503 passed four hours, at 14 405 s.

**Hypothesis, half held.** The queue is the largest cost: 46% of round wall over batches 72–80, and
59% under the target, against 32% before (`locks.py`). It is not behind six-arm walks. In 78–80,
three-quarters of it passed while another round's clock run had asked or was holding, and 72 clock
runs over four batches against 9 before. A waiting clock run stops grants on both lanes, so lane 2
stood free for 3 545–5 365 s a batch (`lanes.py`, calibrated on planted logs in both directions).
No line records an export's build: the one measured, round 1512's, took 249 s, not 10–20 minutes.

**Built.** `doc/reviews/1513-…md`, which is review 1456's tables re-taken over batches 72–80 and
split before and under the target. Beside it in `doc/reviews/1513/` are `lanes.py` (what the queue
waited behind), `budgets.py` (the wall against the brief's budget), `cites.sh`, and copies of
`premises.py` (the new heads, and a hand reading of all 93 paragraphs) and `reading.sh` (the brief
range). Three lines follow from it. Section 0 item 3: a round's clock figures are one hold. The
template: a change whose list is unmoved on the corpus owes no six arms (12 of 15 round runs moved
nothing), and a record's figure is re-run at the brief's HEAD (five of eleven figure misses).

**Found.** Premise misses fell from 14 of 18 (72–74) to 9 of 34 (75–80), and 17 of the 23 are
figures or places. Hypotheses missed 21 of 41, 15 of them mechanisms, which are found by profiling.
40 of 53 records are exactly 40 lines. Briefs are over their 60 + 25 caps in 1490, 1502 and 1509.
Batch seventy-nine's commit came 37.8 h after its gates, a stop with no line saying why. **Owed**
(ADR 1863 section 4): the gate's freeze of lane 2 to an instruments slot; `batches`'s pattern.

**Files.** `doc/reviews/1513-review-1456-re-taken-under-the-performance-target.md`; `doc/reviews/1513/`
(`lanes.py`, `budgets.py`, `premises.py`, `reading.sh`, `cites.sh`); `doc/todo/02-every-round.md`
(section 0, one line); `doc/todo/_brief-template.md` (two lines, one pointer); ADR 1863; this record.

**Gates.** Each review script: exit 0 (nine commands); `bash -n` on both shell scripts: exit 0.
`cargo test -p conformance --no-fail-fast`: exit 101, 430 passed, 1 failed, `names` on slot 1's mid-edit `progress.rs:267`. No walk was run, so no lock was asked for.
Duration 1 716 s.
