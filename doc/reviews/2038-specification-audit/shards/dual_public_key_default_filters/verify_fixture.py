#!/usr/bin/env python3
"""Verify the *emitted PDF bytes* and decrypt their actual CMS/AES payloads with OpenSSL.

Usage from any directory:
  python3 .../verify_fixture.py DISTINCT.pdf SHARED_CONTROL.pdf

The checked-in testnutzer DER certificate/private key are used only as the fixture recipient.
The verifier validates classic xref offsets, extracts the Encrypt dictionary, the Catalog /Lang
hex string and the page content stream by xref object, then decrypts those extracted bytes.
"""

from __future__ import annotations

import hashlib
import pathlib
import re
import subprocess
import sys
import tempfile


ROOT = next(parent for parent in pathlib.Path(__file__).resolve().parents if (parent / "Cargo.toml").exists())
CERT_DER = ROOT / "crates/pdf-syntax/tests/public_key_witnesses/testnutzer.certificate.der"
KEY_DER = ROOT / "crates/pdf-syntax/tests/public_key_witnesses/testnutzer.key.der"
EXPECTED_STREAM = b"BT /F1 12 Tf 20 50 Td (Stream key) Tj ET"
EXPECTED_LANG = b"fr-FR"
EXPECTED_PERMISSION_BYTES = bytes.fromhex("ffffff f4")


def run(*args: str) -> bytes:
    return subprocess.run(args, check=True, stdout=subprocess.PIPE).stdout


def exact_classic_xref(pdf: bytes) -> dict[int, bytes]:
    startxref = int(pdf.rsplit(b"startxref\n", 1)[1].splitlines()[0])
    assert pdf[startxref : startxref + 5] == b"xref\n"
    lines = pdf[startxref:].splitlines()
    assert lines[1] == b"0 7"
    offsets: dict[int, int] = {}
    for object_number in range(1, 7):
        line = lines[object_number + 2]
        assert line[10:] == b" 00000 n "
        offsets[object_number] = int(line[:10])
        assert pdf[offsets[object_number] :].startswith(f"{object_number} 0 obj\n".encode())

    objects: dict[int, bytes] = {}
    for object_number, offset in offsets.items():
        end = pdf.find(b"endobj", offset)
        assert end >= 0
        objects[object_number] = pdf[offset:end]
    return objects


def recipient_entries(encrypt_object: bytes) -> tuple[tuple[bytes, bytes], dict[bytes, tuple[bytes, int]]]:
    default_match = re.search(rb"/StmF\s*/([A-Za-z0-9_.+-]+)\s+/StrF\s*/([A-Za-z0-9_.+-]+)", encrypt_object)
    assert default_match, "both independent default names should be present"
    defaults = (default_match.group(1), default_match.group(2))
    entries: dict[bytes, tuple[bytes, int]] = {}
    for match in re.finditer(
        rb"/([A-Za-z0-9_.+-]+)\s*<<\s*/Type\s*/CryptFilter\b(.*?)>>", encrypt_object, re.S
    ):
        name, body = match.group(1), match.group(2)
        assert b"/CFM /AESV2" in body
        length = re.search(rb"/Length\s+(\d+)", body)
        recipients = re.search(rb"/Recipients\s*\[\s*<([0-9A-Fa-f]+)>\s*\]", body)
        assert length and int(length.group(1)) == 128, "Table 25 requires AESV2 Length 128 bits"
        assert recipients, "Table 27 requires recipient byte-string array for StmF/StrF filters"
        entries[name] = (bytes.fromhex(recipients.group(1).decode("ascii")), int(length.group(1)))
    assert all(name in entries for name in defaults)
    return defaults, entries


def decrypt_cms(cms: bytes, cert_pem: pathlib.Path, key_pem: pathlib.Path, work: pathlib.Path, label: str) -> bytes:
    cms_file = work / f"{label}.cms.der"
    cms_file.write_bytes(cms)
    return run(
        "openssl", "cms", "-decrypt", "-binary", "-inform", "DER", "-in", str(cms_file),
        "-recip", str(cert_pem), "-inkey", str(key_pem),
    )


