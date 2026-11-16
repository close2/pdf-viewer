"""Build a minimal ICC.2:2023 v5 Lab display profile for the Table 65 fallback probe.

Primary basis: doc/md/ICC.2-2023.md §§7.2.20, 8.2, 8.4, 9.2.1, 9.2.38,
10.2.17, and 11.2.10. The profile uses the ICC.2-permitted mpet type for A2B0/B2A0;
the project's ICC.1 parser does not implement mpet. Profile ID is zero, which §7.2.20
allows to mean not calculated. This is a scratch fixture generator, not a production asset.
"""
import struct
from pathlib import Path

OUT = Path(__file__).with_name("icc2_v5_lab_mpe.icc")

def mluc(text: str) -> bytes:
    raw = text.encode("utf-16-be")
    return b"mluc" + b"\0" * 4 + struct.pack(">II", 1, 12) + b"enUS" + struct.pack(">II", len(raw), 28) + raw

def xyz_tag(x: float, y: float, z: float) -> bytes:
    return b"XYZ " + b"\0" * 4 + struct.pack(">iii", *(round(v * 65536) for v in (x, y, z)))

def mpet_identity_3() -> bytes:
    # ICC.2 Table 63: mpet header, 3 input/output channels, one element, position record.
    # ICC.2 Table 120: matf, 3x3 identity followed by the three zero offsets.
    mat = b"matf" + b"\0" * 4 + struct.pack(">HH", 3, 3)
    mat += struct.pack(">12f", 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)
    offset = 24
    return b"mpet" + b"\0" * 4 + struct.pack(">HHI", 3, 3, 1) + struct.pack(">II", offset, len(mat)) + mat

def build() -> bytes:
    tags = [
        (b"desc", mluc("ICC.2 Lab fallback test")),
        (b"cprt", mluc("Synthetic test profile")),
        (b"wtpt", xyz_tag(0.9642, 1.0, 0.8249)),
        (b"A2B0", mpet_identity_3()),
        (b"B2A0", mpet_identity_3()),
    ]
    header = bytearray(128)
    header[8:12] = bytes((5, 0, 0, 0))
    header[12:16] = b"mntr"
    header[16:20] = b"Lab "
    header[20:24] = b"XYZ "
    header[24:36] = struct.pack(">6H", 2026, 11, 16, 0, 0, 0)
    header[36:40] = b"acsp"
    header[40:44] = b"APPL"
    header[64:68] = struct.pack(">I", 0)
    header[68:80] = xyz_tag(0.9642, 1.0, 0.8249)[8:20]
    # A zero Profile ID is explicitly permitted by ICC.2 §7.2.20.
    table_end = 128 + 4 + 12 * len(tags)
    out = bytearray(header)
    out += struct.pack(">I", len(tags))
    body = bytearray()
    entries = []
    for signature, payload in tags:
        while (table_end + len(body)) % 4:
            body.append(0)
        offset = table_end + len(body)
        entries.append((signature, offset, len(payload)))
        body += payload
    for entry in entries:
        out += struct.pack(">4sII", *entry)
    out += body
    struct.pack_into(">I", out, 0, len(out))
    return bytes(out)

if __name__ == "__main__":
    data = build()
    OUT.write_bytes(data)
    print(f"{OUT}: {len(data)} bytes; v{data[8]}.{data[9] >> 4}; class={data[12:16]!r}; data={data[16:20]!r}; PCS={data[20:24]!r}")
