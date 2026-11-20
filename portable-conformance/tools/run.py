#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Runs the suite's checks against one or more processors, each driven through an adapter.

An adapter is any executable that answers three requests (README.md, "The adapter protocol"):

    <adapter> render <pdf> <page> <dpi> <out.png>   draw one page, zero-based, to an 8-bit PNG
    <adapter> text <pdf> <page>                      print one page's extracted text as UTF-8
    <adapter> query <pdf> <question ...>             print the one-line answer to a question

and exits 3 for a request its processor cannot be asked, which the table shows as `n/a` rather
than as a failure. Nothing here knows which processor is behind an adapter.

A check's verdict comes from the manifest alone: the expected value and its tolerance are written
there with the clause that fixes them. Agreement between processors is never consulted.

Usage:
    run.py --pdfs <dir> --adapter <path> [--adapter <path> ...] [--work <dir>] [--json <file>]
           [manifest.toml ...]
"""

import argparse
import json
import pathlib
import subprocess
import sys
import tomllib
import zlib

SUITE = pathlib.Path(__file__).resolve().parent.parent
CANNOT = 3

# The classification thresholds of `marked`, `unmarked` and `between` (README.md, "Pixels"). Every
# pixel check paints black (section 10.4.2.2: 0.0 is black) on white paper and probes at least
# five device pixels from any edge the clause fixes, so the probe sees paint or paper and nothing
# in between; the thresholds are a quarter of the range from each end, wide enough for a processor
# that converts through an ICC profile (section 10.3) whose black is not exactly 0.
DARK = 64
LIGHT = 191


def read_png(path: pathlib.Path) -> tuple[int, int, list[tuple[int, int, int]]]:
    """Width, height and RGB pixels of an 8-bit, non-interlaced PNG (ISO/IEC 15948)."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    position, idat, header = 8, bytearray(), None
    while position < len(data):
        length = int.from_bytes(data[position : position + 4], "big")
        kind = data[position + 4 : position + 8]
        body = data[position + 8 : position + 8 + length]
        position += 12 + length
        if kind == b"IHDR":
            header = body
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    if header is None:
        raise ValueError(f"{path}: no IHDR")
    width = int.from_bytes(header[0:4], "big")
    height = int.from_bytes(header[4:8], "big")
    depth, colour, interlace = header[8], header[9], header[12]
    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(colour)
    if depth != 8 or channels is None or interlace != 0:
        raise ValueError(f"{path}: depth {depth}, colour type {colour}, interlace {interlace}")
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    rows, previous, at = [], bytearray(stride), 0
    for _ in range(height):
        kind, line = raw[at], bytearray(raw[at + 1 : at + 1 + stride])
        at += 1 + stride
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            corner = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                estimate = left + up - corner
                pa, pb, pc = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
                predictor = left if pa <= pb and pa <= pc else up if pb <= pc else corner
                line[i] = (line[i] + predictor) & 0xFF
        rows.append(line)
        previous = line
    pixels = []
    for line in rows:
        for x in range(width):
            sample = line[x * channels : (x + 1) * channels]
            if channels <= 2:
                rgb, alpha = (sample[0],) * 3, sample[1] if channels == 2 else 255
            else:
                rgb, alpha = tuple(sample[:3]), sample[3] if channels == 4 else 255
            # Composited onto white paper, the page's own backdrop (section 11.4.7).
            pixels.append(tuple((c * alpha + 255 * (255 - alpha)) // 255 for c in rgb))
    return width, height, pixels


class Adapter:
    def __init__(self, path: str):
        self.path = pathlib.Path(path).resolve()
        self.name = self.path.name

    def ask(self, *arguments: str) -> tuple[int, str]:
        done = subprocess.run(
            [str(self.path), *arguments], capture_output=True, timeout=120, check=False
        )
        out = done.stdout.decode("utf-8", errors="replace")
        if done.returncode not in (0, CANNOT):
            out = done.stderr.decode("utf-8", errors="replace").strip()[-300:]
        return done.returncode, out


def probe_xy(check: dict, dpi: float, height: int) -> tuple[int, int]:
    """A probe's raster pixel: `pixel` as given, or `at` in default user space (section 8.3.2.3,
    origin at the lower left, 72 units to the inch) on an unrotated page whose MediaBox starts at
    the origin."""
    if "pixel" in check:
        return check["pixel"][0], check["pixel"][1]
    x, y = check["at"]
    return int(x * dpi / 72), int(height - y * dpi / 72)


def judge_pixel(check: dict, raster, dpi: float) -> str | None:
    width, height, pixels = raster
    kind = check["type"]
    if kind == "size":
        if (width, height) != (check["width"], check["height"]):
            return f"size {width}x{height}, expected {check['width']}x{check['height']}"
        return None
    x, y = probe_xy(check, dpi, height)
    if not (0 <= x < width and 0 <= y < height):
        return f"probe ({x}, {y}) outside a {width}x{height} raster"
    rgb = pixels[y * width + x]
    if kind == "marked" and max(rgb) > DARK:
        return f"({x}, {y}) is {rgb}, expected marked (every component <= {DARK})"
    if kind == "unmarked" and min(rgb) < LIGHT:
        return f"({x}, {y}) is {rgb}, expected unmarked (every component >= {LIGHT})"
    if kind == "neutral" and max(rgb) - min(rgb) > check["tolerance"]:
        return f"({x}, {y}) is {rgb}, components differ by more than {check['tolerance']}"
    if kind == "between" and not all(DARK < c < LIGHT for c in rgb):
        return f"({x}, {y}) is {rgb}, expected between {DARK} and {LIGHT}"
    return None


def run_test(manifest: dict, pdf: pathlib.Path, adapter: Adapter, work: pathlib.Path):
    """`(verdict, detail)`: verdict is pass, fail, n/a or error."""
    failures, rasters = [], {}
    for check in manifest["check"]:
        kind = check["type"]
        if kind in ("marked", "unmarked", "neutral", "between", "size"):
            page = check.get("page", 0)
            if page not in rasters:
                out = work / f"{manifest['id']}.{adapter.name}.{page}.png"
                code, said = adapter.ask("render", str(pdf), str(page), str(manifest["dpi"]), str(out))
                if code == CANNOT:
                    return "n/a", "render"
                if code != 0 or not out.exists():
                    return "error", f"render: {said}"
                rasters[page] = read_png(out)
            failure = judge_pixel(check, rasters[page], manifest["dpi"])
        elif kind == "text":
            code, said = adapter.ask("text", str(pdf), str(check["page"]))
            if code == CANNOT:
                return "n/a", "text"
            if code != 0:
                return "error", f"text: {said}"
            # Section 9.10 maps codes to characters and says nothing of the spaces and line ends an
            # extractor puts around a run, so only the leading and trailing white space is ignored.
            got = said.strip()
            failure = None if got == check["equals"] else f"text {got!r}, expected {check['equals']!r}"
        elif kind == "answer":
            code, said = adapter.ask("query", str(pdf), *check["question"].split(" "))
            if code == CANNOT:
                return "n/a", f"query {check['question'].split(' ')[0]}"
            if code != 0:
                return "error", f"query: {said}"
            got = said.strip()
            failure = None if got == check["equals"] else f"{check['question']}: {got!r}, expected {check['equals']!r}"
        else:
            raise ValueError(f"{manifest['id']}: unknown check type {kind}")
        if failure:
            failures.append(failure)
    return ("fail", "; ".join(failures)) if failures else ("pass", "")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--pdfs", required=True, help="the directory generate.py wrote")
    parser.add_argument("--adapter", action="append", required=True)
    parser.add_argument("--work", default=None, help="where rasters go (default: <pdfs>/work)")
    parser.add_argument("--json", default=None, help="also write every verdict here")
    parser.add_argument("manifests", nargs="*")
    options = parser.parse_args(argv)
    pdfs = pathlib.Path(options.pdfs)
    work = pathlib.Path(options.work) if options.work else pdfs / "work"
    work.mkdir(parents=True, exist_ok=True)
    adapters = [Adapter(path) for path in options.adapter]
    paths = [pathlib.Path(p) for p in options.manifests] or sorted((SUITE / "tests").glob("*.toml"))
    manifests = []
    for path in paths:
        with path.open("rb") as file:
            manifests.append(tomllib.load(file))
    clause_key = lambda m: [int(part) for part in m["clause"].split(".")]
    manifests.sort(key=lambda m: (clause_key(m), m["id"]))
    results = []
    for manifest in manifests:
        pdf = pdfs / f"{manifest['id']}.pdf"
        row = {"id": manifest["id"], "clause": manifest["clause"], "kind": manifest["kind"]}
        for adapter in adapters:
            verdict, detail = run_test(manifest, pdf, adapter, work)
            row[adapter.name] = {"verdict": verdict, "detail": detail}
        results.append(row)
    names = [adapter.name for adapter in adapters]
    print("| clause | test | kind | " + " | ".join(names) + " |")
    print("|---|---|---|" + "---|" * len(names))
    for row in results:
        cells = [row[name]["verdict"] for name in names]
        print(f"| {row['clause']} | {row['id']} | {row['kind']} | " + " | ".join(cells) + " |")
    print()
    for row in results:
        for name in names:
            if row[name]["verdict"] != "pass":
                print(f"{row['id']} [{name}] {row[name]['verdict']}: {row[name]['detail']}")
    for name in names:
        passed = sum(row[name]["verdict"] == "pass" for row in results)
        print(f"{name}: {passed} of {len(results)} pass")
    if options.json:
        pathlib.Path(options.json).write_text(json.dumps(results, indent=1, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
