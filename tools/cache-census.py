#!/usr/bin/env python3
"""Count what an agent's prompt cache lost to the gaps between its requests, and price it.

    python3 -I -B tools/cache-census.py [options] TRANSCRIPT.jsonl...

(`-B`, because `-I` ignores `PYTHONDONTWRITEBYTECODE`, and a module imported from `tools/` would
leave a `tools/__pycache__` that the workspace glob `tools/*` reads as a crate: trap 114.)

A transcript is a Claude Code session's JSONL file — a round's is the file its harness task
output links to, `readlink -f /tmp/claude-<uid>/<project>/<session>/tasks/a*.output`. They are
megabytes each, and this script is the way to read them: it prints aggregates and never a record.

Each assistant request is one `requestId`, written as several records that repeat one `usage`;
it is counted once. Its **send time** is the timestamp of the last non-assistant record before
it (the tool result it answers), and the **gap** before it is its send time less the previous
request's: the time the cached prefix sat unread. The cache's lifetime is "measured from the start
of the request that writes or reads the cache entry" (Anthropic's prompt-caching page), which is
why the gap is taken send to send. `--buckets` prints the miss rate by gap, which is how the
lifetime this script assumes is checked against the transcripts themselves. `--since` and
`--until` close the window on a batch's span, its open and the next batch's open, each a merge
commit's `%cI`.

The price is in base-input-token equivalents, from Anthropic's pricing page: a 5-minute cache write
is 1.25 times an input token, a 1-hour write 2 times, and a hit 0.05 times on Claude Opus 5.5
(`--read 0.1` for a model without that footnote). The polled alternative replaces each gap after a
command by one request every `--wall` seconds, each reading the whole context and writing a poll's
call and answer; ADR 1976 has the figures and the reasoning.
"""

import argparse
import json
import math
import re
import statistics
import sys
from collections import Counter, defaultdict
from datetime import datetime

POLL_CREATED = 400  # one poll's tool call and its few-line answer, written to the cache
POLL_OUTPUT = 150  # one poll's tool call, generated
OUTPUT_PRICE = 5.0  # an output token, in input tokens (Opus 5.5: $20 against $4 per MTok)
NEW_TAIL = 2000  # the tool result a request after a wait adds whether or not the cache held


def seconds(stamp):
    return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()


def requests(path):
    """The transcript's requests in order, each with its send time, usage and tool calls."""
    ordered, by_id, last_other = [], {}, None
    with open(path, encoding="utf-8", errors="replace") as lines:
        for line in lines:
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            if "timestamp" not in record:
                continue
            when = seconds(record["timestamp"])
            if record.get("type") != "assistant":
                last_other = when
                continue
            message = record.get("message", {})
            key = record.get("requestId") or message.get("id")
            if key not in by_id:
                by_id[key] = {
                    "send": when if last_other is None else last_other,
                    "usage": {},
                    "tools": [],
                    "model": message.get("model"),
                    "stamp": record["timestamp"],
                }
                ordered.append(by_id[key])
            request = by_id[key]
            if message.get("usage"):
                request["usage"] = message["usage"]
            for block in message.get("content") or []:
                if isinstance(block, dict) and block.get("type") == "tool_use":
                    request["tools"].append((block.get("name"), block.get("input") or {}))
    return ordered


def what_waited(tools):
    """A short name for what the request before a gap had asked to run."""
    if not tools:
        return "no tool: a turn's end, a hand-back or a resume"
    kinds = set()
    for name, given in tools:
        if name != "Bash":
            kinds.add(name)
            continue
        command = given.get("command", "")
        if "wait.sh" in command:
            kind = "tools/wait.sh"
        elif "bounded.sh" in command:
            kind = "tools/bounded.sh" + next(
                (" " + flag for flag in ("--clock", "--long") if flag in command), "")
        elif re.search(r"\bcargo\b", command):
            kind = "cargo, unbounded"
        elif re.search(r"\buntil\b|\bwhile\b|\bsleep\b", command):
            kind = "a wait-loop of the round's own"
        else:
            kind = "another command"
        kinds.add("Bash: " + kind + (" [background]" if given.get("run_in_background") else ""))
    return " + ".join(sorted(kinds))


