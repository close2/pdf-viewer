#!/usr/bin/env python3
"""Seed the `formcalc_cascade` fuzz target with forms whose calculations read one another.

    python3 fuzz/seed_formcalc_cascade.py fuzz/corpus/formcalc_cascade

The target's framing: one stimulus byte (even: the merge, odd: a person's entry into `F0`), then
the calculations of the fields `F0`, `F1`, ... separated by NUL bytes. The seeds are written for
this file: a chain read forwards and backwards, a cycle of two and of three, a self-read, a
converging cycle, a fan-out, a script that names every occurrence, one outside the subset and one
that does not parse.
"""

import hashlib
import os
import sys

FORMS = [
    ["F1 + 1", "F2 + 1", "F3 + 1", "1"],
    ["1", "F0 + 1", "F1 + 1", "F2 + 1"],
    ["F1 + 1", "F0 + 1"],
    ["F1 + 1", "F2 + 1", "F0 + 1"],
    ["F0 + 1"],
    ["F1 / 2 + 1", "F0 / 2 + 1"],
    ["F1 + F2 + F3 + F4", "F4 + 1", "F4 + 2", "F4 + 3", "7"],
    ["Sum(F[*])", "2", "3"],
    ["F1 = F0 + 1", "F0 = F1 + 1"],
    ["var i = 0 while (i < 100000) do i = i + 1 endwhile i", "F0", "F0"],
    ["F1.#value", "F0 &"],
    ["$ = F1", "$ = F0", "$ = F0 + F1"],
    ["", "", "", "", "", "", "", ""],
]


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    written = 0
    for form in FORMS:
        for mode in (0, 1):
            data = bytes([mode]) + "\0".join(form).encode("utf-8")
            name = os.path.join(directory, hashlib.sha256(data).hexdigest())
            if not os.path.exists(name):
                with open(name, "wb") as handle:
                    handle.write(data)
                written += 1
    print(f"seed_formcalc_cascade.py: {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
