#!/usr/bin/env python3
"""Seed the `shaping` fuzz target from the UCD's own bidirectional test cases and cursive text.

    python3 fuzz/seed_shaping.py fuzz/corpus/shaping

Two populations, both already in the tree. `data/unicode/BidiCharacterTest.txt` states about
ninety thousand lines of text with a paragraph direction each — the file `pdf_font::shaping`
is measured against (ADR 1413) — and every hundredth line becomes a seed, its direction in the
target's own first byte (0 finds it by rules P2 and P3, 1 is left to right, 2 right to left).
And the Arabic letters `data/unicode/ArabicShaping.txt` gives a joining type, strung into words
with marks, a lam before an alef, digits and brackets between them, because the bidirectional
file holds almost no cursive text and joining is the half of the module it would not reach
(ADR 1414). A fixed generator seed, so a re-run writes the same files. Seeds are named by
content, so a re-run adds nothing.
"""

import hashlib
import os
import random
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def write(out, data):
    name = hashlib.sha256(data).hexdigest()[:40]
    path = os.path.join(out, name)
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "fuzz", "corpus", "shaping")
    os.makedirs(out, exist_ok=True)
    added = 0

    # The test file's direction field: 0 left to right, 1 right to left, 2 auto.
    direction = {"0": 1, "1": 2, "2": 0}
    with open(os.path.join(ROOT, "data", "unicode", "BidiCharacterTest.txt"), encoding="utf-8") as f:
        cases = [line for line in f if line.strip() and not line.startswith("#")]
    for line in cases[::100]:
        fields = line.split(";")
        text = "".join(chr(int(cp, 16)) for cp in fields[0].split())
        added += write(out, bytes([direction.get(fields[1], 0)]) + text.encode("utf-8"))

    letters = []
    with open(os.path.join(ROOT, "data", "unicode", "ArabicShaping.txt"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            code, _, kind, _ = [field.strip() for field in line.split(";")]
            if kind in ("D", "R", "L", "C"):
                letters.append(chr(int(code, 16)))
    marks = [chr(cp) for cp in range(0x064B, 0x0653)]
    between = [" ", "(", ")", "1", "2", "‍", "‌", "a", "\n", " "]
    lam, alefs = "ل", ["ا", "آ", "أ", "إ"]
    generator = random.Random(1293)
    for _ in range(400):
        text = []
        for _ in range(generator.randint(1, 12)):
            roll = generator.random()
            if roll < 0.6:
                text.append(generator.choice(letters))
            elif roll < 0.75:
                text.append(generator.choice(marks))
            elif roll < 0.85:
                text.append(lam + generator.choice(marks + [""]) + generator.choice(alefs))
            else:
                text.append(generator.choice(between))
        added += write(out, bytes([generator.randint(0, 2)]) + "".join(text).encode("utf-8"))

    print(f"seed_shaping: {added} new seeds in {out}")


if __name__ == "__main__":
    main()
