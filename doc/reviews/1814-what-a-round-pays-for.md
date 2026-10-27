# 1814 — What a round pays for, and what it can stop paying for

Status: **review**, a record that is never rewritten. Session 1814, a review beside batch one hundred
and twenty-one, asked for by the owner on 2026-10-27: "please add one round for investigating
possible ways to save tokens (MD file compaction,... but also maybe unnecessary work in agents.
maybe it's more efficient to only compile and fix tests in the main thread after every round
finished, stuff like this. take a step back and look for not so obvious ways to reduce tokens and
work without compromising quality)". The decision is ADR 2462; the owner's part is Q680; ADR 2463
is unused. **It builds nothing in `crates/`**: its instrument is `doc/reviews/1814/work.py`, beside
review 1769's `tools/token-census.py`, `tools/cache-census.py` and `doc/reviews/1769/levers.py`.

The data is the harness's transcripts of batches 117–121 (sessions 1771–1813): the 43 rounds, the
five batch orchestrators, the twelve Sonnet sub-tasks the rounds launched, and the top-level session.
I worked in `scratchpad/r1814/`, with the transcripts linked into directories of their own as
review 1769 found the census needs. Batch 121 was read while four of its rounds still ran, so its
figures are a floor. Prices are `tools/cache-census.py`'s (Opus 5.5 $4 per million input tokens, a
read 0.05 of that, a five-minute write 1.25 times; Fable 5.1 $10, a read 0.025, an hour write 2),
the input side only.

## 1. Measured

### 1.1 Since ADR 2372: `python3 -I -B tools/token-census.py <rounds-dir> --rounds 1771-1813`

| batch | calls | $ input, rounds | Opus rounds: mean calls | median last context | past 200 calls | past 200k |
|---|---|---|---|---|---|---|
| 115 (review 1769) | 1 475 | 74.06 | 201 | 323k | 4 of 7 | 7 of 7 |
| 117 | 1 197 | 53.72 | 165 | 257k | 2 of 7 | 5 of 7 |
| 118 | 1 299 | 63.07 | 157 | 273k | 1 of 8 | 7 of 8 |
| 119 | 1 319 | 55.63 | 152 | 284k | 2 of 8 | 6 of 8 |
| 120 | 1 479 | 65.83 | 185 | 292k | 1 of 8 | 8 of 8 |
| 121, floor | 1 432 | 64.96 | 179 | 293k | 2 of 8 | 8 of 8 |

The rounds cost **$60.6 a batch against batch 115's $74.06**, 18 % less, at $0.042–0.049 a call
against $0.050. What moved is the count of long rounds (8 of 39 past 200 calls, against 4 of 7);
what did not is the context at hand-back, a median of 282k. **ADR 2372 rule 1's handover never
fired**: 34 of 39 Opus rounds passed 200k and none handed over, because nothing tells a round its
context — `token-census.py` was run by one round in 43. Rule 2 barely moved: results over 4 000
characters are 1 004 in 43 rounds, 23 a round, against review 1769's 401 in 16, 25 a round.
Rules 3 and 4 hold: `tools/cache-census.py --keep-warm` over the rounds finds 2 lapses in 6 584
requests and keep-warm requests worth 2.91 M equivalents against 32.46 M of writes avoided; the
twelve Sonnet sub-tasks cost $2.70 in all (`token-census.py` over their transcripts).
**Q635's item 2 never ran**: `work.py <dir> --first` finds all 42 rounds and five orchestrators on
`general-purpose`, because the `round` type is not registered in the session that launches them.

`levers.py <rounds-dir> <orchestrator.jsonl> --rounds 1771-1813` replays the same levers: the first
request re-read is 21 % of reads, the start set 7 %, the requests after a wait 18 %; results cut to
4 000 characters 15 % fewer reads, to 8 000 4 %; **a 200k cap $221.38 against $300.56 as run, 26 %
less, with 40 handovers**; a 150k cap 30 % with 76.

### 1.2 What the context holds: `work.py <rounds-dir> --fit`

