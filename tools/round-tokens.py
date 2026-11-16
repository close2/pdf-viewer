#!/usr/bin/env python3
"""What a round's tokens were spent on, read from the agent transcripts the harness keeps.

A round's price is set by two numbers and their product: how many requests it makes and how
large its context is when it makes them, because every request re-reads the whole context from the
prompt cache. So the cost of a line of text is its size times the number of requests made after it
arrived, and what this script prints is where the context came from and how often it was re-read
(ADR 2913 has the census it was written for).

    python3 -I tools/round-tokens.py <transcript directory> [--rounds FIRST-LAST]

The directory is the harness's per-session `subagents/` directory under `~/.claude/projects/`. A
transcript is taken as a round's when its first 3000 bytes name `round <number>`, which is how a
brief's pointer names it; a handover continuation is a second transcript of the same round and is
printed as its own row.

Per transcript: requests, tool calls, the context at its end, the sums of each kind of token, the
estimated price, and how many of its requests were a wait that came back unfinished. Over all of
them: the tool results' characters by size, and the shell commands' by what they ran. The prices
are list rates per million tokens and are an estimate, never a bill.

It reads and prints; it writes nothing.
"""

import collections
import glob
import json
import os
import re
import sys

# Dollars per million tokens: input, cache write (five minutes), cache read, output.
RATES = {"input_tokens": 5.0, "cache_creation_input_tokens": 6.25,
         "cache_read_input_tokens": 0.5, "output_tokens": 25.0}

# A shell command is classed by what it runs after any leading `cd …&&`, `export …;` or
# `VAR=value`, first match wins; the classes are the ones a round's own habits name.
CLASSES = ["cargo nextest", "cargo clippy", "cargo test", "cargo build", "cargo check", "cargo fmt",
           "tools/wait.sh", "tools/state.sh", "tools/round.sh", "tools/batch.sh", "git diff",
           "git log", "git show", "git status", "grep", "sed", "cat", "rg", "ls", "tail", "head",
           "wc", "python3", "for ", "find", "du"]

SIZE_BUCKETS = [(1000, "under 1k"), (4000, "1k-4k"), (10000, "4k-10k"), (None, "10k and over")]


def text_of(content):
    """A tool result's text: a string, or the text blocks of a list."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(b.get("text", "") for b in content if isinstance(b, dict))
    return ""


def class_of(command):
    """The class of a shell command, after its leading directory and variable settings."""
    c = re.sub(r"^(\s*(cd|export|source)\s+[^;&\n]*(&&|;)\s*)+", "", command.strip())
    c = re.sub(r"^((env\s+)?[A-Z_][A-Z0-9_]*=(\"[^\"]*\"|'[^']*'|\S+)\s+)+", "", c)
    for k in CLASSES:
        if c.startswith(k):
            return k.strip()
    return "other"


def price(usage):
    return sum(usage.get(k, 0) * r for k, r in RATES.items()) / 1e6


def bucket_of(n):
    for limit, name in SIZE_BUCKETS:
        if limit is None or n < limit:
            return name
    raise AssertionError("the last bucket has no limit")


def read_transcript(path):
    """Requests (deduplicated by message id), tool uses by id, and tool results by use id."""
    usage, uses, results = {}, {}, {}
    with open(path, errors="replace") as f:
        for line in f:
            try:
                d = json.loads(line)
            except json.JSONDecodeError:
                continue
            m = d.get("message")
            if not isinstance(m, dict):
                continue
            if d.get("type") == "assistant":
                if m.get("usage"):
                    usage[m.get("id")] = m["usage"]
                for b in m.get("content") or []:
                    if isinstance(b, dict) and b.get("type") == "tool_use":
                        uses[b["id"]] = (m.get("id"), b)
            elif d.get("type") == "user" and isinstance(m.get("content"), list):
                for b in m["content"]:
                    if isinstance(b, dict) and b.get("type") == "tool_result":
                        results[b.get("tool_use_id")] = text_of(b.get("content", ""))
    return usage, uses, results


def main(argv):
    if len(argv) < 2 or argv[1].startswith("-"):
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 2
    first, last = 0, 10**9
    if "--rounds" in argv:
        first, last = (int(x) for x in argv[argv.index("--rounds") + 1].split("-"))
    rows, sizes, classes = [], collections.Counter(), collections.defaultdict(lambda: [0, 0])
    size_chars = collections.Counter()
    for path in sorted(glob.glob(os.path.join(argv[1], "*.jsonl"))):
        with open(path, errors="replace") as f:
            m = re.search(r"round (\d{4})", f.read(3000))
        if not m or not first <= int(m.group(1)) <= last:
            continue
        usage, uses, results = read_transcript(path)
        context = [u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
                   + u.get("cache_read_input_tokens", 0) for u in usage.values()]
        unfinished = 0
        for use_id, (message_id, b) in uses.items():
            text = results.get(use_id, "")
            n = len(text)
            sizes[bucket_of(n)] += 1
            size_chars[bucket_of(n)] += n
            if b["name"] == "Bash":
                command = b.get("input", {}).get("command", "")
                k = class_of(command)
                classes[k][0] += n
                classes[k][1] += 1
                if "tools/wait.sh" in command and "still running after this call" in text:
                    unfinished += 1
        rows.append((os.path.basename(path)[6:14], int(m.group(1)), len(usage), len(uses),
                     context[-1] if context else 0,
                     sum(u.get("cache_read_input_tokens", 0) for u in usage.values()),
                     sum(u.get("output_tokens", 0) for u in usage.values()),
                     sum(price(u) for u in usage.values()), unfinished))
    print("transcript round requests calls context_end cache_read output $ unfinished_waits")
    for r in sorted(rows, key=lambda r: r[1]):
        print(f"{r[0]} {r[1]} {r[2]:5d} {r[3]:5d} {r[4]:8d} {r[5]:10d} {r[6]:7d} {r[7]:7.2f} {r[8]:4d}")
    total = sum(r[7] for r in rows)
    print(f"{len(rows)} transcripts, ${total:.2f}, {sum(r[2] for r in rows)} requests, "
          f"{sum(r[8] for r in rows)} unfinished waits")
    print("\ntool results by size: calls, characters")
    for _, name in SIZE_BUCKETS:
        print(f"  {name:14s} {sizes[name]:6d} {size_chars[name]:10d}")
    print("\nshell commands by class: characters returned, calls")
    for k, (n, calls) in sorted(classes.items(), key=lambda x: -x[1][0]):
        print(f"  {k:14s} {n:10d} {calls:6d}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
