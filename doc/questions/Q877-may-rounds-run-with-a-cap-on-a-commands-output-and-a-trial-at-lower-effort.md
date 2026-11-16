# Q877 — May rounds run with a cap on a command's output, and a trial at a lower effort?

Source: round 2048, your evaluation round ("Can we do something to reduce token load"). The census is
`tools/round-tokens.py` (ADR 2913); the review is `doc/reviews/2048-the-build-and-the-agents.md` §5.

## What a round's tokens are spent on

Over the 21 round transcripts of rounds 2029–2047: $416 at list rates, 3 630 requests. Each request
re-reads the whole context, so a round costs its request count times its mean context. Round 2047's
final context was 335 565 tokens: 23 316 present from the first request (the system prompt, the tool
schemas, `CLAUDE.md` and the memory index), about 100 000 of command output, and 152 884 of the model's
own output — commands, edits and thinking.

The documents a round is told to read are about 8 000 tokens of that; compacting them, `CLAUDE.md`
included, saves at most a few per cent, and this round proposes no change to `CLAUDE.md`.

## The two levers that are not a round's to pull

1. **A cap on what one command returns.** ADR 2372 asks for any output that may pass 4 000 characters
   to go to a file. The census finds 488 of 3 730 results over that, carrying 54 % of every result
   character — mostly `sed -n` and `grep` over source and the ledger. A rule the rounds break 23 times
   each is a rule to enforce rather than repeat. The harness can truncate a command's output at a set
   length (in Claude Code, the `BASH_MAX_OUTPUT_LENGTH` setting; that it applies to subagents is a
   hypothesis to check once). At 8 000 characters it would have cut 0.59 M characters across the 21
   rounds, ~7 000 tokens of each round's final context; at 4 000, 1.72 M, ~22 000 — measured by
   truncating the transcripts' results, not by running rounds so. A round that needs the middle of a long output
   reads it with a range, which is the rule already.
2. **The model's own output is the largest share** of a round's context, and most of it is thinking.
   The reasoning effort is set per agent type. A lower effort for rounds is a quality question before it
   is a cost one — the Sonnet trial of 2026-10-27 is the precedent that a cheaper round can cost more
   in defects — so it would be a trial: three rounds of one kind at the lower setting, judged by
   their gates, their defects found at merge, and `tools/round-tokens.py`.

## Recommendation

1. **Yes** to a cap at 8 000 characters for round agents, set by the orchestrator, and measured after
   five rounds with `tools/round-tokens.py`.
2. **No trial yet.** First let the post-merge train's removal of the tier-2 walks (and their polls)
   and the brief's shortening (review §6) show in the census; a trial of the effort setting is worth
   its risk only if rounds then still pass 250k tokens of context.
