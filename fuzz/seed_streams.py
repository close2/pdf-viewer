#!/usr/bin/env python3
"""Seed the fuzz targets whose input is one object out of a document, from the documents.

    find -L doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \\
      | python3 fuzz/seed_streams.py <target> <dir> -

`<target>` is one of the six below, and each takes from a document exactly the bytes that target
reads — never a whole file, because each of these targets wraps its input in a document of its own
or reads a stream's decoded bytes, and a whole file mutated there spends every execution on bytes
the target never looks at:

    xmp            a metadata stream's decoded packet (§14.3.2, `/Type /Metadata`)
    sfnt           an embedded TrueType program (§9.9, a stream stating `/Length1` and no `/Length2`)
    cmap           an embedded CMap file (§9.7.5.3, `/Type /CMap`)
    ccitt          a `CCITTFaxDecode` stream's coded bytes behind the six head bytes `ccitt.rs`
                   reads its Table 11 parameters from, taken from the stream's own `/DecodeParms`
    crypt          the inside of a `/Filter /Standard` encryption dictionary (§7.6.4, Table 21),
                   which is what `crypt.rs` places between `<<` and `>>`
    variable_text  a field's `/DA` and `/V` (§12.7.4.3), laid out as the two halves
                   `variable_text.rs` splits its input into
    jpeg_bands     a `DCTDecode` stream's own bytes (§7.4.8) behind the band-height byte
                   `jpeg_bands.rs` reads first, one frame per shape (below)

The `-` reads the list of documents from standard input, one per NUL, for the reason
`seed_x509.py` gives: one run counts everything, where `xargs` would print a summary per batch.

**A reader of raw bytes, and a lower bound because of it.** An object inside an object stream
(§7.5.7) is not seen, and a stream whose `/Length` is indirect is read to its `endstream`. What the
script finds is printed, so a population that came out small says so rather than being assumed.
Only unfiltered and `FlateDecode` streams are decoded; a seed whose target reads decoded bytes and
whose filter is anything else is skipped. Seeds are named by SHA-256, so a re-run adds only what
is new, and nothing past `MAX_SEED` is written: `page.rs`'s ceiling, for the merge cost
`doc/verify.md` records of that target.

**`jpeg_bands` takes one frame per shape, not every frame.** The documents hold some two hundred
thousand `DCTDecode` streams, and written whole they were 2.3 GB through a corpus link into the main
checkout, most of them photographs from the same few cameras. What decides which branch of a band
plan a frame reaches is its *shape* — the start-of-frame marker (baseline, extended, progressive),
the sample precision, the component count and each component's sampling factors, the restart
interval `DRI` states, and whether the height is left to a `DNL` marker — never what the picture
shows, so `frame_shape` reads those out of the marker segments and the smallest frame of each shape
is the one written: fewest bytes per execution, and the same plan. A stream whose markers do not
parse is a shape of its own, so the decoder's refusals keep a seed. ADR 1559.
"""

import hashlib
import mmap
import os
import re
import sys
import zlib

MAX_SEED = 256 * 1024
MAX_DICTIONARY = 8 * 1024
TARGETS = ("xmp", "sfnt", "cmap", "ccitt", "crypt", "variable_text", "jpeg_bands")
# A `DCTDecode` frame past this is skipped for `jpeg_bands`: every execution decodes the frame
# three times, and a frame of a few hundred lines is cut into bands as surely as a large one.
MAX_JPEG = 64 * 1024
# A document past this is not read for `jpeg_bands`. The 93% of the documents under it hold 44 of
# the population's 126 GB, and a frame shape is a property of a producer's encoder rather than of
# a document's length, so what the cap costs is a shape only a long document states. ADR 1559 says
# how many shapes the population under the cap holds; a shape the cap loses is a sentence there.
MAX_JPEG_DOCUMENT = 4 * 1024 * 1024

OBJECT = re.compile(rb"\d+\s+\d+\s+obj\b")
STREAM = re.compile(rb"stream\r?\n")
DIRECT_LENGTH = re.compile(rb"/Length\s+(\d+)(?!\s+\d+\s+R)")


