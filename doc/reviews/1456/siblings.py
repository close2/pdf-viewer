#!/usr/bin/env python3
"""Review 1401 section 1.4, re-taken: siblings, duplicated walks, re-takes, in the records.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1456/siblings.py <first-session> <last-session> [-v]
Per record (whitespace folded): does it name a sibling (sibling, neighbour, "slot <n>")? does a
`**Gates.**` sentence say a gate failed on another slot's file (a sentence holding a failure word
and a slot/sibling word)? does it export or build HEAD itself (`-head`, "export of HEAD", "HEAD's
tree")? does it re-take a `doc/performance.md` section 3e row? -v prints each matched sentence.
"""
import glob, re, sys
lo, hi = int(sys.argv[1]), int(sys.argv[2]); v = "-v" in sys.argv
FAIL = re.compile(r"\b(fails?|failed|exit(s|ed)? 101|101 on|red)\b", re.I)
WHO = re.compile(r"\b(slot [1-6]|sibling|neighbou?r|in-flight|mid-edit)", re.I)
HEAD = re.compile(r"r1\d{3}-head|export(ed|s)? (of )?HEAD|HEAD export|built HEAD|HEAD's (own )?(tree|build)", re.I)
SIXE = re.compile(r"(section|§) ?3e|performance\.md", re.I)
n = {"records": 0, "sibling": 0, "gate_on_sibling": 0, "head_export": 0, "perf_3e": 0}
for f in sorted(glob.glob("doc/history/*.md")):
    m = re.match(r"doc/history/(\d+)-", f)
    if not m or not lo <= int(m.group(1)) <= hi:
        continue
    t = re.sub(r"\s+", " ", open(f, encoding="utf-8").read())
    n["records"] += 1
    n["sibling"] += bool(re.search(r"sibling|neighbou?r|slot [1-6]", t, re.I))
    sents = re.split(r"(?<=[.;])\s", t)
    g = [s for s in sents if FAIL.search(s) and WHO.search(s)]
    if g:
        n["gate_on_sibling"] += 1
    h = [s for s in sents if HEAD.search(s)]
    n["head_export"] += bool(h)
    p = [s for s in sents if SIXE.search(s)]
    n["perf_3e"] += bool(p)
    if v:
        for k, ss in (("gate", g), ("head", h), ("3e", p)):
            for s in ss:
                print(f"{m.group(1)} [{k}] {s[:220]}")
print(n)
