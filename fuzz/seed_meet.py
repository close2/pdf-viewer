#!/usr/bin/env python3
"""Seed the `meet` fuzz target with the shapes the meet's own tests state, and with polygons
that coincide, cross and run along a pixel's sides.

    python3 fuzz/seed_meet.py fuzz/corpus/meet

The target's framing: a head byte (bit 0 a third set, bit 1 the fine grid, bits 2-3 and 4-5 the
pixel), then per set a shape byte (bit 0 even-odd, bit 1 a second subpath), and per subpath a
count byte (three plus it modulo six points) and the points. On the coarse grid a coordinate is
one byte, `byte / 16 - 4`, so a pixel edge at 0, 1, 2 or 3 is a whole number of sixteenths and
coincident edges are what the seeds are made of.
"""

import hashlib
import os
import random
import sys


def coarse(value):
    return max(0, min(255, round((value + 4) * 16)))


def subpath(points):
    return bytes([len(points) - 3]) + b"".join(bytes([coarse(x), coarse(y)]) for x, y in points)


def seed(pixel, sets):
    """`sets` is a list of (even_odd, [subpath points, ...])."""
    head = (1 if len(sets) == 3 else 0) | (pixel[0] << 2) | (pixel[1] << 4)
    out = bytes([head])
    for even_odd, subpaths in sets:
        out += bytes([(1 if even_odd else 0) | (2 if len(subpaths) == 2 else 0)])
        out += b"".join(subpath(p) for p in subpaths)
    return out


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    seeds = [
        # The meet's own tests: a horizontal edge through the pixel, coincident and disjoint
        # half-planes, and two triangles crossing inside one pixel.
        seed((2, 2), [(False, [[(-3, 2.2), (9, 2.2), (9, 9), (-3, 9)]]),
                      (False, [[(-3, -3), (2.6, -3), (2.6, 9), (-3, 9)]])]),
        seed((2, 3), [(False, [[(-3, -3), (2.6, -3), (2.6, 9), (-3, 9)]]),
                      (False, [[(-3, -3), (2.6, -3), (2.6, 9), (-3, 9)]])]),
        seed((2, 3), [(False, [[(-3, -3), (2.3, -3), (2.3, 9), (-3, 9)]]),
                      (False, [[(2.7, -3), (9, -3), (9, 9), (2.7, 9)]])]),
        seed((0, 0), [(False, [[(0, 0), (1, 1), (0, 1)]]),
                      (True, [[(1, 0), (1, 1), (0, 1)]])]),
        # A ring under each rule, an edge along the pixel's side, and three sets.
        seed((1, 1), [(True, [[(0, 0), (3, 0), (3, 3), (0, 3)], [(1.25, 1.25), (1.75, 1.25), (1.75, 1.75), (1.25, 1.75)]]),
                      (False, [[(1, 1), (2, 1), (2, 2)]])]),
        seed((1, 1), [(False, [[(0, 0), (3, 0), (3, 3), (0, 3)], [(1.25, 1.25), (1.75, 1.25), (1.75, 1.75), (1.25, 1.75)]]),
                      (False, [[(1, 0.5), (1, 2.5), (2.5, 1.5)]]),
                      (True, [[(0.5, 1.5), (2.5, 1.0), (2.5, 2.0)]])]),
    ]
    generator = random.Random(1495)
    for _ in range(40):
        sets = []
        for _ in range(generator.choice((2, 3))):
            paths = [[(generator.randint(-1, 5) / 2 + generator.choice((0, 0.0625, 0.5)),
                       generator.randint(-1, 5) / 2 + generator.choice((0, 0.0625, 0.5)))
                      for _ in range(generator.randint(3, 8))]
                     for _ in range(generator.choice((1, 1, 2)))]
            sets.append((generator.random() < 0.5, paths))
        seeds.append(seed((generator.randint(0, 2), generator.randint(0, 2)), sets))
    written = 0
    for data in seeds:
        name = os.path.join(directory, hashlib.sha256(data).hexdigest())
        if not os.path.exists(name):
            with open(name, "wb") as handle:
                handle.write(data)
            written += 1
    print(f"seed_meet.py: {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