def filters(dictionary):
    """The filter names a stream dictionary states, in order."""
    one = re.search(rb"/Filter\s*/(\w+)", dictionary)
    if one:
        return [one.group(1)]
    many = re.search(rb"/Filter\s*\[([^\]]*)\]", dictionary)
    return re.findall(rb"/(\w+)", many.group(1)) if many else []


def streams(data):
    """Each (dictionary, raw body) of a stream stated outside an object stream."""
    for found in OBJECT.finditer(data):
        yield from streams_from(data, found.end())


def dct_streams(data):
    """`streams`, for `jpeg_bands`: each stream whose dictionary names `DCT`, found from the name.

    Searching for the name and stepping back to its object is C's work over the bytes, where
    `streams` runs a regular expression over every object of the document; the dictionaries
    found are the ones `streams` would yield that mention `DCT`, which is all `seeds` keeps."""
    at = data.find(b"/DCT")
    seen = set()
    while at >= 0:
        opening = data.rfind(b"obj", max(0, at - MAX_DICTIONARY), at)
        if opening >= 0 and opening not in seen:
            seen.add(opening)
            for dictionary, body in streams_from(data, opening + 3):
                yield dictionary, body
        at = data.find(b"/DCT", at + 4)


def streams_from(data, start):
    """The one stream whose dictionary opens at `start`, if one does; `streams`'s body."""
    opening = STREAM.search(data, start, start + MAX_DICTIONARY)
    if not opening:
        return
    dictionary = data[start:opening.start()]
    if b"endobj" in dictionary or not dictionary.lstrip().startswith(b"<<"):
        return
    body_start = opening.end()
    length = DIRECT_LENGTH.search(dictionary)
    if length and body_start + int(length.group(1)) <= len(data):
        body = data[body_start:body_start + int(length.group(1))]
    else:
        end = data.find(b"endstream", body_start)
        if end < 0:
            return
        body = data[body_start:end].rstrip(b"\r\n")
    yield dictionary, body


def decoded(dictionary, body):
    """The stream's decoded bytes, where no filter or only `FlateDecode` is stated."""
    named = filters(dictionary)
    if not named:
        return body
    if named == [b"FlateDecode"]:
        for window in (zlib.MAX_WBITS, -zlib.MAX_WBITS):
            try:
                return zlib.decompressobj(window).decompress(body, MAX_SEED + 1)
            except zlib.error:
                continue
    return None


def integer(dictionary, key, default):
    found = re.search(rb"/" + key + rb"\s+(-?\d+)", dictionary)
    return int(found.group(1)) if found else default


def boolean(dictionary, key, default):
    found = re.search(rb"/" + key + rb"\s+(true|false)", dictionary)
    return (found.group(1) == b"true") if found else default


def ccitt_head(dictionary):
    """`ccitt.rs`'s six head bytes, from `/DecodeParms` (Table 11) and the image's `/Height`."""
    parameters = re.search(rb"/DecodeParms\s*\[?\s*<<(.*?)>>", dictionary, re.S)
    parameters = parameters.group(1) if parameters else b""
    k = integer(parameters, b"K", 0)
    columns = integer(parameters, b"Columns", 1728)
    rows = integer(parameters, b"Rows", integer(dictionary, b"Height", 0))
    if not 0 < columns <= 0x0FFF:
        return None
    coding = 0 if k < 0 else (1 if k == 0 else 2)
    flags = (
        (1 if boolean(parameters, b"EndOfLine", False) else 0)
        | (2 if boolean(parameters, b"EncodedByteAlign", False) else 0)
        | (4 if boolean(parameters, b"EndOfBlock", True) else 0)
    )
    damaged = integer(parameters, b"DamagedRowsBeforeError", 0)
    return bytes([coding, columns >> 8, columns & 0xFF, min(max(rows, 0), 255), flags,
                  min(max(damaged, 0), 255)])


def skip_string(data, at):
    """The index after the literal string (§7.3.4.2) that opens at `at`."""
    depth, i = 0, at
    while i < len(data):
        c = data[i]
        if c == 0x5C:
            i += 2
            continue
        if c == 0x28:
            depth += 1
        elif c == 0x29:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return len(data)


