#!/usr/bin/env python3
"""Builds the Latin-Greek-Cyrillic pack: Noto Serif and Noto Sans Mono as bare CFF programs.

`doc/lgc-pack.md` is the layout and the argument; ADR 2630 is the decision and ADR 2934 the
build. Run as

    python3 -I -B tools/lgc-pack.py --fetch <empty dir>
    python3 -I -B tools/lgc-pack.py --source <dir> --out <empty dir> [--zip <dir>]

The first downloads the inputs from the google/fonts commit `COMMIT` names and checks each by
digest; the second reads them from `<dir>`, checks them again, and writes one `.cff` per face
`FACES` names, the licence beside them, and `SHA256SUMS`;
`crates/pdf-transform/src/archive/fonts/lgc_pack.rs` carries the same digests and refuses a program
whose bytes are not the ones this script wrote. `--zip` adds `quorra-lgc-pack-<n>.zip`, the
directory `lgc-pack/` holding those files, which is what CI publishes beside the binaries.

**What each of ADR 2630's four requirements on a pack became here.**

1. google/fonts carries the family only as variable fonts, so each face is one static instance of
   them, made by fontTools' `instancer` at the weight the style names and the default width.
2. A `glyf` program is admitted under no key of a `Type1` font dictionary (ISO 32000-2 Table 124),
   so each face is written as a bare CFF program -- the form the shipped Foxit faces take, admitted
   under a `Type1` dictionary as `/FontFile3 /Type1C` -- with every quadratic outline restated as the
   cubic one that draws the same curve.
3. A simple font reaches a glyph of a bare CFF by its name in the charset (ISO 32000-2 section
   9.6.5), so every glyph is named by the Adobe Glyph List: one glyph under each name the list gives
   a character the source maps, and `uniXXXX` (`uXXXXX` above the BMP) for a character it names
   none for. A character the list names twice (`afii10017` and `Acyrillic`) is drawn by two copies.
   One departure, the owner's choice (A888, ADR 2952): a name version 1.x of the list mapped to two
   characters, whose 2.0 character the source does not map, is drawn by the glyph of its other 1.x
   character (`DOUBLE_MAPPED`).
4. Each charstring states its own width as its first operand and calls no subroutine, so the width
   the converter restates is the one a reader of the program finds (ADR 2642).

Noto Sans Mono is published in no italic, so its two oblique faces are this script's: the upright
instance's outlines slanted by 12 degrees, the `ItalicAngle` `data/standard-fonts/afm/Courier-Oblique.afm`
and `Courier-BoldOblique.afm` state for the standard fixed-pitch obliques, each point moved right by
its height times the angle's tangent and every advance kept (ADR 3048).

The programs are Modified Versions under the SIL Open Font License 1.1, which reserves no font name
for Noto (ADR 2630's licence reading), so each keeps the family's name and its copyright notice.
"""

import argparse
import hashlib
import math
import os
import sys
import urllib.request
import zipfile

from fontTools import agl
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

# The pack's number, in its zip's name: the next one whenever a byte of the output moves, which is
# whenever the digests `lgc_pack.rs` carries change.
PACK_VERSION = 3

# google/fonts at a commit that holds both families' variable fonts and their licence.
COMMIT = "097bc1b8c04c3224087c2ae95f7a923859b778cf"
RAW = f"https://raw.githubusercontent.com/google/fonts/{COMMIT}/ofl/"

# The inputs: name -> (where it is fetched from, its SHA-256).
SOURCES = {
    "NotoSerif[wdth,wght].ttf": (
        RAW + "notoserif/NotoSerif%5Bwdth,wght%5D.ttf",
        "4d8e6761424656867019081a1a01336f3cb086982682698714054fc33f782713"),
    "NotoSerif-Italic[wdth,wght].ttf": (
        RAW + "notoserif/NotoSerif-Italic%5Bwdth,wght%5D.ttf",
        "e87acbc6c0efd0d9a20d6a8cbbda2b266c14be3a3a6f5af8ec9d7b2460570ad1"),
    "NotoSansMono[wdth,wght].ttf": (
        RAW + "notosansmono/NotoSansMono%5Bwdth,wght%5D.ttf",
        "2cb2adb378a8f574213e23df697050b83c54c27df465a2015552740b2769a081"),
    "OFL.txt": (
        RAW + "notoserif/OFL.txt",
        "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a"),
}

