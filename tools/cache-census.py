#!/usr/bin/env python3
"""Count what an agent's prompt cache read, what it lost and why, and price it.

    python3 -I -B tools/cache-census.py [options] TRANSCRIPT.jsonl...
    python3 -I -B tools/cache-census.py --self-test

(`-B`, because `-I` ignores `PYTHONDONTWRITEBYTECODE`, and a module imported from `tools/` would
leave a `tools/__pycache__` that the workspace glob `tools/*` reads as a crate: trap 114.)

A transcript is a Claude Code session's JSONL file — a round's is the file its harness task
output links to, `readlink -f /tmp/claude-<uid>/<project>/<session>/tasks/a*.output`, and the
orchestrator's is the session's own file beside its `subagents/` directory. They are megabytes
each (the orchestrator's is tens of them), and this script is the way to read them: it streams
them line by line and prints aggregates and usage figures, never a record's text.

**A request.** Each assistant request is one `requestId`, written as several records that repeat
one `usage`; it is counted once. Its **send time** is the timestamp of the last message record —
a tool result, a prompt or an attachment — before it, and never earlier than the end of the request
before it: a request that follows another with nothing between them (a retry, a turn that went on)
was sent after that one ended. The bookkeeping records a main session also writes (queue
operations, titles, modes, turn durations) are not message records and move no send time. The
**gap** before a request is its send time less the previous request's send time, and the **quiet**
is its send time less the previous request's last record: the cache's lifetime is "measured from
the start of the request that writes or reads the cache entry" (Anthropic's prompt-caching page), a
request's start lies between those two, so the gap is the longest the entry can have sat unread and
the quiet the shortest.

**A miss** is a request that read more than `MISS_FLOOR` tokens fewer from the cache than the
request before it had left there (that one's read plus its write): it had to write them again.
Every miss is named by the first of these that holds — a first request; a compaction (the session
rewrote its own history); a model change (a cache belongs to one model); a gap over the lifetime,
after a report or a turn's end or while a command ran; a prefix change within the lifetime, named by
the attachment kinds that arrived before it; or, when none holds, unexplained. **The lifetime** is a
transcript's own: an hour where any of its requests wrote at the hour (`ephemeral_1h_input_tokens`,
which is how Claude Code writes a main session's cache), five minutes otherwise; `--lifetime`
prints what the data bound it to, the longest quiet a hit outlived against the shortest gap a miss
followed. **A keep-warm request** is one whose only call re-issues `tools/wait.sh <log>` after the
call before it reached its wall ("still running after this call", ADR 1976): it exists to read the
cache inside the lifetime and does nothing else, and `--keep-warm` counts and prices them against
the whole-context write each run of them avoided.

`--since` and `--until` close the window on a batch's span, its open and the next batch's open,
each a merge commit's `%cI` (habit 110). `--buckets` prints the miss rate by gap, `--parent` what
ended each wait past the lifetime in a main session (a task notification, the owner, a command),
and `--tsv FILE` one line of figures per request.

The price is in base-input-token equivalents of each request's own model, from Anthropic's pricing
page: a 5-minute cache write is 1.25 times an input token, a 1-hour write 2 times, output 5 times,
and "[c]ache hits and refreshes on Claude Opus 5.5 and Claude Sonnet 5.5 are priced at 0.05x the
base input price", on Claude Fable 5.1 at 0.025x (`MODELS`; `--read` overrides it for every model).
The dollar figure is the same count at the page's base prices, an API-price equivalent of the input
side alone: a transcript's `output_tokens` is the count its stream opened with (a request that wrote a
37 kB file records 16), so no output is read from it, and a keep-warm request's is taken as a poll's
`POLL_OUTPUT`. The polled
alternative replaces each gap after a command by one request every `--wall` seconds, each reading
the whole context and writing a poll's call and answer; ADR 1976 has the figures and the reasoning.
"""

import argparse
import json
import math
import os
import re
import statistics
import sys
import tempfile
from collections import Counter, defaultdict
from datetime import datetime

