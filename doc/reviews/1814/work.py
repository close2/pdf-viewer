#!/usr/bin/env python3
"""Review 1814's instrument: what a round's tool calls cost, by the kind of work each one was.

A request re-reads its whole context, so a tool call costs two things: the request it adds (its
context, read from the cache) and its result, carried by every later request of the round. This
prices each call as `context x read` plus `result tokens x later requests x read` plus the
result's one cache write, and sums the calls by kind — a cargo build or test, an export, a wait,
a Markdown read (whole or as a range, and by which document), a code read, an edit, git, a
subagent — so that a rule that removes a kind of call can be priced before it is adopted.

Tokens per character are fitted once from the context's growth between consecutive requests. Prices are `tools/cache-census.py`'s table (Opus 5.5 $4 per million input
tokens, a read at 0.05 of that, a 5-minute write 1.25 times; Sonnet 5.5 $2). It prints counts,
kinds and paths, never what a file or a result held.

    python3 -I -B doc/reviews/1814/work.py <dir-of-round-transcripts> [--by-round] [--paths N]
    python3 -I -B doc/reviews/1814/work.py <dir> --reds | --where | --fit | --first
"""

import argparse
import glob
import json
import os
import re
from collections import defaultdict
from datetime import datetime

PRICE = {"opus": 4.0, "sonnet": 2.0}
READ, WRITE = 0.05, 1.25

DOC_CLASSES = [
    ("brief, prompt, role", re.compile(r"/home/AI/.*(brief|prompts)[^/]*\.md|scratchpad/prompts-|agents/round\.md")),
    ("spec doc/md", re.compile(r"doc/md/")),
    ("ledger", re.compile(r"doc/conformance/|ledger")),
    ("adr", re.compile(r"doc/adr/")),
    ("trap/habit", re.compile(r"doc/(traps|habits)/")),
    ("CLAUDE.md", re.compile(r"CLAUDE\.md")),
    ("todo", re.compile(r"doc/todo/")),
    ("history", re.compile(r"doc/history/")),
    ("rfc/review/question", re.compile(r"doc/(rfc|reviews|questions)/")),
    ("environment/handover/plan", re.compile(r"doc/(environment|HANDOVER|PLAN|state-of-play|crate-map|orchestrator-handover)")),
    ("other md", re.compile(r"\.md\b")),
]
RANGE = re.compile(r"sed -n|head -|tail -|grep|awk")
MEASURE = re.compile(r"census|compare_raster|magick|\bhold|arms?[-./]|drive|quorra|xwd|corpus|bounded\.sh|state\.sh|"
                     r"perf |callgrind|valgrind|examples/|target/.*/(debug|release)/")


def build_kind(cmd):
    """The build-and-test sub-kind of a command, or None when it is none of them."""
    if "batch.sh export" in cmd or ("cargo-target" in cmd and "reflink" in cmd):
        return "build/test: export"
    if "cargo test -p conformance" in cmd:
        return "build/test: conformance"
    if re.search(r"cargo (\+\S+ )?(clippy|fmt)|rustfmt", cmd):
        return "build/test: tier 1"
    if re.search(r"cargo (\+\S+ )?(test|nextest)|batch\.sh gates", cmd):
        return "build/test: tests"
    if re.search(r"cargo (\+\S+ )?(build|run|check|bench|doc)", cmd):
        return "build/test: build/run"
    return None


