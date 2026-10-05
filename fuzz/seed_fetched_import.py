#!/usr/bin/env python3
"""Seed the `fetched_import` fuzz target with form data a server could answer with.

    python3 fuzz/seed_fetched_import.py fuzz/corpus/fetched_import [<file.xfdf>...]

The target's first byte chooses the route (the reader, the document in front or behind, a
submission or a fetched import) and the rest is the body `Command::Respond` carries (ADR 1527). A
body a mutator assembles from nothing never opens as FDF or parses as XFDF, so this writes FDF
files naming the target form's own fields and template — `b.c`, `check`, `list`, `t` — and every
XFDF file named on the command line, each under all eight routes. Named by content, so a re-run
adds nothing new.
"""

import hashlib
import os
import sys

# Bodies of the `/FDF` dictionary: §12.7.8.3's Table 243 and Table 246 against the target's names.
FDF = [
    "/Fields [<< /T (b) /Kids [<< /T (c) /V (value) >>] >>]",
    "/Fields [<< /T (b.c) /V <FEFF00410042> >> << /T (check) /V /On >>]",
    "/Fields [<< /T (list) /V [(one) (two)] /Opt [(one) (two) (three)] >>]",
    "/Fields [<< /T (check) /Ff 4096 /SetFf 2 /ClrFf 1 /F 4 /SetF 32 /ClrF 2 /AS /On >>]",
    "/Fields [<< /T (b) /Kids [<< /T (c) /RV (<p>x</p>) /V (y) /A << /S /ResetForm >> >>] >>]",
    "/Status (Thank you) /ID [<0102> <0304>] /Fields [<< /T (b.c) /V (z) >>]",
    "/ID [<0506> <0708>] /Fields [<< /T (nobody) /V (1) >>]",
    "/Encoding /Shift_JIS /Fields [<< /T (b.c) /V <82a2> >>]",
    "/Annots [<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] /Page 0 /Contents (note) >>]",
    "/Pages [<< /Templates [<< /TRef << /Name (t) >> /Fields [<< /T (c) /V (p) >>] /Rename true >>] >>]",
    "/Pages [<< /Templates [<< /TRef << /Name (t) /F (library.pdf) >> >>] >>]",
    "/Fields [<< /T (check) /APRef << /N << /Name (t) /F (library.pdf) >> >> >>]",
    "/EmbeddedFDFs [(inner.fdf)] /Fields [<< /T (b.c) /V (e) >>]",
]

XFDF_FIELDS = (
    '<?xml version="1.0" encoding="UTF-8"?>\n'
    '<xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve">\n'
    '  <ids original="0102" modified="0304"/>\n'
    '  <fields><field name="b"><field name="c"><value>xfdf</value></field></field>\n'
    '  <field name="check"><value>On</value></field>\n'
    '  <field name="list"><value>one</value><value>two</value></field></fields>\n'
    '</xfdf>\n'
)


def write(out, data):
    path = os.path.join(out, hashlib.sha256(data).hexdigest()[:40])
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def routes(out, body, xfdf):
    """Every route a body can arrive by: bit 0 is the reader, so it is fixed by the body's format."""
    added = 0
    for route in range(8):
        if (route & 1) == (1 if xfdf else 0):
            added += write(out, bytes([route]) + body)
    return added


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "fuzz/corpus/fetched_import"
    os.makedirs(out, exist_ok=True)
    added = 0
    for entries in FDF:
        body = (f"%FDF-1.2\n1 0 obj\n<< /FDF << {entries} >> >>\nendobj\n"
                "trailer\n<< /Root 1 0 R >>\n%%EOF\n")
        added += routes(out, body.encode("latin-1"), False)
    added += routes(out, XFDF_FIELDS.encode("utf-8"), True)
    for name in sys.argv[2:]:
        with open(name, "rb") as handle:
            added += routes(out, handle.read(), True)
    print(f"seed_fetched_import: {added} new seeds in {out}")


if __name__ == "__main__":
    main()
