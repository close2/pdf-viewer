# 1716 — An hour on `script` and `vfs_write` and half an hour on three more find no crash

Session 1442. Status: **accepted**. Context: ADRs 1423, 1424, 1559, 1694, 1717; traps 107, 111,
112; `doc/verify.md`'s fuzz lines; `fuzz/seeds.sh`.

## 1. Before: the census

`fuzz/seeds.sh check` on the five targets, one `--tree 6` hold (373 s, peak 0.37 GiB): every one
**current** — disk against fresh `INITED cov`, `script` 13 515 / 13 513, `vfs_write` 5 968 / 5 968,
`aform` 1 369 / 1 369, `forms_data` 1 387 / 1 285, `embed` 1 257 / 1 257.

## 2. The campaigns

Built `-O -s none` by the census, run by path, a scratch corpus first and `fuzz/corpus/<target>`
second, artefacts and libFuzzer's fork directory to scratch, under each line's `-rss_limit_mb`,
`-timeout` and `-max_len`. Two departures from ADR 1694's recipe, each for a reason:

- **Fork mode past every stop** (`-fork=1 -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1`),
  so that a crash in the first minutes would not end an hour that was meant to be an hour; every
  stop would have been an artefact to read, and none came.
- **All five in one hold of the second lane**, `script` and `vfs_write` for 3 600 s and the other
  three for 1 800 s beside them: 3 611 s of lane time for 3.5 h of campaign, peak 0.53 GiB over
  the tree. It still cost the batch: four siblings' small walks queued behind it for 860 s to
  1 134 s while the first lane held the arms.

A fork-mode parent prints no `INITED`, and its running `cov` is not the same count as an
in-process run's, so each target's figure *after* is one `-runs=0` pass over the disk corpus with
the campaign's finds beside it (one hold, 347 s). Edges, before → after, executions, and what the
last tenth of the run added:

| target | time | disk census | finds | after | executions | last tenth |
|---|---|---|---|---|---|---|
| `script` | 3 600 s | 13 515 | 6 463 | 20 170 | 2 062 988 | 19 958 → 20 126 |
| `vfs_write` | 3 600 s | 5 968 | 4 850 | 8 306 | 1 702 007 | 8 286 → 8 313 |
| `aform` | 1 800 s | 1 369 | 2 274 | 2 062 | 24 379 423 | 2 047 → 2 064 |
| `forms_data` | 1 800 s | 1 387 | 2 598 | 2 558 | 22 318 946 | 2 549 → 2 562 |
| `embed` | 1 800 s | 1 257 | 1 887 | 2 653 | 9 225 201 | 2 627 → 2 652 |

**No crash, no timeout and no memory refusal on any of the five**: every artefact directory is
empty and every libFuzzer exit was 0, so no regression test is owed and nothing is kept under
`fuzz/artifacts/`. ADR 1424's classes were not needed; none of these targets reaches a codec.

## 3. What this says and does not

`script` and `vfs_write` are slowing — 168 and 27 edges, 0.8 % and 0.3 %, in the last six minutes
— and not saturated. The hour's final figures sit below ADR 1694's ten-minute ones (20 819 and
8 401), which is a statement about two counts and not about the targets: that table was an
in-process run's running count on another tree, this one a pass over a kept corpus on today's, and
round 1429's finds are gone with its scratch, so no pass over them can be made. The finds here are
in `scratchpad/r1442/run/<target>/corpus` and were not merged into the owner's corpus (ADR 1423).

## 4. After: the census, and two things it found

The census re-run after the campaigns, on the tree slot 1 had moved since, read `vfs_write`,
`aform`, `forms_data` and `embed` current (5 968 / 5 969, 1 369 / 1 369, 1 404 / 1 302,
1 257 / 1 257) and **`script` not judged**: both passes stopped on the target's own clock, which
ADR 1717 is; after it, `script` reads current at 13 535 / 13 533. And slot 1 raised
`wire::VERSION` to 9, so `script_wire` was re-seeded by its arm (32 seeds, the old names
overwritten) and its census reads current at 710 / 710.
