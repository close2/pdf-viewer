# 1815 — Progress, or stuck: the batches are at two tails that only decisions move

A review round beside batch one hundred and twenty-one, 2026-10-27, asked for by the owner; ADR
2464; ADR 2465 unused; Q681. I worked in `scratchpad/r1815/`. No ledger row moves; it builds
nothing in `crates/`, `raster/` or `hosts/`. Review 1815 is the answer.
**The request.** "investigating if we are making progress or if we are again (slightly) stuck in
planing, doing unnecessary work,...", and whether batches 120 and 121 followed the narrowing to
ledger rows and PDF/A conversions in substance.
**Built.** `doc/reviews/1815/denominators.py` reads git alone. For each batch merge it prints the
ledger's rows by status, every move typed as intake, finish or demotion, the notes rewritten
without a move, and the lines and bytes added by category. `doc/reviews/1815/slots.py` adds up two
tables beside it, `records.tsv` (records 1704–1813) and `briefs.tsv` (the 107 briefed slots). Two
Sonnet readers filled `records.tsv` and a third `briefs.tsv`; they are judgements, not counts.
I checked them against the merge diffs and corrected round 1803's row.
**Found.** Batches 109–116 moved 4 ledger rows in 64 slots; 35 of the 51 moves since were intake.
Nine of batch 108's twelve `partial` rows are still `partial`, and five of the seven leaf families
left name a block outside the tree (A348, A66, an unacquirable policy, a platform limit, CI). PDF/A
part 4 went 703 → 795 over 109–119, about seven documents a slot, then −3 and up to +4 over the
seven narrowed archive slots; about 280 refusal instances sit behind `stop`s, the fence and the
press profile. 76.5 % of the round hours of 109–120 moved neither denominator (39 % under the
narrowing); eight of sixteen measure slots overturned an earlier figure; §12.2 was briefed in five
batches and §10.7.4 in six (its note 78.7k characters). The batches are not stuck in planning: 80
of 110 records built something.
**Decided (ADR 2464, accepted).**
- R1: a narrowed brief opens on its frontier, and briefs no more slots than the frontier has
  objects. Where the spare slots go is proposed, pending Q681.
- R2: a CI-evidenced row is briefed only on a CI run read.
- R3: a row gets a fourth brief only with a named owed sentence, and its note does not grow without
  a move.
- R4: a continuing slot names a status or a count as its finish line.
- R5: a measure slot re-takes a figure before overturning it.

Q681 asks the owner which blocks to reopen, where the spare slots go, and whether a CI branch may be
pushed.
**Gates.** `python3 -I -B -m py_compile` on both scripts exit 0; both scripts exit 0 on the worktree.
`cargo test -p conformance --no-fail-fast`: first run exit 101, two reds in `records`, both this
record's draft (41 lines; a gate not stated); after the cut, exit 101 with one red, `records`
naming round 1814's record, which was still being written, and none of mine; then `--test records
--test adr_status --test evidence --test questions` exit 0.
