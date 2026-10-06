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

**Each target keeps one seed per shape, not every seed** (ADRs 1559, 1571). The documents hold
some two hundred thousand `DCTDecode` streams and tens of thousands of each other kind, most of them
from the same few producers, and written whole they were gigabytes through a corpus link into the
main checkout. What decides which branch a target's code takes is the seed's *shape* — for
`jpeg_bands` the start-of-frame marker, the sample precision, the component count and each
component's sampling factors, the restart interval `DRI` states, and whether the height is left to a
`DNL` marker — never what the picture, the text or the glyphs say, so each target's shape function
below reads those out of the seed and the smallest seed of each shape is the one written: fewest
bytes per execution, and the same branches. A seed whose structure does not parse is a shape of its
own, so the readers' refusals keep a seed. `--every` writes each seed found instead, which is the
population the shapes are proved against.

**A document is read only if its memory map holds the name its target needs** (`NEEDLES`), and
then from the object that names it; over the 400 first pdf.js documents that finds exactly the
seeds the regular expression over every object found (ADR 1571).
"""

import mmap
import os
import re
import sys
import zlib

from seed_shape import Smallest, bucket, mapped, names_any, text_forms

MAX_SEED = 256 * 1024
MAX_DICTIONARY = 8 * 1024
TARGETS = ("xmp", "sfnt", "cmap", "ccitt", "crypt", "variable_text", "jpeg_bands")
# What a document must state in the clear to hold a seed for each target: the name its stream
# dictionary, its security dictionary or its field carries. A document naming none is passed over
# by a search through its map, which is how the 126 GB is read in minutes; one naming it is read
# from the object that names it (`named_streams`) rather than by a regular expression over every
# object. A name written with a `#` escape (§7.3.5) is not found, and neither was it before: the
# regular expressions below match it literally.
NEEDLES = {
    "xmp": b"/Metadata",
    "sfnt": b"/Length1",
    "cmap": b"/CMap",
    "ccitt": b"CCITTFax",
    "crypt": b"/Standard",
    "variable_text": b"/DA",
    "jpeg_bands": b"/DCT",
}
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


def named_streams(data, needle):
    """Each (dictionary, raw body) of a stream stated outside an object stream whose dictionary
    holds `needle`, found from the name.

    Searching for the name and stepping back to its object is C's work over the bytes, where a
    regular expression over every object of the document is Python's; the dictionaries found are
    the ones such a scan yields that hold the name, which is all `seeds` keeps. Over the 974 pdf.js
    documents the two find the same `DCTDecode` streams (ADR 1559)."""
    at = data.find(needle)
    seen = set()
    while at >= 0:
        opening = data.rfind(b"obj", max(0, at - MAX_DICTIONARY), at)
        if opening >= 0 and opening not in seen:
            seen.add(opening)
            for dictionary, body in streams_from(data, opening + 3):
                yield dictionary, body
        at = data.find(needle, at + len(needle))


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


# **Each target's shape**, computed on the seed exactly as the target reads it, and the reason it
# is the shape: what in the target's code branches on it. ADR 1571 has what each was measured to
# keep against the population written whole.

NAME = rb"/[^\s/\[\]<>(){}%]*"
NUMBER = rb"[+-]?(?:\d+\.?\d*|\.\d+)"


# The properties `pdf_model::xmp`'s accessors look for, each a branch when it is present.
XMP_ACCESSED = (
    rb"<(dc:title|dc:creator|dc:description|dc:subject|pdf:Producer|pdf:Keywords|xmp:CreatorTool|"
    rb"xmp:CreateDate|xmp:ModifyDate)\b")


def xmp_shape(packet):
    """An XMP packet's schemas — the namespaces it binds — and the RDF forms it uses.

    `pdf_model::xmp` resolves every property against the `xmlns` bindings in scope and reads each
    value as text, `Alt`, `Seq`, `Bag` or a structure; what decides its branches is which forms
    and which encodings a packet states, and the namespaces are the schemas the accessors look
    for. The values themselves are text it copies; which of the accessors' properties a packet
    states and whether each is empty, the form of a date (§7.9.4's reading of XMP's dates), an
    empty value, a packet cut short, the orders of its element and attribute counts and its
    lexical forms are branches too, and the proof found seeds of one schema set apart on each. A
    packet under 64 bytes is a shape of its own: it is the parser's refusals, a byte at a time."""
    namespaces = frozenset(re.findall(rb"xmlns:[\w.-]+\s*=\s*[\"']([^\"']*)[\"']", packet))
    forms = frozenset(re.findall(rb"<rdf:(Alt|Seq|Bag|li|Description|value)\b", packet))
    return (
        namespaces,
        forms,
        packet.find(b"<?xpacket") >= 0,
        packet.find(b"parseType") >= 0,
        bool(re.search(rb"&(?:#|\w+;)", packet)),
        packet.find(b"<![CDATA[") >= 0,
        packet.find(b"<!--") >= 0,
        packet.find(b"<!DOCTYPE") >= 0,
        packet[:2] in (b"\xfe\xff", b"\xff\xfe") or packet.find(b"\x00") >= 0,
        bool(re.search(rb"<rdf:Description[^>]*\s(?!xmlns|rdf:about)[\w.-]+:[\w.-]+\s*=", packet)),
        frozenset(
            (name, not re.sub(rb"<[^>]*>|\s", b"", value), re.sub(rb"\d", b"0", value)
             if name.endswith(b"Date") else None)
            for name, value in re.findall(XMP_ACCESSED + rb"[^>]*>(.{0,512}?)</\1", packet, re.S)
        ),
        bool(re.search(rb">\s*</", packet)),
        packet.find(b"<x:xmpmeta") >= 0 and packet.find(b"</x:xmpmeta>") < 0,
        text_forms(packet),
        bucket(len(re.findall(rb"<[\w.-]+:[\w.-]+[\s>/]", packet))),
        bucket(len(re.findall(rb"\s[\w.-]+:[\w.-]+\s*=", packet))),
        packet if len(packet) < 64 else None,
    )


