#!/usr/bin/env python3
"""Builds each test's PDF from the readable source in its manifest.

A manifest's `[[object]]` tables are the file's indirect objects in order, numbered from 1; the
first is the document catalog. Each `body` is PDF syntax written by hand, so every byte of the
generated file is in the manifest for a reviewer to read. The generator adds only what a person
should not compute by hand: the header, each stream's `/Length`, the cross-reference table's byte
offsets (ISO 32000-2 section 7.5.4) and the trailer (section 7.5.5).

The output is a pure function of the manifest: no date, no identifier, no random byte. Running it
twice writes the same files, and `--check` says whether a directory already holds them.

Usage:
    generate.py <out-dir> [manifest.toml ...]     # every manifest under tests/ when none is named
    generate.py --check <out-dir>                  # exit 1 when a file differs from its manifest
"""

import pathlib
import sys
import tomllib

SUITE = pathlib.Path(__file__).resolve().parent.parent


def build(manifest: dict) -> bytes:
    """The bytes of the manifest's file."""
    source = manifest["pdf"]
    version = source.get("version", "2.0")
    # Section 7.5.2: the header, then a comment of four bytes above 127 so that a transfer that
    # treats the file as text can tell it is not.
    out = bytearray(f"%PDF-{version}\n%\xe2\xe3\xcf\xd3\n".encode("latin-1"))
    offsets = []
    for number, obj in enumerate(source["object"], start=1):
        offsets.append(len(out))
        body = obj["body"].strip()
        out += f"{number} 0 obj\n".encode("latin-1")
        if "stream" in obj:
            data = obj["stream"].encode("latin-1")
            if not (body.startswith("<<") and body.endswith(">>")):
                raise ValueError(f"object {number}: a stream's body is a dictionary")
            if "/Length" in body:
                raise ValueError(f"object {number}: /Length is the generator's to write")
            body = f"{body[:-2].rstrip()} /Length {len(data)} >>"
            out += body.encode("latin-1") + b"\nstream\n" + data + b"\nendstream\n"
        else:
            out += body.encode("latin-1") + b"\n"
        out += b"endobj\n"
    xref = len(out)
    count = len(offsets) + 1
    # Section 7.5.4: each entry is exactly 20 bytes, its end-of-line a space and a line feed.
    out += f"xref\n0 {count}\n".encode("latin-1")
    out += b"0000000000 65535 f \n"
    for offset in offsets:
        out += f"{offset:010d} 00000 n \n".encode("latin-1")
    extra = source.get("trailer", "")
    out += f"trailer\n<< /Size {count} /Root 1 0 R {extra}>>\n".encode("latin-1")
    out += f"startxref\n{xref}\n%%EOF\n".encode("latin-1")
    return bytes(out)


def manifests(named: list[str]) -> list[pathlib.Path]:
    if named:
        return [pathlib.Path(name) for name in named]
    return sorted((SUITE / "tests").glob("*.toml"))


def main(argv: list[str]) -> int:
    check = argv[:1] == ["--check"]
    if check:
        argv = argv[1:]
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    out_dir = pathlib.Path(argv[0])
    out_dir.mkdir(parents=True, exist_ok=True)
    stale = 0
    for path in manifests(argv[1:]):
        with path.open("rb") as file:
            manifest = tomllib.load(file)
        target = out_dir / f"{manifest['id']}.pdf"
        data = build(manifest)
        if check:
            if not target.exists() or target.read_bytes() != data:
                print(f"differs: {target}")
                stale += 1
        else:
            target.write_bytes(data)
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
