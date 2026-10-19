# 1976 — A long command is waited for in calls inside the cache's lifetime

Session 1570, slot 7 of batch ninety. Status: **accepted, built**. Context: the owner, 2026-10-19:
"evaluate if we could (easily) improve input caching for llm requests. (I would like to avoid, that we
lose the cached token calls, because commands -- i am thinking about our compilation and tests -- take
too long, and we don't send anything while they run …)". Code: `tools/wait.sh`, `tools/cache-census.py`,
`tools/conformance/tests/wait.rs`. ADR 1977 puts the rule into the round's contract. It cites no clause
and moves no ledger row.

## 1. What the transcripts say

`tools/cache-census.py` reads a session's JSONL transcripts and prints aggregates; the figures below
are its output on 2026-10-19 over the 48 round transcripts the orchestrator's task links still
resolve (`readlink -f …/tasks/a*.output`), and in brackets over all 445 subagent transcripts of the
session. Every round ran on Claude Opus 5.5 and wrote its cache at the five-minute lifetime.

- **7 945 requests wrote 103.3 M tokens to the cache and read 1 680.9 M** [64 953; 663.5 M; 14 322 M].
- **332 requests — 4.2 % — followed a gap over 300 s, and they wrote 88.2 M tokens: 85 % of all the
  writing** [1 968, 3.0 %; 525.8 M, 79 %]. The median gap is 8.5 s; 84 exceeded 600 s [909].
- **A gap past the lifetime is a whole miss.** `--buckets`, by gap: no miss among the 7 457 requests
  after a gap under 300 s, 1 of 12 in [300, 330) s, 21 of 25 in [330, 400) s, and all 295 past 400 s.
  The edge sits where Anthropic's caching page puts it — the lifetime "is measured from the start of
  the request that writes or reads the cache entry" — which is why the census takes the gap from one
  request's send to the next's; a miss rewrites the whole context, 177 000 to 433 000 tokens in the gaps listed.
- **The brief's premise held in direction and was low in size.** It counted 13 745 "requests" and
  169 M written, but a request is written as several records that repeat one `usage`; counted once
  per `requestId` the writing is 103 M and the long gaps' share 85 %, not 52 %. Its 0.1× for a read
  is the general rate; Opus 5.5's is 0.05× (below).

## 2. What the gaps were

The hypothesis was that the long gaps are `tools/bounded.sh` waits. **They are waits on bounded runs,
but the wait is the round's own loop, sized to the harness's ceiling.** Of the 332: 219 (61.0 M) came
after a wait-loop the round wrote itself — `timeout 590 bash -c 'until grep -q "^bounded: exit" …'`,
`for i in $(seq 1 57)` — and 95 (23.1 M) after a foreground `tools/bounded.sh`, `--clock` and `--long`
among them; 242 of the 332 lasted 500–615 s. The Bash tool's ceiling is 600 s, the rounds wrote their
loops to it, and every loop that ran to its end outlived the cache by five minutes. The two gaps that
followed no command (a resume, a hand-back) are not waits and nothing here reaches them.

## 3. The price, and the polled alternative

Anthropic's pricing page, in units of a base input token (Opus 5.5: $4 / MTok): a five-minute write
1.25×, a one-hour write 2×, "Cache hits and refreshes on Claude Opus 5.5 and Claude Sonnet 5.5 are
priced at 0.05x the base input price", output 5×. The cache cost the 48 rounds **213.2 M equivalents**
in all; the 330 gaps after a command cost **110.0 M** of it. Waited for in calls of 240 s, each gap
becomes ⌈gap / 240⌉ requests, every one a read of the whole context at 0.05× plus a poll's 400 tokens
written and 150 generated: **15.0 M with 585 more requests, saving 94.9 M equivalents — 45 % of what
the cache cost the rounds** [519.0 M of 1 545.6 M, 34 %]. At $4 / MTok that is about $380 over the 48
rounds, $8 a round. A poll costs 0.05 × C + 1 250 against a miss's 1.2 × C, so for a 270 000-token
context polling wins on any wait shorter than about 88 minutes; no round's wait is that long. A
120 s wall would save 83.5 M, a 270 s one 95.2 M: the saving is flat past 180 s, and the wall is
chosen for the margin below.

## 4. The decision

- **A sibling, `tools/wait.sh`, and not `tools/bounded.sh --heartbeat`.** The wrapper's contract is
  that it holds the lock and the tree's bound for the whole of its command and ends with one line; a
  wrapper that returned early would have to detach itself, and every caller and test of its 1 591
  lines reads it as foreground. `wait.sh --start <log> -- <command>` runs any command — the wrapper
  included, which then holds its lock in the detached session as it would in the foreground — in a
  session and process group of its own, with its output and a closing `wait: exit <n> after <s>s`
  in `<log>` and its pid, group and start in `<log>.pid`.
- **Every call returns within its wall**, 240 s by default and refused past 270 s: the lifetime is
  counted from the start of the previous request, so the model's own turn sits inside the five
  minutes too, and the census shows no miss under 300 s. The exit status is the command's when it
  ended, 75 when the wall came first, 70 when the group is gone without its line (killed); the last
  line printed is always the verdict, so a command's own 70 or 75 is told apart.
- **The watcher is not the run.** A wait the harness's timeout kills leaves the command running, the
  next `tools/wait.sh <log>` picks it up, and `--stop <log>` ends the group the round started, which
  is the kill-by-group rule of `doc/environment.md` with the group written down for it.
- **A log the script did not start** — a harness background task's output, an older `> log &` — is
  waited for with `--until <ERE>`, or by default on `tools/bounded.sh`'s closing line.
- `tools/wait.sh --self-test` holds the cases, and `cargo test -p conformance --test wait` runs it.

## 5. What it costs, and what it does not reach

A round now issues a call every four minutes of a long command instead of one every ten, and reads
three lines of progress each time; that is the 585 requests priced above. The orchestrator writes its
cache at the hour and is not a round; ADR 1977 says why the rule stops at the rounds. Whether the
saving arrives is a count, not a claim: `tools/cache-census.py --since <the batch's start>` over the
next batch's transcripts prints it.
