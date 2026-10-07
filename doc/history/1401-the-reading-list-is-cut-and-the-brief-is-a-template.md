# 1401 — The reading list is cut to five short things, and the brief is a template

Extra slot of batch sixty-two, 2026-10-07. ADRs 1638, 1639. Review
`doc/reviews/1401-are-we-making-progress-and-what-a-round-reads.md`. No ledger row moved; no
question written.

**Measured.** Eight batch commits (`5f1bdc55..HEAD`) and 49 records: a round is 130 min on average,
the batch's six slots busy 43–75% of the slot-hours (the slowest round governs), the merge's serial
tail 0.3–1.3 h; gate wall 1 304 → 1 576 s over nine batches; 43 ledger status moves of which 39
were two re-readings, against 679 note-only edits; 85 ADRs; records 37.9 lines against a cap of 40;
the brief 240 lines with 149 identical across sixteen batches and 155 of lessons; the trap index
cited five times in 49 records while its ten most-cited rows carry 78% of all citations; four
wrong premises in one batch's six contracts; five re-takes of a banded frame table; the fifth-round
rule ignored by every round and printed to each. The code grew 41 800 lines; the ledger is the
wrong instrument for a period whose output was RFC 0008, the drive and the turn gate.

**Done.** `doc/todo/02` section 0 (the round on one page) and section 8 step 2 pointing at
`doc/todo/_brief-template.md`; section 2 rule 4 retired for a round run in a batch;
`doc/environment.md` opens with a one-line-per-rule block; `doc/traps/every-round.md` (ten rows,
verbatim from the full index) and `doc/habits/every-round.md` (ten pointers); `doc/HANDOVER.md`
rewritten as an index of 88 lines; `tools/round.sh` with short per-kind lists, four new kinds and
`--lines`; `doc/todo/README.md` lists the template. Reading before: 2 748 lines every round plus
577–4 272 by kind; after: 567 plus a brief of at most 85, and by kind 1 097–4 010 (`tools/round.sh
--lines <kind>`, whole files counted where a section is named).

**Decided, declined, owed** (ADR 1638): a premise is checked by the orchestrator with evidence; a
held figure is read, not re-taken; a wall budget per slot; the instruments slot rotates; the merge
is unchanged; two locks and a weekly full set declined; `tools/batch.sh arms` and
`tools/bounded.sh --lock` owed to the next instruments slot.

**Gates.** `cargo test -p conformance`: exit 0 (every test binary passed; `traps`, `records`,
`state_sections`, `the_frontier_map` among them); `tools/batch.sh check`: exit 0 (every line `none` or
`clean`; 53 conformance test binaries ok); `bash -n tools/round.sh`: exit 0; `tools/round.sh --lines`
prints the five every-round entries. First heavy run beside `ps -u AI -o nlwp=` summed: 176 tasks.

Files: the review, the two ADRs, this record, `doc/HANDOVER.md`, `doc/todo/02-every-round.md`,
`doc/todo/README.md`, `doc/todo/_brief-template.md`, `doc/environment.md`, `doc/traps/README.md`,
`doc/traps/every-round.md`, `doc/habits.md`, `doc/habits/every-round.md`, `tools/round.sh`.
