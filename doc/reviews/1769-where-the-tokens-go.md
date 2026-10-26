# 1769 — Where the tokens go, and how to spend fewer

Status: **review**, a record that is never rewritten. Session 1769, a "script" round run beside
batch one hundred and sixteen. It answers the owner's request of 2026-10-26: "I have looked at my
usage and it said, that >80% of my usage come from big context. Please use a round to evaluate, if we
can improve context size. we might even should clear this context between batches. Are there reasons
not to? (AFAIK everything should be documented anyways). Let the round also look for other ways to
save tokens." The decision is ADR 2372; the owner's part of it is Q635. **It builds nothing in
`crates/`**: its instrument is `tools/token-census.py`, its replay `doc/reviews/1769/levers.py`.

The data is the harness's transcripts: one JSONL line per content block, the request's `usage` on
each `assistant` line. I read the orchestrator session's own transcript and its `subagents/`
directory beside it, and I worked in `scratchpad/r1769/`. Batch 115 is sessions 1752–1759 (finished);
batch 116 is 1760–1767, read while 1760–1765 were still running, so its figures are a floor. Prices
are `tools/cache-census.py`'s `MODELS` (Opus 5.5 $4 per million input tokens, a read at 0.05 of
that; Sonnet 5.5 $2, 0.05; Fable 5.1 $10, 0.025; a 5-minute write 1.25 times, a one-hour write 2) —
the input side only, because a transcript records the output count its stream opened with.

## 1. Measured

### 1.1 The rounds: `python3 -I -B tools/token-census.py <subagents-dir> --rounds 1752-1767`

| round | tools | written | read | last context | >200k requests | their share of $ | $ input | waits |
|---|---|---|---|---|---|---|---|---|
| 1752 | 146 | 264k | 24.2 M | 264k | 56 | 48 % | 6.16 | 24 |
| 1753 | 185 | 255k | 30.4 M | 255k | 74 | 50 % | 7.35 | 37 |
| 1754 | 202 | 375k | 45.2 M | 375k | 119 | 73 % | 10.91 | 16 |
| 1755 | 287 | 450k | 83.0 M | 450k | 229 | 88 % | 18.85 | 50 |
| 1756 | 121 | 239k | 18.9 M | 239k | 36 | 36 % | 4.98 | 12 |
| 1757 | 211 | 323k | 42.3 M | 323k | 114 | 66 % | 10.07 | 46 |
| 1758 | 258 | 405k | 63.1 M | 405k | 165 | 78 % | 14.65 | 36 |
| 1759 (Sonnet) | 65 | 150k | 7.1 M | 150k | 0 | 0 % | 1.09 | 13 |
| **batch 115** | **1 475** | **2.46 M** | **314.1 M** | | **793** | **69 %** | **74.06** | **234** |
| batch 116, floor | 1 149 | 2.10 M | 204.7 M | | 524 | 57 % | 50.53 | 134 |

`write` equals the last context to within a few thousand tokens in every round: the cache was written
once, as the context grew, and never again. **The harness's "subagent_tokens" is the last context
plus the final turn's output**, not what a round cost: 1757's 328k against a last context of 323k,
1748's 528k against 522k, 1767's 142k against 140k. What 1757 cost is 42.3 M tokens read and 0.32 M
written — 130 times the figure the hand-back prints.

`--detail` prints the context every 25 tool calls. It grows by a median 1.53k tokens per call from a
first request of 39k (40k for Sonnet), linearly, so a round's reads are quadratic in its calls:

| round | 0 | 25 | 50 | 75 | 100 | 150 | 200 | 250 | end |
|---|---|---|---|---|---|---|---|---|---|
| 1755 | 39k | 111k | 192k | 229k | 259k | 320k | 375k | 419k | 450k (287) |
| 1757 | 39k | 98k | 133k | 177k | 202k | 257k | 313k | | 323k (211) |
| 1767 (Sonnet) | 40k | 96k | 133k | | | | | | 140k (59) |

### 1.2 The orchestrator: `python3 -I -B tools/token-census.py <session.jsonl> --spans`

4 459 requests on Fable 5.1 since 2026-09-22: 2 222 M read, 29.1 M written at the hour, **$1 139 on
the input side, of which 94 % in requests whose context passed 200k**. Ten compactions, each at
967k–978k (two at 218k and 823k), each to 5k–40k. A batch adds 70k–140k tokens (the spans' end
contexts: 1704 300k, 1712 423k, 1720 549k, 1728 698k, 1736 794k), so a batch run late in a cycle
re-reads 700k–950k per request; the seventeen batches from 1607 average **$12.0** each. What fills
it, since the compaction of 2026-10-21: tool results 644k characters (the chain's and the merge's
logs, `git` and `cargo` output), round reports delivered as queued notifications 527k, other
messages 489k — about 100k tokens a batch, of which the next batch needs the reports' substance,
which is on disk in the batch's reports file, and the owner's words.

