# 1814 — What a round pays for, and what it can stop paying for

A review beside batch one hundred and twenty-one, 2026-10-27; ADR 2462; ADR 2463 unused; Q680.
I worked in `scratchpad/r1814/`. No ledger row moves; it builds nothing in `crates/`.
**The request.** The owner's of 2026-10-27: ways to save tokens and work — Markdown compaction,
unnecessary work in agents, compiling and testing once after the rounds — without compromising
quality. Review 1814 is the answer.
**Built.** `doc/reviews/1814/work.py` splits a round's input bill over its tool calls (a request's
fixed first context, and what it added as read by every later request and written once), sums them
by kind of work, and with `--fit`, `--reds`, `--where` and `--first` fits tokens per character, sorts
failing build-and-test results by whose file they name, splits build and test between the shared
tree and an export, and prints the first request by agent type.
**Found.** Batches 117–121's rounds: $60.6 a batch against batch 115's $74.06, mean calls 168 against
201, the median context at hand-back 282k unchanged; ADR 2372's 200k handover fired in none of 34
rounds past it, and Q635's `round` type ran in none of 43 (all `general-purpose`, ~40k a first
request, ~15.8k of it tool schemas and harness text). Replayed, a 200k cap is still 26 %. Growth is
results 51.6 %, tool inputs 18.8 %, thinking 23.3 %. By kind: code read and search 46 %, Markdown
22 %, build and test 13 % (24 h of wall), measure 12 %. Rounds' own gates failed on their own files
128 times in 37 rounds; the four merges found two defects no round had. Batch 121's orchestrator
polled at 568 s and rewrote 1.1 M; the top-level session lapsed its hour six times, $18.6.
**Decided.** ADR 2462: the orchestrator runs the census each poll and tells a round past 200k;
rounds launch as `round` in the fresh batch-122 session, measured; no orchestrator outlives its
cache; a read over 8 000 characters is two ranges; conformance once; the brief names traps by
number; `round.sh` drops `CLAUDE.md`. About $17–20 a batch. Q680 asks the owner about moving
`CLAUDE.md`'s scope detail out and about the rounds' effort (recommended: no change).
**Rejected.** Compiling and testing only at the merge, a worktree per round, shorter reports,
polling less, compacting the trap files — review 1814 section 4.
**Premises.** The brief's "every round builds twice": 38 of 43 built in an export, $10 of the $38.5;
its "~25 kB common part" is 15.9k characters. `tools/round.sh script` refuses `ROUND_SESSION=1814`
on this branch; I read its every-round list, printed without the variable.
**Gates.** `cargo test -p conformance --no-fail-fast`: exit 0 on the second run; the first
was exit 101 on `records` alone, this record's own unfinished Gates line. `python3 -I -B -m
py_compile doc/reviews/1814/work.py`: exit 0 (no Python linter is installed); `work.py` and its four
options over batches 117–121: exit 0. `tools/token-census.py` over the rounds, the orchestrators and
the Sonnet sub-tasks: exit 0; `tools/cache-census.py --keep-warm` over rounds and orchestrators and
`--since` over the top-level session: exit 0; `doc/reviews/1769/levers.py`: exit 0.
