#!/usr/bin/env python3
"""Review 1401 section 1.1, re-taken: each batch commit's clock from its body.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1456/slots.py <first-commit> <last-commit>
Reads `git log --format='%H %ct' <first>~1..<last>` and every batch commit's `Round durations`
and gate wall-time lines. Digit groups written with a space ("7 497 s") are joined, "a + b s" is
summed, "about" is dropped. Busy share = sum of walls / (6 x slowest), as review 1401 took it.
"""
import re, subprocess, sys, statistics

first, last = sys.argv[1], sys.argv[2]
log = subprocess.run(["git", "log", "--reverse", "--format=%H %ct", f"{first}~1..{last}"],
                     capture_output=True, text=True, check=True).stdout.split("\n")
commits = [l.split() for l in log if l]
prev_t = int(subprocess.run(["git", "log", "-1", "--format=%ct", f"{first}~1"],
                            capture_output=True, text=True, check=True).stdout)
by_slot = {}
tools = []
every = []
rows = []
for sha, t in commits:
    t = int(t)
    body = subprocess.run(["git", "log", "-1", "--format=%B", sha], capture_output=True,
                          text=True, check=True).stdout
    flat = re.sub(r"\s+", " ", body)
    m = re.search(r"Round durations[^:]*:(.*?)(?:Co-Authored-By|$)", flat)
    if not m:
        prev_t = t
        continue
    head = re.search(r"Batch ([a-z-]+)", flat)
    text = re.sub(r"\b(\d{1,2}) (\d{3})\b", r"\1\2", m.group(1))  # "7 497" -> "7497"
    walls = []
    for part in text.split(";"):
        part = part.strip()
        mm = re.match(r"(\d{3,5})\s+(.*)", part)
        if not mm:
            continue
        # take every "<n> s" before the tool-use count
        seg = re.split(r", \d+\b(?! s)", mm.group(2))[0]
        nums = [int(n) for n in re.findall(r"(\d+) s\b", seg)]
        walls.append((int(mm.group(1)), sum(nums)))
        tu = re.search(r", (\d+)\b(?! s)", mm.group(2))
        if tu and sum(nums):
            tools.append((int(tu.group(1)), sum(nums)))
    g = re.findall(r"([\d ]+) s of gate wall time", flat)
    gate = int(g[-1].replace(" ", "")) if g else None
    secs = [w for _, w in walls]
    slow = max(secs)
    gap = (t - prev_t) / 3600 if prev_t else None
    busy = sum(secs) / (6 * slow)
    rows.append((sha[:8], head.group(1) if head else "?", gap, len(secs), sum(secs) / 3600,
                 slow / 3600, statistics.mean(secs) / 60, busy, gate,
                 (gap - slow / 3600) if gap else None))
    every.extend(secs)
    for i, (_, w) in enumerate(walls[:6]):
        by_slot.setdefault(i + 1, []).append(w)
    prev_t = t

print("commit   batch           gap_h  n  sum_h  slow_h  mean_min  busy  gate_s  tail_h")
for r in rows:
    print(f"{r[0]} {r[1]:<15} {'-' if r[2] is None else f'{r[2]:5.2f}'} {r[3]:2d} {r[4]:6.2f} "
          f"{r[5]:6.2f} {r[6]:8.0f} {r[7]:5.0%} {r[8] if r[8] else '-':>6} "
          f"{'-' if r[9] is None else f'{r[9]:5.2f}'}")
allw = every
busy = [r[7] for r in rows]
tails = [r[9] for r in rows if r[9] is not None]
print(f"rounds {len(allw)}: mean {statistics.mean(allw)/60:.0f} min, min {min(allw)/60:.0f}, "
      f"max {max(allw)/3600:.2f} h; busy {min(busy):.0%}-{max(busy):.0%} (mean {statistics.mean(busy):.0%}); "
      f"merge tail {min(tails):.2f}-{max(tails):.2f} h (median {statistics.median(tails):.2f})")
print("by slot position (mean min): " + ", ".join(
    f"{k}: {statistics.mean(v)/60:.0f}" for k, v in sorted(by_slot.items())))
gaps = [r[2] for r in rows]
slows = [r[5] for r in rows]
gates = [r[8] for r in rows if r[8]]
print(f"batch gap: mean {statistics.mean(gaps):.2f} h, median {statistics.median(gaps):.2f} h; "
      f"slowest round: mean {statistics.mean(slows):.2f} h; rounds over 3 h: "
      f"{sum(w > 10800 for w in allw)}, over 4 h: {sum(w > 14400 for w in allw)}; "
      f"gate wall {min(gates)}-{max(gates)} s")
print(f"tool uses per round: {min(t for t, _ in tools)}-{max(t for t, _ in tools)} "
      f"(mean {statistics.mean(t for t, _ in tools):.0f}); seconds per tool use "
      f"{min(w / t for t, w in tools):.0f}-{max(w / t for t, w in tools):.0f} "
      f"(overall {sum(w for _, w in tools) / sum(t for t, _ in tools):.0f})")
