#!/usr/bin/env python3
"""Seed the `media_container` fuzz target with one small file of each container it reads.

    python3 fuzz/seed_media_container.py fuzz/corpus/media_container

`pdf_model::media::container` reads RIFF WAVE, AVI, AIFF, the ISO base media file format, SWF, ASF
and an MPEG-2 program stream's headers for ISO 32000-2 §13.2.4.3's offset rules (ADRs 2452, 2488,
2489). A fuzzer will not invent a `moov`
box whose `trak` names a chapter track through `tref`, or a `cue ` chunk whose points an `adtl`
list labels, so each seed states the structures the reader walks: a WAVE file with labelled cue
points, an AVI file with an `avih` header and an OpenDML `dmlh`, an AIFF file with a `MARK`
chunk, a QuickTime file with a picture track and a chapter track of two titles, a plain and a
zlib-compressed SWF file with frame labels and a scene tag, an ASF file with a File Properties
Object and a Marker Object, and an MPEG-2 pack header. The target's first byte chooses the
content type (0 WAVE, 1 AVI, 2 AIFF, 3 MP4, 4 SWF, 5 ASF, 6 MPEG program stream, 7 MPEG audio),
so each seed starts with its own. The files are built here rather than taken from documents: the
corpora's few media clips are large and mostly outside the reader's reach, and so exercise little
of it. Seeds are named by content, so a re-run adds nothing.
"""

import hashlib
import os
import struct
import sys
import zlib

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


def swf_body(version):
    # A RECT of Nbits 4 with four zero fields (5 + 16 bits, padded to 3 bytes), 10 frames a second
    # (0x0A00 in 8.8) and 30 frames.
    body = bytes([0b00100000, 0, 0]) + struct.pack("<HH", 0x0A00, 30)

    def tag(kind, data):
        if len(data) < 0x3F:
            return struct.pack("<H", (kind << 6) | len(data)) + data
        return struct.pack("<HI", (kind << 6) | 0x3F, len(data)) + data

    body += tag(1, b"") + tag(43, b"intro\0") + tag(1, b"") + tag(43, b"anchor\0\x01")
    scenes = bytes([2, 0]) + b"Scene 1\0" + bytes([20]) + b"Scene 2\0" + bytes([1, 5]) + b"mid\0"
    body += tag(86, scenes) + tag(39, b"\x01\0\x01\0" + tag(43, b"inner\0") + tag(0, b""))
    return body + tag(1, b"") + tag(0, b"")


def swf(compressed):
    body = swf_body(9)
    head = struct.pack("<I", len(body) + 8)
    if compressed:
        return b"CWS" + bytes([9]) + head + zlib.compress(body)
    return b"FWS" + bytes([9]) + head + body


def asf_guid(data1, data2, data3, data4):
    return struct.pack("<IHH", data1, data2, data3) + bytes(data4)


def asf():
    properties_data = (b"\0" * 40 + struct.pack("<QQQI", 300_000_000, 300_000_000, 0, 2)
                       + b"\0" * 12)
    properties = (asf_guid(0x8CABDCA1, 0xA947, 0x11CF, [0x8E, 0xE4, 0, 0xC0, 0x0C, 0x20, 0x53, 0x65])
                  + struct.pack("<Q", len(properties_data) + 24) + properties_data)
    entries = b""
    for time, text in [(0, "Start"), (150_000_000, "Middle")]:
        wide = (text + "\0").encode("utf-16-le")
        entries += (struct.pack("<QQHII", 0, time, len(wide) + 12, 0, 0)
                    + struct.pack("<I", len(wide) // 2) + wide)
    marker_data = b"\0" * 16 + struct.pack("<IHH", 2, 0, 0) + entries
    markers = (asf_guid(0xF487CD01, 0xA951, 0x11CF, [0x8E, 0xE6, 0, 0xC0, 0x0C, 0x20, 0x53, 0x65])
               + struct.pack("<Q", len(marker_data) + 24) + marker_data)
    body = properties + markers
    header = (asf_guid(0x75B22630, 0x668E, 0x11CF, [0xA6, 0xD9, 0, 0xAA, 0, 0x62, 0xCE, 0x6C])
              + struct.pack("<QI", len(body) + 30, 2) + bytes([1, 2]))
    return header + body


def program_stream():
    return bytes([0, 0, 1, 0xBA, 0x44, 0x00, 0x04, 0x00, 0x04, 0x01, 0x00, 0x00, 0x07, 0xF8])


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        ROOT, "fuzz", "corpus", "media_container")
    os.makedirs(out, exist_ok=True)
    added = 0
    builds = [wave, avi, aiff, mp4, lambda: swf(False), asf, program_stream]
    for head, build in enumerate(builds):
        added += write(out, bytes([head]) + build())
    added += write(out, bytes([4]) + swf(True))
    added += write(out, bytes([7]) + b"ID3\x04\0\0\0\0\0\0")
    print(f"seed_media_container: {added} seeds added to {out}")


if __name__ == "__main__":
    main()
