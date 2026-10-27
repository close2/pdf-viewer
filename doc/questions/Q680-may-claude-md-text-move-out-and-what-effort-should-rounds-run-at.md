# Q680 — May `CLAUDE.md`'s scope detail move into a document of its own, and what effort should rounds run at?

Source: session 1814, answering the owner's request of 2026-10-27 ("investigating possible ways to
save tokens (MD file compaction,... ) ... without compromising quality"). Review 1814 is the
measurement, ADR 2462 section 3 the proposal. Both are the owner's: `CLAUDE.md` is the owner's
statement of the principles, and the effort a subagent runs at is set only when the owner names it.

## The question

1. May the scope list's ratification detail in `CLAUDE.md` — the dates, the Q and A numbers, the ADR
   pointers, and the constructions of the two authoring amendments — move into a document named in
   the file's own "moved out of this file" table, with each exclusion, each amendment and its reason
   kept in `CLAUDE.md` in a sentence? `CLAUDE.md` (31 834 characters) is in the context of every
   request every agent makes; about 12k characters moved is ~3.3k tokens a request, about $1 a batch.
2. Should rounds keep running at the default effort? Thinking is 23.3 % of what a round's context
   accumulates (review 1814 section 1.2), about $11 a batch as it is carried by later requests.

## For

- Item 1 follows a pattern the file already uses (the stack table, the working agreements); no rule
  and no reason is deleted, and a round touching scope opens the document as it opens `doc/stack.md`.
- Item 2 is the largest lever left that no rule of a brief can reach.

## Against, stated fairly

- Item 1 saves about a dollar a batch, and a principle read in full on every request is the reason
  rounds cite it correctly; a moved paragraph is one a round may not open.
- Item 2's cost in quality cannot be read from a transcript: a round that thinks less and is wrong
  shows up as a merge red or a ledger row read wrongly, batches later.

## Recommendation

**Item 1: yes**, if the owner wants the dollar; the wording of what stays is the owner's to approve.
**Item 2: no change** — leave the default, because principle 1 outranks a $11 saving no measurement
can price against quality. The rest of review 1814's savings (ADR 2462 section 2, ~$17–20 a batch)
need no answer here.
