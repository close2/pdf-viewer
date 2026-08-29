#!/usr/bin/env python3
"""Seed `fuzz/corpus/x509/` with every certificate this tree already contains.

    find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \\
        | python3 fuzz/seed_x509.py fuzz/corpus/x509 crates/pdf-model/src/*.rs -

A `-` argument stands for a NUL-separated list of paths on standard input, which is what makes the
whole tree one run rather than one run per `xargs` batch. `find -L` because a worktree's corpora
are symbolic links to the primary checkout's.

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy — the
corpora are large and machine-generated — so a seeded target needs a *recipe*, which the `sfnt`,
`xmp` and `confined_wire` targets each have. This is that recipe for `x509`, and it is a program
rather than a paragraph because reaching a certificate means walking two nested structures.

**And walking them by hand is half its value.** Everything below — X.690's tag-length-value with
clause 8.1.3.6's indefinite lengths, RFC 5652's `ContentInfo` and `SignedData`, and the
`certificates [0] IMPLICIT` member inside it — is a second implementation of `pdf_model::der` and
`pdf_model::cms`, written from the formats rather than from the Rust. Two implementations agreeing
is a check the round-trip tests cannot perform on themselves. See ADR 0229.

**Three routes, because a PDF states a certificate in three quite different places.** Each writes
its finds out with a *definite* length whatever the file used, named by SHA-1, so a re-run adds
only what is new and the three cannot double-count each other.

*The signature carries it.* §12.8.3.3.1 requires the CMS object to include the signer's X.509
signing certificate at minimum, so a signature value is a chain. The value is found by scanning the
raw bytes for the `/Contents <…>` hexadecimal string beside a `/ByteRange` — which reaches every
signature stated outside an object stream — and walked structurally, which is the second
implementation above.

*The document states it as an object.* §12.8.4.3's document security store puts the whole
validation chain in `/DSS`'s `/Certs`, one stream per certificate; Table 255's `/Cert` and Table
238's `/Subject` state certificates directly in a signature or a seed value dictionary. Reaching
those through the file's structure would mean a cross-reference table, object streams and a
`/Filter` pipeline — a PDF reader, on files chosen for being malformed. A DER certificate is
self-delimiting instead, so this route *proposes* by the opening bytes of RFC 5280 §4.1's
`Certificate` and *disposes* by the same walk route one uses — `is_certificate` below says what it
checks and why the check has to reach as far as `Validity`. Each file's `stream` bodies are
inflated and scanned as well, which is where a `/Certs` entry actually lives.

*This tree states it in hexadecimal.* `crates/pdf-model/src/{x509,dsa,pss,ecdsa,eddsa}.rs` carry
the certificates their `fixtures` modules verify against — the P-384, P-521, brainpoolP256r1 and
Ed25519 ones are the only inputs that reach `ecdsa`'s and `eddsa`'s curve arms at all — and until
this route existed `doc/verify.md` asked a round to re-make them with `openssl`. A clone now
re-seeds those arms from the repository. A `.rs` argument takes this route; anything else takes
the other two.
"""

import binascii
import hashlib
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


def extent(data, at):
    """Where the definite-length tag-length-value at `at` ends, or `None` for anything else."""
    if at + 1 >= len(data):
        return None
    length = data[at + 1]
    if length & 0x80 == 0:
        return at + 2 + length
    count = length & 0x7F
    if count == 0 or count > 4 or at + 2 + count > len(data):
        return None
    return at + 2 + count + int.from_bytes(data[at + 2 : at + 2 + count], "big")


def is_certificate(body):
    """Whether `body` is the contents of RFC 5280 §4.1's `Certificate`.

    Three members and no more — `tbsCertificate` and `signatureAlgorithm` are `SEQUENCE`s and
    `signature` is a `BIT STRING`, ending exactly where the outer length said — and then, inside
    the first of them, §4.1's own field list as far as `subjectPublicKeyInfo`.

    **The field list is not belt and braces.** §5.1's `CertificateList` has the identical outer
    shape, repeats its algorithm identifier the way §4.1.1.2 makes a certificate repeat one — so
    that equality does not separate them either — and is what §12.8.4.3's Table 261 puts in `/CRLs`
    immediately beside the certificates in `/Certs`. Without a check that reaches `Validity` this
    route harvests revocation lists, and the largest one on this disk is 1.5 MB, two orders of
    magnitude past any certificate in the corpus. What separates the two is that a certificate
    states two `Time`s where a revocation list states one.
    """
    members = values(body)
    if len(members) != 3:
        return False
    (tbs_tag, tbs_first, tbs_last), (algorithm_tag, first, last), (signature_tag, _, end) = members
    if (tbs_tag, algorithm_tag, signature_tag) != (0x30, 0x30, 0x03) or end != len(body):
        return False

    tbs = body[tbs_first:tbs_last]
    fields = values(tbs)
    # `version [0] EXPLICIT Version DEFAULT v1`, which a v1 certificate omits.
    if fields and fields[0][0] == 0xA0:
        fields = fields[1:]
    if len(fields) < 6 or [tag for tag, _, _ in fields[:6]] != [0x02, 0x30, 0x30, 0x30, 0x30, 0x30]:
        return False

    validity = values(tbs[fields[3][1] : fields[3][2]])
    if len(validity) != 2 or any(tag not in (0x17, 0x18) for tag, _, _ in validity):
        return False

    # RFC 5280 §4.1.1.2 requires `signatureAlgorithm` to hold the same algorithm identifier as
    # `tbsCertificate`'s own `signature` field, and an `AlgorithmIdentifier` opens on its
    # identifier. Paraphrased rather than quoted: this tree verifies quotations against
    # `doc/md/`, which holds ISO 32000-2 and not the RFCs.
    if tbs[fields[1][1] : fields[1][2]] != body[first:last]:
        return False
    inside = values(body[first:last])
    return bool(inside) and inside[0][0] == 0x06