A least-squares fit over 6 542 consecutive request pairs, of the context's growth on the characters
added between them: **results 51.6 % of the growth** (0.382 tokens a character), tool inputs and
text 18.8 % (Edit and Write bodies, commands), **thinking 23.3 %** (0.258 tokens per signature
character), attachments 3.2 %, a request's own overhead 3.1 %. The first request is fixed at
39.7–40.3k for every `general-purpose` agent and 13.7k for the one `Explore` agent; the
instructions attachment (`CLAUDE.md` and the memory index, 37 094 characters, ~10.2k tokens) is in
the one and not the other, so **~15.8k of the 40k is tool schemas and harness text** that a type
with an allowlist can drop part of — the skill listing alone is 18 846 characters in every round.

### 1.3 What the calls were for: `work.py <rounds-dir> --by-round`

Each request's cost — its fixed first context read once, and what it added read by every later
request and written once — split over its calls, sums to $291.00 against the census's $300.56:

| family | calls | $ (5 batches) | share | wall |
|---|---|---|---|---|
| code read and search | 2 545 | 133.99 | 46.1 % | 0.8 h |
| Markdown read | 1 064 | 64.82 | 22.3 % | 0.3 h |
| build and test (tier 1, tests, conformance, exports, their waits and logs) | 1 525 | 38.51 | 13.2 % | 24.4 h |
| measure, drive, hold, census | 1 140 | 36.20 | 12.4 % | 17.1 h |
| edit and write | 109 | 7.49 | 2.6 % | |
| git | 81 | 2.67 | 0.9 % | |

Within the Markdown: the brief, prompt and role $14.6, `doc/md/` ranges $11.7, `doc/todo/` $9.6,
trap and habit files $7.9 (`doc/traps/every-round.md` read whole 29 times), environment, handover
and plan $4.1, `tools/round.sh` lists $1.6, `CLAUDE.md` re-read 34 times $1.4 — a file already in
every request's context. Within build and test, `work.py --where`: 1 138 calls and $28.57 (21.7 h)
in the shared tree, **388 calls and $9.97 (2.7 h) in a round's own export or copy, in 38 of 43
rounds**. Rounds started `cargo test -p conformance` 3.9 times each.

### 1.4 Who finds the defects: `work.py <rounds-dir> --reds`, and the reports

Of 1 195 build-and-test results the rounds read, **128 failed naming a file the round had written,
in 37 of 43 rounds** — its own gate finding its own defect — 125 failed naming only files it had not
written, in 35 rounds (a sibling's red or an old one), and 109 failed naming no file. The merges of
batches 117–120, read from the batch reports and handovers by a Sonnet sub-task: **two defects no
round had found** — the orchestrator's own placement typo (118), and eight unescaped rustdoc links
from two rounds that skipped rustdoc because the shared tree did not compile (120) — four a round
had flagged and left for the merge, and five that were the machine's (trap 280's docs walk under its
data limit twice, three clock gates under load, a CI race). Every merge's `gates --merge` and
`check` exited 0; tier 1 exited 1 three times, each on one of those.

### 1.5 The orchestration

The five batch orchestrators (Opus): $6.27–8.94 each for 166–200 calls; **batch 121's wrote 1.1 M
after 12 gaps of a mean 568 s** — its own wait loop ran past the five-minute lifetime, about $5.25
of its $6.99 (`tools/cache-census.py --keep-warm` over their transcripts). The top-level session
since batch 117: 50 requests, $22.19 on the input side, **$20.03 of it hour-writes after 6 gaps of
100–307 minutes** while a batch ran (`tools/cache-census.py --since 2026-10-26T12:00`). A round's
report is a median 6 254 characters (44 hand-backs, at most 8 023).

## 2. Findings

1. **The round is still the bill, and its context is what it pays for**: about $61 of a batch's
   input, against ~$12 for the two orchestrators. A handover at 200k is still the largest lever
   (26 %, replayed), and it is unused only because its trigger is invisible to the round.
2. **An approved lever has waited five batches on a registration.** A635 approved the allowlisted
   type "measured first"; every round since has run as `general-purpose`, at ~40k a request.
