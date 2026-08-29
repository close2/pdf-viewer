#!/usr/bin/env python3
"""Seed `fuzz/corpus/cms/` with every CMS object this tree already contains.

    find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \\
        | python3 fuzz/seed_cms.py fuzz/corpus/cms -

A `-` argument stands for a NUL-separated list of paths on standard input, which is what makes the
whole tree one run rather than one run per `xargs` batch. `find -L` because a worktree's corpora
are symbolic links to the primary checkout's.

**Why a script and not a checked-in directory.** `fuzz/corpus` is gitignored by policy, so a
seeded target needs a *recipe* — and until this file existed the recipe for `cms` was a sentence
in `doc/verify.md` naming one submodule's signatures, which is the defect ADR 0751 found in
`seed_x509.py`'s argument list and named here without fixing (ADR 0754).

**Three routes, because a PDF holds a CMS object in three quite different places**, and the
structure is `seed_x509.py`'s deliberately: the two seeders read the same files for two different
things over the shared walk in `seed_der.py`. Each find is named by SHA-1, so a re-run adds only
what is new and the three cannot double-count each other.

*The signature is one.* §12.8.3.3.1: "The CMS object shall conform to Internet RFC 5652". Table
255 puts it in `/Contents` as a hexadecimal string beside the `/ByteRange` that excludes it, so a
raw scan reaches every signature stated outside an object stream. This route keeps **the file's own
bytes**, which is what makes it the route that matters: a producer's indefinite-length BER is a
shape no from-scratch input ever forms, and re-encoding it would throw away the one thing the
corpus cannot generate for itself.

*A signature carries more inside it.* RFC 3161 §3.3's `TimeStampToken` is itself a `ContentInfo`,
and a CAdES signature carries one — §12.8.3.4.3's "signature-time-stamp", and the archive
timestamps a long-term signature adds beside it — as an attribute of its `SignerInfo`. Those are
whole CMS objects that no scan of the *file* can see, because the file states them in hexadecimal
inside another CMS object. This route walks RFC 5652's `SignedData` to a signer's attributes,
which is the second implementation ADR 0229 wanted, and it does not enumerate attribute
identifiers: any attribute value that is itself a `ContentInfo` over `id-signedData` is one,
whichever of the half-dozen timestamp attributes carried it.

*The document states one as an object.* §12.8.4.4's Table 262 gives a signature VRI dictionary a
`/TS` — "[a] stream containing the DER-encoded timestamp (see Internet RFC 3161 as updated by
Internet RFC 5816 )". Reaching it through the file's structure would mean a cross-reference table,
object streams and a `/Filter` pipeline — a PDF reader, on files chosen for being malformed. A CMS
object is self-delimiting instead, so this route *proposes* by the opening bytes of RFC 5652's
`ContentInfo` and *disposes* by the same walk. Each file's `stream` bodies are inflated and scanned
as well, which is where a `/TS` entry actually lives.

**There is no fourth route out of this tree's own fixtures, and that is a finding rather than an
omission.** `seed_x509.py` has one because `crates/pdf-model/src/*.rs` state their certificates as
hexadecimal; `cms.rs`'s `fixtures` module *builds* its signature values in Rust at test time, so
there is no constant to read. What that costs is written down in `doc/verify.md`'s block for this
target.
"""

import hashlib
import os
import re
import sys

from seed_der import definite, inflated_streams, paths, signature_values, stated, value_at, values

# RFC 5652 section 5's `id-signedData`, `1.2.840.113549.1.7.2` — the only content type any of the
# signature formats §12.8.3 defines puts in `/Contents`, and what `pdf_model::cms` requires.
ID_SIGNED_DATA = bytes([0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02])