def kind(name, inp, logs):
    """The kind of work a tool call was, and for a Markdown read whether it was whole or a range.

    `logs` maps a log's file name to the kind of the command `tools/wait.sh --start` began into
    it, so that a later poll of that log, or a read of it, is counted with the work it waits on."""
    if name == "Read":
        path = inp.get("file_path", "")
        if path.endswith(".md"):
            return classify_read(path, "offset" in inp or "limit" in inp)
        return "code read", None
    if name in ("Edit", "Write", "NotebookEdit"):
        return "edit/write", None
    if name in ("Agent", "Task"):
        return "subagent", None
    if name in ("Grep", "Glob"):
        return "code search", None
    if name != "Bash":
        return "harness", None
    cmd = inp.get("command", "")
    start = re.search(r"wait\.sh --start (\S+) -- (.*)", cmd, re.S)
    if start:
        inner = start.group(2)
        k = build_kind(inner) or ("measure" if MEASURE.search(inner) else "other work")
        logs[log_name(start.group(1))] = k
        return "started: " + k, None
    poll = re.search(r"wait\.sh (\S+)", cmd)
    if poll or re.search(r"\bsleep \d", cmd):
        k = (logs.get(log_name(poll.group(1))) or by_name(poll.group(1))) if poll else "sleep"
        return "wait on: " + k, None
    k = build_kind(cmd)
    if k:
        return k, None
    if "round.sh" in cmd:
        return "md: round.sh list", "ranged"
    for log in re.findall(r"[\w./-]+\.(?:log|out)\b", cmd):
        if os.path.basename(log) in logs:
            return "log read: " + logs[os.path.basename(log)], None
    md = re.findall(r"[\w./~-]+\.md\b", cmd)
    if md and re.search(r"\b(cat|sed|head|tail|grep|awk|wc)\b", cmd):
        return classify_read(md[0], bool(RANGE.search(cmd)))
    if MEASURE.search(cmd):
        return "measure", None
    if re.match(r"\s*(cd \S+ *(&&|;) *)?git ", cmd):
        return "git", None
    if re.search(r"\b(cat|sed -n|head|tail)\b", cmd) and not re.search(r"\bgrep\b", cmd):
        return "code read", None
    if re.search(r"\b(grep|rg|find|ls)\b", cmd):
        return "code search", None
    return "other work", None


def log_name(path):
    """A log's file name, without the shell punctuation a command line puts after it."""
    return os.path.basename(path.rstrip(";|&)'\""))


def by_name(path):
    """The kind of a log no `--start` in this transcript named, from the words in its file name."""
    name = log_name(path)
    if re.search(r"conf|clippy|build|test|nextest|gates|doc|check|fmt|tier|rustdoc|t\d", name):
        return "build/test (by log name)"
    if re.search(r"hold|arm|census|state|drive|clock|cg|vera|plant|srt|fuzz|after|head|rel|corpus", name):
        return "measure (by log name)"
    return "unknown"


def classify_read(path, ranged):
    for label, pattern in DOC_CLASSES:
        if pattern.search(path):
            return "md: " + label, "ranged" if ranged else "whole"
    return "code read", None


def result_chars(content):
    if isinstance(content, str):
        return len(content)
    if isinstance(content, list):
        return sum(len(b.get("text", "")) if isinstance(b, dict) else len(str(b)) for b in content)
    return 0


def stamp(value):
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except (AttributeError, ValueError):
        return None


