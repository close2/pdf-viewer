# 2372 — A round is sized to its context, and the orchestrator is cleared after a handover

Session 1769, a "script" round beside batch one hundred and sixteen. Status: **accepted** for the
round rules of section 2, **proposed** for the orchestrator's clearing of section 3 until the owner
answers Q635. Context: the owner, 2026-10-26: ">80% of my usage come from big context … we might
even should clear this context between batches. Are there reasons not to?"; review 1769 is the
measurement; ADRs 1976, 1977 and 2039 are the wait rule this keeps. Code: `tools/token-census.py`,
`doc/reviews/1769/levers.py`. Cites no clause and moves no ledger row.

## 1. What decides it

A request re-reads its whole context, and a round's context grows by about 1.5k tokens a tool call
from a fixed 39k, so a round's reads are quadratic in its calls (review 1769 section 1.1). The round
cache no longer lapses — batches 115 and 116 wrote 0 tokens after a gap past the lifetime — so the
reads are what is left to cut: 85 % of a round's input dollars, and 69 % of batch 115's are in
requests above 200k. The four rounds past 200 calls were 74 % of that batch. The levers are ranked
in review 1769 section 3; this ADR takes the ones a brief can carry.

## 2. The rules the briefs carry from the next batch, as section 1 sentences

1. **A slot is sized to finish within about 150 tool calls.** A contract larger than that is two
   sessions in sequence, the second opening on the first's handover file, not one long round. A
   round whose context passes 200 000 tokens — `python3 -I -B tools/token-census.py <tasks-dir>
   --rounds <session>` prints its `last` — with more than a quarter of its work ahead writes a
   handover (what is done, what is next, the premises checked, the files touched, the gates' state)
   into its scratch directory, reports "handed over", and stops. Replayed over batches 115 and 116 a
   200k cap costs 31 % less with seventeen handovers, ten re-orientation calls each priced in.
2. **A read is a range.** `grep -n` before `sed -n`; a file range is at most about 150 lines; an
   output that may pass 4 000 characters — a test run, a census, a sweep — goes to a file in the
   scratch directory and is read by `grep` or `tail`. Results over 4 000 characters carry 18 % of the
   two batches' reads, and nearly all of them were file reads, not test output.
3. **The four-minute wait stands, and a long wait is spent working.** A wait re-issued inside the
   five-minute lifetime costs one cache read (0.05 of a token on Opus 5.5); a lapse costs a write of
   the whole context (1.25), so polling every 240 s is cheaper than lapsing for any wait under about
   80–90 minutes, and a Bash call cannot block longer than 600 s. What is cut is the waiting itself:
   a hold that the lock's log shows will queue behind another walk (`doc/environment.md`'s first
   rule) is asked for after the round's lock-free work, not before it.
4. **A mechanical sub-task goes to a fresh Sonnet subagent**, `subagent_type` `general-purpose`
   with `model` `sonnet` and a self-contained prompt, never a `fork`, which inherits the whole context:
   a census, a multiset proof, a formatting pass, a sweep whose answer is a count. Its report is a few
   lines; twenty calls there, and the one report the round then carries, cost about a sixth of
   twenty calls at 300k in the round and their results carried to its end.
5. **The brief's common part is read once and kept short**; each 1 000 tokens cut from it saves
   about $0.30 a batch. The prompt section is not shortened for tokens: its boilerplate is about
   0.7k tokens, $0.20 a batch, and it carries the rules that keep the cache.

## 3. The orchestrator is cleared between batches (proposed — Q635)

**When**: after the merge and `tools/batch.sh close`, before the next batch's first launch — the one
moment no round runs, no background wait is held and no report is owed to the session; never with a
round in flight, because a report is delivered to the session that launched it.

**The handover first**: the orchestrator writes the batch's handover file beside its brief, holding
what memory does not: the owner's words since the last memory write, verbatim; the reds the merge
called the machine's, by test name; the next batch's candidates with the premise each was checked
against; the numbers already assigned; and a line saying nothing runs. The memory index and the
running log are updated in the same pass. The new session reads `doc/orchestrator-handover.md`, the
memory index and that file, and nothing else up front.

**What it buys**: over the seventeen batches from 1607 the orchestrator read 452 M at an average
context of 300k–950k; cleared to 65k it would have read 137 M — 70 % fewer, about $4.6 a batch — and
each lapse of its hour would write ~100k rather than up to 950k. It costs a first write of ~50k at
the hour, about $1, and the judgement the handover does not carry.

## 4. Rejected

Polling less often or past 240 s (the lapse is dearer for any wait the tool allows); shortening the
prompt for tokens (cents); a cap below 150k (it saves 2 points more than 200k with nearly twice the
handovers).