### 1.3 The cache's misses: `python3 -I -B tools/cache-census.py --keep-warm <transcripts>`

| rounds | requests | written | after a gap past the lifetime | keep-warm requests |
|---|---|---|---|---|
| 1490–1569, before ADR 1976 | 14 558 | 214.3 M | 683, wrote 186.3 M (87 % of all writing) | 0 |
| 1571–1751, after | 26 526 | 48.7 M | 18, wrote 1.5 M (3 %) | 712, read 215.0 M |
| 1752–1767 | 2 591 | 4.6 M | **0** | 87 (3.4 %), read 24.2 M |

**The four-minute wait did what the owner meant it to**: the round cache has not lapsed once in
batches 115 and 116, and a batch writes 2.1–2.5 M where batches 1490–1564 wrote 10–25 M. The 87
keep-warm requests cost 1.34 M equivalents against 13.47 M for the whole-context writes they avoided.
The orchestrator since 2026-10-12: 2 064 requests, 11 gaps past its hour wrote 4.5 M — 52 % of its
writing — each ended by the owner's prompt (6), a notification (3) or the harness (2).

### 1.4 The levers: `python3 -I -B doc/reviews/1769/levers.py <subagents-dir> <session.jsonl>`

Over the sixteen rounds, $125.83 and 524.7 M read as run: the first request re-read by every later
one is **101.9 M (19 %)**; the brief, the prompt file and `tools/round.sh`'s list as read in the first
forty calls 26.8 M (5 %); the requests after a wait 97.7 M (19 %); tool results cut to 4 000
characters would carry 96.1 M fewer (18 %), to 2 000 172.1 M (33 %). A round handed over at a 200k
context, with a 20k handover and ten re-orientation calls, would have cost $87.25 (31 % less; 17
handovers); at 150k $84.50, at 250k $99.72, at 300k $107.68. The orchestrator cleared to 65k at each
batch's first launch would have read 137 M rather than 452 M over the seventeen batches (70 % fewer,
$113 → $34).

## 2. Findings

1. **A round's cost is its calls times its context, and the long rounds carry it.** The four rounds
   past 200 calls (1754, 1755, 1757, 1758) are 74 % of batch 115's $74; the Sonnet slot's 65 calls
   cost $1.09, and an Opus round of 121 calls (1756) $4.98. Price per token explains a factor of two
   between them; the rest is the count of calls, squared.