def load(path, keep_text=False):
    """The round's model, its requests in order, and its tool calls.

    A request is a distinct `message.id`, carrying its context (uncached + written + read) and the
    characters it added: its thinking (by signature length), its tool inputs and text, and the
    results and attachments that came back before the next request."""
    requests, calls, by_id, current = [], [], {}, None
    model = "opus"
    for line in open(path, encoding="utf-8"):
        try:
            d = json.loads(line)
        except ValueError:
            continue
        msg = d.get("message") or {}
        kind_ = d.get("type")
        if kind_ == "assistant":
            if "sonnet" in str(msg.get("model", "")):
                model = "sonnet"
            mid = msg.get("id")
            if current is None or mid != current["id"]:
                u = msg.get("usage") or {}
                ctx = u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0) + u.get("cache_read_input_tokens", 0)
                current = {"id": mid, "ctx": ctx, "sig": 0, "inp": 0, "att": 0, "res": 0, "calls": []}
                requests.append(current)
            for b in msg.get("content") or []:
                if not isinstance(b, dict):
                    continue
                if b.get("type") == "thinking":
                    current["sig"] += len(b.get("signature", ""))
                elif b.get("type") == "text":
                    current["inp"] += len(b.get("text", ""))
                elif b.get("type") == "tool_use":
                    call = {"req": len(requests) - 1, "name": b.get("name"), "input": b.get("input") or {},
                            "chars": 0, "inp": len(json.dumps(b.get("input"))), "t0": stamp(d.get("timestamp")), "wall": 0.0}
                    calls.append(call)
                    current["calls"].append(call)
                    by_id[b.get("id")] = call
        elif kind_ == "user" and isinstance(msg.get("content"), list):
            for b in msg["content"]:
                if isinstance(b, dict) and b.get("type") == "tool_result" and b.get("tool_use_id") in by_id:
                    call = by_id[b["tool_use_id"]]
                    call["chars"] = result_chars(b.get("content"))
                    requests[call["req"]]["res"] += call["chars"]
                    if keep_text:
                        content = b.get("content")
                        call["text"] = content if isinstance(content, str) else " ".join(
                            x.get("text", "") for x in content or [] if isinstance(x, dict))
                    t1 = stamp(d.get("timestamp"))
                    if t1 and call["t0"]:
                        call["wall"] = max(0.0, t1 - call["t0"])
        elif kind_ == "attachment" and current is not None:
            a = d.get("attachment") or {}
            if a.get("type") != "prompt_snapshot":
                current["att"] += len(json.dumps(a))
    return model, requests, calls


# Tokens per character, fitted by least squares over batches 117-121's 6 534 consecutive request
# pairs (review 1814 section 1.2): a request's context minus the one before it, against the
# characters added between them.
TOK_RESULT, TOK_INPUT, TOK_SIGNATURE, TOK_ATTACH = 0.382, 0.383, 0.258, 0.275


def price(model, requests, calls):
    """Splits the round's input bill over its calls, so the parts sum to the bill.

    Each request costs its fixed first context (`requests[0]`) read once, and everything it added
    is read by every later request and written once. A request's fixed part, thinking and text go
    to its calls in equal shares; a result and a tool input go to their own call."""
    base = PRICE[model] / 1e6
    n = len(requests)
    first = requests[0]["ctx"] if requests else 0
    for r, req in enumerate(requests):
        later = n - r - 1
        rate = base * (later * READ + WRITE)
        shared = base * first * READ + (req["sig"] * TOK_SIGNATURE + req["att"] * TOK_ATTACH) * rate
        k = max(1, len(req["calls"]))
        for c in req["calls"]:
            c["tokens"] = c["chars"] * TOK_RESULT
            c["usd"] = shared / k + (c["tokens"] + c["inp"] * TOK_INPUT) * rate


FAIL = re.compile(r"error\[E\d+\]|^error(:| aborting)|test result: FAILED|FAILED|panicked at|^warning: |Diff in |"
                  r"exit[= ]?(1|101)\b|could not compile", re.M)
EDITS = re.compile(r"sed -i|python3 .*write|cat > |tee ")


def reds(paths):
    """Build-and-test results that show a failure, by whether they name a file the round had edited.

    A failure naming a file the round wrote is its own gate finding its own defect; one naming only
    files it did not write is a sibling's red or an old one, in the shared tree."""
    tally = defaultdict(int)
    for path in paths:
        _, _, calls = load(path, keep_text=True)
        edited, logs = set(), {}
        own = other = 0
        for c in calls:
            inp = c["input"]
            if c["name"] in ("Edit", "Write"):
                edited.add(os.path.basename(inp.get("file_path", "")))
            cmd = inp.get("command", "") if c["name"] == "Bash" else ""
            if EDITS.search(cmd):
                edited.update(os.path.basename(f) for f in re.findall(r"[\w/.-]+\.rs", cmd))
            k, _ = kind(c["name"], inp, logs)
            if "build/test" not in k or k.startswith("started"):
                continue
            tally["result reads"] += 1
            text = c.get("text", "")
            if not FAIL.search(text):
                continue
            files = {os.path.basename(f) for f in re.findall(r"[\w/.-]+\.rs", text)}
            key = "own file" if files & edited else "other files only" if files else "no file named"
            tally["failing, " + key] += 1
            own += key == "own file"
            other += key == "other files only"
        tally["rounds with an own-file failure"] += own > 0
        tally["rounds with an other-file failure"] += other > 0
    for k, v in tally.items():
        print(f"{k:40} {v}")


