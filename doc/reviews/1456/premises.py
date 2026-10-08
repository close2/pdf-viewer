#!/usr/bin/env python3
"""Review 1401 section 1.4, re-taken: the records' "Premise" paragraphs.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1456/premises.py <first-session> <last-session> [-v]
A paragraph opening `**Premise` (to the next blank line) is classed `held` unless it says that a part
did not hold (the phrases in MISSED), then `missed`; a record with no such paragraph is `none`.
-v prints each paragraph so the classing can be read, which is how it was checked.
"""
import glob, re, sys
MISSED = re.compile(r"[Nn]ot held|did not hold|[Dd]id not\b|[Nn]ot quite|[Hh]alf held|in part|"
                    r"not in its figure|parts held|[Tt]wo did not|Not held|wrong|was not|were not")
# Review 1456's reading of records 1402-1455 by hand, paragraph by paragraph, which is what its
# section 1.4 states: `more` held but needed more than stated; the misses by class. The phrase
# classing above is a first pass, and the script prints where the two disagree.
BY_HAND = {
    "held": "1402 1404 1413 1420 1425 1430 1435 1442 1445 1449 1451 1455",
    "more": "1403 1409 1410 1415 1421 1427",
    "missed: a figure": "1407 1419 1431 1437 1446",
    "missed: a place": "1412 1418 1443",
    "missed: a mechanism": "1405 1416 1417 1422 1428 1434 1453",
    "missed: what a feature reaches": "1408 1411 1414 1423 1424 1426 1429 1432 1433 1438 1439 1444 1450 1452",
}
lo, hi = int(sys.argv[1]), int(sys.argv[2]); verbose = "-v" in sys.argv
count = {"held": [], "missed": [], "none": []}
for f in sorted(glob.glob("doc/history/*.md")):
    m = re.match(r"doc/history/(\d+)-", f)
    if not m or not lo <= int(m.group(1)) <= hi:
        continue
    text = open(f, encoding="utf-8").read()
    p = re.search(r"^\*\*Premises?\.?\*\*.*?(?=\n\s*\n|\Z)", text, re.S | re.M)
    k = "none" if not p else "missed" if MISSED.search(p.group(0)) else "held"
    count[k].append(m.group(1))
    if verbose and p:
        print(f"--- {m.group(1)} [{k}]\n{p.group(0)}\n")
for k, v in count.items():
    print(f"{k}: {len(v)}  {' '.join(v)}")
hand = {n: ("missed" if k.startswith("missed") else "held") for k, v in BY_HAND.items() for n in v.split()}
off = [f"{n} (phrases {k}, by hand {hand[n]})" for k, v in count.items() for n in v
       if n in hand and hand[n] != k]
if off:
    print("phrase classing against the hand reading (`more` counts as held): " + "; ".join(off))
