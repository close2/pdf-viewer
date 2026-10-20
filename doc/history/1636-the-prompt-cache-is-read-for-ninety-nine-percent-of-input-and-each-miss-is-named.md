# 1636 — The prompt cache is read for 99 % of input, and each miss is named

Slot 6 of batch ninety-nine, 2026-10-20, instruments as kind `docs`: no ADR, question or `crates/` file.

**Premise.** Held: records 1613 and 1621 and ADRs 1976 and 1977 say what the brief says, and batch
96 alone reproduces 1621's figures (1 518 requests, 3.1 M written, 376.1 M read). The live pricing
page charges a Sonnet 5.5 hit 0.05×, and that is the price used. **Hypothesis.** Rounds over 95 %:
held, 99.12 %. Orchestrator lower: did not hold, 99.39 %. One structural miss, a resume: held for the
rounds; the orchestrator's one was a compaction. Five minutes: held for the rounds, soft from 300 s to
360 s; the orchestrator writes at the hour, and its data shows the hour.

**Built.** `tools/cache-census.py`: a miss reads over 2 048 tokens fewer than the request before left
cached, named by the first account that holds (first request, compaction, model change, gap over the
lifetime after a turn's end or during a command, prefix change by attachment kind, unexplained); send
times from message records, never before the previous request ended; each transcript's lifetime from
its 1-hour writes; prices per model; `--lifetime`, `--keep-warm`, `--parent`, `--tsv`, `--self-test`
(13 cases). Output is not priced: a transcript's `output_tokens` is the stream's opening count (habit
141). Its test is `tools/conformance/tests/cache_census.rs`.

**Census**, rounds 1607–1630, `--since 2026-10-20T01:09:15+02:00 --until 2026-10-20T12:53:06+02:00`:
3 952 requests, 892.2 M input, 99.12 % read; 25 misses — 24 first requests (0.98 M written, read 0,
a batch's eight sent within 8 s) and round 1611's resume after 6 406 s (0.42 M); none unexplained. All
155 requests in [240, 300) s hit; 84 keep-warm re-issues (2.1 % of requests) cost 1.33 M equivalents
and spared 12.78 M; the cache cost 54.0 M ($211 at API prices). Orchestrator, same window: 226
requests, 99.39 % read, 1 miss (the compaction at 00:17:21Z), 27 gaps over 600 s all hits. Lifetime,
all 518 rounds: misses from a 312 s gap, 89 of 95 later hits inside 360 s; orchestrator since
2026-09-22: hits to 3 544 s, misses from 3 747 s, 36 past the hour (33 after a turn's end).

**The answer.** Caching works as intended: 99.1 % of what the rounds send and 99.4 % of what the
orchestrator sends is read from the cache. What is written again is each round's first prompt, a round
resumed after it reported, and the orchestrator's compaction near a million tokens; nothing is
unexplained. The 240 s wall is right and its keep-warm calls repay about tenfold. Nothing should change.

**Gates.** `--self-test`: exit 0, 13 cases; 5 planted faults caught (8, 1, 2, 2, 3 failing).
`cargo test -p conformance --test cache_census`: exit 0. `cargo test -p conformance --no-fail-fast`:
exit 101 on siblings' mid-edits (§7.4.7's frontier row; `viewer-qt`/`viewer-gtk` `saving.rs`,
`requests.rs`; `viewer-ffi/src/window/asked.rs`), none mine; my export: 56 binaries green, 8 failing
on the git index an export lacks. `cargo fmt --check -p conformance`, clippy `-D warnings`: exit 0.
**Files.** `tools/cache-census.py`, `tools/conformance/tests/cache_census.rs`, habit 141 in
`doc/habits/measuring.md`, this record. 1 400 s.
