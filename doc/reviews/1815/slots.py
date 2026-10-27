#!/usr/bin/env python3
"""Review 1815's second instrument: where batches 109-121's round hours went, slot by slot.

    python3 -I -B doc/reviews/1815/slots.py <repository>

It reads two tables beside it. `records.tsv` is one line per record `doc/history/1704`-`1813`:
its kind, an outcome, whether its premise held, the ledger moves and archive census it states, and
whether it says it re-measured or overturned an earlier round's figure (`repeats`) or left part of
its contract to a later batch (`carried`). `briefs.tsv` is one line per slot of the thirteen batch
briefs, with the earliest batch whose brief carried the same object (`first_briefed`). **Both are
classifications made by reading, not counts**: they were written by a reader with the column rules
in their headers, and this script only adds them up.

It joins each record to the wall seconds its batch commit's "Round durations" line states, so the
time shares cover batches 109-120, the merged ones.
"""

import collections
import csv
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
COMMITS = ("e883ed13 3e08a117 aed9d0a1 9993fbc0 7f1f3f15 193ce42c ecb6bea0 004f8e73 298e1852 "
           "88339689 b46ff72e 60f975c8").split()
LANES = [("archive", "archive"), ("xfa", "xfa"), ("ledger", "clause"), ("clause", "clause"),
         ("pixels", "pixels"), ("oracle", "pixels"), ("host", "host"), ("measure", "measure"),
         ("instruments", "measure"), ("loop", "measure"), ("writer", "clause"),
         ("dependency", "other"), ("code-quality", "docs"), ("docs", "docs"), ("review", "docs"),
         ("script", "docs")]


def table(name):
    with open(os.path.join(HERE, name), encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t", quoting=csv.QUOTE_NONE))


def lane(kind):
    kind = kind.lower()
    return next((name for word, name in LANES if word in kind), "other")


def seconds(repo):
    """Each round's wall seconds, from the batch commits' "Round durations" lines."""
    found = {}
    group = re.compile(r"(?<![0-9])([0-9]{1,3}(?:[  ][0-9]{3})+)(?![0-9])")
    for commit in COMMITS:
        body = subprocess.run(["git", "-C", repo, "log", "-1", "--format=%B", commit],
                              capture_output=True, text=True, check=True).stdout.replace("\n", " ")
        body = group.sub(lambda m: re.sub(r"[  ]", "", m.group(1)), body)
        line = re.search(r"Round durations[^:]*:(.*?)(?:Co-Authored-By|$)", body)
        for part in (line.group(1) if line else "").split(";"):
            pair = re.search(r"\b(1[78][0-9]{2})\b\D{0,40}?([0-9]{3,6}) s\b", part)
            if pair:
                found[int(pair.group(1))] = int(pair.group(2))
    return found


def moved(record):
    """Whether the record states a status move, or a document freed."""
    if "->" in record["ledger"] or "→" in record["ledger"]:
        return "a ledger status moved"
    if re.search(r"freed [1-9]", record["freed_refused"]):
        return "a document freed"
    return "neither"


def main():
    repo = sys.argv[1]
    records, briefs, wall = table("records.tsv"), table("briefs.tsv"), seconds(repo)
    print("== per batch: records, outcomes, premise failed, repeats, carried; briefed slots and those carried")
    carried_slots = collections.Counter(b["batch"] for b in briefs if b["first_briefed"] != b["batch"])
    all_slots = collections.Counter(b["batch"] for b in briefs)
    for batch in sorted({r["batch"] for r in records}):
        mine = [r for r in records if r["batch"] == batch]
        outcomes = collections.Counter(r["outcome"] for r in mine)
        failed = sum(r["premise"].startswith("failed") for r in mine)
        repeats = sum(r["repeats"].startswith("Y") for r in mine)
        carried = sum(r["carried"].startswith("Y") for r in mine)
        print(f"{batch}: {len(mine)} records {dict(sorted(outcomes.items()))}; premise failed {failed}, "
              f"repeats {repeats}, carried {carried}; briefed {all_slots[batch]}, of an earlier object {carried_slots[batch]}")
    total = collections.Counter(r["outcome"] for r in records)
    print("all:", dict(sorted(total.items())), "repeats", sum(r["repeats"].startswith("Y") for r in records),
          "carried", sum(r["carried"].startswith("Y") for r in records), "of", len(records))
    print("\n== round seconds, batches 109-120, by lane and by what the slot moved")
    by_lane, slots, by_move, sum_all = collections.Counter(), collections.Counter(), collections.Counter(), 0
    for record in records:
        time = wall.get(int(record["session"]))
        if time is None:
            continue
        by_lane[lane(record["kind"])] += time
        slots[lane(record["kind"])] += 1
        by_move[moved(record)] += time
        sum_all += time
    for name, time in by_lane.most_common():
        print(f"{name:8} {slots[name]:3} slots {time:7} s {100 * time / sum_all:5.1f} %")
    for name, time in by_move.most_common():
        print(f"{name:22} {time:7} s {100 * time / sum_all:5.1f} %")
    print("\n== objects briefed in the most batches")
    spans = collections.defaultdict(set)
    for brief in briefs:
        for row in filter(None, (c.strip() for c in brief["clause_rows"].split(","))):
            if row != "-":
                spans[row].add(int(brief["batch"]))
    for row, batches in sorted(spans.items(), key=lambda item: -len(item[1]))[:8]:
        print(f"{row:14} {len(batches)} batches {sorted(batches)}")


if __name__ == "__main__":
    main()
