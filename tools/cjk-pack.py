#!/usr/bin/env python3
"""Builds the CJK pack: one CID-keyed program per Adobe character collection, ordered by its CIDs.

`doc/cjk-pack.md` is the layout and the argument; ADR 2302 is the decision. Run as

    python3 -I -B tools/cjk-pack.py --source <dir> --cmaps data/cmaps --out <empty dir>

where `<dir>` holds the four region-specific Source Han Sans Regular programs, the family's
`LICENSE.txt` and `SourceHanSans_JP_sequences.txt`, each named and checked by digest in
`SOURCES` below. The output is one `.otf` per collection, the licence beside them, and
`SHA256SUMS`; `crates/pdf-transform/src/archive/fonts/pack.rs` carries the same digests and refuses
a program whose bytes are not the ones this script wrote.

**Why each glyph sits where it does.** ISO 32000-2 section 9.7.3 makes a character collection an
ordered set of glyphs whose order determines each glyph's CID, so glyph index N of a pack program
is CID N of its collection. A CID is filled only through a mapping Adobe publishes for the
collection, never by a guess:

1. the collection's `Uni*-UTF32-H` CMap (Unicode -> CID): the source's `cmap` glyph for that value;
2. its `-V` CMap where it names a CID the `-H` one does not: that glyph through the source's `vert`;
3. for Adobe-Japan1, `UniJIS-UCS2-HW-H`/`-HW-V` likewise through `hwid` (and `vert` after it);
4. for Adobe-Japan1, the Ideographic Variation Database's Adobe-Japan1 sequences (the family's
   `SourceHanSans_JP_sequences.txt`) through the source's format 14 `cmap`.

Every other CID is a glyph with no outline, and the converter refuses a page showing one unless the
collection's own `Adobe-*-UCS2` table makes it a space (a mark lost is never written).

The programs are a Modified Version under the SIL Open Font License 1.1, so none keeps the Reserved
Font Name: each is renamed `QuorraCJK-<Ordering>` in its `name` table, its CFF Name INDEX and its
Font DICTs, and carries the licence's copyright notice unchanged (A550's precedent).
"""

import argparse
import hashlib
import os
import re
import sys

from fontTools.cffLib import FDSelect
from fontTools.misc.psCharStrings import T2CharString
from fontTools.ttLib import TTFont

# The inputs, by name and SHA-256: Source Han Sans 2.005R's region-specific subset programs.
SOURCES = {
    "SourceHanSansJP-Regular.otf": "40d1b760d1135539f6b6e0ee2b9f415de6d97576f7676840b06306c7c190c074",
    "SourceHanSansCN-Regular.otf": "e2bc8a2e7f37474b774fff8db758681ece40bb6947a90d571bce9dd60671a8e4",
    "SourceHanSansTW-Regular.otf": "5034bc32aa64bc93ce673fe05752724bb31fc6757c6bc1d23e925052c54fd2ac",
    "SourceHanSansKR-Regular.otf": "d8299926a5284f7461b46fa0617cc05d4f1fce8f034ffff78084225ce8b38071",
    "SourceHanSans_JP_sequences.txt": None,
    "LICENSE.txt": None,
}

# Ordering -> (source, supplement, horizontal CMap, vertical CMap, half-width CMaps, IVS file).
COLLECTIONS = {
    "Japan1": ("SourceHanSansJP-Regular.otf", 7, "UniJIS-UTF32-H", "UniJIS-UTF32-V",
               ("UniJIS-UCS2-HW-H", "UniJIS-UCS2-HW-V"), "SourceHanSans_JP_sequences.txt"),
    "GB1": ("SourceHanSansCN-Regular.otf", 5, "UniGB-UTF32-H", "UniGB-UTF32-V", (), None),
    "CNS1": ("SourceHanSansTW-Regular.otf", 7, "UniCNS-UTF32-H", "UniCNS-UTF32-V", (), None),
    "Korea1": ("SourceHanSansKR-Regular.otf", 2, "UniKS-UTF32-H", "UniKS-UTF32-V", (), None),
    "KR": ("SourceHanSansKR-Regular.otf", 9, "UniAKR-UTF32-H", None, (), None),
}

HEX = re.compile(r"<([0-9a-fA-F]+)>")


