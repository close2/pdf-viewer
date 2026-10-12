#!/usr/bin/env python3
"""Review 1513: each round's wall against the wall budget its brief gave it.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1513/budgets.py <first-commit> <last-commit>
A brief is /home/AI/batch-<first session>-brief.md, read for each `## Slot` heading's `session <n>`
and its `**Wall budget**: <h> h`. A round's wall is its batch commit body's `Round durations` figure,
read as doc/reviews/1456/slots.py reads it (digit groups joined, "a + b s" summed). Prints every round
with its budget and share, then the rounds over budget and the distribution of shares.
"""
import glob, re, subprocess, sys, statistics

def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout

budget = {}
for b in glob.glob("/home/AI/batch-1[3-9][0-9][0-9]-brief.md"):
    for part in re.split(r"(?m)^## Slot", open(b, encoding="utf-8").read())[1:]:
        s = re.search(r"session (\d{4})", part.splitlines()[0])
        w = re.search(r"\*\*Wall budget\*\*: (\d+(?:\.\d+)?) h", part)
        if s and w:
            budget[int(s.group(1))] = float(w.group(1)) * 3600
first, last = sys.argv[1], sys.argv[2]
rows = []
for sha in git("log", "--reverse", "--format=%H", f"{first}~1..{last}").split():
    flat = re.sub(r"\s+", " ", git("log", "-1", "--format=%B", sha))
    m = re.search(r"Round durations[^:]*:(.*?)(?:Co-Authored-By|$)", flat)
    if not m:
        continue
    text = re.sub(r"\b(\d{1,2}) (\d{3})\b", r"\1\2", m.group(1))
    for part in text.split(";"):
        mm = re.match(r"\s*(\d{4})\s+(.*)", part)
        if not mm:
            continue
        seg = re.split(r", \d+\b(?! s)", mm.group(2))[0]
        wall = sum(int(n) for n in re.findall(r"(\d+) s\b", seg))
        n = int(mm.group(1))
        rows.append((sha[:8], n, wall, budget.get(n)))
print("commit   round  wall_s  budget_s  share")
for sha, n, wall, b in rows:
    print(f"{sha} {n}  {wall:>6}  {'-' if b is None else f'{b:>8.0f}'}  {'-' if not b else f'{wall / b:5.0%}'}")
judged = [(n, w, b) for _, n, w, b in rows if b]
over = [(n, w, b) for n, w, b in judged if w > b]
print(f"rounds {len(rows)}, with a budget {len(judged)}; over it {len(over)}: "
      + ", ".join(f"{n} {w} s of {b:.0f}" for n, w, b in over))
shares = [w / b for _, w, b in judged]
print(f"share of budget: median {statistics.median(shares):.0%}, mean {statistics.mean(shares):.0%}, "
      f"over 75% {sum(s > 0.75 for s in shares)}, over 90% {sum(s > 0.9 for s in shares)}")