def dictionary_inside(data, opening):
    """The bytes between the `<<` at `opening` and its matching `>>`, strings stepped over."""
    depth, i = 0, opening
    while i < len(data) - 1:
        if data[i] == 0x28:
            i = skip_string(data, i)
            continue
        pair = data[i:i + 2]
        if pair == b"<<":
            depth += 1
            i += 2
            continue
        if pair == b">>":
            depth -= 1
            i += 2
            if depth == 0:
                return data[opening + 2:i - 2]
            continue
        if data[i] == 0x3C:
            end = data.find(b">", i)
            i = end + 1 if end > 0 else len(data)
            continue
        i += 1
    return None


def standard_security_dictionaries(data):
    for found in re.finditer(rb"/Filter\s*/Standard\b", data):
        opening = data.rfind(b"<<", max(0, found.start() - MAX_DICTIONARY), found.start())
        while opening >= 0:
            inside = dictionary_inside(data, opening)
            if inside is not None and found.start() < opening + 2 + len(inside):
                yield inside
                break
            opening = data.rfind(b"<<", max(0, found.start() - MAX_DICTIONARY), opening)


ESCAPES = {ord("n"): b"\n", ord("r"): b"\r", ord("t"): b"\t", ord("b"): b"\b", ord("f"): b"\f"}


def literal(data, at):
    """The bytes of the literal string opening at `at`, §7.3.4.2's escapes applied."""
    end = skip_string(data, at)
    raw, out, i = data[at + 1:end - 1], bytearray(), 0
    while i < len(raw):
        c = raw[i]
        if c == 0x5C and i + 1 < len(raw):
            n = raw[i + 1]
            octal = re.match(rb"[0-7]{1,3}", raw[i + 1:i + 4])
            if octal:
                out.append(int(octal.group(0), 8) & 0xFF)
                i += 1 + len(octal.group(0))
                continue
            out += ESCAPES.get(n, bytes([n]))
            i += 2
            continue
        out.append(c)
        i += 1
    return bytes(out)


def string_after(data, key):
    """The first literal or hexadecimal string after `key` in `data`."""
    found = re.search(key + rb"\s*([(<])", data)
    if not found:
        return None
    at = found.start(1)
    if data[at] == 0x28:
        return literal(data, at)
    end = data.find(b">", at)
    digits = re.sub(rb"[^0-9A-Fa-f]", b"", data[at + 1:end])
    if len(digits) % 2:
        digits += b"0"
    return bytes.fromhex(digits.decode())


def field_halves(data):
    """Each field's `/DA` and `/V`, as the two equal halves `variable_text.rs` splits."""
    for found in re.finditer(rb"/DA\s*\(", data):
        low = data.rfind(b"obj", 0, found.start())
        high = data.find(b"endobj", found.start())
        if low < 0 or high < 0 or high - low > MAX_DICTIONARY:
            continue
        body = data[low:high]
        appearance = string_after(body, rb"/DA")
        value = string_after(body, rb"/V")
        if appearance is None or value is None:
            continue
        width = max(len(appearance), len(value))
        yield appearance.ljust(width, b" ") + value.ljust(width, b" ")


def states_dct(path):
    """Whether a document of at most `MAX_JPEG_DOCUMENT` bytes names the `DCTDecode` filter at all.

    `jpeg_bands` reads only those, and the documents are 126 GB: a search through a memory map
    is the operating system's and C's, where the regular expressions `streams` runs are Python's,
    and on a document that names no `DCT` the regular expressions find nothing to keep. Past
    `MAX_JPEG_DOCUMENT` a document is not read for this target at all."""
    if os.path.getsize(path) > MAX_JPEG_DOCUMENT:
        return False
    with open(path, "rb") as handle:
        try:
            with mmap.mmap(handle.fileno(), 0, access=mmap.ACCESS_READ) as mapped:
                return mapped.find(b"/DCT") >= 0
        except ValueError:
            return False


