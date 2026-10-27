#!/usr/bin/env python3
"""Review 1815's first instrument: what batches 109-120 did to the ledger, and what they wrote.

    python3 -I -B doc/reviews/1815/denominators.py <repository>

Git alone, no build. For each batch merge on `main` (the table `MERGES`, batch 108's merge first as
the base) it prints the ledger's rows by status, every status a row left and took since the
previous merge, the lines each category of path gained, and, over the whole span, the bytes those
lines carry. The working tree's ledger is read last, so a batch still running is seen as it stands.

A move is *intake* when a row leaves `out-of-scope` or `inapplicable` (a row the project newly
owes), *finish* when it reaches `implemented` from `partial`, `departed` or `reported`, and
*demotion* when it leaves `implemented`. The owner's own `Answer …` commits are left out of the
line counts, and so are the `doc/questions/A*` files anywhere.
"""

import collections
import re
import subprocess
import sys

MERGES = [
    (108, "ee365e4f"), (109, "e883ed13"), (110, "3e08a117"), (111, "aed9d0a1"), (112, "9993fbc0"),
    (113, "7f1f3f15"), (114, "193ce42c"), (115, "ecb6bea0"), (116, "004f8e73"), (117, "298e1852"),
    (118, "88339689"), (119, "b46ff72e"), (120, "60f975c8"),
]
CATEGORIES = ["code", "tests", "tools", "ledger", "records", "adr", "traps+habits", "reviews",
              "questions", "todo", "doc-other", "data"]
LEDGER = "doc/conformance/ledger.toml"


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=True).stdout


def rows(text):
    """Each row's clause number mapped to its (status, note)."""
    found = {}
    for block in text.split("[[clause]]")[1:]:
        clause = re.search(r'^clause = "([^"]+)"', block, re.M).group(1)
        status = re.search(r'^status = "([^"]+)"', block, re.M).group(1)
        note = re.search(r"^note = (.*)$", block, re.M)
        found[clause] = (status, note.group(1) if note else "")
    return found


def category(path):
    prefixes = [("doc/history/", "records"), ("doc/adr/", "adr"), ("doc/traps", "traps+habits"),
                ("doc/habits", "traps+habits"), ("doc/reviews/", "reviews"),
                ("doc/questions/", "questions"), ("doc/todo/", "todo")]
    for prefix, name in prefixes:
        if path.startswith(prefix):
            return name
    if path == LEDGER:
        return "ledger"
    if path.startswith("doc/"):
        return "doc-other"
    if path.startswith("tools/"):
        return "tools"
    if path.endswith(".rs"):
        tests = "/tests/" in path or "/benches/" in path or path.endswith(("_tests.rs", "/tests.rs"))
        return "tests" if tests else "code"
    return "data"


def kind_of(before, after):
    if before in ("out-of-scope", "inapplicable"):
        return "intake"
    if after == "implemented":
        return "finish"
    if before == "implemented":
        return "demotion"
    return "other"


def owned(path):
    return not path.split("/")[-1].startswith("A") or not path.startswith("doc/questions/")


def main():
    repo = sys.argv[1]
    states = [(batch, commit, rows(git(repo, "show", f"{commit}:{LEDGER}"))) for batch, commit in MERGES]
    with open(f"{repo}/{LEDGER}", encoding="utf-8") as handle:
        states.append(("tree", "worktree", rows(handle.read())))
    statuses = sorted({s for _, _, r in states for s, _ in r.values()})
    print("== rows by status at each merge")
    print("batch commit " + " ".join(statuses))
    for batch, commit, r in states:
        count = collections.Counter(s for s, _ in r.values())
        print(batch, commit, " ".join(str(count[s]) for s in statuses))
    print("\n== status moves since the previous merge (intake / finish / demotion / other), notes rewritten without a move")
    for (_, _, before), (batch, _, after) in zip(states, states[1:]):
        moves = [(c, before[c][0], s) for c, (s, _) in after.items() if c in before and before[c][0] != s]
        kinds = collections.Counter(kind_of(b, a) for _, b, a in moves)
        notes = sum(1 for c, (s, n) in after.items() if c in before and before[c] == (s, before[c][1]) and before[c][1] != n)
        print(f"{batch}: {len(moves)} moved {dict(kinds)}, {notes} notes rewritten; "
              + " ".join(f"{c}:{b}->{a}" for c, b, a in moves))
    for batch, _, r in (states[0], states[-2], states[-1]):
        print(f"partial at {batch}:", " ".join(c for c, (s, _) in r.items() if s == "partial"))
    print("\n== lines added per batch, by category (the owner's Answer commits left out)")
    print("batch " + " ".join(CATEGORIES))
    for (_, previous), (batch, commit) in zip(MERGES, MERGES[1:]):
        added = collections.Counter()
        for line in git(repo, "log", "--first-parent", "--format=%H %s", f"{previous}..{commit}").splitlines():
            sha, subject = line.split(" ", 1)
            if subject.startswith("Answer "):
                continue
            for stat in git(repo, "show", "--numstat", "--format=", sha).splitlines():
                plus, _, path = stat.split("\t")
                if plus != "-" and owned(path):
                    added[category(path)] += int(plus)
        print(batch, " ".join(str(added[c]) for c in CATEGORIES))
    print("\n== bytes on the added lines of the whole span's diff, by category")
    size, count, path = collections.Counter(), collections.Counter(), None
    for line in git(repo, "diff", "--no-renames", MERGES[0][1], MERGES[-1][1]).splitlines():
        if line.startswith("+++ "):
            path = None if line == "+++ /dev/null" else line[6:]
        elif path and line.startswith("+") and owned(path):
            size[category(path)] += len(line) - 1
            count[category(path)] += 1
    for name, value in size.most_common():
        print(f"{name:13} {count[name]:6} lines {value:8} bytes")


if __name__ == "__main__":
    main()