def where(paths):
    """Build-and-test calls in the shared tree against those in a round's own export or copy."""
    tally = defaultdict(lambda: [0, 0.0, 0.0, set()])
    for path in paths:
        model, requests, calls = load(path)
        if not requests:
            continue
        price(model, requests, calls)
        logs = {}
        for c in calls:
            k, _ = kind(c["name"], c["input"], logs)
            if "build/test" not in k:
                continue
            cmd = c["input"].get("command", "") if c["name"] == "Bash" else ""
            place = ("export or copy" if re.search(r"scratchpad/r\d+/[\w-]+(/| |$)|cargo-target/r\d+", cmd)
                     else "shared tree")
            t = tally[place]
            t[0] += 1
            t[1] += c["usd"]
            t[2] += c["wall"]
            t[3].add(path)
    for place, (n, usd, wall, rounds) in tally.items():
        print(f"{place:16} calls {n:5} ${usd:7.2f} wall {wall / 3600:6.2f} h rounds {len(rounds)}")


def fit(paths):
    """Least squares of a request's context minus the one before it on the characters added between.

    The coefficients are the tokens per character of a result, a tool input or text, a thinking
    block's signature and an attachment; the constant is a request's own overhead."""
    xs, ys = [], []
    for path in paths:
        _, requests, _ = load(path)
        for a, b in zip(requests, requests[1:]):
            dy = b["ctx"] - a["ctx"]
            if 0 <= dy <= 60_000:
                xs.append([a["res"], a["inp"] + sum(c["inp"] for c in a["calls"]), a["sig"], a["att"], 1.0])
                ys.append(dy)
    k = 5
    m = [[sum(x[i] * x[j] for x in xs) for j in range(k)] for i in range(k)]
    v = [sum(x[i] * y for x, y in zip(xs, ys)) for i in range(k)]
    for i in range(k):
        for j in range(i + 1, k):
            f = m[j][i] / m[i][i]
            for col in range(i, k):
                m[j][col] -= f * m[i][col]
            v[j] -= f * v[i]
    coef = [0.0] * k
    for i in reversed(range(k)):
        coef[i] = (v[i] - sum(m[i][col] * coef[col] for col in range(i + 1, k))) / m[i][i]
    parts = [coef[i] * sum(x[i] for x in xs) for i in range(k)]
    names = ["result", "tool input and text", "thinking (signature)", "attachment", "per request"]
    print(f"{len(xs)} consecutive request pairs, growth {sum(ys)} tokens")
    for name, c, part in zip(names, coef, parts):
        print(f"{name:24} {c:8.3f} tokens/char  {part:12.0f} tokens  {100 * part / sum(ys):5.1f} %")


