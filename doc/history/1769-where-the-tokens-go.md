# 1769 — Where the tokens go, and how to spend fewer

A "script" round beside batch one hundred and sixteen, 2026-10-26; ADR 2372; ADR 2373 unused; Q635.
I worked in `scratchpad/r1769/`. No ledger row moves; it builds nothing in `crates/`.
**The request.** The owner's of 2026-10-26: ">80% of my usage come from big context", whether to
clear the orchestrator's context between batches, and other savings. Review 1769 is the answer.
**Built.** `tools/token-census.py` counts, per agent of a session's transcripts, requests, calls,
reads, writes, the context at the last and largest request, the requests past 200k and their share,
the input side in dollars at `tools/cache-census.py`'s prices (imported, one table), and waits;
`--detail` the context every 25 calls, the start-set's and the waits' reads and the largest results
by tool and first input line; `--spans` the orchestrator by batch. `doc/reviews/1769/levers.py`
replays the levers. `doc/HANDOVER.md` gains the census's row; `doc/todo/02` section 0 gains item 7.
**Found.** Batch 115's rounds: 314.1 M read, 2.46 M written, $74.06 on the input side, 69 % of it in
requests past 200k; the four rounds past 200 calls were 74 % of it. A context grows ~1.5k a call
from a fixed 39k first request, which is 19 % of all reads. The harness's "subagent_tokens" is the
last context plus the final output, not the cost (1757: 328k shown, 42.3 M read). The orchestrator:
$1 139 since 2026-09-22, $12.0 a batch since 1607, 94 % past 200k, compacting at ~970k. The
four-minute wait holds: 0 writes after a gap past the lifetime in batches 115–116, against 87 % of
all writing in 1490–1569. Break-even of polling against a lapse: ~80–90 minutes on Opus.
**Decided.** ADR 2372: slots sized to ~150 calls and a handover past a 200k context (31 % less,
replayed); a read is a range; long waits spent working; mechanical sub-tasks to a fresh Sonnet
subagent; clearing the orchestrator after a handover, proposed until Q635 (70 % fewer of its reads).
**Premises.** The brief's 2 000-line `cargo test` output did not hold — the largest results are file
reads; its prompt boilerplate is ~0.7k tokens, not 1.1–1.3k per section. `tools/round.sh script`
refuses `ROUND_SESSION=1769` (the brief has no slot) and lists the JavaScript reading; I read its
every-round list. No `tokens` section in `tools/state.sh`: its figures are gates' outputs, and the
transcripts' directory is named by a session id that a clear changes.
**Gates.** `cargo test -p conformance --no-fail-fast`: exit 101, one red, not mine — `records`, on
1764's 47-line record; `commands`, `documents`, `evidence`, `adr_status` and the rest pass.
`python3 -I -B tools/token-census.py` on my own transcript: exit 0; on batches 115–116: exit 0.
`python3 -I -B doc/reviews/1769/levers.py`: exit 0. `tools/cache-census.py` over three periods: exit 0.