def is_signed_data(body):
    """Whether `body` is the contents of an RFC 5652 `ContentInfo` carrying a `SignedData`.

    Two members: `contentType`, which must be `id-signedData` exactly, and `content [0] EXPLICIT`
    holding one value. Unlike `seed_x509.py`'s recogniser this needs no field list to separate a
    sibling structure from the real one — the eleven octets of the identifier do that on their own,
    which is why the pattern below can carry them and the scan can be this cheap.
    """
    members = values(body)
    if len(members) < 2:
        return False
    (type_tag, type_first, type_last), (content_tag, content_first, content_last) = members[:2]
    if type_tag != 0x06 or body[type_first:type_last] != ID_SIGNED_DATA or content_tag != 0xA0:
        return False
    return bool(values(body[content_first:content_last]))


# A `ContentInfo` over `id-signedData`, from its `SEQUENCE` header through the identifier that
# names it: the length in any of X.690's forms, then the eleven octets of `id-signedData`. Written
# as a pattern so that the scan itself runs in C — over a corpus this is tried on every byte of
# every file — and each alternative consumes exactly its own length octets so that the identifier
# stays aligned.
CANDIDATE = re.compile(
    rb"\x30(?:[\x00-\x7f]|\x80|\x81.|\x82..|\x83...|\x84....)\x06\x09"
    # Escaped, because an object identifier is arbitrary octets and this one opens on `0x2A`,
    # which is `*` — a quantifier, silently applied to the length octet in front of it. The
    # pattern then matched nothing at all and every route that used it reported an honest zero.
    + re.escape(ID_SIGNED_DATA),
    re.S,
)

# What names a CMS object a document keeps outside a signature: §12.8.4.4's `/VRI` and its `/TS`,
# the `/DSS` that holds the VRI dictionary, and the `/ByteRange` beside every signature. A file
# stating none of them in the clear is not inflated; `seed_der.inflated_streams` says what that
# gate costs and what it gives up.
#
# **`/TS` is the expensive member and it is here on a measurement.** It is two octets where the
# others are five to ten, so it is named by far more files than actually hold one — 325 of a 6000
# document sample state it and none of the other three — and inflating those is about 34 minutes
# over the tree, most of this recipe's whole wall clock. What it buys, on that same sample, is 599
# CMS objects that no other route reaches, because a document whose catalogue and `/DSS` are inside
# an object stream states nothing else a scan can see. `seed_x509.py` declined a comparable trade
# for 118 certificates; this one is five times the return for one run of a recipe, so it is taken.
COLLECTIONS = (b"/ByteRange", b"/DSS", b"/VRI", b"/TS")


def signature(value):
    """A `/Contents` blob with the space its producer reserved after it removed.

    §12.8.1 has the signature written into a placeholder the producer sized in advance — the
    `/ByteRange` gap — so the hexadecimal string almost always runs on past the CMS object in
    zeros. Trimming to the object's own end is what keeps a seed the size of a signature rather
    than the size of a reservation, and a blob whose first value does not delimit is kept whole:
    a truncated CMS object is exactly the input this corpus is for.
    """
    found = value_at(value, 0)
    if found is not None and found[2] <= len(value):
        return value[: found[2]]
    return value.rstrip(b"\x00")


def signed_data_body(cms):
    """The contents of a `ContentInfo`'s `SignedData`, or `None`.

    `ContentInfo ::= SEQUENCE { contentType, content [0] EXPLICIT ANY }` and the content is
    `SignedData ::= SEQUENCE { … }`, so this is two unwraps and a tag check.
    """
    content_info = values(cms)
    if not content_info:
        return None
    _, first, last = content_info[0]
    inside = cms[first:last]
    explicit = [value for value in values(inside) if value[0] & 0xC0 == 0x80]
    if not explicit:
        return None
    _, first, last = explicit[0]
    signed = values(inside[first:last])
    if not signed:
        return None
    return inside[first:last][signed[0][1] : signed[0][2]]


# How far a timestamp inside a timestamp is followed. A long-term signature's archive timestamp
# is a CMS object over a CMS object, and each layer can carry another; this is the depth past
# which a file has stopped describing a signature and started describing this walk's stack.
MAX_NESTING = 8


