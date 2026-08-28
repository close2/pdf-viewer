#!/usr/bin/env python3
"""Seed `fuzz/corpus/confined_wire/` with what a real confined worker writes.

    cargo build --release -p viewer-confined --bins
    python3 fuzz/seed_confined_wire.py \
        target/pdf-view-worker fuzz/corpus/confined_wire doc/*.pdf

**Why a script and not a Rust helper.** `fuzz/corpus` is gitignored by policy — the corpora are
large and machine-generated — so a seeded target needs a *recipe* rather than a checked-in
directory, and the `sfnt` and `xmp` targets each have one written down. This is that recipe for
`confined_wire`, and it is a program rather than a paragraph because the transport has a question
per panel and then some.

**And speaking the wire format by hand is half its value.** Everything below — the 18-byte
greeting, the one-byte kind and eight-byte big-endian length of a frame, the payload each command
and each question carries after its discriminant — is a second implementation of
`viewer_confined::protocol`, written from the format rather than from the Rust. Two
implementations agreeing is a check the round-trip tests cannot perform on themselves. See
ADR 0223.

**The discriminants themselves are read out of the Rust, and the coverage is checked**, which is
the eight-hundred-and-sixteenth session's change and the same rule `MAGIC` below is under: a
number is a fact that can be counted, so it is not written down here. This list used to carry the
numbers 1 to 25 by hand and it went **seven** questions stale — `Offset` and `FieldSelection` (ADR
0225), `Fields` (0235), `FreeTextAt` (0238), `Highlight` (0357), `Readback` (0422) and `View`
(0737) all arrived after it was written, and the corpus for this target therefore stated nothing
about any of their answers. The four-hundred-and-forty-fifth session found four of them by hand and
`doc/verify.md` still said four. So the shape of each payload stays hand-written, because that is
the second implementation, and the *population* is now derived: a discriminant `query_kind` states
and this file does not name stops the script rather than being seeded past.

What it keeps is every *payload* the worker sends back — events, answers and any refusal — plus
the command and query payloads it sent, named by their SHA-1 so a re-run adds only what is new.
The document bytes are the one exception: a `Command::Open` payload is the whole file and the
corpus would be the corpus.

Run it at a small viewport (below) so that `Query::Frame`'s answer is a few kilobytes of pixels
rather than four megabytes; the raster path is a length check either way and libFuzzer is faster on
small seeds.
"""
import hashlib
import os
import pathlib
import re
import struct
import subprocess
import sys

WORKER = sys.argv[1]
OUT = sys.argv[2]
DOCS = sys.argv[3:]

PROTOCOL = pathlib.Path(__file__).resolve().parent.parent / "crates/viewer-confined/src/protocol.rs"
SOURCE = PROTOCOL.read_text()


# The greeting's magic, **read out of the Rust rather than written down here.**
#
# It was written down, as `PDFVCF02`, and the seven-hundred-and-thirty-sixth session found it one
# behind: `MAGIC` had moved to `PDFVCF03` when the panel answers landed and nothing said so, so
# this seeder had been refusing to run — and `confined_wire`'s corpus had been empty — for however
# long. A pinned constant is right (a format change must stop this script, not be seeded past) and
# a *copy* of one is not, which is `CLAUDE.md`'s rule about a fact that can be counted.
def magic():
    found = re.search(r'const MAGIC: &\[u8; 8\] = b"(\w{8})"', SOURCE)
    if not found:
        raise SystemExit(f"{PROTOCOL} states no MAGIC this script can read")
    return found.group(1).encode()


# Every discriminant one of `protocol.rs`'s two `mod *_kind` blocks states, by its own name.
#
# Read rather than copied, for `magic`'s reason and with a sharper consequence: a *number* that
# goes stale here seeds the wrong question, and a *name* that goes missing is caught below.
def kinds(module):
    block = re.search(rf"mod {module} \{{(.*?)\n\}}", SOURCE, re.S)
    if not block:
        raise SystemExit(f"{PROTOCOL} states no `mod {module}` this script can read")
    return {name: int(value) for name, value in re.findall(r"const (\w+): u8 = (\d+);", block.group(1))}


MAGIC = magic()
QUERY_KIND = kinds("query_kind")
COMMAND_KIND = kinds("command_kind")
HANDSHAKE = 8 + 1 + 8 + 1
# The frame layer stays written down, because it is five kinds that do not grow — a command and a
# question out, events, an answer and a refusal back — and it is the half of the second
# implementation that is worth having by hand.
FRAME_COMMAND, FRAME_QUERY, FRAME_REFUSAL = 1, 2, 5

def u8(v): return bytes([v])
def u32(v): return struct.pack(">I", v)
def u64(v): return struct.pack(">Q", v)
def f32(v): return struct.pack(">f", v)
def blob(b): return u64(len(b)) + b
def text(s): return blob(s.encode())
def frame(kind, payload): return u8(kind) + u64(len(payload)) + payload
# A machine-word count crosses as a fixed 64 bits, and a point as two `f32`s written as their bits.
def usize(v): return u64(v)
def point(x, y): return f32(x) + f32(y)
# `Zoom::Scale`, which is discriminant 3 of the six a magnification can be.
def scale(v): return u8(3) + f32(v)
# `Viewing`: where the reader is looking, as `Command::View` states it and `Answer::View` returns it.
def viewing(page, magnification, scroll): return usize(page) + scale(magnification) + point(*scroll)