POLL_CREATED = 400  # one poll's tool call and its few-line answer, written to the cache
POLL_OUTPUT = 150  # one poll's tool call, generated
OUTPUT_PRICE = 5.0  # an output token, in input tokens: $20 against $4 per MTok on Opus 5.5, and so on
NEW_TAIL = 2000  # the tool result a request after a wait adds whether or not the cache held
WRITE_HOUR = 2.0  # a one-hour cache write, in input tokens
MISS_FLOOR = 2048  # tokens fewer read than the request before left cached, before it is a miss
LARGE = 20000  # a context this large shows a miss plainly; the lifetime is bound from these alone
HOUR = 3600.0

# Base input price in $ per MTok, and a cache hit's price as a multiple of it, from Anthropic's
# pricing page (the table and its footnotes 1 and 2). A model not named here is priced at the
# page's general 0.1x and no dollar figure.
MODELS = {
    "claude-opus-5-5": (4.0, 0.05),
    "claude-sonnet-5-5": (2.0, 0.05),
    "claude-fable-5-1": (10.0, 0.025),
}
GENERAL_READ = 0.1

# The record types that carry the conversation; the rest of a main session's records are its own
# bookkeeping, written whenever the harness likes, and would move a send time they did not cause.
MESSAGE_TYPES = {"user", "attachment"}

# Attachments appended after the cached prefix, every turn or every few: they lengthen the context
# and change nothing a request before them wrote, so a miss beside one of them is not named by it.
ROUTINE_ATTACHMENTS = {
    "total_tokens_reminder", "batching_reminder_sent", "bash_output_audience_note",
    "queued_command", "silent_turn_reminder", "task_status", "edited_text_file",
}

WALL_LINE = "still running after this call"  # tools/wait.sh's answer when its wall came first


def seconds(stamp):
    return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()


def texts_of(content):
    """The text a user record carries, as a list of strings."""
    if isinstance(content, str):
        return [content]
    return [block.get("text", "") for block in content or []
            if isinstance(block, dict) and block.get("type") == "text"]


def tool_results_of(content):
    """The text of each tool result a user record carries."""
    found = []
    for block in content if isinstance(content, list) else []:
        if isinstance(block, dict) and block.get("type") == "tool_result":
            body = block.get("content")
            found.append(body if isinstance(body, str) else " ".join(
                part.get("text", "") for part in body or [] if isinstance(part, dict)))
    return found


def arrival_kinds(record):
    """What a message record brought: a result, a prompt, a notification, an attachment's kind."""
    kinds = set()
    if record.get("type") == "attachment":
        attachment = record.get("attachment") or {}
        kind = str(attachment.get("type"))
        kinds.add("attachment:" + kind)
        if kind == "queued_command":
            prompt = attachment.get("prompt") or attachment.get("content") or ""
            prompt = prompt if isinstance(prompt, str) else json.dumps(prompt)
            kinds.add("notification" if "<task-notification>" in prompt[:200] else "prompt")
        return kinds
    content = (record.get("message") or {}).get("content")
    if tool_results_of(content):
        kinds.add("result")
    for text in texts_of(content):
        if record.get("isMeta"):
            kinds.add("meta")
        elif re.match(r"\s*<task-notification>", text):
            kinds.add("notification")
        elif text.strip():
            kinds.add("prompt")
    return kinds


