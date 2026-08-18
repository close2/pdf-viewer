#!/usr/bin/env python3
"""Seed the `page` target with §7.8.2's *nested* content streams, straddling the memo's line.

    python3 fuzz/seed_nested_content.py fuzz/corpus/page

**Why a generator and not a corpus.** `seed_page.py` seeds from real documents, which is right
for the object graph and useless for this: the route decision ADR 0427 added is taken on a
*decoded* size, and no document on this disk states a form XObject, a tiling pattern's cell, a
Type 3 glyph description or an annotation appearance whose decode outgrows four mebibytes. That
is trap 8 exactly — a corpus finds what documents contain, not what the code decides on — so the
inputs that reach the pumped branch have to be built.

**What each seed is.** A whole one-page document, well formed, whose page draws through one of
§7.8.2's four self-contained content streams, with that stream's decoded size chosen to land
either side of `DecodedStreams::allowance`: `tiny` and `under` are decoded whole and memoised,
`edge` sits just below the line, and `over` and `bomb` are pumped through the window. Half the
form seeds are truncated inside their deflate stream, which is the ADR 0343 prefix rule on the
same route. Every one is under the target's own 256 KiB input ceiling, because a seed past it is
copied, mutated and thrown away every time.

**What the content is.** Mostly §7.2.4 comments, so that megabytes cost a scan rather than four
million operators, with marks, an inline image and a text object every few hundred lines — so
that the window's three boundary cases (a comment cut by a refill, §8.9.7's lookahead, a token
straddling one) are all crossed by every seed rather than only by the lucky ones.
"""

import os
import sys
import zlib

CATALOG = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
PAGES = b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n"


def assemble(objects):
    """Wraps a list of object bodies in a header, a cross-reference table and a trailer."""
    out = bytearray(b"%PDF-1.7\n")
    offsets = []
    for body in objects:
        offsets.append(len(out))
        out += body
    xref_at = len(out)
    size = len(offsets) + 1
    out += f"xref\n0 {size}\n".encode() + b"0000000000 65535 f \n"
    for offset in offsets:
        out += f"{offset:010} 00000 n \n".encode()
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n".encode()
    return bytes(out)


def stream(number, entries, data):
    head = f"{number} 0 obj\n<< {entries} /Length {len(data)} >>\nstream\n".encode()
    return head + data + b"\nendstream\nendobj\n"


def flate(payload, damage=0):
    """Deflates `payload`, dropping `damage` bytes off the end so the decode stops short."""
    encoded = zlib.compress(payload, 9)
    return encoded[: len(encoded) - damage] if damage else encoded


def content(mebibytes, comment=97):
    """Roughly `mebibytes` of content: comments, with marks and an inline image through them."""
    buf = bytearray()
    target = int(mebibytes * 1024 * 1024)
    index = 0
    while len(buf) < target:
        if index % 400 == 0:
            buf += b"q 0 0 1 rg 10 20 30 40 re f Q\n"
        if index % 977 == 0:
            buf += b"BI /W 2 /H 2 /CS /G /BPC 8 ID \x01\x02\x03\x04 EI\n"
        if index % 1301 == 0:
            buf += b"BT /F1 12 Tf (hello) Tj ET\n"
        buf += b"%" + b"p" * comment + b"\n"
        index += 1
    return bytes(buf)


def form(mebibytes, damage=0, filtered=True):
    """§8.10 — invoked twice, and a transparency group so §11.6.6 runs it more than that."""
    data = flate(content(mebibytes), damage) if filtered else content(mebibytes)
    entries = "/Type /XObject /Subtype /Form /BBox [0 0 612 792] " \
              "/Group << /S /Transparency /CS /DeviceCMYK >> /Resources << /Font << /F1 6 0 R >> >>"
    return assemble([
        CATALOG, PAGES,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources "
        b"<< /XObject << /Fx 5 0 R >> /Font << /F1 6 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        stream(4, "", b"/Fx Do /Fx Do"),
        stream(5, entries + (" /Filter /FlateDecode" if filtered else ""), data),
        b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    ])


def pattern(mebibytes, damage=0):
    """§8.7.3.1 — the cell runs once per cell, which is what the memo's line is for."""
    return assemble([
        CATALOG, PAGES,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources "
        b"<< /Pattern << /P0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        stream(4, "", b"/Pattern cs /P0 scn 0 0 300 300 re f"),
        stream(5, "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 60 60] /XStep 60 "
                  "/YStep 60 /Resources << >> /Filter /FlateDecode",
               flate(content(mebibytes, 40), damage)),
    ])


def type3(mebibytes, damage=0):
    """§9.6.4 — the description runs once per character, and Table 110 puts `d1` first."""
    data = flate(b"1000 0 0 0 1000 1000 d1\n" + content(mebibytes, 60), damage)
    return assemble([
        CATALOG, PAGES,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources "
        b"<< /Font << /T3 6 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        stream(4, "", b"BT /T3 24 Tf (AAAA) Tj ET"),
        stream(5, "/Filter /FlateDecode", data),
        b"6 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] "
        b"/FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 5 0 R >> "
        b"/Encoding << /Type /Encoding /Differences [65 /a] >> /FirstChar 65 /LastChar 65 "
        b"/Widths [1000] /Resources << >> >>\nendobj\n",
    ])


def appearance(mebibytes, damage=0):
    """§12.5.5 — read twice by construction: once for §7.4.1's damage, once for the drawing."""
    return assemble([
        CATALOG, PAGES,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> "
        b"/Contents 4 0 R /Annots [5 0 R] >>\nendobj\n",
        stream(4, "", b"0 0 1 rg 1 2 3 4 re f"),
        b"5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 200 200] /F 4 "
        b"/AP << /N 6 0 R >> >>\nendobj\n",
        stream(6, "/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << >> "
                  "/Filter /FlateDecode", flate(content(mebibytes, 50), damage)),
    ])


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: seed_nested_content.py <corpus-dir>")
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    written = 0
    for mebibytes, tag in [(0.01, "tiny"), (1.0, "under"), (3.9, "edge"), (8.0, "over"),
                           (40.0, "bomb")]:
        seeds = {
            f"form_{tag}.pdf": form(mebibytes),
            f"form_{tag}_damaged.pdf": form(mebibytes, damage=6),
            f"pattern_{tag}.pdf": pattern(mebibytes),
            f"type3_{tag}.pdf": type3(mebibytes),
            f"appearance_{tag}.pdf": appearance(mebibytes),
        }
        for name, data in seeds.items():
            if len(data) > 256 * 1024:
                print(f"  {name}: {len(data)} bytes is past the target's ceiling, skipped")
                continue
            with open(os.path.join(out_dir, name), "wb") as handle:
                handle.write(data)
            written += 1
    with open(os.path.join(out_dir, "form_unfiltered.pdf"), "wb") as handle:
        handle.write(form(0.02, filtered=False))
    written += 1
    print(f"{written} seed(s) in {out_dir}")


if __name__ == "__main__":
    main()
