#!/usr/bin/env python3
"""Review 1769's levers: what each saving would have saved, replayed over the transcripts as run.

    python3 -I -B doc/reviews/1769/levers.py <subagents-dir> <orchestrator.jsonl> [--rounds 1752-1767] [--since 1607]

It reads `tools/token-census.py`'s `Transcript` (and through it `tools/cache-census.py`'s prices) and
replays each round's request sequence with one thing changed, pricing reads and writes at the
round's model's list price. A replay is a bound, not a forecast: it moves tokens, never the work.

  base        the first request (system prompt, tool schemas, CLAUDE.md, memory, the prompt) as
              re-read by every later request
  cap         a round that reaches a context cap hands over: the next request starts from the first
              request's size plus a handover, and pays a new first write and ten re-orientation calls
  results     every tool result cut to a size, the cut carried by every later request
  waits       the requests after a wait or a poll, and what they read
  start-set   the brief, the prompt file and `tools/round.sh`'s list as read in the first forty calls
  clear       the orchestrator's context reset to a handover's size at each batch's first launch
"""

import argparse
import importlib.util
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
TOOLS = os.path.join(HERE, "..", "..", "..", "tools")
spec = importlib.util.spec_from_file_location("token_census", os.path.join(TOOLS, "token-census.py"))
tc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tc)

HANDOVER = 20_000
REORIENT = 10
ORCHESTRATOR_HANDOVER = 65_000


def rates(model):
    """A model's read and five-minute write price per token, in dollars."""
    base, read_ratio = tc.PRICES.MODELS.get(model, (4.0, tc.PRICES.GENERAL_READ))
    return base * read_ratio / 1e6, base * tc.WRITE_FIVE_MINUTES / 1e6


def as_run(agent):
    """The input side as run: reads and writes at the round's model's price."""
    read, write = rates(agent.requests[0]["model"])
    return sum(r["read"] for r in agent.requests) * read + sum(r["write"] for r in agent.requests) * write


def capped(agent, cap):
    """The input side had the round handed over each time its context passed `cap`."""
    read, write = rates(agent.requests[0]["model"])
    rs = agent.requests
    base, offset, cost, handovers = rs[0]["context"], 0, 0.0, 0
    for i, r in enumerate(rs):
        context = r["context"] - offset
        if context > cap:
            handovers += 1
            offset = r["context"] - (base + HANDOVER)
            context = base + HANDOVER
            cost += (base + HANDOVER) * write + REORIENT * (base + HANDOVER + 15_000) * read
        grown = rs[i + 1]["context"] - r["context"] if i + 1 < len(rs) else 0
        cost += context * read + max(grown, 0) * write
    return cost, handovers


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("subagents")
    parser.add_argument("orchestrator")
    parser.add_argument("--rounds", default="1752-1767")
    parser.add_argument("--since", type=int, default=1607)
    args = parser.parse_args()
    low, high = (int(x) for x in args.rounds.split("-"))
    agents = [a for a in (tc.Transcript(p) for p in tc.transcripts([args.subagents]))
              if a.session and low <= a.session <= high and a.requests]
    total = sum(as_run(a) for a in agents)
    reads = sum(r["read"] for a in agents for r in a.requests)
    print(f"{len(agents)} rounds {args.rounds}: ${total:.2f} input side as run, {reads / 1e6:.1f} M read")
    base = waits = start_carried = 0.0
    excess = {2000: 0.0, 4000: 0.0, 8000: 0.0}
    for a in agents:
        a.measure_results()
        rs = a.requests
        base += rs[0]["context"] * (len(rs) - 1)
        tools = [t for r in rs for t in r["tools"]]
        for t in tools:
            if t["chars"] and t.get("tokens"):
                for size in excess:
                    if t["chars"] > size:
                        excess[size] += (t["chars"] - size) * t["tokens"] / t["chars"] * a.carriers(t["request"])
            if t["wait"] and t["request"] + 1 < len(rs):
                waits += rs[t["request"] + 1]["read"]
        start_carried += sum(t.get("carried", 0) for t in tools[:40] if tc.START_SET.search(t["label"]))
    share = lambda tokens: f"{tokens / 1e6:.1f} M, {100 * tokens / reads:.0f} % of reads"
    print(f"base: the first request re-read {share(base)}")
    print(f"start-set: {share(start_carried)}")
    print(f"waits: the requests after them read {share(waits)}")
    for size, tokens in excess.items():
        print(f"results cut to {size} characters: {share(tokens)} fewer")
    for cap in (150_000, 200_000, 250_000, 300_000):
        cost, handovers = zip(*(capped(a, cap) for a in agents))
        print(f"cap {cap // 1000}k: ${sum(cost):.2f} ({100 * (1 - sum(cost) / total):.0f} % less), {sum(handovers)} handovers")
    orchestrator = tc.Transcript(args.orchestrator)
    starts = []
    for index, session, batch in orchestrator.launches:
        key = batch or session
        if key not in [k for _, k in starts]:
            starts.append((index, key))
    read, _ = rates("claude-fable-5-1")
    run = cleared = 0.0
    spans = 0
    for n, (begin, key) in enumerate(starts):
        if key < args.since:
            continue
        end = starts[n + 1][0] if n + 1 < len(starts) else len(orchestrator.requests)
        part = orchestrator.requests[begin:end]
        if not part:
            continue
        spans += 1
        first = part[0]["context"]
        for r in part:
            run += r["read"]
            # a compaction inside the span has already reset the context below its start
            cleared += r["context"] - first + ORCHESTRATOR_HANDOVER if r["context"] >= first // 2 else r["context"]
    print(f"orchestrator, {spans} batches from {args.since}: {run / 1e6:.0f} M read (${run * read:.0f}); "
          f"cleared to {ORCHESTRATOR_HANDOVER // 1000}k at each batch's first launch {cleared / 1e6:.0f} M "
          f"(${cleared * read:.0f}), {100 * (1 - cleared / run):.0f} % fewer")


if __name__ == "__main__":
    main()