3. **Compiling and testing in the round is not the waste.** Rounds' gates found 128 own failures to
   the merges' two; at the merge each would be fixed by a session that did not write the code, in a
   context that carries the fix to the batch's end. What is repeated is the shared tree's breakage:
   125 failures from files a round did not touch, and $10 of exports to get past them.
4. **Thinking is a quarter of what accumulates.** It is the setting the owner names (Q680).
5. **The Markdown is a fifth, and most of it is the work** — clause text, todo files, the ledger. The
   instruction reading a rule can cut is the duplicated part: `CLAUDE.md` read again, the brief's
   restatement of traps, a trap file read whole.
6. **The keep-alive rule broke where nobody priced it**: an orchestrator polling at 568 s, and a
   top-level session idle past its hour, together ~$9 in batch 121 and ~$4 a batch before it.

## 3. Rules, ranked by dollars saved per batch (ADR 2462)

| # | rule | per batch | risk to quality |
|---|---|---|---|
| 1 | the orchestrator runs the census over the batch's linked round transcripts at each poll and tells a round past 200k to hand over if a quarter of its work is ahead; the second session opens on the handover | ≤ $15.8 replayed, ~$8 expected (8 handovers) | judgement lost at a handover; the handover's five items carry it |
| 2 | the batch-122 session is a fresh one (A635's clear), so `round` registers; rounds launch as `round` and the first request is compared | $2.6–4.2 (10–16k a request × 1 312 requests) | none: the same tools a round uses |
| 3 | no gap past the cache's lifetime in an orchestrator: a subagent polls at ≤ 240 s, the top-level session wakes itself before the hour while a batch runs | ~$3.7 (6 lapses in 5 batches at ~$3.1) | none |
| 4 | a read over 8 000 characters is two ranges | ~$2.0 (4 % of reads, replayed) | none |
| 5 | `cargo test -p conformance` once before the report, `--test <name>` while iterating | ~$0.6 and ~0.3 h of the machine | a gate found later inside the round |
| 6 | the brief's discipline paragraph names traps and habits by number; their text stays in their files | ~$0.5 | none: the rule's text and reason stay |
| 7 | `tools/round.sh` stops listing `CLAUDE.md` as a read: it is in every request already | ~$0.3 | none |

Together about **$17–20 a batch expected**, of a batch's ~$85 (rounds' input $61, orchestration
$12, output ~$10 by review 1769's ratio); rule 1 alone is half of it.

## 4. Rejected

- **Compile and test only in the orchestrator, after the rounds** (the owner's lead): it would move
  ~25 own defects a batch (1.4) to a session that did not write them, whose context would carry each
  diagnosis to the end — at ~8 calls of ~3k each, ~600k a batch, a compaction — and it would
  serialise the fixes after the last round. The saving it buys is under the $7.7 a batch build and
  test cost in all, because the round still has to compile what it wrote. Rule 5 takes the repeat.
- **A git worktree per round** (`isolation: "worktree"`): it would end the 125 sibling failures and
  the $2 a batch of exports, at eight cold builds at once on a machine whose memory has failed before
  (`doc/environment.md`) and a real merge of `doc/conformance/ledger.toml`; unpriced, so not taken.
- **Shorter reports**: ~$0.8 a batch carried in all, and the reports file needs their substance.
- **Polling less**: the rounds' cache lapsed twice in 6 584 requests; review 1769's break-even stands.
- **More Sonnet sub-tasks by rule**: twelve cost $2.70; the rule exists, and a task is mechanical or
  it is not.
- **Compacting the trap and habit files**: the every-round pair is 4.6k and 3.3k characters; what
  costs is reading the trap a slot needs, which is the work.
- **Lowering the rounds' effort**, and **moving text out of `CLAUDE.md`**: the owner's (Q680).

## 5. What did not hold

- The brief's premise that every round builds and tests twice: 38 of 43 did build in an export, but
  the export is $10 of the $38.5, and the shared tree's runs are mostly the round's own iteration.
- "Each round reads the brief's common part (~25 kB)": the common part is 15.9k characters; rounds
  read 15–22k of the brief, their own slot included.
- `tools/round.sh script` refuses `ROUND_SESSION=1814` on this branch, whose brief has no slot for
  it, as it refused 1769's; I read its every-round list, printed without the variable.
