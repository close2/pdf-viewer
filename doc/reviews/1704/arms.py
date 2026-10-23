#!/usr/bin/env python3
"""Review 1704 section 1.4: which pages' frame digests moved between arms exports, link by link.

Usage: python3 -I -B doc/reviews/1704/arms.py <arms-dir> <arms-dir>...
Each directory is a `tools/batch.sh arms` export (`<lane>-<scale>x.tsv`: name, oracle ms, raster ms,
digest, mean error). Prints, for each consecutive pair and each of the six arms, the pages whose
digest differs and the pages present in one export only. A chain of two is the whole span's diff.
"""
import sys

ARMS = ["cpu-1x", "gpu-1x", "compute-1x", "cpu-4x", "gpu-4x", "compute-4x"]


def load(path):
    rows = {}
    with open(path, encoding="utf-8") as tsv:
        for text in tsv:
            fields = text.rstrip("\n").split("\t")
            if len(fields) >= 5:
                rows[fields[0]] = (fields[3], fields[4])
    return rows


chain = sys.argv[1:]
for before, after in zip(chain, chain[1:]):
    for arm in ARMS:
        old, new = load(f"{before}/{arm}.tsv"), load(f"{after}/{arm}.tsv")
        moved = [f"{n} ({old[n][1]}->{new[n][1]})" for n in sorted(old) if n in new and old[n][0] != new[n][0]]
        population = sorted(set(old) ^ set(new))
        print(f"{before} -> {after} {arm}: {len(old)}/{len(new)} pages, moved {len(moved)}"
              + (f": {', '.join(moved)}" if moved else "")
              + (f"; in one only: {', '.join(population)}" if population else ""))