def requests(path):
    """The transcript's requests in order, each with its times, usage, calls and what preceded it."""
    ordered, by_id = [], {}
    last_message = None
    arrived, walled = set(), False
    with open(path, encoding="utf-8", errors="replace") as lines:
        for line in lines:
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            if "timestamp" not in record:
                continue
            when = seconds(record["timestamp"])
            kind = record.get("type")
            if kind == "system" and record.get("subtype") == "compact_boundary":
                arrived.add("compaction")
                continue
            if kind != "assistant":
                if kind in MESSAGE_TYPES:
                    last_message = when if last_message is None else max(last_message, when)
                    arrived |= arrival_kinds(record)
                    content = (record.get("message") or {}).get("content")
                    walled = walled or any(WALL_LINE in text for text in tool_results_of(content))
                continue
            message = record.get("message", {})
            if message.get("model") == "<synthetic>":
                continue  # the harness's own notice, never sent to the API
            key = record.get("requestId") or message.get("id")
            if key not in by_id:
                send = when if last_message is None else last_message
                if ordered:
                    send = max(send, ordered[-1]["end"])
                by_id[key] = {
                    "send": send,
                    "end": when,
                    "usage": {},
                    "tools": [],
                    "model": message.get("model"),
                    "stamp": record["timestamp"],
                    "arrived": arrived,
                    "walled": walled,
                }
                ordered.append(by_id[key])
                arrived, walled = set(), False
            request = by_id[key]
            request["end"] = max(request["end"], when)
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


def ended_turn(tools):
    """Whether a request handed its work back or ended its turn rather than asking for a command."""
    return not tools or any(name == "SubagentHandback" for name, _ in tools)


def is_keep_warm(request):
    """A request whose one call re-issues a wait that reached its wall, and nothing else."""
    if not request["walled"] or len(request["tools"]) != 1:
        return False
    name, given = request["tools"][0]
    command = given.get("command", "")
    return name == "Bash" and "wait.sh" in command and "--start" not in command


def figures(request):
    """read, written, written at the hour, uncached input — the request's usage."""
    usage = request["usage"]
    split = usage.get("cache_creation") or {}
    return (usage.get("cache_read_input_tokens") or 0,
            usage.get("cache_creation_input_tokens") or 0,
            split.get("ephemeral_1h_input_tokens") or 0,
            usage.get("input_tokens") or 0)


def cause(previous, request, lifetime, gap):
    """The first account of a miss that holds, in the order the module comment gives."""
    if previous is None:
        return "a first request"
    if "compaction" in request["arrived"]:
        return "a compaction"
    if request["model"] != previous["model"]:
        return "a model change"
    if gap > lifetime:
        return "a gap over the lifetime, " + (
            "after a report or a turn's end" if ended_turn(previous["tools"])
            else "while a command ran")
    changed = sorted(kind.split(":", 1)[1] for kind in request["arrived"]
                     if kind.startswith("attachment:")
                     and kind.split(":", 1)[1] not in ROUTINE_ATTACHMENTS)
    if changed:
        return "a prefix change within the lifetime (" + ", ".join(changed) + ")"
    return "unexplained"


def what_ended(request):
    """What a main session's request after a long wait answered."""
    if "notification" in request["arrived"]:
        return "a task notification"
    if "prompt" in request["arrived"]:
        return "the owner's prompt"
    if "result" in request["arrived"]:
        return "a command's result"
    if "meta" in request["arrived"]:
        return "a message the harness marked as its own (isMeta)"
    return "nothing a record shows"


