#!/usr/bin/env python3
"""Seed `fuzz/corpus/revocation/` with every CRL and OCSP response this tree contains.

    find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \\
        | python3 fuzz/seed_revocation.py fuzz/corpus/revocation -

A `-` argument stands for a NUL-separated list of paths on standard input, which is what makes the
whole tree one run rather than one run per `xargs` batch. `find -L` because a worktree's corpora
are symbolic links to the primary checkout's.

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy, so a
seeded target needs a *recipe*; this is `revocation`'s, and it is `seed_x509.py`'s sibling in every
respect but what it recognises.

**And it is a second implementation of `pdf_signature::revocation`'s two readers**, written from
RFC 5280 section 5.1 and RFC 6960 section 4.2.1 rather than from the Rust — ADR 0229's argument for a seeder
being a program at all, and ADR 1067's for this one existing.

Two routes, because §12.8.4.2 puts the material in two places and says so: "Some of this
information, i.e. certificates, CRLs and OCSP responses, when not already present in the signature,
shall be stored in a document security store (DSS)". So a document states it as its own streams, or
a signature carries it inside §12.8.3.3.2's `adbe-revocationInfoArchival` attribute. Each find is
written out with a *definite* length whatever the file used, named by SHA-1, so a re-run adds only
what is new and the routes cannot double-count each other.
"""

import hashlib
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from seed_der import definite, inflated_streams, paths, signature_values, stated, values


def is_certificate_list(body):
    """Whether `body` is the contents of RFC 5280 section 5.1's `CertificateList`.

    Three members — `tbsCertList` and `signatureAlgorithm` are `SEQUENCE`s and `signatureValue` is
    a `BIT STRING` — and then, inside the first of them, the field list as far as `thisUpdate`.

    **The field list is what separates it from a certificate**, and `seed_x509.py`'s recogniser
    makes the same separation from the other side: §4.1's `Certificate` has the identical outer
    shape and repeats its algorithm identifier the same way, and what tells the two apart is that a
    certificate states two `Time`s inside a `SEQUENCE` where a revocation list states one `Time` as
    a direct member of `tbsCertList`.
    """
    members = values(body)
    if len(members) != 3:
        return False
    (tbs_tag, tbs_first, tbs_last), (algorithm_tag, _, _), (signature_tag, _, end) = members
    if (tbs_tag, algorithm_tag, signature_tag) != (0x30, 0x30, 0x03) or end != len(body):
        return False
    fields = values(body[tbs_first:tbs_last])
    # `version Version OPTIONAL, -- if present, MUST be v2`, an `INTEGER` with no context tag.
    if fields and fields[0][0] == 0x02:
        fields = fields[1:]
    # `signature AlgorithmIdentifier, issuer Name, thisUpdate Time`.
    if len(fields) < 3:
        return False
    return [tag for tag, _, _ in fields[:3]] == [0x30, 0x30, fields[2][0]] and fields[2][0] in (
        0x17,
        0x18,
    )


def is_ocsp_response(body):
    """Whether `body` is the contents of RFC 6960 section 4.2.1's `OCSPResponse`.

    `responseStatus` is an `ENUMERATED` and nothing else in either RFC opens that way, so this one
    recogniser needs no field list: a `SEQUENCE` whose first member is tag `0A` is an
    `OCSPResponse` or it is nothing this tree has a reader for. A second member is admitted and not
    required — the clause says an error response carries none: "If the value of responseStatus is
    one of the error conditions, the responseBytes field is not set."
    """
    members = values(body)
    if not members or members[0][0] != 0x0A:
        return False
    return len(members) <= 2 and members[-1][2] == len(body)


# `adbe-revocationInfoArchival`, `{adbe(1.2.840.113583) acrobat(1) security(1) 8}` as §12.8.3.3.2
# prints it, encoded. Found by scanning rather than by walking the attribute set, because what is
# wanted is the structures *under* it and those are self-delimiting wherever they sit.
ARCHIVAL = bytes([0x2A, 0x86, 0x48, 0x86, 0xF7, 0x2F, 0x01, 0x01, 0x08])

# The two bytes either structure begins with: a `SEQUENCE` whose length is in long form, since no
# CRL and no OCSP response is under 128 octets, and whose length fits four octets. A pattern, so
# that the scan itself runs in C over every byte of every file.
CANDIDATE = re.compile(rb"\x30[\x81-\x84]", re.S)


def stated_material(data):
    """Every CRL and OCSP response `data` states outright, as `(kind, bytes)` pairs."""
    out = [("crl", found) for found in stated(data, CANDIDATE, is_certificate_list)]
    out.extend(("ocsp", found) for found in stated(data, CANDIDATE, is_ocsp_response))
    return out


# What names revocation material a document keeps as an object: §12.8.4.3's `/DSS` with its `/CRLs`
# and `/OCSPs`, §12.8.4.4's `/VRI` with its `/CRL` and `/OCSP`, and the `/ByteRange` beside every
# signature that might carry the attribute. A file stating none of them in the clear is not
# inflated — `seed_der.inflated_streams` documents that trade and what it costs.
COLLECTIONS = (b"/ByteRange", b"/DSS", b"/CRL", b"/OCSP", b"/VRI")


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    # Which route first reached each structure. One CRL is restated by every document a certificate
    # authority's customers produced, so a per-occurrence tally would report thousands of finds
    # over a corpus holding hundreds of structures.
    route_of = {}
    kind_of = {}

    def keep(material, kind, route):
        if len(material) < 50:
            return
        name = hashlib.sha1(material).hexdigest()
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(material)
        route_of.setdefault(name, route)
        kind_of.setdefault(name, kind)

    for path in paths(argv[2:]):
        with open(path, "rb") as handle:
            data = handle.read()
        if b"/ByteRange" in data:
            for value in signature_values(data):
                value = value.rstrip(b"\x00")
                if ARCHIVAL not in value:
                    continue
                for kind, material in stated_material(value):
                    keep(material, kind, "archived")
        # The file's own bytes and then each stream's, one at a time: a document with a thousand
        # streams would otherwise hold every inflated one of them at once.
        for kind, material in stated_material(data):
            keep(material, kind, "stated")
        for buffer in inflated_streams(data, COLLECTIONS):
            for kind, material in stated_material(buffer):
                keep(material, kind, "stated")

    routes = list(route_of.values())
    kinds = list(kind_of.values())
    print(
        f"{len(route_of)} distinct structure(s) written to {directory}: "
        f"{kinds.count('crl')} revocation list(s) and {kinds.count('ocsp')} OCSP response(s); "
        f"{routes.count('archived')} first seen inside a signature's attribute, "
        f"{routes.count('stated')} stated by a document"
    )


if __name__ == "__main__":
    main(sys.argv)
