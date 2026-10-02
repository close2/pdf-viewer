#!/usr/bin/env python3
"""Seed the `find` fuzz target with readbacks that fold, decompose and carry marks, and needles
typed against them.

    python3 fuzz/seed_find.py fuzz/corpus/find

The target's framing: a byte stating the needle's length in bytes (modulo 32), the needle, and
the readback. The populations: every hundredth line of `data/unicode/BidiCharacterTest.txt` with
a needle cut out of its own text; and the cases `viewer_core::select::find` was written for — a
precomposed letter against its decomposition, a presentation form and a ligature against their
letters, Arabic and Hebrew with their marks against the bare letters (ADRs 1465, 1477), a soft
hyphen, a run of spaces — each with the needle typed both ways. A fixed generator seed, so a
re-run writes the same files.
"""

import hashlib
import os
import random
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")

CASES = [
    ("café au lait", "café"),
    ("café au lait", "café"),
    ("ﬁnd the ﬂow", "find"),
    ("ﻟﺎﻪ", "لاه"),
    ("كَتَبَ كُتُب", "كتب"),
    ("שָׁלוֹם", "שלום"),
    ("trans­parency group", "transparency"),
    ("transparency   group", "transparency group"),
    ("가나다 가", "가"),
    ("ISTANBUL İstanbul", "istanbul"),
    ("Straße STRASSE", "strasse"),
]


def write(out, data):
    path = os.path.join(out, hashlib.sha256(data).hexdigest()[:40])
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def framed(text, needle):
    typed = needle.encode("utf-8")[:31]
    return bytes([len(typed)]) + typed + text.encode("utf-8")


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    added = 0
    for text, needle in CASES:
        added += write(out, framed(text, needle))
        added += write(out, framed(needle, text))
    generator = random.Random(1495)
    with open(os.path.join(ROOT, "data", "unicode", "BidiCharacterTest.txt"), encoding="utf-8") as f:
        for number, line in enumerate(f):
            if line.startswith("#") or not line.strip() or number % 100:
                continue
            text = "".join(chr(int(point, 16)) for point in line.split(";")[0].split())
            if not text:
                continue
            start = generator.randrange(len(text))
            needle = text[start:start + generator.randint(1, 5)]
            added += write(out, framed(text, needle))
    print(f"seed_find.py: {added} new seeds in {out}")


if __name__ == "__main__":
    main()