def census(paths, args):
    """Every figure the report prints, as one dictionary; `main` and the self-test both read it."""
    since = seconds(args.since) if args.since else None
    until = seconds(args.until) if args.until else None
    out = {
        "total": Counter(), "preceded": defaultdict(Counter), "gaps": [], "per_file": [],
        "edges": [0, 60, 180, 240, 270, 300, 330, 400, 600, 1800, 3600, 3900, math.inf],
        "bucket_n": defaultdict(Counter), "bucket_miss": defaultdict(Counter),
        "causes": defaultdict(Counter), "ended": Counter(), "before": 0.0, "after": 0.0,
        "newest": None, "hits": defaultdict(list), "expiries": defaultdict(list),
        "keep_warm": Counter(), "rows": [], "lifetimes": {},
    }
    total = out["total"]
    for path in paths:
        listed = requests(path)
        hourly = any(figures(request)[2] for request in listed)
        lifetime = args.ttl_hour if hourly else args.ttl
        kind = "1 h" if hourly else "5 min"
        out["lifetimes"][kind] = lifetime
        lost_here = created_here = gaps_here = 0
        previous, chain, started = None, 0, 0.0
        for request in listed:
            if out["newest"] is None or request["send"] > out["newest"][0]:
                out["newest"] = (request["send"], request["stamp"])
            if since is not None and request["send"] < since:
                previous = request
                continue
            # A batch's census ends where the next batch's rounds begin, and an orchestrator's
            # transcript runs across both, so the window is closed on the right as well (habit 110).
            if until is not None and request["send"] >= until:
                break
            read, created, created_hour, uncached = figures(request)
            base, hit = MODELS.get(request["model"], (None, GENERAL_READ))
            hit = args.read if args.read is not None else hit
            cache_cost = (args.write * (created - created_hour) + WRITE_HOUR * created_hour
                          + hit * read)
            total["requests"] += 1
            total["created"] += created
            total["read"] += read
            total["uncached"] += uncached
            total["created 1h"] += created_hour
            total["spend"] += cache_cost
            if base is not None:
                total["dollars"] += base * (cache_cost + uncached) / 1e6
            created_here += created
            gap = quiet = None
            label = ""
            if previous is None:
                label = "a first request"
                out["causes"][label]["misses"] += 1
                out["causes"][label]["written"] += created
                out["causes"][label]["cost"] += cache_cost
            else:
                gap = request["send"] - previous["send"]
                quiet = request["send"] - previous["end"]
                out["gaps"].append(gap)
                p_read, p_created = figures(previous)[:2]
                left = p_read + p_created
                missed = left - read > MISS_FLOOR
                if left > LARGE:
                    edge = next(i for i in range(len(out["edges"]) - 1)
                                if gap < out["edges"][i + 1])
                    out["bucket_n"][kind][edge] += 1
                    out["bucket_miss"][kind][edge] += missed
                if missed:
                    label = cause(previous, request, lifetime, gap)
                    out["causes"][label]["misses"] += 1
                    out["causes"][label]["written"] += created
                    out["causes"][label]["cost"] += cache_cost - hit * left
                    if left > LARGE and label.startswith("a gap over"):
                        out["expiries"][kind].append((gap, request["stamp"]))
                elif left > LARGE:
                    out["hits"][kind].append((quiet, request["stamp"]))
                if gap > lifetime:
                    total["gaps"] += 1
                    total["gap created"] += created
                    waited = what_waited(previous["tools"])
                    out["preceded"][waited]["gaps"] += 1
                    out["preceded"][waited]["created"] += created
                    out["preceded"][waited]["seconds"] += gap
                    out["ended"][what_ended(request)] += 1
                    lost_here += created
                    gaps_here += 1
                    if any(name == "Bash" for name, _ in previous["tools"]):
                        context = created + read
                        polls = math.ceil(gap / args.wall) - 1
                        out["before"] += args.write * created + hit * read
                        out["after"] += (hit * (context - NEW_TAIL) + args.write * NEW_TAIL
                                         + polls * (hit * context + args.write * POLL_CREATED
                                                    + OUTPUT_PRICE * POLL_OUTPUT))
                        total["command gaps"] += 1
                        total["polls"] += polls
            # A run of keep-warm requests ends at the first request that does other work; that
            # one read its context instead of writing it again, which is what the run was for.
            warm_here = is_keep_warm(request)
            if warm_here:
                warm = out["keep_warm"]
                warm["requests"] += 1
                warm["read"] += read
                warm["created"] += created
                warm["cost"] += cache_cost + uncached + OUTPUT_PRICE * POLL_OUTPUT
                if not chain:
                    started = previous["send"] if previous is not None else request["send"]
                chain += 1
            elif chain:
                # Without the run, this request would have followed the whole wait; it avoided a
                # write only where that wait was longer than the lifetime.
                out["keep_warm"]["runs"] += 1
                if request["send"] - started > lifetime:
                    out["keep_warm"]["avoided"] += (args.write - hit) * (read + created)
                    out["keep_warm"]["runs past"] += 1
                chain = 0
            out["rows"].append((path.rsplit("/", 1)[-1], request["stamp"], gap, quiet, read,
                                created, uncached, label,
                                "keep-warm" if warm_here else ""))
            previous = request
        out["per_file"].append((path.rsplit("/", 1)[-1], created_here, lost_here, gaps_here))
    return out