def sfnt_shape(program):
    """An sfnt's version tag, its table set and `head`'s `indexToLocFormat`.

    `pdf_font::repaired_font_program` and `composite_cycle` find tables by their tags and read
    `loca` in the format `head` states; which tables are present, and which offsets `loca` holds,
    are what their branches turn on, with the glyph count's order and whether any glyph is a
    composite, where the glyphs' outlines are data they step over."""
    if len(program) < 12:
        return ("short",)
    count = min(int.from_bytes(program[4:6], "big"), 64)
    tables = {}
    for index in range(count):
        entry = program[12 + 16 * index:28 + 16 * index]
        if len(entry) < 16:
            break
        tables[entry[:4]] = (int.from_bytes(entry[8:12], "big"), int.from_bytes(entry[12:16], "big"))
    head = tables.get(b"head", (0, 0))[0]
    loca = program[head + 50:head + 52] if b"head" in tables else None
    maxp = tables.get(b"maxp", (0, 0))[0]
    glyphs = int.from_bytes(program[maxp + 4:maxp + 6], "big") if b"maxp" in tables else 0
    return (program[:4], frozenset(tables), loca, bucket(glyphs),
            composite(program, tables, loca, glyphs))


def composite(program, tables, loca, glyphs):
    """Whether any glyph of `glyf` is a composite (a negative contour count), which is what sends
    `composite_cycle` past its first glyph; `None` where `loca` does not read."""
    if b"glyf" not in tables or b"loca" not in tables or loca not in (b"\x00\x00", b"\x00\x01"):
        return None
    start, _ = tables[b"glyf"]
    offsets, _ = tables[b"loca"]
    short = loca == b"\x00\x00"
    width = 2 if short else 4
    for index in range(min(glyphs, 4096)):
        here = program[offsets + width * index:offsets + width * (index + 1)]
        following = program[offsets + width * (index + 1):offsets + width * (index + 2)]
        if len(following) < width:
            return None
        first = int.from_bytes(here, "big") * (2 if short else 1)
        last = int.from_bytes(following, "big") * (2 if short else 1)
        if last > first and program[start + first:start + first + 2] >= b"\x80\x00":
            return True
    return False


