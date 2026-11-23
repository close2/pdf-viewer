#!/usr/bin/env python3
"""Seed the `media_sound` fuzz target with one request of each sample format the worker decodes.

    python3 fuzz/seed_media_sound.py fuzz/corpus/media_sound

`pdf-media-worker` decodes ISO 32000-2 §13.3's sound samples: Table 305's `Raw` and `Signed` at
every width from 1 to 32 bits, and G.711's µ-law and A-law codes of eight (ADR 3297). A request is
`pdf_media_worker::wire`'s layout — a version byte, `/R` as a little-endian `u32`, `/C` as a `u16`,
`/B`, the encoding's byte (0 Raw, 1 Signed, 2 muLaw, 3 ALaw, 4 a WAVE file's little-endian
PCM), then the run — and a fuzzer will not
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
    for encoding, widths in ((0, (1, 4, 8, 12, 16, 24, 32)), (1, (1, 8, 12, 16, 24, 32)), (2, (8,)), (3, (8,)), (4, (16, 24, 32))):
        for bits in widths:
            for channels, rate in ((1, 8000), (2, 22050), (3, 11025)):
                for length in (0, 1, 9, 64, 255):
                    written += write(out, request(rate, channels, bits, encoding, ramp[:length]))
    # RIFF WAVE layout requests (ADRs 3349, 3397): a version byte, the file's length and the
    # window's offset as `u64`s, the carried format (thirteen zero bytes: none yet), then the file —
    # PCM at three widths, A-law and µ-law, with and without a chunk before the format.
    for tag, channels, bits in ((1, 1, 8), (1, 2, 16), (1, 1, 24), (6, 1, 8), (7, 2, 8)):
        for before in (b"", b"LIST" + struct.pack("<I", 3) + b"abc\0"):
            container = (bits + 7) // 8
            fmt = struct.pack("<HHIIHH", tag, channels, 8000, 8000 * channels * container,
                              channels * container, bits)
            body = b"WAVE" + before + b"fmt " + struct.pack("<I", len(fmt)) + fmt
            body += b"data" + struct.pack("<I", 8) + ramp[:8]
            wave = b"RIFF" + struct.pack("<I", len(body)) + body
            written += write(out, struct.pack("<BQQ", VERSION, len(wave), 0) + bytes(13) + wave)
    # WAVE_FORMAT_EXTENSIBLE: the common fields, cbSize 22, valid bits, the channel mask, and a
    # SubFormat GUID naming PCM (1) or IEEE float (3) over the base GUID's tail.
    tail = bytes([0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71])
    for subtag, channels, bits, mask in ((1, 2, 16, 0x3), (1, 6, 24, 0x3F), (3, 2, 32, 0x3), (1, 4, 16, 0x33)):
        container = bits // 8
        fmt = struct.pack("<HHIIHH", 0xFFFE, channels, 48000, 48000 * channels * container,
                          channels * container, bits)
        fmt += struct.pack("<HHII", 22, bits, mask, subtag) + tail
        body = b"WAVE" + b"fmt " + struct.pack("<I", len(fmt)) + fmt
        body += b"data" + struct.pack("<I", 8) + ramp[:8]
        wave = b"RIFF" + struct.pack("<I", len(body)) + body
        written += write(out, struct.pack("<BQQ", VERSION, len(wave), 0) + bytes(13) + wave)
    print(f"{written} seeds written to {out}")


if __name__ == "__main__":
    main()
