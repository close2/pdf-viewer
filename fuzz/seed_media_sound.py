#!/usr/bin/env python3
"""Seed the `media_sound` fuzz target with one request of each sample format the worker decodes.

    python3 fuzz/seed_media_sound.py fuzz/corpus/media_sound

`pdf-media-worker` decodes ISO 32000-2 §13.3's sound samples: Table 305's `Raw` and `Signed` at
every width from 1 to 32 bits, and G.711's µ-law and A-law codes of eight (ADR 3297). A request is
`pdf_media_worker::wire`'s layout — a version byte, `/R` as a little-endian `u32`, `/C` as a `u16`,
`/B`, the encoding's byte (0 Raw, 1 Signed, 2 muLaw, 3 ALaw), then the run — and a fuzzer will not
invent a header whose width and channels agree, so each seed states one: every encoding at each
width §13.3 recommends and a few it permits, mono and stereo and three channels, with runs that
end on a frame and runs that do not. The samples are a ramp built here; no document's sound is
taken. Seeds are named by content, so a re-run adds nothing.
"""

import hashlib
import os
import struct
import sys

VERSION = 1


def write(out, data):
    name = hashlib.sha256(data).hexdigest()[:40]
    path = os.path.join(out, name)
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def request(rate, channels, bits, encoding, run):
    return struct.pack("<BIHBB", VERSION, rate, channels, bits, encoding) + run


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    ramp = bytes(range(256))
    written = 0
    for encoding, widths in ((0, (1, 4, 8, 12, 16, 24, 32)), (1, (1, 8, 12, 16, 24, 32)), (2, (8,)), (3, (8,))):
        for bits in widths:
            for channels, rate in ((1, 8000), (2, 22050), (3, 11025)):
                for length in (0, 1, 9, 64, 255):
                    written += write(out, request(rate, channels, bits, encoding, ramp[:length]))
    print(f"{written} seeds written to {out}")


if __name__ == "__main__":
    main()