def report(out, args, paths):
    total = out["total"]
    mega = 1e6
    share = 100 * total["gap created"] / max(1, total["created"])
    inputs = total["read"] + total["created"] + total["uncached"]
    print(f"{len(paths)} transcripts, {total['requests']} requests")
    print(f"cache written {total['created'] / mega:.1f} M tokens ({total['created 1h'] / mega:.1f} M "
          f"of them at the hour), read {total['read'] / mega:.1f} M")
    print(f"input {inputs / mega:.1f} M tokens: {100 * total['read'] / max(1, inputs):.2f} % read "
          f"from the cache, {100 * total['created'] / max(1, inputs):.2f} % written to it, "
          f"{100 * total['uncached'] / max(1, inputs):.3f} % uncached")
    print(f"{total['gaps']} requests ({100 * total['gaps'] / max(1, total['requests']):.1f} %) followed "
          f"a gap over the lifetime and wrote {total['gap created'] / mega:.1f} M tokens: "
          f"{share:.0f} % of all writing")
    if out["gaps"]:
        ranked = sorted(out["gaps"])
        print(f"gap median {statistics.median(ranked):.1f} s, p90 {ranked[int(.9 * len(ranked))]:.0f} s, "
              f"p99 {ranked[int(.99 * len(ranked))]:.0f} s; over 600 s: {sum(g > 600 for g in ranked)}")

    misses = sum(row["misses"] for row in out["causes"].values())
    print(f"\n{misses} misses (a request reading over {MISS_FLOOR} tokens fewer than the request before "
          f"it left cached), by cause:")
    for label, row in sorted(out["causes"].items(), key=lambda item: -item[1]["written"]):
        print(f"  {row['misses']:5d} misses, {row['written'] / mega:6.2f} M written, "
              f"{row['cost'] / mega:6.2f} M equivalents over a hit  {label}")

    print("\nwhat the request before a gap over the lifetime had asked to run:")
    for kind, row in sorted(out["preceded"].items(), key=lambda item: -item[1]["created"]):
        print(f"  {row['gaps']:5d} gaps, {row['created'] / mega:6.1f} M written after them, "
              f"mean gap {row['seconds'] / row['gaps']:6.0f} s  {kind}")
    if args.parent:
        print("\nwhat ended each wait over the lifetime:")
        for kind, count in out["ended"].most_common():
            print(f"  {count:5d}  {kind}")

    print(f"\nthe {total['command gaps']} gaps after a command: {out['before'] / mega:.1f} M equivalents "
          f"as they were; polled every {args.wall:.0f} s, {out['after'] / mega:.1f} M with "
          f"{total['polls']} more requests — {(out['before'] - out['after']) / mega:.1f} M saved, "
          f"{100 * (out['before'] - out['after']) / max(1, total['spend']):.0f} % of the "
          f"{total['spend'] / mega:.1f} M the cache cost in all (${total['dollars']:.0f} at API "
          f"prices, input side)")
    # Ranked by what was lost and then by how often the cache was left past its lifetime: when
    # every gap was a hit, the count is the only column that names a round that waited too long.
    print(f"\nthe {args.worst} transcripts that lost most:")
    for name, created, lost, gaps_over in sorted(out["per_file"], key=lambda row: (-row[2], -row[3]))[
            :args.worst]:
        print(f"  {name}: {created / mega:.1f} M written, {lost / mega:.1f} M after "
              f"{gaps_over} gaps over the lifetime ({100 * lost / max(1, created):.0f} %)")
    if args.keep_warm:
        warm = out["keep_warm"]
        print(f"\nkeep-warm requests (one call, a `tools/wait.sh` re-issued after its wall): "
              f"{warm['requests']} in {warm['runs']} runs ({warm['runs past']} of them spanning a wait "
              f"past the lifetime), "
              f"{100 * warm['requests'] / max(1, total['requests']):.1f} % of requests; read "
              f"{warm['read'] / mega:.1f} M, wrote {warm['created'] / mega:.2f} M; "
              f"{warm['cost'] / mega:.2f} M equivalents, against "
              f"{warm['avoided'] / mega:.2f} M over a hit for the whole-context writes they avoided")
    if args.lifetime:
        print(f"\nthe lifetime the data bound (contexts over {LARGE} tokens; a hit by its quiet, a "
              f"miss over the lifetime by its gap):")
        for kind in sorted(set(out["hits"]) | set(out["expiries"])):
            hits = sorted(out["hits"][kind], reverse=True)
            expiries = sorted(out["expiries"][kind])
            longest = f"{hits[0][0]:.0f} s ({hits[0][1]})" if hits else "none"
            shortest = f"{expiries[0][0]:.0f} s ({expiries[0][1]})" if expiries else "none"
            past = sum(quiet > out["lifetimes"][kind] for quiet, _ in hits)
            print(f"  written at {kind}: {len(hits)} hits, the longest quiet one outlived {longest}, "
                  f"{past} past {out['lifetimes'][kind]:.0f} s; {len(expiries)} misses over it, the "
                  f"shortest gap one followed {shortest}")
    if args.buckets:
        print(f"\nmiss rate by gap (requests after a context over {LARGE} tokens):")
        for kind in sorted(out["bucket_n"]):
            print(f"  written at {kind}:")
            edges = out["edges"]
            for i in range(len(edges) - 1):
                if not out["bucket_n"][kind][i]:
                    continue
                upper = "inf" if edges[i + 1] == math.inf else f"{edges[i + 1]:.0f}"
                n, m = out["bucket_n"][kind][i], out["bucket_miss"][kind][i]
                print(f"    [{edges[i]:4.0f}, {upper:>4}) s: {n:6d} requests, {m:5d} missed "
                      f"({100 * m / max(1, n):5.1f} %)")
    if args.tsv:
        with open(args.tsv, "w", encoding="utf-8") as sheet:
            sheet.write("transcript\tstamp\tgap_s\tquiet_s\tread\twritten\tuncached\tmiss\t"
                        "keep_warm\n")
            for row in out["rows"]:
                sheet.write("\t".join("" if cell is None else
                                      f"{cell:.1f}" if isinstance(cell, float) else str(cell)
                                      for cell in row) + "\n")


