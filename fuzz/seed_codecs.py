#!/usr/bin/env python3
"""Keep one `jbig2` or `jpx` seed per shape, out of what `image_codec_seeds` wrote.

    python3 fuzz/seed_codecs.py <jbig2|jpx> <every seed's directory> <corpus directory>

`pdf-model`'s `image_codec_seeds` example finds every `JBIG2Decode` and `JPXDecode` codestream a
document holds through `pdf_syntax::Document`, in the framing each target reads, and writes them
all. This script is the selection after it (ADR 1571): it reads that directory, computes each
seed's shape, and writes the smallest of each shape into the corpus directory.

**The shapes**, and why each is the one. What decides which branches a codec takes is how the
codestream is organised and coded, never what the picture shows:

- `jbig2`: whether globals are present, and for the globals and for the page's own segments the
  set of segment types each states with the flags that choose the decoding procedure — a generic
  region's MMR and template bits, a text region's or a symbol dictionary's flag words, a page
  information segment's flags and striping (ITU-T T.88 sections 7.2 to 7.4, cited and not quoted).
  The bitmaps the procedures produce are their data.
- `jpx`: whether the file is a JP2 box structure or a bare codestream, which boxes it states and
  the colour specification method and enumerated space, and from the codestream's main header the
  capabilities, the components' precision and subsampling, whether it is tiled, the coding style's
  progression order, layer count's order, multiple component transform, decomposition levels,
  code-block size and style and wavelet, the quantisation style, and which other marker segments
  the main header holds (ISO/IEC 15444-1 Annexes A and I, cited and not quoted).

Both shapes also carry the order of the seed's length, and `jbig2`'s each segment's referred-to
count and the order of its data's length: an arithmetic decoder's contexts are its data, and the
proof in ADR 1571 found seeds of one coding structure apart on how much of it there is. A seed
whose segments or markers do not parse is a shape of its own, so the decoders' refusals keep a
seed.
"""

import os
import sys

from seed_shape import Smallest, bucket

# T.88 section 7.3's segment types that open with a region segment information field of 17
# bytes, and how many flag bytes follow it; and the types whose flags open the data directly.
REGION_FLAGS = {4: 2, 6: 2, 7: 2, 20: 1, 22: 1, 23: 1, 36: 1, 38: 1, 39: 1, 40: 1, 42: 1, 43: 1}
LEADING_FLAGS = {0: 2, 16: 1, 53: 1}
PAGE_INFORMATION = 48


def segments(data):
    """Each `(type, flags, referred-to count, order of data length)` of the segments `data`
    states, and `("unparsed",)` where a header does not parse or a length runs past the end."""
    out, at = set(), 0
    while at + 6 <= len(data):
        number = int.from_bytes(data[at:at + 4], "big")
        flags = data[at + 4]
        kind, wide_page = flags & 0x3F, flags & 0x40
        at += 5
        referred = data[at] >> 5
        if referred == 7:
            if at + 4 > len(data):
                out.add(("unparsed",))
                break
            referred = int.from_bytes(data[at:at + 4], "big") & 0x1FFFFFFF
            at += 4 + (referred + 8) // 8
        else:
            at += 1
        at += referred * (1 if number <= 256 else 2 if number <= 65536 else 4)
        at += 4 if wide_page else 1
        if at + 4 > len(data):
            out.add(("unparsed",))
            break
        length = int.from_bytes(data[at:at + 4], "big")
        at += 4
        body = data[at:at + length] if length != 0xFFFFFFFF else data[at:]
        if kind in REGION_FLAGS:
            chosen = body[17:17 + REGION_FLAGS[kind]]
        elif kind in LEADING_FLAGS:
            chosen = body[:LEADING_FLAGS[kind]]
        elif kind == PAGE_INFORMATION:
            chosen = body[16:17] + bytes([body[17] & 0x80]) if len(body) >= 19 else b""
        else:
            chosen = b""
        out.add((kind, chosen, referred, bucket(min(length, len(data) - at))))
        if length == 0xFFFFFFFF:
            break
        if at + length > len(data):
            out.add(("unparsed",))
            break
        at += length
    return frozenset(out)


def jbig2_shape(seed):
    """`jbig2.rs` splits its input into the globals and the page's segments; their shapes."""
    split = min(int.from_bytes(seed[:2], "big"), len(seed) - 2)
    globals_, page = seed[2:2 + split], seed[2 + split:]
    return (bool(globals_), segments(globals_), segments(page), bucket(len(seed)))


