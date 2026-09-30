#!/usr/bin/env python3
"""Seed the `forms_data` fuzz target with FDF files, which its corpus had none of.

    python3 fuzz/seed_forms_data.py fuzz/corpus/forms_data

The target reads its input twice: as a §7.9.4 date, and as an FDF file through `Document::open`
and `FormsData::read`. A corpus of dates and short strings reaches the first half only — the
second needs a header, an object holding an `/FDF` dictionary and a trailer naming it, which a
mutator will not assemble from nothing (ADR 0742's lesson, ADR 1423). So this writes one small FDF
file per entry of §12.7.8.3's Table 243 and Table 246 the reader handles, each under the target's
own 8 KiB ceiling, and a few dates in each of §7.9.4's shapes. Named by content, so a re-run adds
nothing new.
"""

import hashlib
import os
import sys

FDF = [
    "/Fields [<< /T (name) /V (value) >>]",
    "/Fields [<< /T (a) /Kids [<< /T (b) /V (1) >> << /T (c) /V /Off >>] >>]",
    "/Fields [<< /T (list) /V [(one) (two)] /Opt [(one) (two) (three)] >>]",
    "/Fields [<< /T (flags) /Ff 4096 /SetFf 2 /ClrFf 1 /F 4 /SetF 32 /ClrF 2 >>]",
    "/Fields [<< /T (rich) /RV (<p>value</p>) /V <FEFF00410042> >>]",
    "/Fields [<< /T (ap) /AP << /N << /On 2 0 R >> >> /AS /On >>]",
    "/Fields [<< /T (act) /A << /S /ResetForm >> /AA << /K << /S /JavaScript /JS (1) >> >> >>]",
    "/Status (done) /F (target.pdf) /ID [<0102> <0304>]",
    "/Encoding /Shift_JIS /Fields [<< /T <82a0> /V <82a2> >>]",
    "/Annots [<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] /Page 0 /Contents (note) >>]",
    "/Annots [<< /Subtype /Ink /Rect [0 0 50 50] /Page 1 /InkList [[0 0 10 10 20 0]] >>]",
    "/Differences [<< /T (x) /V (y) >>] /Target (frame)",
    "/Pages [<< /Templates [<< /TRef << /Name (t) >> /Fields [] >>] >>]",
    "/EmbeddedFDFs [(inner.fdf)] /JavaScript << /Before (a) /After (b) >>",
]

DATES = [b"D:20260930", b"D:2026093012", b"D:20260930120000Z", b"D:20260930120000+02'00'",
         b"D:20260930120000-05'30", b"D:1999", b"20260930120000Z00'00'"]


def write(out, data):
    path = os.path.join(out, hashlib.sha256(data).hexdigest()[:40])
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "fuzz/corpus/forms_data"
    os.makedirs(out, exist_ok=True)
    added = 0
    for entries in FDF:
        body = (f"%FDF-1.2\n1 0 obj\n<< /FDF << {entries} >> >>\nendobj\n"
                "2 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
                "trailer\n<< /Root 1 0 R >>\n%%EOF\n")
        added += write(out, body.encode("latin-1"))
    for date in DATES:
        added += write(out, date)
    print(f"seed_forms_data: {added} new seeds in {out}")


if __name__ == "__main__":
    main()