2. **"Big context" is the owner's 80 % and it is real**: 69 % of batch 115's round input and 94 % of
   the orchestrator's is in requests above 200k. Per batch, roughly $74 of round input, $14 of round
   output (the harness's own `cost-state` ratio of output to reads, 0.22 %), $12 of orchestrator input
   and $2 of its output — about $100, nine tenths of it the rounds.
3. **The fixed first request is a fifth of every round's reads.** 39k tokens — the harness's system
   prompt and tool schemas, `CLAUDE.md` (29 674 characters), the memory index (5 669), the prompt —
   re-read by every request. The prompt's boilerplate is about 0.7k of it.
4. **The largest results are file reads, not test output.** The brief's premise — a 2 000-line
   `cargo test` read for one line — did not hold: the rounds already `tail` and `grep` their tests.
   Of results over 4 000 characters, 297 were `sed -n`/`cat` of a file (152.8 M carried later), 65
   `grep` (26.0 M), 39 everything else (17.4 M). The ten largest were 20–31k characters: a 600-line
   `sed -n` of `render-cpu/src/shading.rs`, a whole `cat` of `image/convert.rs`, a 240-line range of
   `doc/performance.md`, a `WebFetch`, the brief read as `head -300`.
5. **The waits are cheap per call and dear in sum, and the rule is right.** 365 of 2 612 calls (14 %)
   wait; the requests after them read 19 %. A wait re-issued inside the lifetime costs one read,
   0.05 C on Opus for a context C, plus a poll's ~1.3k equivalents; a lapse costs a write of C, 1.25 C,
   in place of that read. **Break-even**: at C = 300k a poll is 16.3k equivalents and a lapse's excess
   360k, so 22 polls — **about 88 minutes** of 240-second polls; at C = 100k, 19 polls, 77 minutes.
   A Bash call cannot block past 600 s, so a round that must stay alive always polls more cheaply
   than it lapses. The one cheaper move for a long wait is to hold a smaller context while waiting.
   On the orchestrator's hour (0.025 C a read, 2 C a write) the break-even is 70–79 hourly polls — about
   three days — so its cache is always worth keeping warm while it is working.
6. **The orchestrator re-reads its whole campaign every turn.** Its context at a batch's end is the
   last two to eight batches; clearing it to a handover at each batch start would cut its reads by
   70 %, about $4.6 a batch, and make each hour-lapse a write of ~100k rather than 300k–950k.
7. **The harness's summary undercounts by two orders**, so a round cannot see its own cost; the
   census is the command that can, and a round can run it on its own transcript.

## 3. Recommendations, ranked by dollars saved per batch

| # | lever | per batch | decided in |
|---|---|---|---|
| 1 | a slot sized to finish inside ~150 calls; a contract past that is two sessions in sequence with a handover file, and a round past ~200k context with a quarter of its work ahead hands over | ~$19–23 (31 %) at a 200k cap | ADR 2372 rule 1 |
| 2 | a read is a range: `grep -n` first, `sed -n` of at most ~150 lines, an output that may pass 4 000 characters written to a file and read by `grep`/`tail` | ~$5–11 (half to all of the 18 %) | rule 2 |
| 3 | clear the orchestrator between batches, after a handover file | ~$6.5 (reads $4.6, hour-lapses ~$2) | Q635 |
| 4 | a round agent type with a tool allowlist, trimming the harness's part of the 39k | up to ~$3, unmeasured | Q635 item 2 |
| 5 | mechanical sub-tasks to a fresh Sonnet subagent (never `fork`, which inherits the context) | ~$1.4 per twenty-call task | rule 4 |
| 6 | a long wait spent on the round's other work, or its hold queued after the lock-free work | ≤ $2.4 (the keep-warm reads) | rule 3 |
| 7 | the brief's common part kept short | ~$0.3 per 1 000 tokens cut | rule 5 |
| — | a shorter prompt section | ~$0.2: **rejected** as a saving | — |
| — | polling less, or longer than 240 s | **rejected**: the lapse costs 1.2 C and the break-even is ~80–90 minutes | — |

## 4. Clearing the orchestrator between batches

**For.** Everything a batch needs is on disk: the brief, the prompts, the reports file, the
placement script, the commit message, the chain script, the memory files and the running log, and
`doc/orchestrator-handover.md`. The compaction already drops the history at ~970k; a clear at each
batch start drops it at ~100k, earlier and at a moment chosen, not at whatever turn crosses the line.
A compaction's summary is written by the model about itself under pressure; a handover file is
written deliberately and can be checked.

**Against, weighed.** (a) **In-flight state lives only in the session**: a report is delivered to
the session that launched the round, a cleared session cannot `SendMessage` a round it did not launch,
and a background wait is lost. A clear is safe only when nothing runs — after the merge and
`tools/batch.sh close`, before the next launch. (b) **The owner's mid-batch words** that are not yet
in a memory file are lost; the handover must carry them verbatim. (c) **Judgement re-derived from
files costs turns**: which reds are the machine's, what a sibling edited, which candidate failed its
premise; written down, it is one file read, about 10–20k tokens once. (d) The first batch after a
clear starts with no felt sense of the last; the reports file is the substitute, and it exists.
(e) A clear forfeits the cached prefix: the first request writes ~50k at the hour, about $1 — less
than six turns' reads at 700k ($0.18 each), and a batch has fifty.

**What the handover must hold that memory does not today**: the owner's words since the last memory
write, verbatim; the reds the merge called the machine's, by test name; the next batch's candidates
with the premise each was checked against; the numbers already assigned (sessions, ADRs, `Q`s);
anything still running — which must be nothing. **Recommendation: clear**, by the rule in ADR 2372
section 3, once Q635 is answered.

## 5. What did not hold

- The brief's 1 100–1 300 tokens of prompt boilerplate: a prompt section is 3 660–3 870 characters,
  about 1k tokens in all; cutting it saves cents.
- The brief's "2 000-line `cargo test` output": none in the two batches' ten largest results per
  round; finding 4.
- `tools/round.sh script` lists the JavaScript-engine reading — a "script" round there is an RFC 0008
  round — and refuses `ROUND_SESSION=1769` on this branch, whose brief has no slot for it; I read its
  every-round list, which it printed without the variable.
