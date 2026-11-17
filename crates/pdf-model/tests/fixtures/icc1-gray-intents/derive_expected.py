"""Derive the RGB literals used by the PDF consumer draft from ICC bytes.

The table-selection mapping is ICC.1:2022 Table 25 / §§9.2.1–9.2.3.
The LUT knots are read from the generated mft2 tags rather than copied from the
generator. The XYZ-to-sRGB matrix and transfer curve are the same explicit,
published constants used by the sibling ICC consumer tests. This predicts this
renderer profile's display pixels; it is not an ISO-mandated screen rendering.
"""
from pathlib import Path
import hashlib
import struct

HERE = Path(__file__).resolve().parent
data = (HERE / "gray-display-three-intents.icc").read_bytes()
tags = struct.unpack_from(">I", data, 128)[0]
entries = {}
for index in range(tags):
    sig, offset, size = struct.unpack_from(">4sII", data, 132 + 12 * index)
    entries[sig] = (offset, size)

matrix = (
    (3.134136, -1.617036, -0.490662),
    (-0.978755, 1.916142, 0.033454),
    (0.071950, -0.228988, 1.405386),
)

def as_srgb(xyz):
    result = []
    for row in matrix:
        linear = min(1.0, max(0.0, sum(a * b for a, b in zip(row, xyz))))
        encoded = 12.92 * linear if linear <= 0.0031308 else 1.055 * linear ** (1.0 / 2.4) - 0.055
        result.append(round(255.0 * encoded))
    return result

print(f"profile_sha256={hashlib.sha256(data).hexdigest()}")
print("sample=Gray 0.5 (exact middle knot of 3-point mft2 grid)")
for tag in (b"A2B0", b"A2B1", b"A2B2"):
    offset, size = entries[tag]
    assert data[offset:offset + 4] == b"mft2" and size == 86
    input_channels, output_channels, grid_points = data[offset + 8:offset + 11]
    assert (input_channels, output_channels, grid_points) == (1, 3, 3)
    input_entries, output_entries = struct.unpack_from(">HH", data, offset + 48)
    assert (input_entries, output_entries) == (2, 2)
    clut_offset = offset + 52 + input_channels * input_entries * 2
    knots = struct.unpack_from(">9H", data, clut_offset)
    # The middle grid point contains three consecutive unsigned 1.15 XYZ values.
    xyz = tuple(knots[3 + c] / 32768.0 for c in range(3))
    print(f"{tag.decode()} xyz={xyz!r} rgb={as_srgb(xyz)}")
