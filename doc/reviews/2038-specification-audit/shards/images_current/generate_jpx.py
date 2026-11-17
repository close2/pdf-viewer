#!/usr/bin/env python3
"""Reproduce the two lossless 2x1 JP2 fixture arrays embedded in the Rust draft.

Requires Pillow 13.0.0 built with OpenJPEG (Pillow features.check('jpg_2000')).
The code-2 container differs from code-1 only in cdef's opacity-kind field; image
samples are separately authored already multiplied by opacity.
"""
from hashlib import sha256
from io import BytesIO
from PIL import Image, features

assert features.check("jpg_2000"), "Pillow needs OpenJPEG/JPEG 2000 support"


def encode_rgba(samples: bytes) -> bytearray:
    image = Image.frombytes("RGBA", (2, 1), samples)
    output = BytesIO()
    image.save(
        output,
        format="JPEG2000",
        irreversible=False,
        quality_mode="rates",
        quality_layers=[0],
    )
    return bytearray(output.getvalue())


# ISO 32000-2 Table 87 code 1: straight RGB and embedded opacity.
straight = encode_rgba(bytes([255, 0, 0, 128, 0, 0, 255, 64]))
# Code 2: RGB samples are already multiplied by opacity.
premultiplied = encode_rgba(bytes([128, 0, 0, 128, 0, 0, 64, 64]))

# Locate JP2's Channel Definition box and change only the fourth channel's
# channel type from opacity (1) to premultiplied opacity (2). Each CDEF entry
# is six bytes; channel 3's type begins 2-byte component + entry index * 6 + 2.
def set_opacity_kind(data: bytearray, kind: int) -> None:
    marker = data.index(b"cdef")
    box_start = marker - 4
    box_length = int.from_bytes(data[box_start:marker], "big")
    assert box_length == 34
    assert int.from_bytes(data[marker + 4 : marker + 6], "big") == 4
    opacity_entry = marker + 4 + 2 + 3 * 6
    assert int.from_bytes(data[opacity_entry : opacity_entry + 2], "big") == 3
    assert int.from_bytes(data[opacity_entry + 2 : opacity_entry + 4], "big") == 1
    data[opacity_entry + 2 : opacity_entry + 4] = kind.to_bytes(2, "big")


set_opacity_kind(premultiplied, 2)

for name, data in (("JP2_STRAIGHT_ALPHA", straight), ("JP2_PREMULTIPLIED_ALPHA", premultiplied)):
    print(f"{name}: {len(data)} bytes; sha256={sha256(data).hexdigest()}")
    print("&[" + ", ".join(f"0x{byte:02x}" for byte in data) + "]")
