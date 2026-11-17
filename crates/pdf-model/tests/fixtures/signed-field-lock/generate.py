#!/usr/bin/env python3
"""Generate a valid CAdES PDF signature fixture plus an unsigned field control.

The private key and certificate are fresh test-only credentials written under the output
directory. Regeneration is semantically equivalent but not byte-identical (CMS signingTime).
"""
from pathlib import Path
import subprocess
import sys

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).parent / "fixtures")
OUT.mkdir(parents=True, exist_ok=True)
OUT = OUT.resolve()
ROOM = 16384
OLD = b"/ByteRange [0000000000 0000000000 0000000000 0000000000]"


def run(*args: str) -> None:
    subprocess.run(args, check=True)


def document(signed: bool) -> bytearray:
    value = "/V 4 0 R " if signed else ""
    appearance = b"q 0 g 0 0 100 20 re f Q\n"
    bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 7 0 R] /SigFlags 1 >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 120] /Resources << >> /Annots [5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /Name (Test Only Lock Fixture) /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <",
        f"<< /Type /Annot /Subtype /Widget /FT /Sig /T (signature) {value}/Lock 8 0 R /Rect [0 0 0 0] /F 4 /P 3 0 R /AP << /N 9 0 R >> >>",
        "<< /Type /Annot /Subtype /Widget /FT /Btn /T (name) /V /Off /AS /Off /Rect [10 10 110 30] /F 4 /P 3 0 R /AP << /N << /Off 10 0 R /On 9 0 R >> >> >>",
        "<< /Type /Annot /Subtype /Widget /FT /Btn /T (other) /V /Off /AS /Off /Rect [10 50 110 70] /F 4 /P 3 0 R /AP << /N << /Off 10 0 R /On 9 0 R >> >> >>",
        "<< /Type /SigFieldLock /Action /Include /Fields [(name)] >>",
        f"<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 20] /Resources << >> /Length {len(appearance)} >>\nstream\n{appearance.decode()}endstream",
        "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 20] /Resources << >> /Length 0 >>\nstream\nendstream",
    ]
    data = bytearray(b"%PDF-2.0\n")
    offsets = []
    for number, body in enumerate(bodies, start=1):
        offsets.append(len(data))
        if number == 4:
            if signed:
                data.extend(f"{number} 0 obj\n{body}".encode())
                data.extend(b"0" * ROOM)
                data.extend(b"> >>\nendobj\n")
            else:
                data.extend(b"4 0 obj\nnull\nendobj\n")
        else:
            data.extend(f"{number} 0 obj\n{body}\nendobj\n".encode())
    xref = len(data)
    data.extend(b"xref\n0 11\n0000000000 65535 f \n")
    for offset in offsets:
        data.extend(f"{offset:010d} 00000 n \n".encode())
    data.extend(
        f"trailer\n<< /Size 11 /Root 1 0 R /ID [<30313233343536373839616263646566> <30313233343536373839616263646566>] >>\nstartxref\n{xref}\n%%EOF\n".encode()
    )
    if signed:
        pos = data.index(b"/Contents <") + len(b"/Contents ")
        end = data.index(b">", pos) + 1
        replacement = f"/ByteRange [0000000000 {pos:010d} {end:010d} {len(data)-end:010d}]".encode()
        assert len(replacement) == len(OLD)
        old_pos = data.index(OLD)
        data[old_pos : old_pos + len(OLD)] = replacement
    return data


def main() -> None:
    run("openssl", "req", "-new", "-x509", "-newkey", "rsa:2048", "-sha256", "-nodes", "-days", "3650",
        "-subj", "/CN=Quorra Form Lock Fixture/O=Quorra Test Only", "-addext", "basicConstraints=critical,CA:FALSE",
        "-addext", "keyUsage=critical,digitalSignature", "-keyout", str(OUT / "test-only-key.pem"),
        "-out", str(OUT / "test-only-cert.pem"))
    run("openssl", "x509", "-in", str(OUT / "test-only-cert.pem"), "-outform", "DER", "-out", str(OUT / "test-only-cert.der"))
    pdf = document(True)
    value_start = pdf.index(b"/Contents <") + len(b"/Contents ")
    value_end = pdf.index(b">", value_start) + 1
    unsigned_bytes = bytes(pdf[:value_start] + pdf[value_end:])
    (OUT / "cades-signed-content.bin").write_bytes(unsigned_bytes)
    run("openssl", "cms", "-sign", "-binary", "-cades", "-md", "sha256", "-in", str(OUT / "cades-signed-content.bin"),
        "-signer", str(OUT / "test-only-cert.pem"), "-inkey", str(OUT / "test-only-key.pem"), "-outform", "DER", "-out", str(OUT / "signature.der"))
    signature = (OUT / "signature.der").read_bytes().hex().upper().encode()
    capacity = value_end - value_start - 2
    assert len(signature) <= capacity
    pdf[value_start + 1 : value_end - 1] = signature + b"0" * (capacity - len(signature))
    (OUT / "signed-lock.pdf").write_bytes(pdf)
    unsigned = document(False)
    (OUT / "unsigned-lock.pdf").write_bytes(unsigned)
    run("openssl", "cms", "-verify", "-binary", "-inform", "DER", "-in", str(OUT / "signature.der"),
        "-content", str(OUT / "cades-signed-content.bin"), "-CAfile", str(OUT / "test-only-cert.pem"), "-out", "/dev/null")
    run("qpdf", "--check", str(OUT / "signed-lock.pdf"))
    run("qpdf", "--check", str(OUT / "unsigned-lock.pdf"))
    run("openssl", "x509", "-in", str(OUT / "test-only-cert.pem"), "-noout", "-dates", "-subject")
    (OUT / "test-only-key.pem").unlink(missing_ok=True)
    print(f"Generated exact-xref signed and unsigned controls under {OUT}")


if __name__ == "__main__":
    main()
