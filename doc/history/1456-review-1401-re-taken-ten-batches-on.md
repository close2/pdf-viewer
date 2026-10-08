# 1456 — Review 1401 re-taken ten batches on

Slot 1 of batch seventy-two, 2026-10-08, a docs round. ADR 1748; ADR 1749 and Q352 not used; no
row moved; no code.

**Premise.** Held: review 1401's section 1 names each figure and how it was taken, and its scripts
are gone. Did not hold for `tools/state.sh batches` as "the sums": from batch sixty-four the bodies
write "7 497 s", the section's `[0-9]+ s\b` reads the last group (2 135 s printed for batch
seventy-one's 47 135 s) and finds no gate line, so the review reads its own `slots.py` instead.

**Built.** `doc/reviews/1456-ten-batches-on-the-template.md`, in review 1401's shape, over batches
63–71 against 53–62. Six scripts are committed beside it under `doc/reviews/1456/`, each named at
the table it fills: `slots.py` (the clock), `locks.py` (the lock's queue from
`/home/AI/heavy-walk.log`, the union of a round's waits), `batches.py` (ledger moves, ADRs, records,
code and doc lines), `reading.sh` (`tools/round.sh --lines` per kind, brief parts), `premises.py`
(with the hand reading as `BY_HAND`, printing the seven records where the phrases disagree) and
`siblings.py`. Working copies are in `scratchpad/r1456/`.

**Found.** The batch gap fell from 5.98 h to 3.04 h (mean), and no round of 54 passed 4 h (10 of 62
before). The lock's queue is the largest term: 32% of round wall over batches 64–71, and 52% in
batch 71. Ledger: 4 status moves, all in batch 63, and 29 note-only edits against 685. 29 of the 47
records with a premise say part did not hold, with no trend; 21 of the 29 were a mechanism or a
feature's reach. 17 of 54 records name a gate red on a sibling's in-flight edit (4 of 49 before).

**Rules (review section 3).** Kept: 1, 2, 4, 5, 6, 7, 8, 9. Changed by ADR 1748: 3 (a premise is a
command and its output; a hypothesis is briefed as one and tested first) and 10 (a review's
scripts are committed beside it). Edited: `doc/todo/_brief-template.md` (the slot's premise line
and a hypothesis line, and the reason), `doc/todo/02` section 0 item 2 (test the hypothesis first).

**Owed.** `tools/state.sh batches` reading digit groups goes to the slot holding `tools/state.sh`
(slot 5). The navigational re-read, skipped at batch 70, goes to the next instruments brief.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 101, 422 passed and 2 failed. Both
failures are siblings' work in flight: `records` on records 1459–1461 and `round_numbers` on slot
5's `bounded.rs:1907`; none is mine. `cargo run -p conformance --bin pointers`: exit 0.
`--bin quotations`: exit 0. `tools/batch.sh check`: exit 1, only on slot 2's then-unfinished
record 1457; the fuzz workspace check was clean after queuing 2 411.5 s behind the batch's arms.
Both `doc/todo` edits were restored to HEAD at 17:55:30 by another writer and were re-applied
(`scratchpad/r1456/todo.patch`). Duration 3 760 s.
