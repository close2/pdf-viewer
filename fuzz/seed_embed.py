#!/usr/bin/env python3
"""Seed the `embed` fuzz target from the faces this machine offers.

    find /usr/share/fonts -size -128k \\( -name '*.ttf' -o -name '*.otf' \\) -print0 \\
      | xargs -0 python3 fuzz/seed_embed.py fuzz/corpus/embed

`pdf_font::embed::for_embedding` writes a face the machine supplies into a document (ADR 1414),
so the population is the machine's own sfnt files: `glyf` faces for §9.9.1's `/FontFile2` and
`CFF ` ones for the subsetter ADR 1449 added. Each seed is the target's framing — a count, that
many glyph indices two bytes each, the face — under three glyph sets: the first few glyphs, a
spread across the face, and none at all (which keeps `.notdef` alone). A face past 128 KiB is
skipped: the subsetter's cost is in the glyphs kept, not the face's size, and a large seed is
paid for on every execution.
"""

import hashlib
import os
import struct
import sys

MAX_SEED = 128 * 1024


def glyph_count(face):
    """`maxp`'s `numGlyphs`, or None for bytes with no table directory that names one."""
    if len(face) < 12:
        return None
    (tables,) = struct.unpack(">H", face[4:6])
    for index in range(tables):
        at = 12 + 16 * index
        record = face[at:at + 16]
        if len(record) < 16:
            return None
        tag, _, offset, _ = struct.unpack(">4sIII", record)
        if tag == b"maxp" and offset + 6 <= len(face):
            return struct.unpack(">H", face[offset + 4:offset + 6])[0]
    return None


def framed(glyphs, face):
    return bytes([len(glyphs)]) + b"".join(struct.pack(">H", g) for g in glyphs) + face


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    faces = written = 0
    for path in argv[2:]:
        try:
            with open(path, "rb") as handle:
                face = handle.read()
        except OSError:
            continue
        count = glyph_count(face)
        if count is None or len(face) > MAX_SEED:
            continue
        faces += 1
        spread = sorted({(count * k) // 9 for k in range(1, 9)} - {0})
        for glyphs in (list(range(1, min(count, 9))), spread, []):
            seed = framed(glyphs[:63], face)
            name = os.path.join(directory, hashlib.sha256(seed).hexdigest())
            if not os.path.exists(name):
                with open(name, "wb") as handle:
                    handle.write(seed)
                written += 1
    print(f"seed_embed.py: {faces} faces read, {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