def tokens(request):
    usage = request["usage"]
    return (usage.get("cache_creation_input_tokens") or 0,
            usage.get("cache_read_input_tokens") or 0,
            usage.get("cache_creation") or {})


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("transcripts", nargs="+")
    parser.add_argument("--ttl", type=float, default=300, help="the cache's lifetime, s")
    parser.add_argument("--wall", type=float, default=240, help="the polled alternative's wall, s")
    parser.add_argument("--write", type=float, default=1.25, help="a cache write's price")
    parser.add_argument("--read", type=float, default=0.05, help="a cache hit's price")
    parser.add_argument("--since", help="count only requests sent at or after this ISO time")
    parser.add_argument("--until", help="count only requests sent before this ISO time")
    parser.add_argument("--worst", type=int, default=5, help="transcripts listed by their loss")
    parser.add_argument("--buckets", action="store_true", help="the miss rate by gap")
    args = parser.parse_args()
    since = seconds(args.since) if args.since else None
    until = seconds(args.until) if args.until else None

    total = Counter()
    preceded = defaultdict(Counter)
    gaps, per_file = [], []
    edges = [0, 60, 180, 240, 270, 300, 330, 400, 600, math.inf]
    bucket_n, bucket_miss = Counter(), Counter()
    before = after = 0.0
    newest = None
    for path in args.transcripts:
        lost_here = created_here = gaps_here = 0
        previous = None
        for request in requests(path):
            if newest is None or request["send"] > newest[0]:
                newest = (request["send"], request["stamp"])
            if since is not None and request["send"] < since:
                previous = request
                continue
            # A batch's census ends where the next batch's rounds begin, and an orchestrator's
            # transcript runs across both, so the window is closed on the right as well (habit 110).
            if until is not None and request["send"] >= until:
                break
            created, read, split = tokens(request)
            total["requests"] += 1
            total["created"] += created
            total["read"] += read
            total["created 1h"] += split.get("ephemeral_1h_input_tokens") or 0
            total["spend"] += args.write * created + args.read * read
            created_here += created
            if previous is not None:
                gap = request["send"] - previous["send"]
                gaps.append(gap)
                if created + read > 50000:
                    edge = next(i for i in range(len(edges) - 1) if gap < edges[i + 1])
                    bucket_n[edge] += 1
                    bucket_miss[edge] += created > (created + read) / 2
                if gap > args.ttl:
                    kind = what_waited(previous["tools"])
                    total["gaps"] += 1
                    total["gap created"] += created
                    preceded[kind]["gaps"] += 1
                    preceded[kind]["created"] += created
                    preceded[kind]["seconds"] += gap
                    lost_here += created
                    gaps_here += 1
                    if any(name == "Bash" for name, _ in previous["tools"]):
                        context = created + read
                        polls = math.ceil(gap / args.wall) - 1
                        before += args.write * created + args.read * read
                        after += (args.read * (context - NEW_TAIL) + args.write * NEW_TAIL
                                  + polls * (args.read * context + args.write * POLL_CREATED
                                             + OUTPUT_PRICE * POLL_OUTPUT))
                        total["command gaps"] += 1
                        total["polls"] += polls
            previous = request
        per_file.append((path.rsplit("/", 1)[-1], created_here, lost_here, gaps_here))

    # A census that counted nothing prints a page of zeros that reads as "no gap lost the cache";
    # it is a sentence about the arguments, so it is said as one and the exit is not 0 (trap 13).
    if total["requests"] == 0:
        held = f"the newest request they hold is stamped {newest[1]}" if newest else "they hold none"
        window = (f" at or after {args.since}" if args.since else "") + (
            f" before {args.until}" if args.until else "")
        print(f"cache-census: no request counted in {len(args.transcripts)} transcripts{window}; "
              f"{held}", file=sys.stderr)
        return 2

    mega = 1e6
    share = 100 * total["gap created"] / max(1, total["created"])
    print(f"{len(args.transcripts)} transcripts, {total['requests']} requests")
    print(f"cache written {total['created'] / mega:.1f} M tokens ({total['created 1h'] / mega:.1f} M "
          f"of them at the hour), read {total['read'] / mega:.1f} M")
    print(f"{total['gaps']} requests ({100 * total['gaps'] / max(1, total['requests']):.1f} %) followed "
          f"a gap over {args.ttl:.0f} s and wrote {total['gap created'] / mega:.1f} M tokens: "
          f"{share:.0f} % of all writing")
    if gaps:
        ranked = sorted(gaps)
        print(f"gap median {statistics.median(ranked):.1f} s, p90 {ranked[int(.9 * len(ranked))]:.0f} s, "
              f"p99 {ranked[int(.99 * len(ranked))]:.0f} s; over 600 s: {sum(g > 600 for g in ranked)}")
    print(f"\nwhat the request before a gap over {args.ttl:.0f} s had asked to run:")
    for kind, row in sorted(preceded.items(), key=lambda item: -item[1]["created"]):
        print(f"  {row['gaps']:5d} gaps, {row['created'] / mega:6.1f} M written after them, "
              f"mean gap {row['seconds'] / row['gaps']:6.0f} s  {kind}")
    print(f"\nthe {total['command gaps']} gaps after a command: {before / mega:.1f} M equivalents as "
          f"they were; polled every {args.wall:.0f} s, {after / mega:.1f} M with {total['polls']} more "
          f"requests — {(before - after) / mega:.1f} M saved, "
          f"{100 * (before - after) / max(1, total['spend']):.0f} % of the {total['spend'] / mega:.1f} M "
          f"the cache cost in all")
    # Ranked by what was lost and then by how often the cache was left past its lifetime: when
    # every gap was a hit, the count is the only column that names a round that waited too long.
    print(f"\nthe {args.worst} transcripts that lost most:")
    for name, created, lost, gaps_over in sorted(per_file, key=lambda row: (-row[2], -row[3]))[
            :args.worst]:
        print(f"  {name}: {created / mega:.1f} M written, {lost / mega:.1f} M after "
              f"{gaps_over} gaps over {args.ttl:.0f} s ({100 * lost / max(1, created):.0f} %)")
    if args.buckets:
        print("\nmiss rate by gap (requests over 50 000 tokens of context; a miss writes over half):")
        for i in range(len(edges) - 1):
            upper = "inf" if edges[i + 1] == math.inf else f"{edges[i + 1]:.0f}"
            print(f"  [{edges[i]:4.0f}, {upper:>4}) s: {bucket_n[i]:6d} requests, {bucket_miss[i]:5d} "
                  f"missed ({100 * bucket_miss[i] / max(1, bucket_n[i]):5.1f} %)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
