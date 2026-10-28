"""Ranks the residue of a `tools/safedocs survey` log by documents (ADR 2508).

    python3 -I -B tools/survey-residue.py <survey log>

Reads the log's `  incomplete: <name>: [Kind { detail }, ...]` lines, folds every number to `N` and every
`/Name` to `/X` so that one defect reported with different operands is one row, and prints the forty
rows that name the most documents, each as `<documents> <kind> <detail>`. A document counts once per row.
"""
import collections
import re
import sys

documents_by_row = collections.defaultdict(set)
for line in open(sys.argv[1]):
    if not line.startswith("  incomplete: "):
        continue
    name, rest = line[len("  incomplete: "):].split(": ", 1)
    for kind, detail in re.findall(r"(\w+) \{ (.*?) \}(?:, |\])", rest):
        detail = re.sub(r"\d+", "N", detail)
        detail = re.sub(r"/[A-Za-z0-9_.+-]+", "/X", detail)
        documents_by_row[(kind, detail[:150])].add(name)
for (kind, detail), names in sorted(documents_by_row.items(), key=lambda row: -len(row[1]))[:40]:
    print(len(names), kind, detail)
