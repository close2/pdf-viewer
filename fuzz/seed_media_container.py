#!/usr/bin/env python3
"""Seed the `media_container` fuzz target with one small file of each container it reads.

    python3 fuzz/seed_media_container.py fuzz/corpus/media_container

`pdf_model::media::container` reads RIFF WAVE, AVI, AIFF and the ISO base media file format's
headers for ISO 32000-2 §13.2.4.3's offset rules (ADR 2452). A fuzzer will not invent a `moov`
box whose `trak` names a chapter track through `tref`, or a `cue ` chunk whose points an `adtl`
list labels, so each seed states the structures the reader walks: a WAVE file with labelled cue
points, an AVI file with an `avih` header and an OpenDML `dmlh`, an AIFF file with a `MARK`
chunk, and a QuickTime file with a picture track and a chapter track of two titles. The target's
first byte chooses the content type (0 WAVE, 1 AVI, 2 AIFF, 3 MP4), so each seed starts with its
own. The files are built here rather than taken from documents: no corpus document holds a media
clip section, and so none exercises the reader. Seeds are named by content, so a re-run adds
nothing.
"""

import hashlib
import os
import struct
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


def riff(chunk_id, data, endian="<"):
    pad = b"\0" if len(data) % 2 else b""
    return chunk_id + struct.pack(endian + "I", len(data)) + data + pad


def wave():
    fmt = struct.pack("<HHIIHH", 1, 1, 8000, 16000, 2, 16)
    cue = struct.pack("<I", 2)
    labels = b"adtl"
    for ident, position, name in [(1, 0, b"Start"), (2, 4000, b"Chapter One")]:
        cue += struct.pack("<II4sIII", ident, position, b"data", 0, 0, position)
        labels += riff(b"labl", struct.pack("<I", ident) + name + b"\0")
    body = b"WAVE" + riff(b"fmt ", fmt) + riff(b"data", b"\0" * 64)
    body += riff(b"cue ", cue) + riff(b"LIST", labels)
    return riff(b"RIFF", body)


def avi():
    avih = struct.pack("<10I", 40000, 0, 0, 0, 250, 0, 1, 0, 320, 240) + b"\0" * 16
    hdrl = b"hdrl" + riff(b"avih", avih)
    odml = b"odml" + riff(b"dmlh", struct.pack("<I", 500) + b"\0" * 244)
    return riff(b"RIFF", b"AVI " + riff(b"LIST", hdrl) + riff(b"LIST", odml))


def aiff():
    # 44 100 Hz as an 80-bit extended number: exponent 16383 + 15, significand 44100 << 48.
    rate = struct.pack(">HQ", 16383 + 15, 44100 << 48)
    comm = struct.pack(">HIH", 1, 88200, 16) + rate
    marks = struct.pack(">H", 2)
    for ident, position, name in [(1, 0, b"Start"), (2, 44100, b"Chorus")]:
        marks += struct.pack(">HIB", ident, position, len(name)) + name
        if len(name) % 2 == 0:
            marks += b"\0"
    body = b"AIFF" + riff(b"COMM", comm, ">") + riff(b"MARK", marks, ">")
    return b"FORM" + struct.pack(">I", len(body)) + body


def box(kind, payload):
    return struct.pack(">I", len(payload) + 8) + kind + payload


def full(kind, fields):
    return box(kind, b"\0\0\0\0" + fields)


def trak(ident, handler, scale, table, extra=b""):
    mdia = box(b"mdia", full(b"mdhd", struct.pack(">IIII", 0, 0, scale, 0))
               + full(b"hdlr", b"\0\0\0\0" + handler)
               + box(b"minf", box(b"stbl", table)))
    return box(b"trak", full(b"tkhd", struct.pack(">III", 0, 0, ident)) + extra + mdia)


def mp4():
    ftyp = box(b"ftyp", b"qt  \0\0\0\0")
    titles = b""
    for title in [b"Intro", b"Chapter One"]:
        titles += struct.pack(">H", len(title)) + title
    mdat = box(b"mdat", titles)
    chunk_at = len(ftyp) + 8
    text = trak(2, b"text", 1000,
                full(b"stts", struct.pack(">III", 1, 2, 3000))
                + full(b"stsz", struct.pack(">IIII", 0, 2, 7, 13))
                + full(b"stsc", struct.pack(">IIII", 1, 1, 2, 1))
                + full(b"stco", struct.pack(">II", 1, chunk_at)))
    video = trak(1, b"vide", 600, full(b"stsz", struct.pack(">II", 0, 250)),
                 box(b"tref", box(b"chap", struct.pack(">I", 2))))
    moov = box(b"moov", full(b"mvhd", struct.pack(">IIII", 0, 0, 600, 6000)) + video + text)
    return ftyp + mdat + moov


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        ROOT, "fuzz", "corpus", "media_container")
    os.makedirs(out, exist_ok=True)
    added = 0
    for head, build in enumerate([wave, avi, aiff, mp4]):
        added += write(out, bytes([head]) + build())
    print(f"seed_media_container: {added} seeds added to {out}")


if __name__ == "__main__":
    main()
