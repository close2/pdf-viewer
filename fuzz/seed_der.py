#!/usr/bin/env python3
"""X.690's encoding, and where a PDF keeps DER — shared by `seed_x509.py` and `seed_cms.py`.

Two seeders harvest two different structures out of the same files, and everything *below* the
structure is the same work: the tag-length-value walk with clause 8.1.3.6's indefinite lengths, the
`stream` inflation that reaches a `/Certs` or a `/TS` entry, the `/Contents` hexadecimal beside a
`/ByteRange`, and the argument list that makes the whole tree one run. This module is that half.

**It is a second implementation of `pdf_model::der`, written from X.690 rather than from the
Rust**, which is ADR 0229's argument for a seeder being a program at all: two implementations
agreeing is a check the round-trip tests cannot perform on themselves. It is one module rather
than two copies of one for the reason `CLAUDE.md` principle 4 gives — a copied parser drifts, and
then the agreement it was written to demonstrate is between a reader and a fork of itself.

Nothing here knows what a certificate or a signature is. `is_certificate` and `is_signed_data`
live beside the seeder that needs them, and [`stated`] is what puts a recogniser to work.
"""

import binascii
import os
import re
import sys
import zlib


def values(data, start=0):
    """Every tag-length-value in `data`, as `(identifier, first, last)` byte offsets."""
    out = []
    at = start
    end = len(data)
    while at < end - 1:
        identifier = data[at]
        at += 1
        if identifier == 0 and data[at] == 0:
            break
        length = data[at]
        at += 1
        if length == 0x80:
            # X.690 clause 8.1.3.6: the contents run to an end-of-contents marker that is this
            # value's rather than a child's, so the children have to be walked to find it.
            stop = end_of_contents(data, at)
            out.append((identifier, at, stop))
            at = stop + 2
            continue
        if length & 0x80:
            count = length & 0x7F
            length = int.from_bytes(data[at : at + count], "big")
            at += count
        out.append((identifier, at, at + length))
        at += length
    return out


def end_of_contents(data, at):
    """Where an indefinite-length value's contents stop."""
    end = len(data)
    while at < end - 1:
        if data[at] == 0 and data[at + 1] == 0:
            return at
        at += 1
        length = data[at]
        at += 1
        if length == 0x80:
            at = end_of_contents(data, at) + 2
        elif length & 0x80:
            count = length & 0x7F
            at += count + int.from_bytes(data[at : at + count], "big")
        else:
            at += length
    return end


def definite(identifier, body):
    """One tag-length-value with a definite length, whatever the file used."""
    if len(body) < 128:
        header = bytes([identifier, len(body)])
    else:
        octets = len(body).to_bytes((len(body).bit_length() + 7) // 8, "big")
        header = bytes([identifier, 0x80 | len(octets)]) + octets
    return header + body


def value_at(data, at):
    """The one value beginning at `at`, as `(first, last, end)`, or `None` for anything else.

    `first` and `last` bound the contents and `end` is where the whole value stops, which are the
    same offset for a definite length and two apart for clause 8.1.3.6's indefinite one — the
    end-of-contents marker belongs to the value and not to its contents.

    A length in more than four octets is refused rather than read: it states a value larger than
    any file this seeds a corpus from, and reading it would be arithmetic on a stranger's digits
    for no gain.
    """
    if at + 1 >= len(data):
        return None
    length = data[at + 1]
    if length == 0x80:
        first = at + 2
        last = end_of_contents(data, first)
        return (first, last, last + 2)
    if length & 0x80 == 0:
        first = at + 2
        return (first, first + length, first + length)
    count = length & 0x7F
    if count == 0 or count > 4 or at + 2 + count > len(data):
        return None
    first = at + 2 + count
    last = first + int.from_bytes(data[at + 2 : at + 2 + count], "big")
    return (first, last, last)


def stated(data, opening, accept):
    """Every whole value in `data` that `opening` proposes and `accept` disposes of.

    The two halves are the point. A DER structure is self-delimiting, so it can be found in a
    buffer nobody parsed — but its *opening bytes* are shared with every other structure of the
    same shape, and a format usually has a sibling with that shape (RFC 5280 §5.1's
    `CertificateList` is `seed_x509.py`'s). So `opening` is a pattern, which keeps the scan itself
    in C over every byte of every file, and `accept` reads the contents with the walk above.
    """
    out = []
    at = 0
    while (match := opening.search(data, at)) is not None:
        at = match.start()
        found = value_at(data, at)
        if found is not None:
            first, last, end = found
            if end <= len(data) and accept(data[first:last]):
                out.append(data[at:end])
                at = end
                continue
        at += 1
    return out


STREAM = re.compile(rb"stream\r?\n")

# A stream body long enough to hold what these seeders look for and short enough that inflating
# every one of them over a corpus stays a scan rather than a job. Nothing here is a budget this
# program states; it is this script's own patience.
INFLATE_CEILING = 16 << 20


def inflated_streams(data, keys):
    """Each `stream` body in `data` that inflates, for a file naming one of `keys` in the clear.

    Both windows are tried: `FlateDecode` is zlib, and a producer that wrote a raw deflate stream
    instead is exactly the kind of file this corpus is made of.

    **`keys` is a deliberate trade and it is not free.** Inflating every stream of every file costs
    about 0.6 s per document from the crawl and this gate costs milliseconds, which over the
    corpora these recipes are pointed at is the difference between a quarter of an hour and most of
    a day — and a recipe nobody will run seeds nothing. What it gives up is what a document keeps
    inside a Flate stream while naming none of the keys in the clear, which a catalogue that is
    itself in an object stream does. The raw scan runs on every file regardless, so nothing a
    document states uncompressed is missed either way.
    """
    if not any(key in data for key in keys):
        return
    for match in STREAM.finditer(data):
        body = data[match.end() : match.end() + INFLATE_CEILING]
        for window in (15, -15):
            try:
                out = zlib.decompressobj(window).decompress(body, INFLATE_CEILING)
            except zlib.error:
                continue
            if len(out) > 2:
                yield out
                break


CONTENTS = re.compile(rb"/Contents\s*<([0-9A-Fa-f\s]+)>")


def signature_values(data):
    """Every `/Contents` hexadecimal string in `data`, decoded.

    §12.8.1 makes a signature's value "a hexadecimal string", and Table 255 puts it in `/Contents`
    beside the `/ByteRange` that excludes it — which is what makes a raw scan reach every signature
    stated outside an object stream without being a PDF reader. The caller decides whether the
    file is worth scanning at all; this yields whatever it finds.
    """
    for match in CONTENTS.finditer(data):
        try:
            yield binascii.unhexlify(re.sub(rb"\s", b"", match.group(1)))
        except binascii.Error:
            continue


def paths(arguments):
    """The files to read, with `-` standing for a NUL-separated list on standard input.

    `find … -print0 | xargs` runs a command once per batch of arguments, and over this tree's
    corpora that is around thirty batches — thirty processes, each counting only its own, so the
    one number the caller wants is the one thing the run does not print. Reading the list keeps it
    one process and one answer.
    """
    for argument in arguments:
        if argument != "-":
            yield argument
            continue
        for name in sys.stdin.buffer.read().split(b"\0"):
            if name:
                yield os.fsdecode(name)
