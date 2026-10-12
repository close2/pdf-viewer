#!/usr/bin/env python3
"""Review 1456 section 1.4, re-taken by review 1513: the records' premise and hypothesis paragraphs.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1513/premises.py <first-session> <last-session> [-v]
Changed from doc/reviews/1456/premises.py: from about record 1472 a record heads the paragraph with
its own verdict, `**Premise, held.**` or `**Hypothesis, half held.**`, which 1456's `**Premise.**`
pattern does not match, and ADR 1748 split the slot's line into a premise and a hypothesis. So each
of the two paragraphs is found by `**Premise` or `**Hypothesis` at a line start (to the next blank
line) and classed by the verdict its head states where it states one, else by 1456's phrases. -v prints
each paragraph, which is how BY_HAND below was read; the script prints where the two disagree.
"""
import glob, re, sys
MISSED = re.compile(r"[Nn]ot held|did not hold|[Dd]id not\b|[Nn]ot quite|[Hh]alf held|in part|"
                    r"not in its figure|parts held|[Tt]wo did not|wrong|was not|were not|changed")
HEAD = re.compile(r"^\*\*(Premises?|Hypothes[ei]s)\b([^*]*)\*\*", re.M)
# Review 1513's reading of records 1456-1508 by hand, paragraph by paragraph (premise, then
# hypothesis). Classes as review 1456 section 1.4 named them, with `more` = held but needed more.
BY_HAND = {
    ("premise", "held"): "1462 1464 1473 1474 1475 1476 1480 1481 1482 1483 1485 1486 1488 1489 1491 "
                         "1493 1494 1495 1497 1498 1499 1500 1502 1503 1504 1506",
    ("premise", "more"): "1458 1477 1492",
    ("premise", "missed: a figure"): "1456 1459 1466 1467 1468 1472 1490 1496 1501 1505 1507",
    ("premise", "missed: a place"): "1457 1461 1465 1478 1479 1484",
    ("premise", "missed: a mechanism"): "1460 1470 1487",
    ("premise", "missed: what a feature reaches"): "1463 1469 1471",
    ("hypothesis", "held"): "1463 1467 1469 1470 1473 1474 1475 1476 1477 1479 1492 1502 1507",
    ("hypothesis", "more"): "1482 1489 1491 1496 1497 1499 1500",
    ("hypothesis", "missed: a mechanism"): "1464 1478 1484 1485 1486 1487 1488 1490 1493 1494 1495 "
                                           "1498 1503 1504 1506",
    ("hypothesis", "missed: what a feature reaches"): "1462 1480 1501",
    ("hypothesis", "missed: a figure"): "1483 1505",
    ("hypothesis", "missed: a clause reading"): "1481",
}
lo, hi = int(sys.argv[1]), int(sys.argv[2]); verbose = "-v" in sys.argv
count = {k: {"held": [], "missed": [], "none": []} for k in ("premise", "hypothesis")}
for f in sorted(glob.glob("doc/history/*.md")):
    m = re.match(r"doc/history/(\d+)-", f)
    if not m or not lo <= int(m.group(1)) <= hi:
        continue
    text = open(f, encoding="utf-8").read()
    for kind in ("premise", "hypothesis"):
        p = None
        for h in HEAD.finditer(text):
            if h.group(1).lower().startswith(kind[:5]):
                end = re.search(r"\n\s*\n|\Z", text[h.start():])
                p = (h, text[h.start():h.start() + end.start()])
                break
        if not p:
            count[kind]["none"].append(m.group(1))
            continue
        head = p[0].group(2)
        k = ("missed" if MISSED.search(head) else "held") if re.search(r"held|hold|changed", head) \
            else ("missed" if MISSED.search(p[1]) else "held")
        count[kind][k].append(m.group(1))
        if verbose:
            print(f"--- {m.group(1)} {kind} [{k}]\n{p[1]}\n")
for kind, c in count.items():
    for k, v in c.items():
        print(f"{kind} {k}: {len(v)}  {' '.join(v)}")
hand = {(n, kd): ("missed" if k.startswith("missed") else "held")
        for (kd, k), v in BY_HAND.items() for n in v.split()}
off = [f"{n} {kd} (phrases {k}, by hand {hand[(n, kd)]})" for kd, c in count.items()
       for k, v in c.items() for n in v if (n, kd) in hand and hand[(n, kd)] != k]
for (kd, k), v in BY_HAND.items():
    print(f"by hand, {kd} {k}: {len(v.split())}")
if off:
    print("phrase classing against the hand reading (`more` counts as held): " + "; ".join(off))
