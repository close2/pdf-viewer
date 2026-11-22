#!/usr/bin/env python3
"""Seed the `media_aac` fuzz target with MP4 files whose sound track is real AAC.

    python3 fuzz/seed_media_aac.py <streams> fuzz/corpus/media_aac

`media_aac` decodes an MP4 file's AAC stream in the media worker (ADRs 3381, 3382), and a fuzzer
will not invent an AAC stream that decodes past its first element, so the seeds are ISO/IEC
14496-26's own conformance streams: `<streams>` is the directory of its `compressedMp4/` files
(`PDF_MEDIA_MP4_STREAMS`), fetched from ISO's Publicly Available Standards and never committed.
Files past 256 KiB are left out, so that an input stays a fraction of a second; pass
`-max_len=262144` to the run. Seeds are named by content, so a re-run adds nothing.
"""

import hashlib
import os
import sys

LARGEST = 256 << 10


def main():
    streams, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    added = 0
    for name in sorted(os.listdir(streams)):
        if not name.endswith(".mp4"):
            continue
        with open(os.path.join(streams, name), "rb") as handle:
            data = handle.read()
        if len(data) > LARGEST:
            continue
        path = os.path.join(out, hashlib.sha256(data).hexdigest()[:40])
        if not os.path.exists(path):
            with open(path, "wb") as handle:
                handle.write(data)
            added += 1
    print(f"{added} seed(s) added to {out}")


if __name__ == "__main__":
    main()
