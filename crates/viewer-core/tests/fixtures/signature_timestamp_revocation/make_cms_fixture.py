#!/usr/bin/env python3
"""Extract or append the signature-time-stamp unsigned CMS attribute.

OpenSSL creates a genuinely signed detached CMS value. This script first extracts the actual
SignerInfo.signature octets so OpenSSL TS can issue a token over them. It then adds the token as an
unsigned attribute, leaving all signed bytes unchanged.
"""
import sys
from pathlib import Path


def tlv(tag: int, content: bytes) -> bytes:
    length = len(content)
    if length < 128:
        head = bytes([length])
    else:
        encoded = length.to_bytes((length.bit_length() + 7) // 8, "big")
        head = bytes([0x80 | len(encoded)]) + encoded
    return bytes([tag]) + head + content


def seq(*parts: bytes) -> bytes:
    return tlv(0x30, b"".join(parts))


def oid(text: str) -> bytes:
    arcs = [int(x) for x in text.split(".")]
    encoded = bytearray([40 * arcs[0] + arcs[1]])
    for arc in arcs[2:]:
        items = [arc & 0x7F]
        arc >>= 7
        while arc:
            items.append(0x80 | (arc & 0x7F))
            arc >>= 7
        encoded.extend(reversed(items))
    return tlv(0x06, bytes(encoded))


def read_tlv(data: bytes, at: int = 0) -> tuple[int, bytes, int]:
    tag = data[at]
    first = data[at + 1]
    if first < 0x80:
        length, start = first, at + 2
    else:
        count = first & 0x7F
        length = int.from_bytes(data[at + 2 : at + 2 + count], "big")
        start = at + 2 + count
    end = start + length
    return tag, data[start:end], end


def children(contents: bytes) -> list[bytes]:
    result: list[bytes] = []
    at = 0
    while at < len(contents):
        _, _, end = read_tlv(contents, at)
        result.append(contents[at:end])
        at = end
    if at != len(contents):
        raise ValueError("trailing bytes in DER container")
    return result


def content(raw: bytes) -> tuple[int, bytes]:
    tag, inside, end = read_tlv(raw)
    if end != len(raw):
        raise ValueError("trailing top-level DER bytes")
    return tag, inside


def signer_parts(cms: bytes) -> tuple[list[bytes], list[bytes], list[bytes]]:
    _, ci = content(cms)
    ci_parts = children(ci)
    _, signed_data_explicit = content(ci_parts[1])
    _, signed_data = content(signed_data_explicit)
    sd_parts = children(signed_data)
    _, signer_set = content(sd_parts[-1])
    signer_infos = children(signer_set)
    if len(signer_infos) != 1:
        raise ValueError("fixture expects one SignerInfo")
    _, signer_content = content(signer_infos[0])
    return ci_parts, sd_parts, children(signer_content)


def extract_signature(cms: bytes) -> bytes:
    _, _, parts = signer_parts(cms)
    sig_index = next(i for i, part in enumerate(parts) if content(part)[0] == 0x04)
    return content(parts[sig_index])[1]


def attach_token(cms: bytes, token: bytes) -> bytes:
    ci_parts, sd_parts, signer_info_parts = signer_parts(cms)
    if signer_info_parts[-1][0] == 0xA1:
        raise ValueError("base signer unexpectedly already has unsigned attributes")
    timestamp_attribute = seq(
        oid("1.2.840.113549.1.9.16.2.14"),
        tlv(0x31, token),
    )
    signer_info = seq(*(signer_info_parts + [tlv(0xA1, timestamp_attribute)]))
    sd_parts[-1] = tlv(0x31, signer_info)
    ci_parts[1] = tlv(0xA0, seq(*sd_parts))
    return seq(*ci_parts)


def main() -> None:
    root = Path.cwd()
    mode = sys.argv[1]
    if mode == "extract-signature":
        cms = (root / sys.argv[2]).read_bytes()
        (root / sys.argv[3]).write_bytes(extract_signature(cms))
    elif mode == "attach-token":
        token = (root / sys.argv[2]).read_bytes()
        cms = (root / sys.argv[3]).read_bytes()
        (root / sys.argv[4]).write_bytes(attach_token(cms, token))
    else:
        raise ValueError(f"unknown mode: {mode}")


if __name__ == "__main__":
    main()