# The twelve names version 1.2 of the Adobe Glyph List (22 Oct 1998) mapped to two characters, each
# to the one 2.0 does not keep: the lines its own header counts ("12 glyph names are mapped to two
# UVs each") and marks `Duplicate`. The 1.2 file is no longer published by Adobe; the copy read is
# the Wayback Machine's of http://partners.adobe.com/asn/developer/type/glyphlist.txt at
# 20010215130957, SHA-256 62ad74724df1b2ea160fe45afd414cc6382e21872235249224468360521b855c. A name
# takes its glyph from here only where the source maps no glyph for its 2.0 character and does map
# this one; where it maps neither, the face has no glyph of that name (ADR 2952).
DOUBLE_MAPPED = {
    "Delta": 0x0394,           # 2.0: U+2206 INCREMENT; 1.2 also GREEK CAPITAL LETTER DELTA
    "Omega": 0x03A9,           # 2.0: U+2126 OHM SIGN; 1.2 also GREEK CAPITAL LETTER OMEGA
    "Scedilla": 0xF6C1,        # 2.0: U+015E; 1.2 also the Corporate Use copy
    "Tcommaaccent": 0x021A,    # 2.0: U+0162; 1.2 also LATIN CAPITAL LETTER T WITH COMMA BELOW
    "fraction": 0x2215,        # 2.0: U+2044 FRACTION SLASH; 1.2 also DIVISION SLASH
    "hyphen": 0x00AD,          # 2.0: U+002D HYPHEN-MINUS; 1.2 also SOFT HYPHEN
    "macron": 0x02C9,          # 2.0: U+00AF MACRON; 1.2 also MODIFIER LETTER MACRON
    "mu": 0x03BC,              # 2.0: U+00B5 MICRO SIGN; 1.2 also GREEK SMALL LETTER MU
    "periodcentered": 0x2219,  # 2.0: U+00B7 MIDDLE DOT; 1.2 also BULLET OPERATOR
    "scedilla": 0xF6C2,        # 2.0: U+015F; 1.2 also the Corporate Use copy
    "space": 0x00A0,           # 2.0: U+0020 SPACE; 1.2 also NO-BREAK SPACE
    "tcommaaccent": 0x021B,    # 2.0: U+0163; 1.2 also LATIN SMALL LETTER T WITH COMMA BELOW
}

# Output file -> (source, weight, PostScript name, full name, slant in degrees). The four styles of
# the Foxit serif face and of its fixed face; Noto Sans Mono is published in no italic, so the fixed
# face's two italics are its upright instances slanted (ADR 3048).
FACES = {
    "NotoSerif-Regular.cff": ("NotoSerif[wdth,wght].ttf", 400, "NotoSerif-Regular",
                              "Noto Serif Regular", 0),
    "NotoSerif-Bold.cff": ("NotoSerif[wdth,wght].ttf", 700, "NotoSerif-Bold",
                           "Noto Serif Bold", 0),
    "NotoSerif-Italic.cff": ("NotoSerif-Italic[wdth,wght].ttf", 400, "NotoSerif-Italic",
                             "Noto Serif Italic", 0),
    "NotoSerif-BoldItalic.cff": ("NotoSerif-Italic[wdth,wght].ttf", 700, "NotoSerif-BoldItalic",
                                 "Noto Serif Bold Italic", 0),
    "NotoSansMono-Regular.cff": ("NotoSansMono[wdth,wght].ttf", 400, "NotoSansMono-Regular",
                                 "Noto Sans Mono Regular", 0),
    "NotoSansMono-Bold.cff": ("NotoSansMono[wdth,wght].ttf", 700, "NotoSansMono-Bold",
                              "Noto Sans Mono Bold", 0),
    "NotoSansMono-Oblique.cff": ("NotoSansMono[wdth,wght].ttf", 400, "NotoSansMono-Oblique",
                                 "Noto Sans Mono Oblique", 12),
    "NotoSansMono-BoldOblique.cff": ("NotoSansMono[wdth,wght].ttf", 700,
                                     "NotoSansMono-BoldOblique", "Noto Sans Mono Bold Oblique", 12),
}


def digest(path):
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def names_by_character():
    """Every Adobe Glyph List name of each character, in the list's own order."""
    names = {}
    for name, values in agl.LEGACY_AGL2UV.items():
        if len(values) == 1:
            names.setdefault(values[0], []).append(name)
    return names


def borrowed_names(cmap):
    """Each character whose glyph a `DOUBLE_MAPPED` name borrows in a face mapping `cmap`.

    A name borrows only where the list as it stands gives it a character the face lacks, so a face
    that maps that character keeps the list's reading."""
    borrowed = {}
    for name, character in DOUBLE_MAPPED.items():
        (listed,) = agl.LEGACY_AGL2UV[name]
        if listed not in cmap and character in cmap:
            borrowed.setdefault(character, []).append(name)
    return borrowed


def own_name(character):
    """The name the Adobe Glyph List specification builds for a character it lists no name for."""
    return f"uni{character:04X}" if character <= 0xFFFF else f"u{character:05X}"