# The two bytes a `Certificate` begins with: a `SEQUENCE` whose length is in long form, since no
# certificate is under 128 octets, and whose length fits four octets. Written as a pattern so that
# the scan itself runs in C — over a corpus this is called on every byte of every file.
CANDIDATE = re.compile(rb"\x30[\x81-\x84]", re.S)


def stated_certificates(data):
    """Every certificate `data` states outright, wherever in it they sit.

    `CANDIDATE` proposes and `is_certificate` disposes. The whole buffer is scanned rather than the
    places §12.8.4.3 and Tables 238 and 255 name, because finding *those* places means being a PDF
    reader; see this module's documentation.
    """
    out = []
    at = 0
    while (match := CANDIDATE.search(data, at)) is not None:
        at = match.start()
        stop = extent(data, at)
        first = at + 2 + (data[at + 1] & 0x7F)
        if stop is not None and stop <= len(data) and is_certificate(data[first:stop]):
            out.append(data[at:stop])
            at = stop
        else:
            at += 1
    return out


STREAM = re.compile(rb"stream\r?\n")

# A stream body long enough to hold a certificate and short enough that inflating every one of
# them over a corpus stays a scan rather than a job. Nothing here is a budget this program states;
# it is this script's own patience.
INFLATE_CEILING = 16 << 20

# What names a certificate a document keeps as an object: §12.8.4.3's `/DSS` and its `/Certs`,
# §12.8.4.4's `/VRI`, Table 255's `/Cert`, and the `/ByteRange` beside every signature. A file
# stating none of them in the clear is not inflated.
#
# **A deliberate trade, measured both ways, and it is not free.** Inflating every stream of every
# file costs about 0.6 s per document from the crawl and this gate costs 12 ms, which over the
# corpora this recipe is pointed at is the difference between a quarter of an hour and most of a
# day — and a recipe nobody will run seeds nothing. What it gives up, measured on 2000 crawl
# documents that state none of these keys: 118 certificates, 15 of them reachable no other way,
# every one of them inside a Flate stream. A document whose catalogue is itself in an object
# stream states `/DSS` nowhere a scan can see, and `/ObjStm` is no cheaper a gate — 40% of that
# sample states it. The raw scan below runs on every file regardless, so nothing a document
# states uncompressed is missed either way.
COLLECTIONS = (b"/ByteRange", b"/DSS", b"/Cert", b"/VRI")


def inflated_streams(data):
    """Each `stream` body in `data` that inflates, which is where a `/Certs` entry lives.

    Both windows are tried: `FlateDecode` is zlib, and a producer that wrote a raw deflate stream
    instead is exactly the kind of file this corpus is made of.
    """
    if not any(key in data for key in COLLECTIONS):
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


# A `fixtures` module's certificate, as this tree writes one: a `&str` of hexadecimal split
# across lines with a trailing backslash. The name is what says it is a certificate rather than a
# signature or a key, and the constants are matched by it rather than listed here.
FIXTURE = re.compile(rb"[A-Z0-9_]*CERTIFICATE[A-Z0-9_]*\s*:\s*&str\s*=\s*\"(.*?)\"\s*;", re.S)


def fixture_certificates(source):
    """Every certificate a Rust source file states in hexadecimal."""
    out = []
    for match in FIXTURE.finditer(source):
        try:
            value = binascii.unhexlify(re.sub(rb"[^0-9A-Fa-f]", b"", match.group(1)))
        except binascii.Error:
            continue
        out.extend(stated_certificates(value))
    return out


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


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    # Which route first reached each certificate. A chain's root is stated by a hundred documents
    # and a `/DSS` restates what the signature above it already carried, so a per-occurrence tally
    # would report tens of thousands of finds over a corpus that holds a thousand certificates.
    route_of = {}

    def keep(certificate, route):
        if len(certificate) < 50:
            return
        name = hashlib.sha1(certificate).hexdigest()
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(certificate)
        route_of.setdefault(name, route)

    for path in paths(argv[2:]):
        with open(path, "rb") as handle:
            data = handle.read()
        if path.endswith(".rs"):
            for certificate in fixture_certificates(data):
                keep(certificate, "fixture")
            continue
        if b"/ByteRange" in data:
            for match in CONTENTS.finditer(data):
                try:
                    value = binascii.unhexlify(re.sub(rb"\s", b"", match.group(1)))
                except binascii.Error:
                    continue
                for certificate in certificates(value.rstrip(b"\x00")):
                    keep(certificate, "signed")
        # The file's own bytes and then each stream's, one at a time: a document with a thousand
        # streams would otherwise hold every inflated one of them at once.
        for certificate in stated_certificates(data):
            keep(certificate, "stated")
        for buffer in inflated_streams(data):
            for certificate in stated_certificates(buffer):
                keep(certificate, "stated")

    routes = list(route_of.values())
    print(
        f"{len(route_of)} distinct certificate(s) written to {directory}: "
        f"{routes.count('signed')} first seen inside a signature, "
        f"{routes.count('stated')} stated by a document, "
        f"{routes.count('fixture')} out of a fixture"
    )


if __name__ == "__main__":
    main(sys.argv)