def nested(cms, depth=0):
    """Every CMS object `cms` carries inside its signers' attributes, at any depth.

    `SignedData`'s last `SET` is `signerInfos` — its second is `digestAlgorithms`, which has the
    same tag — and a `SignerInfo`'s `[0]` and `[1]` members are its signed and unsigned attributes.
    Each `Attribute` is `SEQUENCE { attrType, attrValues SET OF ANY }`, and this keeps whatever
    value is a `ContentInfo` in its own right.

    Each is re-encoded with a *definite* length, whatever the file used, because it is lifted out
    of the middle of a buffer and its own header is not recoverable from the walk. That is the one
    place this recipe does not preserve a producer's encoding, and route one — which keeps the
    outer object verbatim — is where the corpus's indefinite-length BER comes from.
    """
    body = signed_data_body(cms)
    if body is None or depth >= MAX_NESTING:
        return []
    signer_infos = [value for value in values(body) if value[0] == 0x31]
    if not signer_infos:
        return []
    _, first, last = signer_infos[-1]
    out = []
    signers = body[first:last]
    for tag, signer_first, signer_last in values(signers):
        if tag != 0x30:
            continue
        signer = signers[signer_first:signer_last]
        for member_tag, member_first, member_last in values(signer):
            if member_tag not in (0xA0, 0xA1):
                continue
            out.extend(attribute_values(signer[member_first:member_last], depth))
    return out


def attribute_values(attributes, depth):
    """Every CMS object among the values of one `SET OF Attribute`, and inside those."""
    out = []
    for tag, first, last in values(attributes):
        if tag != 0x30:
            continue
        attribute = values(attributes[first:last])
        if len(attribute) < 2 or attribute[1][0] != 0x31:
            continue
        held = attributes[first:last][attribute[1][1] : attribute[1][2]]
        for value_tag, value_first, value_last in values(held):
            body = held[value_first:value_last]
            if value_tag == 0x30 and is_signed_data(body):
                token = definite(value_tag, body)
                out.append(token)
                out.extend(nested(token, depth + 1))
    return out


def main(argv):
    if len(argv) < 3:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    # Which route first reached each object. One timestamp authority answers a thousand documents
    # and a `/TS` restates what a signature above it already carried, so a per-occurrence tally
    # would report tens of thousands of finds over a corpus of a few thousand objects.
    route_of = {}

    def keep(cms, route):
        # A `ContentInfo` naming `id-signedData` is already thirteen octets of identifier, and
        # nothing under this length has a `SignerInfo` in it at all.
        if len(cms) < 32:
            return
        name = hashlib.sha1(cms).hexdigest()
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(cms)
        route_of.setdefault(name, route)

    for path in paths(argv[2:]):
        with open(path, "rb") as handle:
            data = handle.read()
        if b"/ByteRange" in data:
            for value in signature_values(data):
                # Kept whatever it is: §12.8.3.2's `adbe.x509.rsa_sha1` puts a bare PKCS #1
                # signature here rather than a CMS object, and a reader that must refuse one is a
                # reader whose refusal is worth fuzzing.
                blob = signature(value)
                keep(blob, "signature")
                for token in nested(blob):
                    keep(token, "nested")
        # The file's own bytes and then each stream's, one at a time: a document with a thousand
        # streams would otherwise hold every inflated one of them at once.
        for token in stated(data, CANDIDATE, is_signed_data):
            keep(token, "stated")
        for buffer in inflated_streams(data, COLLECTIONS):
            for token in stated(buffer, CANDIDATE, is_signed_data):
                keep(token, "stated")

    routes = list(route_of.values())
    print(
        f"{len(route_of)} distinct CMS object(s) written to {directory}: "
        f"{routes.count('signature')} a signature value, "
        f"{routes.count('nested')} inside a signer's attributes, "
        f"{routes.count('stated')} stated by a document"
    )


if __name__ == "__main__":
    main(sys.argv)
