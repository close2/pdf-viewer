#!/usr/bin/env python3
"""One seed per shape: the selection every recipe over the whole document population makes.

    find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' -print0 \\
      | python3 fuzz/seed_shape.py naming JBIG2Decode JPXDecode | xargs -0 …

As a program it is the search the Python seeders make before they read a document, for a seeder
that is not Python: it prints, NUL-separated, the documents of the list on standard input that
hold any of the names given.

A seeder that keeps every object it finds keeps what the population holds, and the population on
this disk is some ninety thousand documents and 126 GB. Kept whole, each recipe over it wrote some
46 000 seeds and 4.5 GB through a corpus link into the main checkout, and a target loads its whole
corpus before its first mutation — `object`'s could not be loaded at all under its memory limit.
What a target's code does with an input is decided by a few properties of it — a CMap's codespace
ranges, a crypt filter's revision, an sfnt's table set — and not by which of a thousand producers
wrote the rest of its bytes. So each seeder states, per target, the **shape** that decides the
branches, beside the reason, and keeps the smallest seed of each: fewest bytes per execution, and
the same branches. `jpeg_bands` took this rule first (ADR 1559); ADR 1571 extends it to every
recipe that reads the documents, and says per target what the shape is and what it was measured to
keep.

**A shape is a claim about the target's code, and it is measured rather than trusted.** A shape
that is too coarse loses the edges two seeds of one shape reached apart; ADR 1571's proof runs each
recipe both ways over a sample of the documents and compares libFuzzer's `INITED cov`, which is the
only figure that says what the selection cost.

Also here: the memory map every recipe reads a document through, because a document of six
gigabytes read whole into the interpreter is half of a round's memory budget, and a search through
a map is the operating system's and C's rather than Python's.
"""

import contextlib
import hashlib
import mmap
import os
import re
import sys


def sha256(seed):
    """The name most recipes give a seed: its SHA-256, so a re-run adds only what is new."""
    return hashlib.sha256(seed).hexdigest()


class Smallest:
    """The smallest seed offered for each shape, ties broken by name so a re-run chooses the same.

    Seeds are held until `write`, because the smallest of a shape is known only after every
    document has been read; what is held is one seed per shape, which is the bound."""

    def __init__(self, name=sha256):
        self.name = name
        self.chosen = {}
        self.offered = 0

    def offer(self, shape, seed):
        """Keeps `seed` if it is the smallest yet of `shape`."""
        self.offered += 1
        seed = bytes(seed)
        name = self.name(seed)
        held = self.chosen.get(shape)
        if held is None or (len(seed), name) < held[:2]:
            self.chosen[shape] = (len(seed), name, seed)

    def write(self, directory):
        """Writes each shape's seed into `directory`; how many were not there already."""
        os.makedirs(directory, exist_ok=True)
        written = 0
        for _, name, seed in self.chosen.values():
            path = os.path.join(directory, name)
            if not os.path.exists(path):
                with open(path, "wb") as handle:
                    handle.write(seed)
                written += 1
        return written

    def summary(self):
        """The words a recipe prints about its selection."""
        held = sum(length for length, _, _ in self.chosen.values())
        return f"{self.offered} found, {len(self.chosen)} shapes, {held} bytes kept"


@contextlib.contextmanager
def mapped(path):
    """The document at `path` as a read-only memory map, or `b""` for an empty file.

    **A map answers `in` wrongly**: `b"/DSS" in m` is `False` for a map that holds it, because a
    map's membership test is a byte's. Every search of a map here is `find`, and a slice of one is
    `bytes` again."""
    with open(path, "rb") as handle:
        if os.fstat(handle.fileno()).st_size == 0:
            yield b""
            return
        with mmap.mmap(handle.fileno(), 0, access=mmap.ACCESS_READ) as data:
            yield data


def names_any(data, needles):
    """Whether `data` holds any of `needles`, each searched for by `find`."""
    return any(data.find(needle) >= 0 for needle in needles)


# The lexical forms a reader of text written in PDF's syntax (§7.2) branches on, whatever the
# text says: each end-of-line marker, the white-space characters, a byte outside ASCII, and a
# literal string's escapes (§7.3.4.2) and a hexadecimal string's case (§7.3.4.3). A CMap, a
# security handler's dictionary and a `/DA` are tokenised by those branches before any of their
# values is read, and ADR 1571's proof found seeds of one shape apart on exactly these.
TEXT_FORMS = {
    name: re.compile(pattern)
    for name, pattern in {
        "carriage return alone": rb"\r(?!\n)",
        "carriage return and line feed": rb"\r\n",
        "line feed alone": rb"(?<!\r)\n",
        "tab": rb"\t",
        "nul": rb"\x00",
        "form feed": rb"\x0c",
        "high byte": rb"[\x80-\xff]",
        "octal escape": rb"\\[0-7]",
        "other escape": rb"\\[^0-7]",
        "lower-case hexadecimal": rb"<[0-9a-f\s]*[a-f][0-9a-f\s]*>",
        "comment": rb"%",
    }.items()
}


def text_forms(text):
    """The set of `TEXT_FORMS` a seed uses."""
    return frozenset(name for name, pattern in TEXT_FORMS.items() if pattern.search(text))


def bucket(count):
    """A count as its power of two, the granularity at which a bound or a loop changes branch."""
    return max(count, 0).bit_length()


def main(argv):
    if len(argv) < 3 or argv[1] != "naming":
        sys.exit(__doc__)
    needles = [os.fsencode(needle) for needle in argv[2:]]
    for name in sys.stdin.buffer.read().split(b"\0"):
        if not name:
            continue
        try:
            with mapped(name) as data:
                held = names_any(data, needles)
        except (OSError, ValueError):
            continue
        if held:
            sys.stdout.buffer.write(name + b"\0")


if __name__ == "__main__":
    main(sys.argv)
