#!/usr/bin/env python3
"""The token census: where a session's and its rounds' tokens go, counted from their transcripts.

A request to the model re-reads its whole context, so a session's cost is the sum over its requests
of the context each one carried — roughly *tool calls x context size* — and not the size of what it
wrote. This counts that sum from the transcripts the harness keeps, one JSONL line per content
block, each `assistant` line carrying the request's `message.usage`:

  requests     distinct `message.id`s; the usage repeats on every block of one request, so it
               is counted once per id
  tools        `tool_use` blocks
  out          `output_tokens` as the transcript records it — the value at the start of the
               stream, so a floor and not the bill (the harness's `cost-state` line has the total)
  uncached     `input_tokens`
  write        `cache_creation_input_tokens`, split by `cache_creation` into the 5-minute and
               one-hour lifetimes
  read         `cache_read_input_tokens`
  last, max    the context of the last and the largest request: uncached + write + read
  >200k        the requests whose context passed 200 000 tokens, and their share of `$in`
  $in          the input side in dollars at the API's list prices, from `tools/cache-census.py`'s
               `MODELS` (a model's base price and its read ratio; a 5-minute write 1.25 times the
               base, a one-hour write 2 times) — one price table, kept there; output is left out
               because the transcript does not hold it
  waits        tool calls that wait or poll: `tools/wait.sh <log>`, a `sleep`, an `until`/`while`
               loop, `TaskOutput`, `BashOutput`, `Monitor`

What a miss of the cache cost and why it happened — a gap past the lifetime, a compaction — is
`tools/cache-census.py`'s question and it answers it; this script asks what the context *holds*.

`--detail` adds, per agent, the context every 25 tool calls, the waits and what the requests after
them read, the reading the round did before its work, and the largest tool results: their size
in characters, their tokens as the context grew across them (shared by characters where a request
issued several), what they cost in later reads (tokens x the requests that carried them), the tool
and the first line of its input — the file read or the command run, never a result's content.
`--spans` cuts a session that launched rounds into one span per batch, from the first launch of a
batch's brief to the next batch's, with the context at the span's end and its compactions.

    python3 -I -B tools/token-census.py <tasks-or-subagents-dir> --rounds 1752-1767
    python3 -I -B tools/token-census.py <dir> --rounds 1769 --detail
    python3 -I -B tools/token-census.py <session.jsonl> --spans

A directory is read for `*.jsonl` and `*.output` (a task's `.output` is a link to its transcript,
followed and counted once); an agent is named by the round number in its `.meta.json` description
or its first prompt, and grouped by the `batch-<first>` its prompt names. The transcripts are data:
this prints counts and names, never what a file or a result held, and a command's first line is
cut at 90 characters with anything shaped like a credential replaced.
"""

import argparse
import importlib.util
import json
import os
import re
import sys
from datetime import datetime


