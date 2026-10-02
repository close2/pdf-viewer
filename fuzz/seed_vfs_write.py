#!/usr/bin/env python3
"""Seed the `vfs_write` fuzz target with small documents and each of the five write verbs.

    find -L doc/pdf.js/test/pdfs -name '*.pdf' -size -32k -print0 \\
      | xargs -0 python3 fuzz/seed_vfs_write.py fuzz/corpus/vfs_write

The target's framing: a verb byte, a selector byte, the document's length as four bytes little
endian, the document, and the bytes written. Every document past 32 KiB is skipped — each
execution mounts it, and a write re-reads it — and each kept one is framed six ways: a PDF
inserted into `pages/` (the next document on the list), a page deleted, a file embedded and one
removed, a `meta/info.json` stating a title and an author, and the staged transaction of a write
at an offset and a truncation.
"""

import hashlib
import os
import struct
import sys

MAX_DOCUMENT = 32 * 1024
INFO = b'{"Title": "seeded", "Author": "fuzz/seed_vfs_write.py", "Keywords": null}'


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    documents = []
    for path in argv[2:]:
        try:
            with open(path, "rb") as handle:
                data = handle.read()
        except OSError:
            continue
        if len(data) <= MAX_DOCUMENT and data.startswith(b"%PDF"):
            documents.append(data)
    written = 0
    for index, document in enumerate(documents):
        other = documents[(index + 1) % len(documents)]
        for verb, selector, payload in (
            (0, index % 3, other),
            (1, index % 3, b""),
            (2, 8, b"note.txt" + b"an embedded file"),
            (3, 0, b""),
            (4, 0, INFO),
            (5, 7, b"staged bytes"),
        ):
            seed = bytes([verb, selector]) + struct.pack("<I", len(document)) + document + payload
            name = os.path.join(directory, hashlib.sha256(seed).hexdigest())
            if not os.path.exists(name):
                with open(name, "wb") as handle:
                    handle.write(seed)
                written += 1
    print(f"seed_vfs_write.py: {len(documents)} documents framed, {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
