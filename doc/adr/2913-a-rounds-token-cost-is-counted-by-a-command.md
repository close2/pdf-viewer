# 2913 — A round's token cost is counted by a command

Session 2048. Status: **accepted, built.** It cites no clause and moves no ledger row.
Context: `tools/round-tokens.py`, ADR 2372 (review 1769's census, the range-read rule), ADR 1976 (the
waits' price), `doc/reviews/2048-the-build-and-the-agents.md`.

## 1. Why a command

Review 1769 priced the rounds' tokens once, by hand, and its numbers have been quoted since. "Where
knowledge lives" asks for the command that counts a fact rather than the fact, and a token census is
the kind of fact that goes stale fastest: every brief, habit and tool change moves it. The census this
round ran is therefore kept as `tools/round-tokens.py`, which reads the harness's transcripts and
prints, per round, its requests, tool calls, the context at its end, the cache reads, the output and
an estimated price; and over all of them, the tool results' characters by size and the shell
commands' by what they ran.

## 2. What it found the first time (rounds of 2026-11-14 to 2026-11-16, 21 transcripts)

- $416 at list rates over 3 630 requests; cache reads are 85 % to 90 % of every round's price, so a
  round's cost is its request count times its mean context.
- A round's final context is 220k to 375k tokens; the first request's is 23k (the system prompt, the
  tool schemas, `CLAUDE.md` and the memory index).
- Tool results ≥ 4 000 characters — ADR 2372's limit for an output to land in context — were 488 of
  3 730 results and 54 % of the characters; `sed -n`, `grep` and `cat` of source and of the ledger are
  most of them. The cargo commands' own output is about 1 %.
- 228 requests (6.3 %, 7.2 % of the price) were a `tools/wait.sh` that came back unfinished.

The review ranks what follows from these.

## 3. What it is not

It reads transcripts, which live outside the tree and are the harness's format, not this project's;
when the format changes the script says less, and fails no gate. Its prices are list rates written in
the script and are an estimate. It writes nothing, and no gate runs it.