def options(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("transcripts", nargs="*")
    parser.add_argument("--ttl", type=float, default=300, help="a 5-minute write's lifetime, s")
    parser.add_argument("--ttl-hour", type=float, default=HOUR, help="a 1-hour write's lifetime, s")
    parser.add_argument("--wall", type=float, default=240, help="the polled alternative's wall, s")
    parser.add_argument("--write", type=float, default=1.25, help="a 5-minute cache write's price")
    parser.add_argument("--read", type=float, default=None,
                        help="a cache hit's price for every model (default: each model's own)")
    parser.add_argument("--since", help="count only requests sent at or after this ISO time")
    parser.add_argument("--until", help="count only requests sent before this ISO time")
    parser.add_argument("--worst", type=int, default=5, help="transcripts listed by their loss")
    parser.add_argument("--buckets", action="store_true", help="the miss rate by gap")
    parser.add_argument("--lifetime", action="store_true", help="the lifetime the data bound")
    parser.add_argument("--keep-warm", action="store_true",
                        help="the requests that only re-issue a wait")
    parser.add_argument("--parent", action="store_true", help="what ended each long wait")
    parser.add_argument("--tsv", help="write one line of figures per request to this file")
    parser.add_argument("--self-test", action="store_true", help="run the planted transcripts")
    return parser.parse_args(argv)


def main(argv=None):
    args = options(sys.argv[1:] if argv is None else argv)
    if args.self_test:
        return self_test()
    if not args.transcripts:
        print("cache-census: no transcript named", file=sys.stderr)
        return 2
    out = census(args.transcripts, args)
    # A census that counted nothing prints a page of zeros that reads as "no gap lost the cache";
    # it is a sentence about the arguments, so it is said as one and the exit is not 0 (trap 13).
    if out["total"]["requests"] == 0:
        newest = out["newest"]
        held = f"the newest request they hold is stamped {newest[1]}" if newest else "they hold none"
        window = (f" at or after {args.since}" if args.since else "") + (
            f" before {args.until}" if args.until else "")
        print(f"cache-census: no request counted in {len(args.transcripts)} transcripts{window}; "
              f"{held}", file=sys.stderr)
        return 2
    report(out, args, args.transcripts)
    return 0


# The self-test plants one transcript of each lifetime with every account of a miss in it, at times
# chosen so that each account holds and the one before it does not, and asserts that the census
# names each (trap 13: an instrument that finds nothing is first shown to find what was planted).

def _stamp(t):
    return datetime.fromtimestamp(1_800_000_000 + t).astimezone().isoformat()


def _assistant(t, key, model, read, created, hour=False, tools=()):
    usage = {"input_tokens": 2, "cache_read_input_tokens": read,
             "cache_creation_input_tokens": created, "output_tokens": 4,
             "cache_creation": {"ephemeral_5m_input_tokens": 0 if hour else created,
                                "ephemeral_1h_input_tokens": created if hour else 0}}
    content = [{"type": "tool_use", "id": f"{key}-{i}", "name": name, "input": given}
               for i, (name, given) in enumerate(tools)] or [{"type": "text", "text": "."}]
    return {"type": "assistant", "timestamp": _stamp(t), "requestId": key,
            "message": {"model": model, "usage": usage, "content": content}}


def _result(t, text="ok"):
    return {"type": "user", "timestamp": _stamp(t), "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "x", "content": text}]}}


def _prompt(t, text="go"):
    return {"type": "user", "timestamp": _stamp(t), "message": {"role": "user", "content": text}}


def _attachment(t, kind):
    return {"type": "attachment", "timestamp": _stamp(t), "attachment": {"type": kind}}


def _bash(command):
    return [("Bash", {"command": command})]


def self_test():
    opus, fable = "claude-opus-5-5", "claude-fable-5-1"
    walled = "wait: a.log still running after this call's 240 s; re-issue tools/wait.sh a.log"
    round_records = [
        _prompt(0),
        _assistant(1, "r1", opus, 0, 40000),  # a first request
        _result(10),
        _assistant(11, "r2", opus, 40000, 1000, tools=_bash("tools/wait.sh --start a.log -- x")),
        _result(12),
        _assistant(13, "r3", opus, 41000, 500, tools=_bash("tools/wait.sh a.log")),
        _result(253, walled),
        _attachment(253, "total_tokens_reminder"),
        _assistant(255, "r4", opus, 41500, 300, tools=_bash("tools/wait.sh a.log")),  # keep-warm
        _result(495, walled),
        _assistant(497, "r5", opus, 41800, 200, tools=_bash("tools/wait.sh a.log")),  # keep-warm
        _result(560, "wait: a.log ended"),
        _assistant(562, "r6", opus, 42000, 300, tools=_bash("cargo build")),  # ends the run
        _result(970),
        _assistant(972, "r7", opus, 0, 43000, tools=[("SubagentHandback", {})]),  # a 410 s gap
        _prompt(5000, "and one more thing"),
        _assistant(5002, "r8", opus, 30000, 14000, tools=_bash("ls")),  # after a report
        _attachment(5010, "deferred_tools_delta"),
        _result(5010),
        _assistant(5012, "r9", opus, 10000, 34000, tools=_bash("ls")),  # a prefix change
        _result(5020),
        _attachment(5020, "batching_reminder_sent"),
        _assistant(5022, "r10", opus, 20000, 24000, tools=_bash("ls")),  # unexplained
        _result(5030),
        _assistant(5032, "r11", opus, 44000, 100),
        _assistant(5090, "r11", opus, 44000, 100),  # the same request, still being written
        _assistant(5100, "r12", opus, 44100, 100),  # nothing between: sent when r11 ended
        _result(5110, walled),
        _assistant(5112, "r13", opus, 44200, 100, tools=_bash("tools/wait.sh --start b.log -- y")),
    ]
    main_records = [
        _prompt(0),
        _assistant(1, "m1", fable, 0, 60000, hour=True),
        _prompt(2000, "<task-notification>done</task-notification>"),
        _assistant(2002, "m2", fable, 60000, 1000, hour=True),  # a hit after 2 000 s at the hour
        _prompt(6000, "<task-notification>done</task-notification>"),
        _assistant(6002, "m3", fable, 0, 61000, hour=True),  # a gap over the hour
        _prompt(6010),
        {"type": "system", "subtype": "compact_boundary", "timestamp": _stamp(6011)},
        {"type": "queue-operation", "timestamp": _stamp(6500)},  # bookkeeping: no send time
        _assistant(6012, "m4", fable, 29000, 41000, hour=True),  # a compaction
    ]
    fails = 0

    def check(name, wanted, got):
        nonlocal fails
        ok = wanted == got
        fails += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {name}" + ("" if ok else f": wanted {wanted!r}, got {got!r}"))

    with tempfile.TemporaryDirectory(prefix="cache-census-") as scratch:
        paths = []
        for name, records in (("round.jsonl", round_records), ("main.jsonl", main_records)):
            path = os.path.join(scratch, name)
            with open(path, "w", encoding="utf-8") as sheet:
                sheet.write("\n".join(json.dumps(record) for record in records) + "\n")
            paths.append(path)
        out = census(paths, options(["--keep-warm", "--lifetime"] + paths))
    causes = {label: row["misses"] for label, row in out["causes"].items()}
    check("two first requests, one per transcript", 2, causes.get("a first request"))
    check("a 410 s gap while a command ran is over five minutes", 1,
          causes.get("a gap over the lifetime, while a command ran"))
    check("a resume after a hand-back and a notification after the hour are turn ends", 2,
          causes.get("a gap over the lifetime, after a report or a turn's end"))
    check("a prefix change is named by its attachment", 1,
          causes.get("a prefix change within the lifetime (deferred_tools_delta)"))
    check("a routine attachment names nothing, so that miss is unexplained", 1,
          causes.get("unexplained"))
    check("a compaction is its own cause", 1, causes.get("a compaction"))
    check("eight misses in all", 8, sum(causes.values()))
    warm = out["keep_warm"]
    check("two keep-warm requests in one run, over a 548 s wait", (2, 1, 1),
          (warm["requests"], warm["runs"], warm["runs past"]))
    hour_hits = sorted(out["hits"]["1 h"], reverse=True)
    check("the hour's longest hit outlived 1 999 s of quiet", 1999,
          round(hour_hits[0][0]) if hour_hits else None)
    five = sorted(out["expiries"]["5 min"])
    check("the shortest gap a five-minute miss followed is 410 s", 410,
          round(five[0][0]) if five else None)
    rows = [row for row in out["rows"] if row[0] == "round.jsonl"]
    check("a request with nothing before it was sent when the one before ended", 60,
          round(next(row[2] for row in rows if row[1] == _stamp(5100))))
    main_rows = [row for row in out["rows"] if row[0] == "main.jsonl"]
    check("a queue operation moves no send time", 10, round(main_rows[-1][2]))
    check("a request is counted once however many records repeat it", 17, out["total"]["requests"])
    if fails:
        print(f"cache-census self-test: {fails} case(s) failed")
        return 1
    print("cache-census self-test: every case holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