def first(paths):
    """The first request's context by agent type and model, from each transcript's `.meta.json`."""
    rows = defaultdict(list)
    for path in paths:
        _, requests, _ = load(path)
        meta = {}
        try:
            with open(path.removesuffix(".jsonl") + ".meta.json", encoding="utf-8") as handle:
                meta = json.load(handle)
        except (OSError, ValueError):
            pass
        if requests:
            rows[(meta.get("agentType", "?"), meta.get("model") or "inherited")].append(requests[0]["ctx"])
    for (agent_type, model), values in sorted(rows.items()):
        print(f"{agent_type:16} {model:10} agents {len(values):3} first request {min(values)}-{max(values)} tokens")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("--by-round", action="store_true")
    ap.add_argument("--paths", type=int, default=0, help="the N Markdown paths read whole that cost most")
    ap.add_argument("--reds", action="store_true", help="failing build-and-test results, own file or not")
    ap.add_argument("--where", action="store_true", help="build-and-test in the shared tree or an export")
    ap.add_argument("--fit", action="store_true", help="tokens per character, fitted")
    ap.add_argument("--first", action="store_true", help="first request by agent type")
    a = ap.parse_args()
    files = sorted(glob.glob(os.path.join(a.dir, "*.jsonl")))
    for flag, run in ((a.reds, reds), (a.where, where), (a.fit, fit), (a.first, first)):
        if flag:
            run(files)
            return
    total = defaultdict(lambda: [0, 0, 0.0, 0.0])
    paths = defaultdict(lambda: [0, 0, 0.0])
    rounds = 0
    for path in sorted(glob.glob(os.path.join(a.dir, "*.jsonl"))):
        model, requests, calls = load(path)
        if not requests:
            continue
        rounds += 1
        price(model, requests, calls)
        mine = defaultdict(lambda: [0, 0, 0.0, 0.0])
        logs = {}
        for c in calls:
            k, sub = kind(c["name"], c["input"], logs)
            key = k + (" (" + sub + ")" if sub else "")
            for bucket in (total[key], mine[key]):
                bucket[0] += 1
                bucket[1] += c["chars"]
                bucket[2] += c["usd"]
                bucket[3] += c["wall"]
            if sub == "whole":
                p = c["input"].get("file_path") or (re.findall(r"[\w./~-]+\.md\b", c["input"].get("command", "")) or ["?"])[0]
                p = re.sub(r"^/home/AI/pdf-viewer-rounds/|^/home/cl/projects/pdf-viewer/", "", p)
                p = re.sub(r"^/home/AI/.*/", "/home/AI/…/", p)
                paths[p][0] += 1
                paths[p][1] += c["chars"]
                paths[p][2] += c["usd"]
        if a.by_round:
            name = os.path.basename(path)[6:23]
            usd = sum(c["usd"] for c in calls)
            cargo = sum(v[2] for k, v in mine.items() if "build/test" in k)
            md = sum(v[2] for k, v in mine.items() if k.startswith("md:"))
            print(f"{name} {model:6} calls {len(calls):4} $ {usd:6.2f} "
                  f"$build/test {cargo:5.2f} $md {md:5.2f} exports {mine['build/test: export'][0]}")
    grand = sum(v[2] for v in total.values())
    print(f"rounds {rounds}  priced ${grand:.2f}")
    print(f"{'kind':42} {'calls':>6} {'chars':>10} {'$':>8} {'share':>6} {'wall h':>7}")
    for k, v in sorted(total.items(), key=lambda kv: -kv[1][2]):
        print(f"{k:42} {v[0]:6} {v[1]:10} {v[2]:8.2f} {100 * v[2] / grand:5.1f}% {v[3] / 3600:7.2f}")
    families = defaultdict(lambda: [0, 0.0, 0.0])
    for k, v in total.items():
        f = ("build and test" if "build/test" in k else "measure, drive, hold" if "measure" in k
             else "Markdown read" if k.startswith("md:") else "code read and search" if k.startswith("code")
             else "wait, other" if k.startswith("wait") else k)
        families[f][0] += v[0]
        families[f][1] += v[2]
        families[f][2] += v[3]
    print("\nfamilies")
    for f, v in sorted(families.items(), key=lambda kv: -kv[1][1]):
        print(f"{f:42} {v[0]:6} {v[1]:8.2f} {100 * v[1] / grand:5.1f}% {v[2] / 3600:7.2f}")
    if a.paths:
        print("\nMarkdown read whole, costliest paths")
        for p, v in sorted(paths.items(), key=lambda kv: -kv[1][2])[: a.paths]:
            print(f"{p[:70]:70} {v[0]:4} {v[1]:9} {v[2]:7.2f}")


if __name__ == "__main__":
    main()