def digest(path):
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def own_mappings(path):
    """The code -> CID mappings a CMap file states itself, ignoring what it `usecmap`s."""
    out = {}
    mode = None
    with open(path, encoding="latin-1") as handle:
        for line in handle:
            if "begincidrange" in line:
                mode = "range"
                continue
            if "begincidchar" in line:
                mode = "char"
                continue
            if "endcidrange" in line or "endcidchar" in line:
                mode = None
                continue
            if mode is None:
                continue
            codes = HEX.findall(line)
            rest = line.rsplit(">", 1)[-1].split()
            if not rest:
                continue
            cid = int(rest[0])
            if mode == "range" and len(codes) == 2:
                low, high = int(codes[0], 16), int(codes[1], 16)
                for code in range(low, high + 1):
                    out.setdefault(code, cid + code - low)
            elif mode == "char" and len(codes) == 1:
                out.setdefault(int(codes[0], 16), cid)
    return out


def highest_cid(cmaps, ordering, supplement):
    """The collection's last CID, from Adobe's identity CMap for that supplement."""
    mappings = own_mappings(os.path.join(cmaps, f"Adobe-{ordering}-{supplement}"))
    return max(mappings.values())


def single_substitutions(font, tags):
    """Glyph -> glyph over every single substitution the GSUB features `tags` reach."""
    out = {}
    if "GSUB" not in font:
        return out
    table = font["GSUB"].table
    lookups = set()
    for record in table.FeatureList.FeatureRecord:
        if record.FeatureTag in tags:
            lookups.update(record.Feature.LookupListIndex)
    for index in sorted(lookups):
        lookup = table.LookupList.Lookup[index]
        for subtable in lookup.SubTable:
            if lookup.LookupType == 7:
                subtable = subtable.ExtSubTable
            mapping = getattr(subtable, "mapping", None)
            if mapping:
                for source, target in mapping.items():
                    out.setdefault(source, target)
    return out


def variation_sequences(font):
    """(base, selector) -> glyph name from the format 14 `cmap`, defaults resolved."""
    best = font.getBestCmap()
    out = {}
    for table in font["cmap"].tables:
        if table.format != 14:
            continue
        for selector, pairs in table.uvsDict.items():
            for base, name in pairs:
                glyph = name if name is not None else best.get(base)
                if glyph is not None:
                    out[(base, selector)] = glyph
    return out


def fill(source_font, cmaps, source_dir, entry):
    _, _, horizontal, vertical, half_width, ivs = entry
    best = source_font.getBestCmap()
    vert = single_substitutions(source_font, ("vert", "vrt2"))
    hwid = single_substitutions(source_font, ("hwid",))
    glyph_of = {}
    horizontal_map = own_mappings(os.path.join(cmaps, horizontal))
    for code in sorted(horizontal_map):
        cid = horizontal_map[code]
        if code in best:
            glyph_of.setdefault(cid, best[code])
    if vertical:
        for code, cid in sorted(own_mappings(os.path.join(cmaps, vertical)).items()):
            if cid in glyph_of or code not in best:
                continue
            if best[code] in vert:
                glyph_of[cid] = vert[best[code]]
    for name in half_width:
        stated = own_mappings(os.path.join(cmaps, name))
        for code, cid in sorted(stated.items()):
            if cid in glyph_of or code not in best:
                continue
            glyph = hwid.get(best[code])
            if glyph is not None and name.endswith("-V"):
                glyph = vert.get(glyph)
            if glyph is not None:
                glyph_of[cid] = glyph
    if ivs:
        sequences = variation_sequences(source_font)
        with open(os.path.join(source_dir, ivs), encoding="utf-8") as handle:
            for line in handle:
                parts = [part.strip() for part in line.split(";")]
                if line.startswith("#") or len(parts) != 3 or not parts[2].startswith("CID+"):
                    continue
                base, selector = (int(value, 16) for value in parts[0].split())
                cid = int(parts[2][4:])
                glyph = sequences.get((base, selector))
                if glyph is not None:
                    glyph_of.setdefault(cid, glyph)
    return glyph_of


