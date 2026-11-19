#!/usr/bin/env python3
"""Generates the signed fixtures of review 2128's §12.8 audit tests (`signature_value_spec_audit.rs`).

One RSA 2048 key and one self-signed certificate, made fresh on every run and discarded; the name
says it is a test and no authority issued it. Each fixture is the same five-object document with one
signature field, its `/ByteRange` leaving out exactly the `/Contents` hexadecimal string, and a
value made here:

  sha1-with-attributes.pdf   adbe.pkcs7.sha1: the SHA-1 of the byte range encapsulated as id-data
                             content, with OpenSSL's signed attributes (content-type, signing-time,
                             message-digest of that content) - ISO 32000-2 §12.8.3.3.1's shape.
  detached-encapsulating.pdf adbe.pkcs7.detached, but the CMS object encapsulates the 17 bytes
                             "not this document" and carries no signed attributes, so the signature
                             is over those bytes and not over the document.
  pades-no-content-type.pdf  ETSI.CAdES.detached, whole-file range, one signer, message-digest and
                             signing-certificate-v2 signed - and no content-type signed attribute,
                             which §12.8.3.4.3 (a) requires.

Run from anywhere: `python3 generate.py`. Keys are fresh on every run, so the bytes differ, and the
facts the tests read (which value is signed over what, which attribute is absent) do not.
"""
import hashlib
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOM = 8192  # hexadecimal characters reserved for the signature value
PLACEHOLDER = b"[0000000000 0000000000 0000000000 0000000000]"


def document(sub_filter):
    """The five objects and the cross-reference table, with an empty value of ROOM digits."""
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
        b"<< /Type /Pages /Count 0 /Kids [] >>",
        b"<< /Payload (the signed revision) >>",
        b"<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget /Rect [0 0 0 0] >>",
        b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /" + sub_filter
        + b" /ByteRange " + PLACEHOLDER + b" /Contents <" + b"0" * ROOM + b"> >>",
    ]
    out = bytearray(b"%PDF-1.7\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    for offset in offsets:
        out += b"%010d 00000 n \n" % offset
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objects) + 1, xref)
    return out


def ranged(sub_filter):
    """The document with its /ByteRange written; returns the bytes and the two signed parts."""
    out = document(sub_filter)
    start = out.index(b"/Contents <") + len(b"/Contents ")
    end = start + ROOM + 2
    tail = len(out) - end
    value = b"[%010d %010d %010d %010d]" % (0, start, end, tail)
    at = out.index(PLACEHOLDER)
    out[at:at + len(PLACEHOLDER)] = value
    return out, start, end


def fill(out, start, end, value):
    digits = value.hex().encode()
    assert len(digits) <= ROOM, "the value fits its hole"
    out[start + 1:end - 1] = digits + b"0" * (ROOM - len(digits))
    return bytes(out)


def openssl(*args, data=None):
    return subprocess.run(["openssl", *args], input=data, check=True, capture_output=True).stdout