JP2_SIGNATURE = b"\x00\x00\x00\x0cjP  \r\n\x87\n"
# Marker segments of a main header that the shape names by presence (ISO/IEC 15444-1 Table A.2).
MARKERS = {0x53: "COC", 0x5D: "QCC", 0x5E: "RGN", 0x5F: "POC", 0x60: "PPM", 0x55: "TLM",
           0x57: "PLM", 0x63: "CRG", 0x64: "COM"}


def boxes(data, depth=0):
    """Each `(type, contents)` of the boxes `data` states, superboxes' members included."""
    out, at = [], 0
    while at + 8 <= len(data) and depth < 4:
        length = int.from_bytes(data[at:at + 4], "big")
        kind = data[at + 4:at + 8]
        start = at + 8
        if length == 1 and at + 16 <= len(data):
            length = int.from_bytes(data[at + 8:at + 16], "big")
            start = at + 16
        elif length == 0:
            length = len(data) - at
        if length < start - at:
            out.append((b"bad", b""))
            break
        contents = data[start:at + length]
        out.append((kind, contents))
        if kind in (b"jp2h", b"res ", b"uinf"):
            out.extend(boxes(contents, depth + 1))
        at += length
    return out


def main_header(codestream):
    """The main header's shape, from `SOC` to the first `SOT`."""
    if codestream[:2] != b"\xff\x4f":
        return ("no SOC",)
    at, siz, cod, qcd, present = 2, None, None, None, set()
    while at + 4 <= len(codestream):
        if codestream[at] != 0xFF:
            return ("unparsed", siz, cod, qcd, frozenset(present))
        marker = codestream[at + 1]
        if marker == 0x90:
            break
        length = int.from_bytes(codestream[at + 2:at + 4], "big")
        segment = codestream[at + 4:at + 2 + length]
        if marker == 0x51 and len(segment) >= 36:
            numbers = [int.from_bytes(segment[2 + 4 * i:6 + 4 * i], "big") for i in range(8)]
            width, height, x0, y0, tile_width, tile_height, tile_x0, tile_y0 = numbers
            components = int.from_bytes(segment[34:36], "big")
            sampling = frozenset(
                tuple(segment[36 + 3 * i:39 + 3 * i]) for i in range(min(components, 64)))
            tiled = tile_width < width - tile_x0 or tile_height < height - tile_y0
            siz = (segment[:2], components, sampling, tiled, x0 > 0 or y0 > 0)
        elif marker == 0x52 and len(segment) >= 10:
            layers = bucket(int.from_bytes(segment[2:4], "big"))
            cod = (segment[0], segment[1], layers, *segment[4:10])
        elif marker == 0x5C and segment:
            qcd = segment[0] & 0x1F
        elif marker in MARKERS:
            present.add(MARKERS[marker])
        at += 2 + length
    return (siz, cod, qcd, frozenset(present))


def jpx_shape(seed):
    """`jpx.rs` reads a request byte and then the file; the file's shape."""
    file = seed[1:]
    if file.startswith(JP2_SIGNATURE):
        found = boxes(file)
        kinds = frozenset(kind for kind, _ in found)
        header = next((contents[8:14] for kind, contents in found if kind == b"ihdr"), None)
        colour = frozenset(contents[:1] + (contents[3:7] if contents[:1] == b"\x01" else b"")
                           for kind, contents in found if kind == b"colr")
        codestream = next((contents for kind, contents in found if kind == b"jp2c"), b"")
        return ("jp2", kinds, header, colour, main_header(codestream), bucket(len(seed)))
    return ("codestream", main_header(file), bucket(len(seed)))


SHAPES = {"jbig2": jbig2_shape, "jpx": jpx_shape}


def main(argv):
    if len(argv) != 4 or argv[1] not in SHAPES:
        sys.exit(__doc__)
    target, every, directory = argv[1:]
    chosen = Smallest()
    for name in sorted(os.listdir(every)):
        with open(os.path.join(every, name), "rb") as handle:
            seed = handle.read()
        if seed:
            chosen.offer(SHAPES[target](seed), seed)
    written = chosen.write(directory)
    print(f"seed_codecs.py {target}: {chosen.summary()}, {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