def build(ordering, entry, source_dir, cmaps, out_dir):
    source, supplement = entry[0], entry[1]
    font = TTFont(os.path.join(source_dir, source), recalcTimestamp=False)
    glyph_of = fill(font, cmaps, source_dir, entry)
    last = highest_cid(cmaps, ordering, supplement)
    glyph_of = {cid: glyph for cid, glyph in glyph_of.items() if 0 < cid <= last}
    cff = font["CFF "].cff
    top = cff.topDictIndex[0]
    strings = top.CharStrings
    old_select = top.FDSelect
    hmtx = font["hmtx"].metrics
    order = [".notdef"] + [f"cid{cid:05d}" for cid in range(1, last + 1)]
    items, selects, metrics, names = [], [], {}, {}
    hole_private = top.FDArray[0].Private
    for gid, name in enumerate(order):
        glyph = ".notdef" if gid == 0 else glyph_of.get(gid)
        if glyph is None:
            item = T2CharString(program=["endchar"], private=hole_private,
                                globalSubrs=cff.GlobalSubrs)
            selects.append(0)
            metrics[name] = (int(hole_private.defaultWidthX), 0)
        else:
            index = strings.charStrings[glyph]
            item = strings.charStringsIndex[index]
            selects.append(old_select[index])
            metrics[name] = hmtx[glyph]
        items.append(item)
        names[name] = gid
    strings.charStringsIndex.items = items
    strings.charStrings = names
    top.charset = order
    select = FDSelect()
    select.format = 3
    select.gidArray = selects
    top.FDSelect = select
    top.ROS = ("Adobe", ordering, supplement)
    top.CIDCount = last + 1
    new_name = f"QuorraCJK-{ordering}"
    old_name = cff.fontNames[0]
    cff.fontNames = [new_name]
    for key in ("FullName", "FamilyName"):
        if hasattr(top, key):
            setattr(top, key, new_name)
    for fd in top.FDArray:
        fd.FontName = fd.FontName.replace(old_name, new_name)
    font.setGlyphOrder(order)
    font["hmtx"].metrics = metrics
    unicode_to_name = {}
    for code, cid in sorted(own_mappings(os.path.join(cmaps, entry[2])).items()):
        if glyph_of.get(cid) is not None:
            unicode_to_name.setdefault(code, f"cid{cid:05d}")
    kept = []
    for table in font["cmap"].tables:
        # Format 12 alone: a format 4 subtable cannot hold this many segments once the glyphs
        # are in CID order, and nothing the converter writes from a pack program reads `cmap`.
        if table.platformID == 3 and table.format == 12:
            table.cmap = dict(unicode_to_name)
            kept.append(table)
    font["cmap"].tables = kept
    font["post"].formatType = 3.0
    for tag in ("GSUB", "GPOS", "GDEF", "BASE", "VORG", "vhea", "vmtx", "DSIG"):
        if tag in font:
            del font[tag]
    # Every charstring self-contained, its width the first operand of its own first operator:
    # the converter restates a glyph's advance to the file's /W where the two differ, and a width
    # or a stem hint reached only through a subroutine is a site that rewrite does not follow.
    cff.desubroutinize()
    rename(font, ordering, new_name)
    path = os.path.join(out_dir, f"Adobe-{ordering}.otf")
    font.save(path)
    return path, len(glyph_of), last


def rename(font, ordering, postscript):
    """The OFL's condition 3: a Modified Version does not use the Reserved Font Name."""
    family = f"Quorra CJK {ordering}"
    keep = {0, 7, 8, 9, 11, 12, 13, 14}
    table = font["name"]
    table.names = [record for record in table.names
                   if record.nameID in keep and record.platformID == 3
                   and record.langID == 0x409]
    for name_id, value in ((1, family), (2, "Regular"), (3, f"{postscript};quorra-cjk-pack"),
                           (4, family), (5, "Version 1.000"), (6, postscript)):
        table.setName(value, name_id, 3, 1, 0x409)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--source", required=True)
    parser.add_argument("--cmaps", required=True)
    parser.add_argument("--out", required=True)
    arguments = parser.parse_args()
    for name, expected in SOURCES.items():
        path = os.path.join(arguments.source, name)
        if not os.path.exists(path):
            sys.exit(f"{path}: missing")
        if expected is not None and digest(path) != expected:
            sys.exit(f"{path}: not the program this pack is built from")
    if os.listdir(arguments.out):
        sys.exit(f"{arguments.out}: not empty")
    lines = []
    for ordering, entry in COLLECTIONS.items():
        path, filled, last = build(ordering, entry, arguments.source, arguments.cmaps,
                                   arguments.out)
        print(f"Adobe-{ordering}: {filled} of {last} CIDs filled")
        lines.append(f"{digest(path)}  {os.path.basename(path)}")
    licence = os.path.join(arguments.out, "LICENSE.txt")
    with open(os.path.join(arguments.source, "LICENSE.txt"), "rb") as handle:
        text = handle.read()
    with open(licence, "wb") as handle:
        handle.write(text)
    lines.append(f"{digest(licence)}  LICENSE.txt")
    with open(os.path.join(arguments.out, "SHA256SUMS"), "w", encoding="ascii") as handle:
        handle.write("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
