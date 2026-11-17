#!/usr/bin/env python3
"""Build and fill an exact-xref, detached-CMS PDF signature fixture."""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

ROOM = 32768


def layout(crl_path: Path, pdf_path: Path, input_path: Path) -> None:
    crl = crl_path.read_bytes()
    first_id = b"00112233445566778899AABBCCDDEEFF"
    # Both controls share this signed base revision and later vary their DSS OCSP data; keep their
    # document identifiers equal so the xref/trailer control adds no second variable.
    second_id = hashlib.sha256(b"offline-current-crl-window-pair").hexdigest()[:32].upper().encode()
    prefix = (
        b"%PDF-2.0\n"
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> /DSS 6 0 R >>\nendobj\n"
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n"
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>\nendobj\n"
        b"4 0 obj\n<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /Name (CRL Test Signer) /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <"
    )
    suffix = (
        b"> >>\nendobj\n"
        b"5 0 obj\n<< /FT /Sig /T (CurrentCrlFixture) /V 4 0 R >>\nendobj\n"
        b"6 0 obj\n<< /Type /DSS /CRLs [7 0 R] >>\nendobj\n"
        + f"7 0 obj\n<< /Length {len(crl)} >>\nstream\n".encode()
        + crl
        + b"\nendstream\nendobj\n"
    )
    pdf = bytearray(prefix + b"0" * ROOM + suffix)
    starts = {
        int(match.group(1)): match.start()
        for match in re.finditer(rb"(?m)^(\d+) 0 obj$", pdf)
    }
    xref_at = len(pdf)
    pdf.extend(b"xref\n0 8\n0000000000 65535 f \n")
    for number in range(1, 8):
        pdf.extend(f"{starts[number]:010d} 00000 n \n".encode())
    pdf.extend(
        b"trailer\n<< /Size 8 /Root 1 0 R /ID [<"
        + first_id
        + b"><"
        + second_id
        + b">] >>\n"
    )
    pdf.extend(f"startxref\n{xref_at}\n%%EOF\n".encode())
    contents_start = pdf.index(b"/Contents <") + len(b"/Contents ")
    contents_end = pdf.index(b">", contents_start) + 1
    ranges = (0, contents_start, contents_end, len(pdf) - contents_end)
    old = b"/ByteRange [0000000000 0000000000 0000000000 0000000000]"
    new = ("/ByteRange [" + " ".join(f"{part:010d}" for part in ranges) + "]").encode()
    assert len(old) == len(new)
    position = pdf.index(old)
    pdf[position : position + len(old)] = new
    pdf_path.write_bytes(pdf)
    input_path.write_bytes(pdf[:contents_start] + pdf[contents_end:])


def finish(layout_path: Path, cms_path: Path, pdf_path: Path, input_path: Path) -> None:
    pdf = bytearray(layout_path.read_bytes())
    cms = cms_path.read_bytes()
    contents_start = pdf.index(b"/Contents <") + len(b"/Contents ")
    contents_end = pdf.index(b">", contents_start) + 1
    room = contents_end - contents_start - 2
    encoded = cms.hex().upper().encode()
    if len(encoded) > room:
        raise ValueError(f"CMS signature ({len(encoded)} hex chars) exceeds /Contents room {room}")
    pdf[contents_start + 1 : contents_end - 1] = encoded + b"0" * (room - len(encoded))
    pdf_path.write_bytes(pdf)
    input_path.write_bytes(pdf[:contents_start] + pdf[contents_end:])


def main() -> None:
    mode = sys.argv[1]
    if mode == "layout":
        layout(Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4]))
    elif mode == "finish":
        finish(Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4]), Path(sys.argv[5]))
    else:
        raise SystemExit(f"unknown mode: {mode}")


if __name__ == "__main__":
    main()
