"""Build a complete ICC.1 v4.4 monochrome display profile for a PDF consumer draft.

The required-tag basis is ICC.1:2022 Annex G, Table G.6. Three optional A2B
intent transforms deliberately give one gray input distinct XYZ results. This
file is audit evidence, not a production profile asset.
"""
from __future__ import annotations

from datetime import datetime
from itertools import product
from pathlib import Path
import struct

OUT = Path(__file__).with_name("gray-display-three-intents.icc")
D50 = (0.9642, 1.0, 0.8249)


def xyz_tag(values: tuple[float, float, float]) -> bytes:
    return b"XYZ " + bytes(4) + struct.pack(">iii", *(round(v * 65536) for v in values))


def mluc_tag(text: str) -> bytes:
    raw = text.encode("utf-16-be")
    # ICC.1 multiLocalizedUnicodeType with one en-US record; offsets are
    # relative to the beginning of this tag data element.
    return b"mluc" + bytes(4) + struct.pack(">II", 1, 12) + b"enUS" + struct.pack(">II", len(raw), 28) + raw


def identity_gray_trc() -> bytes:
    # ICC.1 curveType with count zero denotes the identity response.
    return b"curv" + bytes(4) + struct.pack(">I", 0)


def fixed15(value: float) -> bytes:
    return struct.pack(">H", min(65535, max(0, round(value * 32768))))


def lut16_gray_to_xyz(mid: tuple[float, float, float]) -> bytes:
    """One input, three output, three-grid-point mft2 transform."""
    out = bytearray(b"mft2" + bytes(4) + bytes((1, 3, 3, 0)))
    # Identity 3x3 matrix in s15Fixed16.
    out.extend(struct.pack(">9i", 65536, 0, 0, 0, 65536, 0, 0, 0, 65536))
    out.extend(struct.pack(">HH", 2, 2))  # input/output table entry counts
    out.extend(struct.pack(">HH", 0, 65535))
    for gray in (0.0, 0.5, 1.0):
        pcs = tuple(D50[i] * gray for i in range(3))
        if gray == 0.5:
            pcs = mid
        for value in pcs:
            out.extend(fixed15(value))
    for _channel in range(3):
        out.extend(struct.pack(">HH", 0, 65535))
    return bytes(out)


def build() -> bytes:
    tags = {
        b"A2B0": lut16_gray_to_xyz((0.45, 0.25, 0.05)),
        # The colorimetric gray midpoint follows the PCS D50 neutral axis.
        b"A2B1": lut16_gray_to_xyz(tuple(v * 0.25 for v in D50)),
        b"A2B2": lut16_gray_to_xyz((0.12, 0.25, 0.75)),
        b"cprt": mluc_tag("Audit test profile; not a real device characterization."),
        b"desc": mluc_tag("Audit gray display profile with distinct intents"),
        b"kTRC": identity_gray_trc(),
        b"wtpt": xyz_tag(D50),
    }
    tags = dict(sorted(tags.items()))
    header = bytearray(128)
    header[4:8] = bytes(4)  # no preferred CMM
    header[8:12] = bytes((4, 0x40, 0, 0))  # ICC.1 v4.4.0
    header[12:16] = b"mntr"  # Display
    header[16:20] = b"GRAY"
    header[20:24] = b"XYZ "
    now = datetime(2026, 11, 17, 0, 0, 0)
    header[24:36] = struct.pack(">6H", now.year, now.month, now.day, now.hour, now.minute, now.second)
    header[36:40] = b"acsp"
    header[40:44] = bytes(4)  # no platform
    header[44:48] = bytes(4)  # no profile flags set
    header[48:56] = bytes(8)  # no manufacturer/model
    header[56:64] = bytes(8)  # media attributes zero
    header[64:68] = struct.pack(">I", 0)  # header default intent: perceptual
    header[68:80] = xyz_tag(D50)[8:20]
    header[80:84] = bytes(4)  # no creator signature
    header[84:100] = bytes(16)  # zero means profile ID not calculated
    header[100:128] = bytes(28)

    table_end = 128 + 4 + 12 * len(tags)
    body = bytearray()
    entries = []
    for signature, payload in tags.items():
        while (table_end + len(body)) % 4:
            body.append(0)
        offset = table_end + len(body)
        entries.append((signature, offset, len(payload)))
        body.extend(payload)
    profile = bytearray(header)
    profile.extend(struct.pack(">I", len(entries)))
    for entry in entries:
        profile.extend(struct.pack(">4sII", *entry))
    profile.extend(body)
    struct.pack_into(">I", profile, 0, len(profile))
    return bytes(profile)


if __name__ == "__main__":
    data = build()
    OUT.write_bytes(data)
    print(f"{OUT}: {len(data)} bytes; v4.4; mntr/GRAY/XYZ; tags={struct.unpack_from('>I', data, 128)[0]}")
