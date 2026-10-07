#!/usr/bin/env python3
"""Seed the `page` target with composite fonts whose `/W` and `/W2` ranges span 65 536 CIDs.

    python3 fuzz/seed_widths.py fuzz/corpus/page

**Why a generator.** §9.7.4.3's second form, `cfirst clast w`, is one statement about a whole
range of CIDs, and a reader that expands it pays for the range rather than for the bytes that
state it. No document on this disk writes thousands of ranges each 65 536 CIDs wide, so
`seed_page.py` cannot hand the fuzzer the input that would find such a cost again; these
documents state it outright (ADR 1596). Each is a whole one-page document with one `Type0` font
shown through `Identity-H` or `Identity-V`, so the interpreter loads the widths and then places
glyphs by them.

**What each seed is.** `wide` states 4096 ranges of 65 536 CIDs back to back and a last range over
every CID there is; `overlap` states the same ranges twice over in reverse, so every statement
after the first meets one already claimed (the clause's "first specification"); `list` is one
dense `c [w1 w2 …]` of 16 384 widths with a hole every hundred elements; `vertical` is `wide`'s
ranges in `/W2` under `Identity-V`; `backward` states ranges whose last CID is below their first.
Each is under the target's 256 KiB input ceiling.
"""

import os
import sys

CATALOG = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
PAGES = b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n"
PAGE = (b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n")
TEXT = b"BT /F1 12 Tf 10 700 Td <0000 0001 FFFF 1234 ABCD> Tj ET"


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


def document(metrics, encoding="Identity-H"):
    """One page showing five codes through a `Type0` font whose descendant states `metrics`."""
    content = f"4 0 obj\n<< /Length {len(TEXT)} >>\nstream\n".encode() + TEXT
    content += b"\nendstream\nendobj\n"
    font = (f"5 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /Seed /Encoding /{encoding} "
            f"/DescendantFonts [6 0 R] >>\nendobj\n").encode()
    descendant = (f"6 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Seed "
                  f"/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> "
                  f"/FontDescriptor 7 0 R /DW 1000 {metrics} >>\nendobj\n").encode()
    descriptor = (b"7 0 obj\n<< /Type /FontDescriptor /FontName /Seed /Flags 4 "
                  b"/FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 "
                  b"/CapHeight 700 /StemV 80 >>\nendobj\n")
    return assemble([CATALOG, PAGES, PAGE, content, font, descendant, descriptor])


def ranges(count, reverse=False, fields=lambda n: f"{n}"):
    order = range(count - 1, -1, -1) if reverse else range(count)
    return " ".join(f"{n * 65536} {n * 65536 + 65535} {fields(n)}" for n in order)


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: seed_widths.py <corpus-dir>")
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    listed = " ".join("/x" if n % 100 == 99 else str(250 + n % 750) for n in range(16384))
    seeds = {
        "widths_wide.pdf": document(f"/W [{ranges(4096)} 0 4294967295 7]"),
        "widths_overlap.pdf": document(f"/W [{ranges(2048)} {ranges(2048, reverse=True)}]"),
        "widths_list.pdf": document(f"/W [0 [{listed}] 0 65535 500]"),
        "widths_vertical.pdf": document(
            f"/W2 [{ranges(2048, fields=lambda n: f'-{n} 500 880')}]", encoding="Identity-V"),
        "widths_backward.pdf": document(
            "/W [" + " ".join(f"{n * 65536 + 65535} {n * 65536} {n}" for n in range(1024)) + "]"),
    }
    written = 0
    for name, data in seeds.items():
        if len(data) > 256 * 1024:
            print(f"  {name}: {len(data)} bytes is past the target's ceiling, skipped")
            continue
        with open(os.path.join(out_dir, name), "wb") as handle:
            handle.write(data)
        written += 1
    print(f"{written} seed(s) in {out_dir}")


if __name__ == "__main__":
    main()
