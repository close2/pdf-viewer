# 2039 — The four-minute wait held over four batches, and three waits in one response are three walls

Session 1601, slot 3 of batch ninety-five. Status: **accepted** — a measurement of ADR 1976's rule
and a change to its census. Context: ADRs 1976, 1977, 2025; trap 204; `doc/history/1570` and
`doc/history/1581`'s owed census. Code: `tools/cache-census.py` (`--until`, the per-transcript count
of gaps). Cites no clause and moves no ledger row.

## 1. The census, and the window it was taken over

Each batch's span is its open and the next batch's open, each a merge commit's `%cI`: batch 91 from
`e44e0850` (2026-10-19T05:53:45Z), batch 92 from `58223a35` (09:18:16Z), and batch 95 from
`b9aa6878` (20:07:16Z). The transcripts are the round agents' own, found by the prompt each opens
with (`You are ROUND <n>`), 1571–1598, every one of them present. The command, per span:

    python3 -I -B tools/cache-census.py --since 2026-10-19T09:18:16Z \
        --until 2026-10-19T20:07:16Z --buckets <the 22 transcripts of rounds 1577–1598>

| batch | rounds | requests | written, M tokens | read, M | gaps over 300 s | missed | cache cost, M equivalents |
|---|---|---|---|---|---|---|---|
| 91 | 6 | 1 169 | 2.0 | 253.9 | 1 (308 s) | 0 | 15.2 |
| 92 | 8 | 1 290 | 2.3 | 287.8 | 0 | 0 | 17.3 |
| 93 | 7 | 1 302 | 2.2 | 278.4 | 0 | 0 | 16.7 |
| 94 | 7 | 1 132 | 2.1 | 247.5 | 3 (353–359 s) | 0 | 15.0 |

Over 92–94: 0 of 3 724 requests missed; the gap's median 9.4 s, p99 254 s, none past 600 s; 184
requests sent 240 to 300 s after the one before, which is the 240-s wall at work, all hits. ADR
1976's 48 rounds wrote 13.0 k tokens a request, 85 % of it after a gap over 300 s; these write
1.8 k, none of it after a gap. **The hypothesis held**: with eight rounds a batch the rule held.

**The instrument sees a miss** (trap 13): the same script over round 1565's transcript, before the
rule, counts 14 gaps over 300 s and 14 misses, 93 % of its writing. The orchestrator's transcript
over 92–94 is no control for the five-minute rule: it writes at the hour (0.3 M of 0.3 M), and its
34 gaps over 300 s, 31 of them turn ends of 1 068 s mean, were all hits.

## 2. What still waited past the cache, and on what

All four gaps over 300 s followed `tools/wait.sh`. Round 1572's was a full 240-s wall after a request
that spent 67 s writing the call (308 s send to send): the 60 s between the wall and the lifetime
are the model's own, and a long deliberation before a wait spends them.
**Round 1598's three were each one response carrying three `timeout 118 tools/wait.sh` calls** —
one wait per Bash call, as trap 204 asks, but the harness runs a response's calls one after another,
so the walls added up: 3 × 118 s, the next request 353, 359 and 358 s after the last. Each hit (a
few hundred tokens written, 152–157 k read), as 4 of 25 in [330, 400) s did under ADR 1976, so
the lifetime is not the 300 s edge exactly — but nothing should rest on a hit past it. The rule
this asks for is trap 204's, one level up: **a response carries one wait**, which the report
proposes, a round not writing traps.

## 3. What the census gained

`--until` closes the window, because an orchestrator's transcript and a resumed round run across
the next batch's open (habit 110). The transcripts list now counts each one's gaps over the
lifetime beside its loss, ranked by both: where every gap was a hit, the loss column is 0 for all
22 rounds and the count is the only column that names the round that waited.
