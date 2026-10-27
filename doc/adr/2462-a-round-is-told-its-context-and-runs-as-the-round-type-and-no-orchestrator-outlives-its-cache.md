# 2462 — A round is told its context and runs as the `round` type, and no orchestrator outlives its cache

Session 1814, a review beside batch one hundred and twenty-one. Status: **accepted** for the rules
of section 2, which the briefs carry from batch 122; **proposed** for section 3 until the owner
answers Q680. Context: the owner, 2026-10-27, asked for "possible ways to save tokens (MD file
compaction,... but also maybe unnecessary work in agents ...) without compromising quality"; review
1814 is the measurement; ADR 2372 is the rule set this amends, and A635 the approval its rule 2
waited on. Code: `doc/reviews/1814/work.py`. Cites no clause and moves no ledger row. ADR 2463 is
unused.

## 1. What decides it

Batches 117–121 cost $60.6 a batch in rounds' input against batch 115's $74.06, because fewer rounds
ran past 200 calls; the context at hand-back did not move (a median 282k), and ADR 2372's handover
at 200k fired in none of the 34 rounds that passed it, because a round cannot see its context
(review 1814 section 1.1). Replayed, that cap is still worth 26 %. Q635's tool allowlist, approved by
A635 "measured first", never ran: every round launched as `general-purpose`, with a first request of
~40k of which ~15.8k is tool schemas and harness text. Compiling and testing inside the round is not
the waste — its own gates found 128 failures in its own files, the merges two defects no round found
— but the shared tree's breakage is, and so are two orchestrator caches that lapsed.

## 2. The rules the briefs carry from batch 122

1. **The orchestrator tells a round its context.** At each of its polls it links the batch's round
   transcripts (the `.jsonl` files only, never a task's `.output`) into one directory and runs
   `python3 -I -B tools/token-census.py <dir> --rounds <first>-<last>`; to a round whose `last` has
   passed 200 000 it sends one message: "your context is <n>k; if a quarter of your work is ahead,
   write the handover of ADR 2372 rule 1 and report handed over". The second session of that slot is
   launched on the handover file, with the same session number. Replayed over batches 117–121: up to
   $15.8 a batch, eight handovers a batch, re-orientation priced in.
2. **Rounds run as the `round` type.** The batch-122 session opens fresh after A635's handover, so
   `~/.claude/agents/round.md` registers; rounds and their Sonnet sub-tasks launch with
   `subagent_type: "round"`, and the first batch's census compares the first request against the
   ~40k of `general-purpose` (`work.py <dir> --first`), adopting the type or not on that number, as
   A635 asked. Expected: 10–16k fewer tokens a request, $2.6–4.2 a batch.
3. **No orchestrator outlives its cache.** An orchestrator that is a subagent polls at 240 s like a
   round (ADR 1977); the top-level session, on its hour, wakes itself before the hour passes while a
   batch runs — one request at its context — rather than lapsing to a write of it. Batch 121's
   orchestrator polled at a mean 568 s and wrote 1.1 M after it; the top-level session lapsed six
   times in five batches, ~$3.1 each.
4. **A read over 8 000 characters is two ranges**, the second asked for once the first showed it is
   needed: 4 % of reads, ~$2 a batch, replayed.
5. **`cargo test -p conformance` runs once before the report**; while iterating a round runs the
   test it is changing by `--test <name>`. Rounds started the whole suite 3.9 times each.
6. **The brief's discipline paragraph names traps and habits by number** and leaves their text in
   their files, where a round reads the ones its slot needs; no trap, habit or reason is deleted.
7. **`tools/round.sh` stops listing `CLAUDE.md` as a read**: it is in every request's context
   already, and 34 rounds in 43 read it again.

What stays: the round's own tier 1 and tests in the shared tree, the export when a sibling's edit
breaks the build, the merge's three tiers once, the four-minute wait, the report's length.

## 3. For the owner (proposed — Q680)

1. **`CLAUDE.md` text moved, not cut**: the scope list's ratification detail (dates, Q and A
   numbers, ADR pointers, the two authoring amendments' constructions) into a document named in the
   file's own "moved out of this file" table, the rule and its reason kept in `CLAUDE.md` in one
   sentence each. Every agent re-reads `CLAUDE.md` on every request; ~12k characters moved is ~3.3k
   tokens a request, about $1 a batch.
2. **The rounds' effort**: thinking is 23.3 % of what a round's context accumulates, about $11 a
   batch carried. The setting is the owner's to name; the recommendation is to leave it, because no
   transcript can show what a lower setting costs in quality, and quality is principle 1.

## 4. Rejected

Compiling and testing only in the orchestrator (it moves ~25 own defects a batch to a session that
did not write them, for less than the $7.7 build and test cost in all); a git worktree per round
(eight cold builds at once on a machine whose memory has failed before, and a real merge of the
ledger, unpriced); shorter reports (~$0.8 a batch carried, and their substance is needed); polling
less (two round lapses in 6 584 requests); compacting the trap and habit files (4.6k and 3.3k
characters). Review 1814 section 4 has each with its number.
