# Q635 — May the orchestrator's context be cleared between batches?

Source: session 1769, answering the owner's request of 2026-10-26 ("we might even should clear this
context between batches. Are there reasons not to?"). Review 1769 is the measurement, ADR 2372
section 3 the rule proposed.

## The question

1. May the orchestrator end its session (or `/clear` it) after each merge and `tools/batch.sh
   close`, and open the next batch in a fresh session that reads `doc/orchestrator-handover.md`, the
   memory index and a handover file the old session wrote?
2. May a round run as an agent type with a tool allowlist (`Bash`, `Read`, `Edit`, `Write`, the
   hand-back, `Agent` for a Sonnet sub-task) so the harness sends fewer tool schemas? Every request
   re-reads a fixed 39k first request — 19 % of a round's reads — and the share the schemas hold is
   unmeasured. It is a configuration change, which no round may make.

## For clearing

- The orchestrator re-reads its whole campaign every turn: a batch adds 70k–140k, and late in a
  cycle each request reads 700k–950k. Cleared to ~65k, the seventeen batches from 1607 would have
  read 70 % fewer tokens — about $4.6 a batch of its $12 — and an hour's lapse would rewrite ~100k
  rather than up to 950k.
- Everything a batch needs is on disk; the compaction already discards the history at ~970k, at a
  turn nobody chose, in a summary the model writes under pressure.

## Against, stated fairly

- A report is delivered only to the session that launched its round, and a cleared session cannot
  message a round it did not launch: a clear is safe only when nothing runs.
- The owner's mid-batch words not yet in memory, and judgement — which reds are the machine's, what a
  sibling touched, which candidate failed its premise — survive only if the handover writes them.
- The first batch after a clear is run from files rather than from memory of the last one.
- The saving is about 6 % of a batch's ~$100; the rounds' own context (ADR 2372 rule 1) is about 20 %.

## Recommendation

**Yes to 1**, by ADR 2372 section 3: clear after the merge and the close, never with a round in
flight, after a handover holding the owner's unrecorded words verbatim, the machine's reds by test
name, the next candidates with their premises, the numbers assigned, and "nothing runs". **Yes to 2,
measured first**: one batch's rounds on the allowlisted type, the census's first-request size
compared. Meanwhile the session continues as now, compacting at ~970k.
