#!/usr/bin/env python3
"""Review 1513: what a round's queued wall was queued behind, per batch, from the wrapper's log.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1513/lanes.py [/home/AI/heavy-walk.log]
doc/reviews/1456/locks.py sums each round's queued wall (the union of its waits); this splits that
wall by what held the lanes meanwhile. A line's time is its ask (tools/bounded.sh writes `asked_ms`),
so a hold runs from ask + wait for `hold` seconds on `lane` (1, 2 or 1+2; `-` is a wait stopped before
a grant). Each second of a round's queued wall is charged to the highest of the holds running then:
a clock run (both lanes), the batch's arms export, a long run, a large walk, a small walk, and `none`
where nothing logged held a lane (a wrapper between its look and its grant, or a holder not on a line
yet); and, apart, the share of it during which another round's (or the merge's) clock run was asked or held. Also the seconds lane 2 stood idle while a clock run waited at the gate. Prints per batch: the round's queued wall by that charge, the lane-seconds held by kind, the
clock runs and their median hold, and the median and longest queue of a small walk.
"""
import collections, datetime, re, statistics, sys
path = sys.argv[1] if len(sys.argv) > 1 else "/home/AI/heavy-walk.log"
RANK = ["clock", "arms", "long", "large", "small"]
lines = []
for line in open(path, encoding="utf-8", errors="replace"):
    f = dict(re.findall(r"(\w+)=(\S+)", line.split(" cmd=")[0]))
    if not {"batch", "round", "wait", "hold", "kind", "lane"} <= f.keys() or not f["batch"].startswith("batch-"):
        continue
    t = datetime.datetime.fromisoformat(line.split()[0]).timestamp()
    w, h = float(f["wait"].rstrip("s")), float(f["hold"].rstrip("s"))
    kind = "arms" if f["round"] == "arms" else f["kind"]
    lines.append(dict(batch=f["batch"], round=f["round"], ask=t, start=t + w, end=t + w + h,
                      wait=w, hold=h, kind=kind, lane=f["lane"]))
holds = [l for l in lines if l["lane"] != "-" and l["hold"] > 0]

def union(iv):
    out = []
    for a, b in sorted(iv):
        if out and a <= out[-1][1]:
            out[-1][1] = max(out[-1][1], b)
        else:
            out.append([a, b])
    return out

def charge(a, b):
    """Seconds of [a, b) by the highest-ranked hold running."""
    live = [h for h in holds if h["start"] < b and h["end"] > a]
    cuts = sorted({a, b, *(max(a, h["start"]) for h in live), *(min(b, h["end"]) for h in live)})
    got = collections.Counter()
    for x, y in zip(cuts, cuts[1:]):
        kinds = {h["kind"] for h in live if h["start"] <= x and h["end"] >= y}
        got[next((k for k in RANK if k in kinds), "none")] += y - x
    return got

def under_clock(a, b, me):
    """Seconds of [a, b) during which a clock run was queued or held: the gate stops new grants
    while a clock run waits for its two lanes (ADR 1756), so the hold below it is not the cause."""
    spans = union([(l["ask"], l["end"]) for l in lines
                   if l["kind"] == "clock" and l["lane"] != "-" and l["round"] != me and l["ask"] < b and l["end"] > a])
    return sum(max(0.0, min(b, y) - max(a, x)) for x, y in spans)

def frozen(ls):
    """Seconds a clock run of the batch held the gate (asked, not yet granted) while nothing held lane 2:
    the gate grants no walk either lane while a clock run waits (tools/bounded.sh, ADR 1684)."""
    gated = union([(l["ask"], l["start"]) for l in ls if l["kind"] == "clock" and l["lane"] != "-" and l["wait"] > 0])
    lane2 = union([(h["start"], h["end"]) for h in holds if h["lane"] in ("2", "1+2")])
    idle = 0.0
    for a, b in gated:
        idle += (b - a) - sum(max(0.0, min(b, y) - max(a, x)) for x, y in lane2)
    return idle

batches = collections.OrderedDict()
for l in lines:
    batches.setdefault(l["batch"], []).append(l)
print("batch            queued  " + "  ".join(f"{k:>6}" for k in RANK + ["none"])
      + " | lane-s held: " + " ".join(f"{k:>6}" for k in RANK) + " | clock runs (median s) | small walk queue median / max s")
for b, ls in batches.items():
    rounds = collections.defaultdict(list)
    for l in ls:
        if re.fullmatch(r"\d{4}", l["round"]) and l["wait"] > 0:
            rounds[l["round"]].append((l["ask"], l["ask"] + l["wait"]))
    got = collections.Counter()
    clocked = 0.0
    for me, iv in rounds.items():
        for a, c in union(iv):
            got.update(charge(a, c))
            clocked += under_clock(a, c, me)
    total = sum(got.values())
    if not total:
        continue
    held = collections.Counter()
    for l in ls:
        if l["lane"] != "-":
            held[l["kind"]] += l["hold"] * (2 if l["lane"] == "1+2" else 1)
    clocks = [l["hold"] for l in ls if l["kind"] == "clock" and re.fullmatch(r"\d{4}", l["round"]) and l["lane"] != "-"]
    small = [l["wait"] for l in ls if l["kind"] == "small" and re.fullmatch(r"\d{4}", l["round"])]
    print(f"{b:<16} {total:>6.0f}  " + "  ".join(f"{got[k] / total:>6.0%}" for k in RANK + ["none"])
          + " | " + " ".join(f"{held[k]:>6.0f}" for k in RANK)
          + f" | {len(clocks):>3} ({statistics.median(clocks) if clocks else 0:>5.0f})"
          + f" | {statistics.median(small) if small else 0:>6.0f} / {max(small) if small else 0:>5.0f}"
          + f" | another's clock run asked or holding {clocked / total:.0%}"
          + f" | lane 2 idle behind the gate {frozen(ls):.0f} s")
