# 2479 — The batch is orchestrated by the top-level session, because a nested batch agent saved no tokens

The merge of batch one hundred and twenty-two; 2026-10-27. Status: **accepted**. ADR 2479 was assigned
to round 1822 and unused there. Context: the owner, 2026-10-26, proposed that one thin long-running
session spawn one batch-orchestrator subagent per batch so that the session's own context stays small
(ADR 2372 section 3, Q635, A635 "yes, please try it! If it doesn't work out, we will simply revert
back."); batches one hundred and seventeen to one hundred and twenty-one ran that way. The owner,
2026-10-27: "evaluate if running sub-agents for batch-orchestrators is noticeable improving token
usage? otherwise go back to using this agent as orchestrator." Code: `tools/token-census.py` is the
instrument (ADR 2372). Cites no clause and moves no ledger row.

## 1. What was measured

`tools/token-census.py` over the round transcripts, the batch agents' transcripts and the top-level
session's own, at list prices on the input side. The orchestration cost of a batch is the batch agent's
input plus the top-level session's input over the batch's span:

| batch | shape | rounds | orchestration |
|---|---|---|---|
| one hundred and fifteen | the session orchestrates | $74 | $4–8 |
| one hundred and sixteen | the session orchestrates (eleven rounds) | $87 | $13.4 |
| one hundred and seventeen | nested batch agent | $54 | $8.5 + $3.0 |
| one hundred and eighteen | nested batch agent | $63 | $7.0 + $3.2 |
| one hundred and nineteen | nested batch agent | $56 | $8.9 + $6.7 |
| one hundred and twenty | nested batch agent | $66 | $6.3 + $3.5 |

The batch agent's context reached 236–313k tokens over 160–200 requests — the same growth the session
itself showed — and every wake of the thin session after a four- to five-hour batch re-wrote 170–220k
tokens of context at the one-hour cache's write price, about $3 a wake. The fall in the rounds' cost
(from about $9.3 to $5.6–8.2 a round) is ADR 2372's round rules, which both shapes carried, not the
shape. Review 1814 priced the two shapes about even from the other side.

## 2. Decision

The top-level session orchestrates a batch itself from batch one hundred and twenty-two: it launches the
rounds, records each report in the batch's `reports` file, launches the Opus validator of each Sonnet
round (the owner, 2026-10-27), runs the merge chain and reads the first CI run. What the nested shape
proved worth keeping is kept: the per-batch handover file written before the next batch, so that a
compaction of the session mid-batch loses nothing; the standing brief as the orchestrator's checklist;
the rules of ADR 2462, in particular that no orchestrator outlives its cache (section 2 rule 3). A batch
agent is no longer spawned. The `round` agent type is listed in neither a nested agent's session nor
this one, so the rounds stay `general-purpose` with `round.md` named first in the prompt, and A635's
allowlist stays unmeasured until a session lists the type (ADR 2462 section 2 rule 2).

## 3. What reopens this

A harness change that lets a subagent hold a one-hour cache, or a batch whose orchestration cost in the
census passes a third of its rounds', is a reason to measure the nested shape again. Until then the
session that writes the brief runs the batch.