def build(entry, source_dir):
    """One face's bare CFF program, as bytes."""
    source, weight, postscript, full, slant = entry
    # x' = x + y * tan(slant): the shear that leans a vertical stroke right by `slant` degrees.
    shear = math.tan(math.radians(slant))
    variable = TTFont(os.path.join(source_dir, source), recalcTimestamp=False)
    font = instancer.instantiateVariableFont(variable, {"wght": weight, "wdth": 100},
                                             updateFontNames=False)
    glyphs = font.getGlyphSet()
    advances = font["hmtx"].metrics
    listed = names_by_character()
    order = [".notdef"]
    drawn = {".notdef": ".notdef"}
    cmap = font.getBestCmap()
    borrowed = borrowed_names(cmap)
    for character, glyph in sorted(cmap.items()):
        for name in listed.get(character, [own_name(character)]) + borrowed.get(character, []):
            if name not in drawn:
                order.append(name)
                drawn[name] = glyph
    strings = {}
    for name in order:
        glyph = drawn[name]
        # The width as the first operand, every charstring self-contained (ADR 2642's item).
        pen = T2CharStringPen(advances[glyph][0], glyphs)
        glyphs[glyph].draw(TransformPen(pen, (1, 0, shear, 1, 0, 0)) if slant else pen)
        strings[name] = pen.getCharString()
    head = font["head"]
    builder = FontBuilder(head.unitsPerEm, isTTF=False)
    builder.updateHead(created=head.created, modified=head.modified)
    builder.setupGlyphOrder(order)
    copyright_notice = font["name"].getDebugName(0)
    # The box of the slanted outlines: a point at height y moves right by y * shear.
    box = [math.floor(head.xMin + min(head.yMin * shear, head.yMax * shear)), head.yMin,
           math.ceil(head.xMax + max(head.yMin * shear, head.yMax * shear)), head.yMax]
    builder.setupCFF(postscript, {
        "FullName": full,
        "FamilyName": "Noto Sans Mono" if "Mono" in postscript else "Noto Serif",
        "Weight": "Bold" if weight == 700 else "Regular",
        "Notice": copyright_notice,
        "ItalicAngle": -slant if slant else font["post"].italicAngle,
        "isFixedPitch": int(font["post"].isFixedPitch),
        "FontBBox": box,
    }, strings, {"defaultWidthX": 0, "nominalWidthX": 0})
    compiled = builder.font["CFF "].compile(builder.font)
    return compiled, len(order)


def fetch(directory):
    """Every input from where `SOURCES` names, refused unless its bytes are the digest's."""
    if os.listdir(directory):
        sys.exit(f"{directory}: not empty")
    for name, (url, expected) in SOURCES.items():
        with urllib.request.urlopen(url, timeout=300) as response:
            data = response.read()
        if hashlib.sha256(data).hexdigest() != expected:
            sys.exit(f"{url}: not the file this pack is built from")
        with open(os.path.join(directory, name), "wb") as handle:
            handle.write(data)
        print(f"{expected}  {name}")


def package(out_dir, zip_dir, names):
    """`quorra-lgc-pack-<n>.zip`: the directory `lgc-pack/` the converter looks for, holding `names`.

    Each entry carries the zip format's earliest time and one mode, in `SHA256SUMS`'s order, so the
    archive's bytes follow the files' and not the moment or the machine that packed them."""
    path = os.path.join(zip_dir, f"quorra-lgc-pack-{PACK_VERSION}.zip")
    with zipfile.ZipFile(path, "x") as archive:
        for name in names:
            entry = zipfile.ZipInfo(f"lgc-pack/{name}", date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            with open(os.path.join(out_dir, name), "rb") as handle:
                archive.writestr(entry, handle.read(), compresslevel=9)
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--fetch", metavar="DIR",
                        help="download the inputs into this empty directory, and build nothing")
    parser.add_argument("--source")
    parser.add_argument("--out")
    parser.add_argument("--zip", metavar="DIR",
                        help="also write quorra-lgc-pack-<n>.zip of the output into this directory")
    arguments = parser.parse_args()
    if arguments.fetch is not None:
        fetch(arguments.fetch)
        return
    if None in (arguments.source, arguments.out):
        parser.error("--source and --out name the build, or --fetch alone a download")
    for name, (_, expected) in SOURCES.items():
        path = os.path.join(arguments.source, name)
        if not os.path.exists(path):
            sys.exit(f"{path}: missing")
        if digest(path) != expected:
            sys.exit(f"{path}: not the file this pack is built from")
    if os.listdir(arguments.out):
        sys.exit(f"{arguments.out}: not empty")
    lines = []
    for file_name, entry in FACES.items():
        program, glyphs = build(entry, arguments.source)
        path = os.path.join(arguments.out, file_name)
        with open(path, "wb") as handle:
            handle.write(program)
        print(f"{file_name}: {glyphs} glyphs, {len(program)} bytes")
        lines.append(f"{digest(path)}  {file_name}")
    licence = os.path.join(arguments.out, "LICENSE.txt")
    with open(os.path.join(arguments.source, "OFL.txt"), "rb") as handle:
        text = handle.read()
    with open(licence, "wb") as handle:
        handle.write(text)
    lines.append(f"{digest(licence)}  LICENSE.txt")
    with open(os.path.join(arguments.out, "SHA256SUMS"), "w", encoding="ascii") as handle:
        handle.write("\n".join(lines) + "\n")
    print("\n".join(lines))
    if arguments.zip is not None:
        names = [line.split("  ", 1)[1] for line in lines] + ["SHA256SUMS"]
        print(package(arguments.out, arguments.zip, names))


if __name__ == "__main__":
    main()