def der(tag, contents):
    n = len(contents)
    if n < 0x80:
        length = bytes([n])
    else:
        octets = n.to_bytes((n.bit_length() + 7) // 8, "big")
        length = bytes([0x80 | len(octets)]) + octets
    return bytes([tag]) + length + contents


def oid(text):
    parts = [int(p) for p in text.split(".")]
    body = bytes([40 * parts[0] + parts[1]])
    for part in parts[2:]:
        chunk = [part & 0x7F]
        part >>= 7
        while part:
            chunk.insert(0, 0x80 | (part & 0x7F))
            part >>= 7
        body += bytes(chunk)
    return der(0x06, body)


def seq(*items):
    return der(0x30, b"".join(items))


def set_of(*items):
    # X.690 clause 11.6: a SET OF is ordered by its members' encodings.
    return der(0x31, b"".join(sorted(items)))


def main():
    work = tempfile.mkdtemp()
    key = os.path.join(work, "signer.key")
    cert = os.path.join(work, "signer.pem")
    openssl("genpkey", "-quiet", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", key)
    openssl("req", "-new", "-x509", "-key", key, "-days", "3650", "-sha256", "-subj",
            "/O=quorra test fixture/CN=quorra spec audit 2128 signer", "-out", cert)
    cert_der = openssl("x509", "-in", cert, "-outform", "DER")

    def cms_sign(content, *extra):
        source = os.path.join(work, "content.bin")
        with open(source, "wb") as handle:
            handle.write(content)
        return openssl("cms", "-sign", "-binary", "-nodetach", "-nosmimecap", "-md", "sha256",
                       "-in", source, "-signer", cert, "-inkey", key, "-outform", "DER", *extra)

    # adbe.pkcs7.sha1, with signed attributes over the encapsulated digest.
    out, start, end = ranged(b"adbe.pkcs7.sha1")
    digest = hashlib.sha1(bytes(out[:start]) + bytes(out[end:])).digest()
    write("sha1-with-attributes.pdf", fill(out, start, end, cms_sign(digest)))

    # adbe.pkcs7.detached whose CMS object signs encapsulated bytes that are not the document.
    out, start, end = ranged(b"adbe.pkcs7.detached")
    write("detached-encapsulating.pdf",
          fill(out, start, end, cms_sign(b"not this document", "-noattr")))

    # ETSI.CAdES.detached without a content-type signed attribute, assembled by hand.
    out, start, end = ranged(b"ETSI.CAdES.detached")
    document_digest = hashlib.sha256(bytes(out[:start]) + bytes(out[end:])).digest()
    sha256 = seq(oid("2.16.840.1.101.3.4.2.1"))
    attributes = [
        seq(oid("1.2.840.113549.1.9.4"), set_of(der(0x04, document_digest))),
        seq(oid("1.2.840.113549.1.9.16.2.47"),
            set_of(seq(seq(seq(der(0x04, hashlib.sha256(cert_der).digest())))))),
    ]
    signed = set_of(*attributes)
    signature = openssl("dgst", "-sha256", "-sign", key, data=signed)
    tbs = cert_der_tbs(cert_der)
    signer_info = seq(
        der(0x02, b"\x01"),
        seq(tbs["issuer"], tbs["serial"]),
        sha256,
        der(0xA0, b"".join(sorted(attributes))),
        seq(oid("1.2.840.113549.1.1.1"), der(0x05, b"")),
        der(0x04, signature),
    )
    signed_data = seq(
        der(0x02, b"\x01"),
        set_of(sha256),
        seq(oid("1.2.840.113549.1.7.1")),
        der(0xA0, cert_der),
        set_of(signer_info),
    )
    content_info = seq(oid("1.2.840.113549.1.7.2"), der(0xA0, signed_data))
    write("pades-no-content-type.pdf", fill(out, start, end, content_info))


def children(encoding):
    """The TLVs inside one DER value, as whole encodings."""
    _, header, length = head(encoding)
    body = encoding[header:header + length]
    out = []
    while body:
        _, h, n = head(body)
        out.append(body[:h + n])
        body = body[h + n:]
    return out


def head(encoding):
    first = encoding[1]
    if first < 0x80:
        return encoding[0], 2, first
    count = first & 0x7F
    return encoding[0], 2 + count, int.from_bytes(encoding[2:2 + count], "big")


def cert_der_tbs(cert_der):
    # Certificate -> tbsCertificate -> [version], serialNumber, signature, issuer, ...
    tbs = children(children(cert_der)[0])
    index = 1 if tbs[0][0] == 0xA0 else 0
    return {"serial": tbs[index], "issuer": tbs[index + 2]}


def write(name, data):
    with open(os.path.join(HERE, name), "wb") as handle:
        handle.write(data)
    print(name, len(data), file=sys.stderr)


if __name__ == "__main__":
    main()
