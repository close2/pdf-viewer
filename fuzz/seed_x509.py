#!/usr/bin/env python3
"""Seed `fuzz/corpus/x509/` with the certificates the corpus's signatures carry.

    python3 fuzz/seed_x509.py fuzz/corpus/x509 doc/pdf.js/test/pdfs/*.pdf

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy — the
corpora are large and machine-generated — so a seeded target needs a *recipe*, which the `sfnt`,
`xmp` and `confined_wire` targets each have. This is that recipe for `x509`, and it is a program
rather than a paragraph because reaching a certificate means walking two nested structures.

**And walking them by hand is half its value.** Everything below — X.690's tag-length-value with
clause 8.1.3.6's indefinite lengths, RFC 5652's `ContentInfo` and `SignedData`, and the
`certificates [0] IMPLICIT` member inside it — is a second implementation of `pdf_model::der` and
`pdf_model::cms`, written from the formats rather than from the Rust. Two implementations agreeing
is a check the round-trip tests cannot perform on themselves. See ADR 0229.

A signature value is found by scanning the file's raw bytes for the `/Contents <…>` hexadecimal
string beside a `/ByteRange`, which reaches every signature the corpus states outside an object
stream — nine of its ten. Each certificate is written out with a *definite* length whatever the
file used, named by its SHA-1 so a re-run adds only what is new.

To this add whatever else is worth a run: the round's two test vectors are an RSA and a P-256
certificate made with `openssl req -new -x509`, and any certificate at all is a legal input.
"""

import binascii
import hashlib
import os
import re
import sys


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


def certificates(signature):
    """Every certificate in a CMS `SignedData`'s `certificates [0] IMPLICIT` member."""
    content_info = values(signature)
    if not content_info:
        return []
    identifier, first, last = content_info[0]
    inside = signature[first:last]
    explicit = [value for value in values(inside) if value[0] & 0xC0 == 0x80]
    if not explicit:
        return []
    _, first, last = explicit[0]
    signed = values(inside[first:last])
    if not signed:
        return []
    body = inside[first:last][signed[0][1] : signed[0][2]]
    out = []
    for identifier, first, last in values(body):
        if identifier != 0xA0:
            continue
        for kind, start, stop in values(body[first:last]):
            out.append(definite(kind, body[first:last][start:stop]))
    return out


CONTENTS = re.compile(rb"/Contents\s*<([0-9A-Fa-f\s]+)>")


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    written = 0
    for path in argv[2:]:
        with open(path, "rb") as handle:
            data = handle.read()
        if b"/ByteRange" not in data:
            continue
        for match in CONTENTS.finditer(data):
            try:
                value = binascii.unhexlify(re.sub(rb"\s", b"", match.group(1)))
            except binascii.Error:
                continue
            for certificate in certificates(value.rstrip(b"\x00")):
                if len(certificate) < 50:
                    continue
                name = hashlib.sha1(certificate).hexdigest()
                with open(os.path.join(directory, name), "wb") as handle:
                    handle.write(certificate)
                written += 1
    print(f"{written} certificate(s) written to {directory}")


if __name__ == "__main__":
    main(sys.argv)
