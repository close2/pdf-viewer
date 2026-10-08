#!/usr/bin/env python3
"""Review 1401 section 1.2, re-taken: what each batch produced.

Usage: PYTHONDONTWRITEBYTECODE=1 python3 doc/reviews/1456/batches.py <first-commit> <last-commit>
For each batch commit C (those whose body has `Round durations`) against the batch commit P before it:
ledger status counts at C; per row, a status change versus a note/code/test-only change (tomllib over
`git show C:doc/conformance/ledger.toml`); ADRs and records added (`--diff-filter=A`), with lines;
`--numstat` binned: code = .rs/.sh/.py/.toml outside doc/, doc = under doc/ or .md, other = the rest.
"""
import collections, re, subprocess, sys, tomllib

def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout

def ledger(c):
    rows = tomllib.loads(git("show", f"{c}:doc/conformance/ledger.toml"))["clause"]
    return {r["clause"]: r for r in rows}

first, last = sys.argv[1], sys.argv[2]
commits = [l.split()[0] for l in git("log", "--reverse", "--format=%H", f"{first}~1..{last}").splitlines()]
commits = [c for c in commits if "Round durations" in git("log", "-1", "--format=%B", c)]
prev = git("log", "-1", "--format=%H", "--grep=Round durations", f"{commits[0]}~1").strip()
print("batch     status-moves note-only  ADRs(lines)  records(mean lines)  code+/-        doc+/-        other+/-   end: impl/partial/departed")
tot = collections.Counter()
for c in commits:
    a, b = ledger(prev), ledger(c)
    moves = [k for k in b if k in a and a[k]["status"] != b[k]["status"]]
    edits = [k for k in b if k in a and a[k]["status"] == b[k]["status"] and
             (a[k].get("note"), a[k].get("code"), a[k].get("test")) != (b[k].get("note"), b[k].get("code"), b[k].get("test"))]
    added = git("diff", "--diff-filter=A", "--name-only", prev, c).split()
    adrs = [f for f in added if re.match(r"doc/adr/\d+-", f)]
    recs = [f for f in added if re.match(r"doc/history/\d+-", f)]
    lines = lambda fs: [git("show", f"{c}:{f}").count("\n") for f in fs]
    bins = collections.defaultdict(lambda: [0, 0])
    for l in git("diff", "--numstat", prev, c).splitlines():
        ad, de, f = l.split("\t", 2)
        if ad == "-":
            continue
        k = "doc" if f.startswith("doc/") or f.endswith(".md") else \
            "code" if re.search(r"\.(rs|sh|py|toml)$", f) else "other"
        bins[k][0] += int(ad); bins[k][1] += int(de)
    st = collections.Counter(r["status"] for r in b.values())
    rl = lines(recs)
    print(f"{c[:8]}  {len(moves):>12} {len(edits):>9}  {len(adrs):>4}({sum(lines(adrs)):>5})  "
          f"{len(recs):>7}({(sum(rl)/len(rl) if rl else 0):5.1f})  "
          f"+{bins['code'][0]:>6}/-{bins['code'][1]:<6} +{bins['doc'][0]:>6}/-{bins['doc'][1]:<6} "
          f"+{bins['other'][0]:>6}/-{bins['other'][1]:<6} {st['implemented']}/{st['partial']}/{st['departed']}"
          + (f"  moved: {', '.join(f'{k} {a[k]['status']}->{b[k]['status']}' for k in moves)}" if 0 < len(moves) <= 6 else ""))
    tot.update(moves=len(moves), edits=len(edits), adrs=len(adrs), adr_lines=sum(lines(adrs)),
               recs=len(recs), rec_lines=sum(rl), code_add=bins["code"][0], code_del=bins["code"][1],
               doc_add=bins["doc"][0], doc_del=bins["doc"][1])
    prev = c
print("totals:", dict(tot))
