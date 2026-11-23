# 3469 — A walk nobody waits on holds its sequences to a minute, and a stop is counted and said

Session 2192. Status: **accepted** and built. Builds on ADR 1603 (the one-second sequence budget) and ADR 1625
(Tier 1's ceilings and floor); answers ADR 3454's "Cost, and what is left". Code: `ViewState::hold_sequences_to`,
`ViewState::sequences_stopped` and `report_stop` (`crates/pdf-model/src/view/scripts/state.rs`), the six stops in
`calculation.rs`, `script_sites.rs` and `form.rs`; `tests/support/synthetic_commit.rs`'s `WALK_SEQUENCE_BUDGET`;
`pdf-script`'s `tests/script_corpus.rs` (`unstopped`). Test: `pdf-model`'s `tests/sequence_budget.rs`.

## Context

ADR 1603 holds a commit's walk of Table 224's `/CO`, and every other sequence of scripts, to one second
(`MAX_SEQUENCE_TIME`), so that a person waiting on a commit reads a stop as one. Tier 1's figures moved with the
machine's load through `safedocs/…/5712688.pdf`: its 468 commits each walk an order whose scripts throw, at about
three quarters of a second a walk on this machine, and a loaded walk was stopped earlier and threw fewer (ADR 3454).
The stop's own sentence was not visible: the report keeps 256 sentences, and that document's throws fill it first.

## The three ways it could have gone

- **A budget counted in work.** The engine counts its own steps per run (`Budget::steps`), but a run's step count
  does not reach `pdf-model`: `ScriptResult` carries none, and adding a field breaks every struct literal of it,
  the owner's audit tests among them. And most of that document's time is outside the engine's steps — each run's
  request and realm update — so steps would bound the wrong thing. Not taken.
- **Stopped documents counted apart, the second kept.** Not enough alone: at the light loads measured below no
  sequence was stopped, and under the load ADR 3454 saw one was, so the document would move between the held
  figures and the apart column with the load, taking ~10 000 runs from the floor each time it moved.
- **The walk given a budget a load cannot reach, the stop still counted.** Taken.

## Decision

1. **The budget is the view state's, and a host may set it.** `ViewState::hold_sequences_to(budget)` replaces
   `MAX_SEQUENCE_TIME` for every sequence: the `/CO` walk, the open sequence, a document trigger's chain, an FDF
   file's scripts, an XFA activation. The viewer keeps the second; nothing a document says changes it.
2. **Both walks hold sequences to a minute** (`WALK_SEQUENCE_BUDGET`): a load must slow a walk sixtyfold to stop
   one the second lets finish. Principle 3's bound stays: each script is still held to `Budget::FIELD_EVENT`'s
   steps and time, a walk to `/CO`'s first 4 096 entries, and the walk to `tools/bounded.sh`.
3. **A stop is counted and said.** `ViewState::sequences_stopped` counts every stop; its sentence is kept past
   the 256-sentence cap, up to sixteen more, so the cap cannot hide why the values are what they are.
4. **Tier 1's runs keep `Budget::FIELD_EVENT`'s steps and take a clock sixty times its 100 ms**
   (`WALK_RUN_BUDGET`), the same ratio: the whole walk at this round's head stopped one run by that clock alone,
   in a minute when the machine could not answer `tools/bounded.sh`'s five-second sample.
5. **Tier 1 counts a stopped document apart**, naming it with its runs and throws; the throw and unparsed
   ceilings and the floor count the rest, the two zero ceilings every run.

## Measured

`5712688.pdf` alone through the walk's runner (`one_document`), three loads, the same answer each time — 9 988
runs, 8 911 throws, no sequence stopped:

| load | wall |
|---|---|
| unpinned, load average 30 to 40 | 360 s |
| pinned to one processor beside three busy loops at nice 19 | 400 s |
| pinned to one processor beside three busy loops at nice 0 | 2 372 s |

The third is six and a half times the first: its 468 commits average five seconds of walk each, so the second
would have stopped every one of them. The held documents' Tier 1 walk, unpinned and pinned to one processor:
21 162 runs, 9 315 throws, 8 unparsed, nothing stopped, both times.

## Cost

A gate that holds a sequence to a minute does not measure what a reader waiting on the second meets; that is
`MAX_SEQUENCE_TIME`'s own test's to say, and the stop at the second is still the viewer's.
