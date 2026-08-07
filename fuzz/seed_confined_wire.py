#!/usr/bin/env python3
"""Seed `fuzz/corpus/confined_wire/` with what a real confined worker writes.

    cargo build --release -p viewer-confined --bins
    python3 fuzz/seed_confined_wire.py \
        target/pdf-view-worker fuzz/corpus/confined_wire doc/*.pdf

**Why a script and not a Rust helper.** `fuzz/corpus` is gitignored by policy — the corpora are
large and machine-generated — so a seeded target needs a *recipe* rather than a checked-in
directory, and the `sfnt` and `xmp` targets each have one written down. This is that recipe for
`confined_wire`, and it is a program rather than a paragraph because the format has 25 questions
in it.

**And speaking the wire format by hand is half its value.** Everything below — the 18-byte
greeting, the one-byte kind and eight-byte big-endian length of a frame, the discriminants of
`Command::Open`, `Command::Resize` and all twenty-five queries — is a second implementation of
`viewer_confined::protocol`, written from the format rather than from the Rust. Two
implementations agreeing is a check the round-trip tests cannot perform on themselves. See
ADR 0223.

What it keeps is every *payload* the worker sends back — events, answers and any refusal — plus
the query payloads it sent, named by their SHA-1 so a re-run adds only what is new. The document
bytes are not kept: a `Command::Open` payload is the whole file and the corpus would be the corpus.

Run it at a small viewport (below) so that `Query::Frame`'s answer is a few kilobytes of pixels
rather than four megabytes; the raster path is a length check either way and libFuzzer is faster on
small seeds.
"""
import hashlib
import os
import struct
import subprocess
import sys

WORKER = sys.argv[1]
OUT = sys.argv[2]
DOCS = sys.argv[3:]

HANDSHAKE = 8 + 1 + 8 + 1
FRAME_COMMAND, FRAME_QUERY = 1, 2

def u8(v): return bytes([v])
def u32(v): return struct.pack(">I", v)
def u64(v): return struct.pack(">Q", v)
def f32(v): return struct.pack(">f", v)
def blob(b): return u64(len(b)) + b
def text(s): return blob(s.encode())
def frame(kind, payload): return u8(kind) + u64(len(payload)) + payload

# One entry per `Query` discriminant, with a plausible argument where it takes one.
QUERIES = [
    ("page_count", u8(1)),
    ("current_page", u8(2)),
    ("page_geometry", u8(3) + u64(0)),
    ("page_label", u8(4) + u64(0)),
    ("link_at", u8(5) + f32(100.0) + f32(100.0)),
    ("field_at", u8(6) + f32(100.0) + f32(100.0)),
    ("caret", u8(7) + f32(100.0) + f32(100.0) + u64(0)),
    ("dirty", u8(8)),
    ("find", u8(9) + text("the")),
    ("selection", u8(10)),
    ("logical_selection", u8(11)),
    ("focus", u8(12)),
    ("frame", u8(13)),
    ("reports", u8(14)),
    ("outline", u8(15)),
    ("layers", u8(16)),
    ("attachments", u8(17)),
    ("collection", u8(18)),
    ("articles", u8(19)),
    ("thumbnail", u8(20) + u64(0)),
    ("properties", u8(21)),
    ("opening", u8(22)),
    ("preferences", u8(23)),
    ("popups", u8(24)),
    ("structure", u8(25)),
]

os.makedirs(OUT, exist_ok=True)
written = 0

def keep(payload):
    global written
    if not payload:
        return
    name = hashlib.sha1(payload).hexdigest()
    path = os.path.join(OUT, name)
    if not os.path.exists(path):
        with open(path, "wb") as handle:
            handle.write(payload)
        written += 1

def read_frame(pipe):
    header = pipe.read(9)
    if len(header) < 9:
        raise SystemExit("the worker closed its output")
    kind = header[0]
    length = struct.unpack(">Q", header[1:])[0]
    payload = pipe.read(length) if length else b""
    return kind, payload

for document in DOCS:
    with open(document, "rb") as handle:
        bytes_of = handle.read()
    worker = subprocess.Popen([WORKER], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    greeting = worker.stdout.read(HANDSHAKE)
    assert greeting[:8] == b"PDFVCF02", greeting[:8]

    resize = u8(5) + u32(48) + u32(64) + f32(1.0)
    worker.stdin.write(frame(FRAME_COMMAND, resize))
    worker.stdin.flush()
    keep(read_frame(worker.stdout)[1])

    open_command = u8(1) + u64(1) + blob(bytes_of) + u8(0) + u8(0)
    worker.stdin.write(frame(FRAME_COMMAND, open_command))
    worker.stdin.flush()
    keep(read_frame(worker.stdout)[1])

    for _name, payload in QUERIES:
        worker.stdin.write(frame(FRAME_QUERY, payload))
        worker.stdin.flush()
        keep(payload)
        kind, answer = read_frame(worker.stdout)
        if kind == 5:
            print(f"  {document} {_name}: refused — {answer.decode(errors='replace')}")
        keep(answer)

    worker.stdin.close()
    worker.wait()
    print(f"{document}: done")

print(f"{written} seed(s) written to {OUT}")
