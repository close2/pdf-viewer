#!/usr/bin/env python3
"""Seed the `media_mp4` fuzz target with MP4 files whose sound track uses each table form.

    python3 fuzz/seed_media_mp4.py fuzz/corpus/media_mp4

`pdf_media_worker::mp4` reads an MP4 file's sound track by ISO/IEC 14496-12's boxes (ADR 3369), and a
fuzzer will not invent a movie box whose sample tables agree with one another, so each seed states
one: sizes in `stsz` (one each, or one for every sample) and in `stz2` at 4, 8 and 16 bits; chunk
offsets in `stco` and `co64`; the movie before its media data and after it; a 64-bit box size; and
an `ES_Descriptor` with each optional field. The media data is a pattern built here; no document's
sound and no fetched stream is taken. Seeds are named by content, so a re-run adds nothing.
"""

import hashlib
import os
import struct
import sys


def write(out, data):
    name = hashlib.sha256(data).hexdigest()[:40]
    path = os.path.join(out, name)
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def box(kind, payload):
    return struct.pack(">I", len(payload) + 8) + kind + payload


def large_box(kind, payload):
    return struct.pack(">I", 1) + kind + struct.pack(">Q", len(payload) + 16) + payload


def full(kind, version, flags, payload):
    return box(kind, struct.pack(">I", (version << 24) | flags) + payload)


def words(*values):
    return struct.pack(">%dI" % len(values), *values)


def descriptor(tag, payload):
    return bytes([tag, len(payload)]) + payload


def esds(es_flags=0, es_extra=b"", config=b"\x11\x90"):
    decoder = bytes([0x40, 0x15]) + bytes(11) + descriptor(5, config)
    stream = b"\x00\x01" + bytes([es_flags]) + es_extra + descriptor(4, decoder)
    return full(b"esds", 0, 0, descriptor(3, stream))


def mp4a(children):
    fields = bytes(6) + struct.pack(">HH", 1, 0) + bytes(6) + struct.pack(">HH", 2, 16)
    fields += bytes(4) + struct.pack(">I", 48000 << 16)
    return box(b"mp4a", fields + children)


SIZES = [10, 20, 30, 15]


def sizes_box(form):
    if form == "stsz":
        return full(b"stsz", 0, 0, words(0, len(SIZES), *SIZES))
    if form == "constant":
        return full(b"stsz", 0, 0, words(12, len(SIZES)))
    bits = {"stz2-4": 4, "stz2-8": 8, "stz2-16": 16}[form]
    if bits == 4:
        packed = bytes([(SIZES[0] << 4 | SIZES[1]) & 0xFF, (SIZES[2] << 4 | SIZES[3]) & 0xFF])
    elif bits == 8:
        packed = bytes(SIZES)
    else:
        packed = struct.pack(">4H", *SIZES)
    return full(b"stz2", 0, 0, bytes(3) + bytes([bits]) + words(len(SIZES)) + packed)


def movie(data_at, form, wide, entry):
    chunks = [data_at, data_at + 80]
    if wide:
        offsets = full(b"co64", 0, 0, words(len(chunks)) + struct.pack(">2Q", *chunks))
    else:
        offsets = full(b"stco", 0, 0, words(len(chunks), *chunks))
    stbl = box(
        b"stbl",
        full(b"stsd", 0, 0, words(1) + entry)
        + full(b"stts", 0, 0, words(2, 3, 1024, 1, 960))
        + full(b"stsc", 0, 0, words(2, 1, 3, 1, 2, 1, 1))
        + sizes_box(form)
        + offsets,
    )
    dinf = box(b"dinf", full(b"dref", 0, 0, words(1) + full(b"url ", 0, 1, b"")))
    minf = box(b"minf", full(b"smhd", 0, 0, bytes(4)) + dinf + stbl)
    hdlr = full(b"hdlr", 0, 0, words(0) + b"soun" + bytes(13))
    mdhd = full(b"mdhd", 0, 0, words(0, 0, 48000, 4032, 0))
    trak = box(b"trak", full(b"tkhd", 0, 1, bytes(80)) + box(b"mdia", mdhd + hdlr + minf))
    return box(b"moov", full(b"mvhd", 0, 0, bytes(96)) + trak)


def mp4(form="stsz", wide=False, data_first=False, large=False, entry=None):
    entry = entry or mp4a(esds())
    ftyp = box(b"ftyp", b"M4A \0\0\0\0M4A isom")
    payload = bytes((index * 37) & 0xFF for index in range(128))
    data = (large_box if large else box)(b"mdat", payload)
    header = 16 if large else 8
    if data_first:
        return ftyp + data + movie(len(ftyp) + header, form, wide, entry)
    length = len(movie(0, form, wide, entry))
    return ftyp + movie(len(ftyp) + length + header, form, wide, entry) + data


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    written = 0
    for form in ["stsz", "constant", "stz2-4", "stz2-8", "stz2-16"]:
        for wide in (False, True):
            written += write(out, mp4(form, wide))
    written += write(out, mp4(data_first=True))
    written += write(out, mp4(large=True))
    written += write(out, mp4(entry=mp4a(esds(0xE0, b"\x00\x09\x03abc\x00\x04"))))
    print("seed_media_mp4.py: %d seeds written to %s" % (written, out))


if __name__ == "__main__":
    main()
