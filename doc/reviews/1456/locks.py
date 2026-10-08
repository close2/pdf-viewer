#!/usr/bin/env python3
"""Review 1401 section 1.1's unmeasured term: the lock's queue, per batch, from the wrapper's log.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1456/locks.py [/home/AI/heavy-walk.log]
One line per hold (ADRs 1646, 1674, 1684). A round's own holds are `round=<4 digits>`; the merge's
are `round=batch-*` or `round=check`. Prints per batch: holds, summed wait and hold for rounds and
for the merge, the longest single wait and whose hold it queued behind.
"""
import re, sys, collections, datetime
path = sys.argv[1] if len(sys.argv) > 1 else "/home/AI/heavy-walk.log"
agg = collections.OrderedDict()
spans = {}
for line in open(path, encoding="utf-8", errors="replace"):
    b = re.search(r"batch=(\S+)", line); r = re.search(r"round=(\S+)", line)
    w = re.search(r"wait=([\d.]+)s", line); h = re.search(r"hold=([\d.]+)s", line)
    if not (b and r and w and h):
        continue
    ts = datetime.datetime.fromisoformat(line.split()[0]).timestamp()
    if re.fullmatch(r"\d{4}", r.group(1)) and float(w.group(1)) > 0:
        spans.setdefault((b.group(1), r.group(1)), []).append((ts, ts + float(w.group(1))))
    who = "round" if re.fullmatch(r"\d{4}", r.group(1)) else "arms" if r.group(1) == "arms" else "merge"
    a = agg.setdefault(b.group(1), {"round": [0, 0.0, 0.0], "merge": [0, 0.0, 0.0], "arms": [0, 0.0, 0.0], "worst": (0.0, "")})
    a[who][0] += 1; a[who][1] += float(w.group(1)); a[who][2] += float(h.group(1))
    if who == "round" and float(w.group(1)) > a["worst"][0]:
        beh = re.search(r"behind=(\S+)", line)
        a["worst"] = (float(w.group(1)), f"round {r.group(1)} behind {beh.group(1) if beh else '?'}")
print("batch               round holds  wait_s   hold_s | merge holds  wait_s  hold_s | arms hold_s | worst round wait")
tot = [0, 0.0, 0.0]
for k, a in agg.items():
    if not k.startswith("batch-"):
        continue
    ro, me = a["round"], a["merge"]
    tot = [tot[0] + ro[0], tot[1] + ro[1], tot[2] + ro[2]]
    print(f"{k:<19} {ro[0]:>11} {ro[1]:>7.0f} {ro[2]:>8.0f} | {me[0]:>11} {me[1]:>7.0f} {me[2]:>7.0f} | {a['arms'][2]:>11.0f} | "
          f"{a['worst'][0]:.0f} s, {a['worst'][1]}")
print(f"rounds' holds in all: {tot[0]}, wait {tot[1]:.0f} s, hold {tot[2]:.0f} s")

# A round may queue several holds at once (a background walk beside a foreground one), so the summed
# wait overstates the round's own queued wall; the union of each round's wait intervals does not.
def union(iv):
    total, end = 0.0, None
    for a, b in sorted(iv):
        if end is None or a > end:
            total += b - a; end = b
        elif b > end:
            total += b - end; end = b
    return total
per_batch = collections.OrderedDict()
for (bt, rnd), iv in spans.items():
    per_batch.setdefault(bt, []).append((rnd, union(iv)))
print("queued wall per round (union of its waits), by batch:")
for bt, lst in per_batch.items():
    if bt.startswith("batch-"):
        print(f"  {bt}: {sum(u for _, u in lst):.0f} s in all; " +
              ", ".join(f"{r} {u:.0f}" for r, u in sorted(lst)))