def frame_shape(frame):
    """What decides a band plan's branch for one JPEG frame (`jpeg_bands`), or `None` where the
    marker segments do not parse.

    The start-of-frame marker, precision, component count and each component's `(H, V)` sampling
    factors, the first `DRI`'s restart interval (0 where none is stated) and whether a `DNL`
    marker follows the first scan, which is how a frame of height 0 states its height (ISO/IEC
    10918-1 section B.2.5)."""
    if not frame.startswith(b"\xff\xd8"):
        return None
    at, sof, interval = 2, None, 0
    while at + 4 <= len(frame):
        if frame[at] != 0xFF:
            return None
        marker = frame[at + 1]
        if marker == 0xFF:
            at += 1
            continue
        length = int.from_bytes(frame[at + 2:at + 4], "big")
        segment = frame[at + 4:at + 2 + length]
        if 0xC0 <= marker <= 0xCF and marker not in (0xC4, 0xC8, 0xCC) and len(segment) >= 6:
            components = segment[5]
            factors = tuple(
                (segment[7 + 3 * i] >> 4, segment[7 + 3 * i] & 15)
                for i in range(components)
                if 8 + 3 * i <= len(segment)
            )
            height = int.from_bytes(segment[1:3], "big")
            sof = (marker, segment[0], components, factors, height == 0)
        elif marker == 0xDD and len(segment) >= 2 and not interval:
            interval = int.from_bytes(segment[0:2], "big")
        elif marker == 0xDA:
            if sof is None:
                return None
            return sof + (interval, frame.find(b"\xff\xdc", at + 2 + length) >= 0)
        at += 2 + length
    return None


def seeds(target, data):
    if target == "crypt":
        yield from standard_security_dictionaries(data)
        return
    if target == "variable_text":
        yield from field_halves(data)
        return
    for dictionary, body in dct_streams(data) if target == "jpeg_bands" else streams(data):
        if target == "jpeg_bands":
            if filters(dictionary) in ([b"DCTDecode"], [b"DCT"]) and len(body) <= MAX_JPEG:
                yield bytes([1]) + body
            continue
        if target == "ccitt":
            if filters(dictionary) == [b"CCITTFaxDecode"]:
                head = ccitt_head(dictionary)
                if head is not None:
                    yield head + body
            continue
        wanted = {
            "xmp": re.search(rb"/Type\s*/Metadata", dictionary),
            "sfnt": b"/Length1" in dictionary and b"/Length2" not in dictionary,
            "cmap": re.search(rb"/Type\s*/CMap", dictionary),
        }[target]
        if wanted:
            out = decoded(dictionary, body)
            if out:
                yield out


def main(argv):
    if len(argv) != 4 or argv[1] not in TARGETS:
        sys.exit(__doc__)
    target, directory = argv[1], argv[2]
    if argv[3] == "-":
        paths = [p for p in sys.stdin.buffer.read().split(b"\0") if p]
    else:
        sys.exit(__doc__)
    os.makedirs(directory, exist_ok=True)
    documents = with_seed = written = 0
    # `jpeg_bands`: the smallest frame of each shape, chosen over every document before any is
    # written, ties broken by name so that a re-run chooses the same seed.
    shapes = {}
    for path in paths:
        try:
            if target == "jpeg_bands" and not states_dct(path):
                documents += 1
                continue
            with open(path, "rb") as handle:
                data = handle.read()
        except OSError:
            continue
        documents += 1
        found = False
        for seed in seeds(target, data):
            if not seed or len(seed) > MAX_SEED:
                continue
            found = True
            digest = hashlib.sha256(seed).hexdigest()
            if target == "jpeg_bands":
                shape = frame_shape(seed[1:])
                if shape not in shapes or (len(seed), digest) < shapes[shape][:2]:
                    shapes[shape] = (len(seed), digest, seed)
                continue
            name = os.path.join(directory, digest)
            if not os.path.exists(name):
                with open(name, "wb") as handle:
                    handle.write(seed)
                written += 1
        with_seed += found
    for _, digest, seed in shapes.values():
        name = os.path.join(directory, digest)
        if not os.path.exists(name):
            with open(name, "wb") as handle:
                handle.write(seed)
            written += 1
    shaped = f", {len(shapes)} frame shapes" if target == "jpeg_bands" else ""
    print(f"seed_streams.py {target}: {documents} documents read, {with_seed} held one{shaped}, "
          f"{written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