def object_key(file_key: bytes, number: int, generation: int = 0) -> bytes:
    material = file_key + number.to_bytes(3, "little") + generation.to_bytes(2, "little") + b"sAlT"
    return hashlib.md5(material).digest()[: min(len(file_key) + 5, 16)]


def decrypt_aesv2(payload: bytes, key: bytes, work: pathlib.Path, label: str) -> bytes:
    assert len(payload) >= 32 and (len(payload) - 16) % 16 == 0
    iv, ciphertext = payload[:16], payload[16:]
    path = work / f"{label}.cipher"
    path.write_bytes(ciphertext)
    return run(
        "openssl", "enc", "-d", "-aes-128-cbc", "-K", key.hex(), "-iv", iv.hex(), "-in", str(path)
    )


def check_pdf(path: pathlib.Path, cert_pem: pathlib.Path, key_pem: pathlib.Path, work: pathlib.Path,
              *, distinct: bool) -> None:
    pdf = path.read_bytes()
    objects = exact_classic_xref(pdf)
    encrypt_object = objects[5]
    assert b"/Filter /Adobe.PubSec" in encrypt_object
    assert b"/SubFilter /adbe.pkcs7.s5" in encrypt_object
    defaults, entries = recipient_entries(encrypt_object)
    if distinct:
        assert defaults[0] != defaults[1]
    else:
        assert defaults[0] == defaults[1]

    file_keys: dict[bytes, bytes] = {}
    for name in set(defaults):
        cms, _ = entries[name]
        envelope_plain = decrypt_cms(cms, cert_pem, key_pem, work, name.decode("ascii"))
        assert len(envelope_plain) == 24
        seed, permissions = envelope_plain[:20], envelope_plain[20:]
        assert permissions == EXPECTED_PERMISSION_BYTES
        # Public-key §7.6.5.3 derivation for one recipient: SHA-1(seed || CMS recipient bytes).
        file_keys[name] = hashlib.sha1(seed + cms).digest()[:16]

    if distinct:
        assert file_keys[defaults[0]] != file_keys[defaults[1]]

    lang_match = re.search(rb"/Lang\s*<([0-9A-Fa-f]+)>", objects[1])
    assert lang_match
    lang_cipher = bytes.fromhex(lang_match.group(1).decode("ascii"))
    lang_plain = decrypt_aesv2(lang_cipher, object_key(file_keys[defaults[1]], 1), work, "catalog-lang")
    assert lang_plain == EXPECTED_LANG, (path.name, lang_plain)

    length_match = re.search(rb"/Length\s+(\d+)\s*>>\s*stream\n", objects[4])
    assert length_match
    stream_length = int(length_match.group(1))
    stream_start = length_match.end()
    stream_cipher = objects[4][stream_start : stream_start + stream_length]
    assert len(stream_cipher) == stream_length
    assert objects[4][stream_start + stream_length :].startswith(b"\nendstream")
    stream_plain = decrypt_aesv2(stream_cipher, object_key(file_keys[defaults[0]], 4), work, "page-content")
    assert stream_plain == EXPECTED_STREAM, (path.name, stream_plain)

    # Table 27: EncryptMetadata is applicable only to an StmF-referenced crypt filter;
    # its default is true. The generated dictionaries omit it and contain no metadata stream.
    assert b"/EncryptMetadata" not in encrypt_object
    print(f"verified {path}: xref, CMS recipient(s), AESV2 Catalog /Lang and page stream; EncryptMetadata default=true")


def main(paths: list[pathlib.Path]) -> None:
    assert len(paths) == 2
    with tempfile.TemporaryDirectory(prefix="verify-s5-pdfs-", dir=pathlib.Path(__file__).resolve().parent) as temp:
        work = pathlib.Path(temp)
        cert_pem = work / "recipient.pem"
        key_pem = work / "recipient-key.pem"
        run("openssl", "x509", "-inform", "DER", "-in", str(CERT_DER), "-out", str(cert_pem))
        run("openssl", "pkey", "-inform", "DER", "-in", str(KEY_DER), "-out", str(key_pem))
        check_pdf(paths[0], cert_pem, key_pem, work, distinct=True)
        check_pdf(paths[1], cert_pem, key_pem, work, distinct=False)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} DISTINCT.pdf SHARED_CONTROL.pdf")
    main([pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])])