def _cache_census():
    """`tools/cache-census.py`, whose price table this script prices by (its name has a hyphen)."""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cache-census.py")
    spec = importlib.util.spec_from_file_location("cache_census", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PRICES = _cache_census()
WRITE_FIVE_MINUTES = 1.25  # a 5-minute cache write, in base input tokens (cache-census.py's module comment)
LONG_CONTEXT = 200_000

ROUND_RE = re.compile(r"\b[Rr][Oo][Uu][Nn][Dd] (\d{3,5})\b")
BATCH_RE = re.compile(r"batch-(\d{3,5})-(?:brief|\d{3,5})")
SECRET_RE = re.compile(r"(?i)(gh[pousr]_|sk-|xox[abp]-|bearer\s|token=|password=|secret=)\S*|\b(?=[A-Za-z0-9_\-]*\d)(?=[A-Za-z0-9_\-]*[A-Za-z])[A-Za-z0-9_\-]{40,}")
WAIT_RES = (
    ("wait.sh", re.compile(r"tools/wait\.sh(?!\s+--start)")),
    ("sleep", re.compile(r"(^|[;&|(]\s*|\s)sleep\s+\d")),
    ("until-loop", re.compile(r"\b(until|while)\b[^\n]*\bdo\b")),
)
WAIT_TOOLS = {"TaskOutput", "BashOutput", "Monitor"}
START_SET = re.compile(
    r"round\.sh|CLAUDE\.md|02-every-round|environment\.md|every-round\.md|batch-\d+-brief|"
    r"round-\d+-brief|prompts-\d+|HANDOVER\.md"
)


def when(stamp):
    """A transcript timestamp as seconds since the epoch."""
    return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()


def label(name, tool_input):
    """The tool and the first line of its input: what was read or run, never what came back."""
    if not isinstance(tool_input, dict):
        return name
    if name == "Bash":
        text = (tool_input.get("command") or "").strip().splitlines() or [""]
        text = " ".join(text[0].split())
    elif name in ("Read", "Edit", "Write", "NotebookEdit"):
        text = tool_input.get("file_path", "")
        if tool_input.get("offset") or tool_input.get("limit"):
            text += f" @{tool_input.get('offset', 0)}+{tool_input.get('limit', '')}"
    elif name == "Grep":
        text = f"{tool_input.get('pattern', '')} in {tool_input.get('path', '.')}"
    elif name == "Glob":
        text = tool_input.get("pattern", "")
    elif name in ("Agent", "Task"):
        text = tool_input.get("description", "")
    else:
        text = ""
    text = SECRET_RE.sub("<redacted>", text)
    return f"{name} {text[:90]}".rstrip()


def wait_kind(name, tool_input):
    """Whether a tool call is a wait or a poll, and which kind."""
    if name in WAIT_TOOLS:
        return name
    if name == "Bash" and isinstance(tool_input, dict):
        command = tool_input.get("command") or ""
        for kind, pattern in WAIT_RES:
            if pattern.search(command):
                return kind
    return None


def result_chars(content):
    """Characters a tool result put into the context (text blocks; an image counts zero)."""
    if isinstance(content, str):
        return len(content)
    if isinstance(content, list):
        return sum(len(block.get("text", "")) for block in content if isinstance(block, dict))
    return 0


class Transcript:
    """One agent's requests, tool calls and results, read from its JSONL transcript."""

    def __init__(self, path):
        self.path = path
        self.name = os.path.basename(path).removesuffix(".jsonl").removeprefix("agent-")
        self.session = None
        self.batch = None
        self.requests = []  # dicts in order of first appearance
        self.tools = {}  # tool_use id -> dict
        self.compactions = []  # (seconds, request index, pre, post)
        self.notification_chars = 0
        self.launches = []  # (request index, round number, batch key)
        self._read(path)
        self._name_from_meta(path)

    def _read(self, path):
        by_id = {}
        first_prompt = None
        with open(path, encoding="utf-8", errors="replace") as handle:
            for line in handle:
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                kind = entry.get("type")
                if kind == "assistant":
                    self._assistant(entry, by_id)
                elif kind == "user":
                    content = entry.get("message", {}).get("content")
                    if first_prompt is None and isinstance(content, str):
                        first_prompt = content
                    if isinstance(content, str) and "<task-notification>" in content:
                        self.notification_chars += len(content)
                    if isinstance(content, list):
                        for block in content:
                            if isinstance(block, dict) and block.get("type") == "tool_result":
                                tool = self.tools.get(block.get("tool_use_id"))
                                if tool is not None:
                                    tool["chars"] = result_chars(block.get("content"))
                                    tool["done"] = when(entry["timestamp"])
                            elif isinstance(block, dict) and block.get("type") == "text":
                                if first_prompt is None:
                                    first_prompt = block.get("text", "")
                elif kind == "system" and entry.get("subtype") == "compact_boundary":
                    meta = entry.get("compactMetadata", {})
                    self.compactions.append(
                        (when(entry["timestamp"]), len(self.requests), meta.get("preTokens"), meta.get("postTokens"))
                    )
        if first_prompt:
            match = ROUND_RE.search(first_prompt)
            self.session = int(match.group(1)) if match else None
            match = BATCH_RE.search(first_prompt)
            self.batch = int(match.group(1)) if match else None

    def _assistant(self, entry, by_id):
        message = entry.get("message", {})
        msg_id = message.get("id") or entry.get("requestId")
        stamp = when(entry["timestamp"])
        request = by_id.get(msg_id)
        if request is None:
            usage = message.get("usage") or {}
            split = usage.get("cache_creation") or {}
            write = usage.get("cache_creation_input_tokens") or 0
            write1h = split.get("ephemeral_1h_input_tokens") or 0
            request = {
                "uncached": usage.get("input_tokens") or 0,
                "write": write,
                "write1h": write1h,
                "write5m": write - write1h,
                "read": usage.get("cache_read_input_tokens") or 0,
                "out": usage.get("output_tokens") or 0,
                "start": stamp,
                "end": stamp,
                "tools": [],
                "model": message.get("model"),
            }
            request["context"] = request["uncached"] + request["write"] + request["read"]
            by_id[msg_id] = request
            self.requests.append(request)
        request["end"] = max(request["end"], stamp)
        index = len(self.requests) - 1 if request is self.requests[-1] else self.requests.index(request)
        for block in message.get("content") or []:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                name = block.get("name", "?")
                tool_input = block.get("input")
                tool = {
                    "name": name,
                    "label": label(name, tool_input),
                    "wait": wait_kind(name, tool_input),
                    "request": index,
                    "chars": 0,
                    "done": None,
                }
                self.tools[block.get("id")] = tool
                request["tools"].append(tool)
                if name in ("Agent", "Task") and isinstance(tool_input, dict):
                    match = ROUND_RE.search(tool_input.get("description", "") or "")
                    batch = BATCH_RE.search(tool_input.get("prompt", "") or "")
                    if match:
                        self.launches.append((index, int(match.group(1)), int(batch.group(1)) if batch else None))

    def _name_from_meta(self, path):
        meta = path.removesuffix(".jsonl") + ".meta.json"
        if os.path.exists(meta):
            try:
                with open(meta, encoding="utf-8") as handle:
                    description = json.load(handle).get("description", "")
            except (OSError, json.JSONDecodeError):
                description = ""
            match = ROUND_RE.search(description)
            if match:
                self.session = int(match.group(1))

    # -- derived figures -------------------------------------------------------------------

    def measure_results(self):
        """Give every tool result its tokens, from the context's growth across its request."""
        for index, request in enumerate(self.requests):
            if not request["tools"] or index + 1 >= len(self.requests):
                continue
            grown = self.requests[index + 1]["context"] - request["context"]
            total_chars = sum(tool["chars"] for tool in request["tools"]) or 1
            later = self.carriers(index)
            for tool in request["tools"]:
                share = grown * tool["chars"] / total_chars if grown > 0 else 0
                tool["tokens"] = share
                tool["carried"] = share * later

    def carriers(self, index):
        """How many later requests carried what entered after request `index`, up to a compaction."""
        bound = len(self.requests)
        for _, at, _, _ in self.compactions:
            if at > index:
                bound = min(bound, at)
        return max(0, bound - index - 1)

    def totals(self):
        """The census line's figures."""
        sums = {key: sum(request[key] for request in self.requests) for key in ("uncached", "write", "write5m", "write1h", "read", "out")}
        sums["requests"] = len(self.requests)
        sums["tools"] = sum(len(request["tools"]) for request in self.requests)
        sums["last"] = self.requests[-1]["context"] if self.requests else 0
        sums["max"] = max((request["context"] for request in self.requests), default=0)
        sums["dollars"] = sum(dollars(request) for request in self.requests)
        sums["long"] = sum(1 for request in self.requests if request["context"] > LONG_CONTEXT)
        sums["long_dollars"] = sum(dollars(request) for request in self.requests if request["context"] > LONG_CONTEXT)
        sums["waits"] = sum(1 for tool in self.tools.values() if tool["wait"])
        return sums


def dollars(request):
    """A request's input side in dollars, at its own model's list price and read ratio."""
    base, read_ratio = PRICES.MODELS.get(request["model"], (0.0, PRICES.GENERAL_READ))
    tokens = (
        request["uncached"]
        + request["write5m"] * WRITE_FIVE_MINUTES
        + request["write1h"] * PRICES.WRITE_HOUR
        + request["read"] * read_ratio
    )
    return tokens * base / 1e6


def transcripts(paths):
    """Every transcript under the paths, each real file once."""
    seen = set()
    for path in paths:
        if os.path.isdir(path):
            names = sorted(os.listdir(path))
            files = [os.path.join(path, name) for name in names if name.endswith((".jsonl", ".output"))]
        else:
            files = [path]
        for file in files:
            real = os.path.realpath(file)
            if real in seen or not os.path.isfile(real) or os.path.getsize(real) == 0:
                continue
            seen.add(real)
            yield real


def thousands(value):
    """A token count in thousands, as the tables print it."""
    return f"{value / 1000:,.0f}k"


HEADER = (
    f"{'agent':<18}{'batch':>6}{'reqs':>6}{'tools':>6}{'out':>7}{'uncached':>9}{'write':>8}{'read':>10}"
    f"{'last':>7}{'max':>7}{'>200k':>6}{'$>200k':>7}{'$in':>9}{'waits':>6}"
)


def census_line(name, batch, sums):
    """One agent's, or one batch's, figures as a table row."""
    share = 100 * sums["long_dollars"] / sums["dollars"] if sums["dollars"] else 0
    return (
        f"{name:<18}{batch:>6}{sums['requests']:>6}{sums['tools']:>6}{thousands(sums['out']):>7}"
        f"{thousands(sums['uncached']):>9}{thousands(sums['write']):>8}{thousands(sums['read']):>10}"
        f"{thousands(sums['last']):>7}{thousands(sums['max']):>7}{sums['long']:>6}{share:>6.0f}%"
        f"{sums['dollars']:>9,.2f}{sums['waits']:>6}"
    )


def detail(agent, top):
    """The per-agent tables `--detail` prints."""
    agent.measure_results()
    tools = [tool for request in agent.requests for tool in request["tools"]]
    first = agent.requests[0]["context"] if agent.requests else 0
    print(f"\n-- {agent.session or agent.name}: first request {thousands(first)} (system, tools, prompt)")
    row = []
    for count in range(0, len(tools), 25):
        row.append(f"{count}:{thousands(agent.requests[tools[count]['request']]['context'])}")
    if tools:
        row.append(f"{len(tools)}:{thousands(agent.requests[-1]['context'])}")
    print("   context by tool call  " + "  ".join(row))
    start = [tool for tool in tools[:40] if START_SET.search(tool["label"])]
    start_tokens = sum(tool.get("tokens", 0) for tool in start)
    start_carried = sum(tool.get("carried", 0) for tool in start)
    all_reads = sum(request["read"] for request in agent.requests)
    print(
        f"   start-set reads {len(start)}: {thousands(start_tokens)} entered, {thousands(start_carried)} re-read later; "
        f"the first request re-read later {thousands(first * agent.carriers(0))} "
        f"(of {thousands(all_reads)} read in all)"
    )
    waits = [tool for tool in tools if tool["wait"]]
    wait_reads = 0
    for tool in waits:
        following = tool["request"] + 1
        if following < len(agent.requests):
            wait_reads += agent.requests[following]["read"]
    kinds = {}
    for tool in waits:
        kinds[tool["wait"]] = kinds.get(tool["wait"], 0) + 1
    print(f"   waits {len(waits)} {kinds}: the requests after them read {thousands(wait_reads)}")
    ranked = sorted(tools, key=lambda tool: tool["chars"], reverse=True)[:top]
    for tool in ranked:
        print(
            f"   {tool['chars']:>9,} ch {thousands(tool.get('tokens', 0)):>6} "
            f"carried {thousands(tool.get('carried', 0)):>7}  {tool['label']}"
        )


def spans(agent):
    """A launching session cut into one span per batch, from its first launch to the next batch's."""
    starts = []
    for index, session, batch in agent.launches:
        key = batch or session
        if not starts or (key != starts[-1][1] and key not in [k for _, k in starts]):
            starts.append((index, key))
    print(f"\n{'batch from':>10}{'reqs':>6}{'tools':>6}{'read':>10}{'write':>9}{'$in':>9}{'end ctx':>9}{'max':>7}{'compact':>8}")
    for number, (begin, key) in enumerate(starts):
        end = starts[number + 1][0] if number + 1 < len(starts) else len(agent.requests)
        part = agent.requests[begin:end]
        if not part:
            continue
        compactions = sum(1 for _, at, _, _ in agent.compactions if begin <= at < end)
        print(
            f"{key:>10}{len(part):>6}{sum(len(r['tools']) for r in part):>6}"
            f"{thousands(sum(r['read'] for r in part)):>10}{thousands(sum(r['write'] for r in part)):>9}"
            f"{sum(dollars(r) for r in part):>9,.2f}{thousands(part[-1]['context']):>9}"
            f"{thousands(max(r['context'] for r in part)):>7}{compactions:>8}"
        )
    for stamp, at, pre, post in agent.compactions:
        print(f"   compaction at request {at} ({datetime.fromtimestamp(stamp):%Y-%m-%d %H:%M}): {pre} -> {post}")
    kinds = {}
    for tool in agent.tools.values():
        kinds.setdefault(tool["name"], [0, 0])
        kinds[tool["name"]][0] += 1
        kinds[tool["name"]][1] += tool["chars"]
    print("   tool results by tool (calls, characters): " + ", ".join(f"{k} {v[0]}/{v[1]:,}" for k, v in sorted(kinds.items(), key=lambda kv: -kv[1][1])))
    print(f"   task notifications: {agent.notification_chars:,} characters")


def main():
    """Read the arguments, count, print."""
    parser = argparse.ArgumentParser(description="Count where a session's and its rounds' tokens go.")
    parser.add_argument("paths", nargs="+", help="transcripts (.jsonl) or directories of them")
    parser.add_argument("--rounds", help="keep agents whose session is in A-B (or one number)")
    parser.add_argument("--detail", action="store_true", help="per-agent growth, waits, start-set reads, largest results")
    parser.add_argument("--top", type=int, default=10, help="largest tool results per agent under --detail")
    parser.add_argument("--spans", action="store_true", help="cut a launching session into one span per batch")
    args = parser.parse_args()
    low, high = None, None
    if args.rounds:
        bounds = args.rounds.split("-")
        low, high = int(bounds[0]), int(bounds[-1])
    agents = []
    for path in transcripts(args.paths):
        agent = Transcript(path)
        if low is not None and (agent.session is None or not low <= agent.session <= high):
            continue
        if agent.requests:
            agents.append(agent)
    if not agents:
        print("no transcript with a request matched", file=sys.stderr)
        return 1
    agents.sort(key=lambda agent: (agent.session or 0, agent.name))
    print(HEADER)
    batches = {}
    for agent in agents:
        sums = agent.totals()
        print(census_line(str(agent.session or agent.name[:17]), str(agent.batch or "-"), sums))
        group = batches.setdefault(agent.batch or 0, {key: 0 for key in sums})
        for key, value in sums.items():
            group[key] = max(group[key], value) if key == "max" else group[key] + value
    if len(agents) > 1:
        for batch, sums in sorted(batches.items()):
            print(census_line(f"batch {batch or '-'}", "all", sums))
    if args.detail:
        for agent in agents:
            detail(agent, args.top)
    if args.spans:
        for agent in agents:
            if agent.launches:
                spans(agent)
    return 0


if __name__ == "__main__":
    sys.exit(main())