def cmap_shape(cmap):
    """A CMap's type and writing mode, the byte lengths of its codespace ranges, and which of
    §9.7.5's operators it uses.

    `CMap::next_code` takes as many bytes as the codespace range that matches says, so the ranges'
    byte lengths are the decoder's branches; the mapping operators — CID or Unicode, range or
    single, `notdef`, `usecmap` — are the parser's, and a `bfrange` whose destination is an array
    is a branch of its own. The codes and CIDs mapped are numbers it stores; how many there are, to
    an order, and the file's lexical forms are the tokeniser's."""
    lengths = set()
    for block in re.findall(rb"begincodespacerange(.*?)endcodespacerange", cmap, re.S):
        for lo, hi in re.findall(rb"<([0-9A-Fa-f\s]*)>\s*<([0-9A-Fa-f\s]*)>", block):
            lengths.add((len(re.sub(rb"\s", b"", lo)) // 2, len(re.sub(rb"\s", b"", hi)) // 2))
    operators = frozenset(re.findall(
        rb"\b(begincidrange|begincidchar|beginbfrange|beginbfchar|beginnotdefrange|"
        rb"beginnotdefchar|usecmap|usefont|beginusematrix|beginrearrangedfont)\b", cmap))
    return (
        integer(cmap, b"CMapType", -1),
        integer(cmap, b"WMode", -1),
        frozenset(lengths),
        operators,
        bool(re.search(rb"<[0-9A-Fa-f]+>\s*<[0-9A-Fa-f]+>\s*\[", cmap)),
        bucket(len(re.findall(rb"<[0-9A-Fa-f]+>", cmap))),
        text_forms(cmap),
    )


def ccitt_shape(seed):
    """Table 11's coding (`/K`'s sign), `/Columns` and the three flags, from the head bytes.

    `pdf_ccitt::decode` branches on the coding and on the flags, and its changing-element
    arithmetic on the row width; `/Rows` decides whether the rows or the end of the data stop it,
    so its order is kept, and `/DamagedRowsBeforeError` whether a damaged row is concealed at all.
    The coded rows are what every image of one producer shares."""
    return (seed[0], seed[1:3], seed[4], bucket(seed[3]), seed[5] > 0)


def crypt_shape(inside):
    """A security handler's `/V`, `/R` and `/Length`, the crypt filters' `/CFM`s and which of them
    `/StmF`, `/StrF` and `/EFF` name, `/EncryptMetadata`, and which of the revision 6 entries are
    stated (§7.6.4, Tables 20 to 21 and 27).

    These choose the algorithm `pdf_syntax` runs — RC4 or AES, which key derivation, whether
    metadata is excepted — and the strings beside them are the keys and hashes it computes on,
    read through the escapes the lexical forms name."""
    def named(key):
        found = re.search(rb"/" + key + rb"\s*/(\w+)", inside)
        return found.group(1) if found else None

    return (
        integer(inside, b"V", -1),
        integer(inside, b"R", -1),
        integer(inside, b"Length", -1),
        frozenset(re.findall(rb"/CFM\s*/(\w+)", inside)),
        named(b"StmF"),
        named(b"StrF"),
        named(b"EFF"),
        boolean(inside, b"EncryptMetadata", True),
        frozenset(key for key in (b"OE", b"UE", b"Perms") if re.search(rb"/" + key + rb"\b", inside)),
        bool(re.search(rb"/O\s*<", inside)),
        text_forms(inside),
    )


# The classes of character a field value's layout treats apart: a space is where a line may
# break, a digit and a letter have different widths in the target's Helvetica, and a tab and the
# rest are each looked up in its encoding.
VALUE_CLASSES = {
    "space": rb" ",
    "digit": rb"[0-9]",
    "letter": rb"[A-Za-z]",
    "tab": rb"\t",
    "punctuation": rb"[!-/:-@\[-`{-~]",
}
DA_TOKEN = re.compile(NAME + rb"|" + NUMBER + rb"|[A-Za-z'\"*]+|\S")


def variable_text_shape(seed):
    """A `/DA`'s operators with each operand reduced to its kind, and what the `/V` holds.

    §12.7.4.3's layout branches on which operators the `/DA` sets, on a font size of zero, which
    is auto-sizing, and on whether the font is `/Helv`, the one the target's `/DR` holds; the
    value's branches are its length's order, its line breaks, a byte outside ASCII, a byte-order
    mark and the classes of character it holds. The two halves are split the way
    `variable_text.rs` splits them."""
    appearance, value = seed[:len(seed) // 2], seed[len(seed) // 2:]
    tokens = []
    for token in DA_TOKEN.findall(appearance):
        if token.startswith(b"/"):
            tokens.append(token if token == b"/Helv" else b"/")
        elif re.fullmatch(NUMBER, token):
            tokens.append(b"0" if float(token) == 0 else b"n")
        else:
            tokens.append(token)
    value = value.rstrip(b" ")
    return (
        tuple(tokens),
        bucket(len(value)),
        bool(re.search(rb"[\r\n]", value)),
        bool(re.search(rb"[\x80-\xff]", value)),
        value[:2] in (b"\xfe\xff", b"\xef\xbb"),
        frozenset(kind for kind, pattern in VALUE_CLASSES.items() if re.search(pattern, value)),
        text_forms(appearance),
    )


SHAPES = {
    "xmp": xmp_shape,
    "sfnt": sfnt_shape,
    "cmap": cmap_shape,
    "ccitt": ccitt_shape,
    "crypt": crypt_shape,
    "variable_text": variable_text_shape,
    "jpeg_bands": lambda seed: frame_shape(seed[1:]),
}


def seeds(target, data):
    if target == "crypt":
        yield from standard_security_dictionaries(data)
        return
    if target == "variable_text":
        yield from field_halves(data)
        return
    for dictionary, body in named_streams(data, NEEDLES[target]):
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
    every = "--every" in argv
    argv = [argument for argument in argv if argument != "--every"]
    if len(argv) != 4 or argv[1] not in TARGETS or argv[3] != "-":
        sys.exit(__doc__)
    target, directory = argv[1], argv[2]
    paths = [p for p in sys.stdin.buffer.read().split(b"\0") if p]
    os.makedirs(directory, exist_ok=True)
    documents = with_seed = 0
    # Every seed under its own name with `--every`, which is the population written whole and
    # what ADR 1571's proof compares the selection against; otherwise the smallest of each shape,
    # chosen over every document before any is written.
    shape = (lambda seed: seed) if every else SHAPES[target]
    chosen = Smallest()
    for path in paths:
        documents += 1
        try:
            if target == "jpeg_bands" and not states_dct(path):
                continue
            with mapped(path) as data:
                if not names_any(data, (NEEDLES[target],)):
                    continue
                found = False
                for seed in seeds(target, data):
                    if not seed or len(seed) > MAX_SEED:
                        continue
                    found = True
                    chosen.offer(shape(seed), seed)
                with_seed += found
        except (OSError, ValueError):
            continue
    written = chosen.write(directory)
    print(f"seed_streams.py {target}: {documents} documents read, {with_seed} held one, "
          f"{chosen.summary()}, {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