# One entry per `Query` discriminant: the constant `query_kind` names it by, and the bytes that
# follow the discriminant. The *shape* is this script's own reading of the format — that is the
# second implementation ADR 0223 wants — and the discriminant is read out of the Rust, because a
# number written down here is a number that goes stale without saying so, which is what happened.
QUERIES = {
    "PAGE_COUNT": b"",
    "CURRENT_PAGE": b"",
    "VIEW": b"",
    "PAGE_GEOMETRY": usize(0),
    "PAGE_LABEL": usize(0),
    "LINK_AT": point(100.0, 100.0),
    "FIELD_AT": point(100.0, 100.0),
    "CARET": point(100.0, 100.0) + usize(0),
    "OFFSET": point(100.0, 100.0) + point(20.0, 30.0),
    "FIELD_SELECTION": point(100.0, 100.0) + usize(0) + usize(3),
    "FREE_TEXT_AT": point(100.0, 100.0),
    "DIRTY": b"",
    "FIND": text("the"),
    "SELECTION": b"",
    "LOGICAL_SELECTION": b"",
    "FOCUS": b"",
    "HIGHLIGHT": b"",
    "FRAME": b"",
    "REPORTS": b"",
    "READBACK": b"",
    "OUTLINE": b"",
    "LAYERS": b"",
    "ATTACHMENTS": b"",
    "COLLECTION": b"",
    "ARTICLES": b"",
    "THUMBNAIL": usize(0),
    "PROPERTIES": b"",
    "OPENING": b"",
    "PREFERENCES": b"",
    "POPUPS": b"",
    "FIELDS": b"",
    "ACCESSIBILITY_TREE": b"",
}

# The check that makes the paragraph above true rather than hopeful. A question the transport
# carries and this script does not ask is a question whose *answer* has never been seeded, and the
# answers are most of what this target decodes — so it stops here rather than seeding 31 of 32 and
# reporting success.
def covered():
    unasked = sorted(QUERY_KIND.keys() - QUERIES.keys(), key=lambda name: QUERY_KIND[name])
    if unasked:
        raise SystemExit(
            f"{PROTOCOL}'s `query_kind` states {len(QUERY_KIND)} questions and this script asks "
            f"{len(QUERIES)}. Unasked: {', '.join(unasked)}.\n"
            "Add one entry per name above, stating the bytes that follow the discriminant, and "
            "re-seed. A question nobody asks is an answer nobody has fuzzed."
        )
    gone = sorted(QUERIES.keys() - QUERY_KIND.keys())
    if gone:
        raise SystemExit(
            f"this script asks for {', '.join(gone)}, which `{PROTOCOL}`'s `query_kind` no longer "
            "states. The format moved; read it and follow."
        )


covered()

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

def exchange(worker, document, name, kind, payload, seed_the_payload=True):
    """Send one frame, keep what it is worth keeping, and say so when the worker refuses.

    The payload sent is a seed too, and not only the answer: `wire::command` and `wire::query` are
    two of the four decoders this target runs over every input, so a valid command is worth as much
    to them as a valid answer is to `wire::answer`. `Command::Open`'s is the exception — it is the
    document, and a corpus of documents is a different target's.
    """
    worker.stdin.write(frame(kind, payload))
    worker.stdin.flush()
    if seed_the_payload:
        keep(payload)
    reply, answer = read_frame(worker.stdout)
    if reply == FRAME_REFUSAL:
        print(f"  {document} {name}: refused — {answer.decode(errors='replace')}")
    keep(answer)


for document in DOCS:
    with open(document, "rb") as handle:
        bytes_of = handle.read()
    worker = subprocess.Popen([WORKER], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    greeting = worker.stdout.read(HANDSHAKE)
    assert greeting[:8] == MAGIC, (greeting[:8], MAGIC)

    exchange(worker, document, "resize",
             FRAME_COMMAND, u8(COMMAND_KIND["RESIZE"]) + u32(48) + u32(64) + f32(1.0))
    exchange(worker, document, "open",
             FRAME_COMMAND, u8(COMMAND_KIND["OPEN"]) + u64(1) + blob(bytes_of) + u8(0) + u8(0),
             seed_the_payload=False)
    # Before the questions rather than after them, so that `Query::View`'s answer is a place the
    # reader was put rather than the one a worker starts at.
    exchange(worker, document, "view",
             FRAME_COMMAND, u8(COMMAND_KIND["VIEW"]) + viewing(0, 1.25, (0.0, 40.0)))

    for name, arguments in sorted(QUERIES.items(), key=lambda entry: QUERY_KIND[entry[0]]):
        exchange(worker, document, name.lower(),
                 FRAME_QUERY, u8(QUERY_KIND[name]) + arguments)

    worker.stdin.close()
    worker.wait()
    print(f"{document}: done")

print(f"{written} seed(s) written to {OUT}")
