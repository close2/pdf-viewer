#!/usr/bin/env python3
"""Writes a machine's clock bands from the logs of `tools/bands.sh retake` (ADR 2662).

    tools/bands.py --logs <dir> --template <machine dir> --out <machine dir>

`<dir>` holds `launch-<n>.log`, `turn-<n>.log` and `toolkit-<n>.log`, each one run of the gate under
`PDFVIEWER_BANDS_RETAKE=1`, which prints every clock figure and judges none. The template is any
machine's directory: it says which rows and which keys a machine file states, in the order the
band files ask for them (`tools/conformance/test-support/machine_bands.rs`). Every band is ADR
1969's rule over every kept run — the smallest x 0.85 to the largest x 1.20, rounded outward — save
the disk probes, which are ADR 2660's shape over every cold sample, and a cold first page, which
takes its row's warm band (ADR 2158). A figure no run kept is refused by name rather than written.
"""
import argparse
import glob
import math
import os
import re
import statistics
import sys
from collections import defaultdict


def runs_of(logs, gate):
    return sorted(glob.glob(os.path.join(logs, f"{gate}-*.log")))


def launch(logs):
    """(row, field) -> [value per run]; plus the disk samples."""
    values, io_rate, io_latency = defaultdict(list), [], []
    for log in runs_of(logs, "launch"):
        lines = open(log, errors="replace").read().splitlines()
        for i, line in enumerate(lines):
            m = re.match(r"launch-path: calibration ([\d.]+) ms", line)
            if m:
                values[("", "calibration_ms")].append(float(m.group(1)))
            m = re.match(r"ratchet: (.*) \((\w+_ms)\): ([\d.]+), band", line)
            if not m:
                continue
            name, field, value = m.groups()
            probe = lines[i + 1] if i + 1 < len(lines) else ""
            if "above one task per physical core" in probe or "not on the machine's own clock" in probe:
                continue
            row = name.split(":")[0] if name.startswith("doc/") else ""
            values[(row, field)].append(float(value))
            rate = re.search(r"the disk's rate ([\d.]+) ms", probe)
            latency = re.search(r"its latency ([\d.]+) ms", probe)
            if rate:
                io_rate.append(float(rate.group(1)))
            if latency:
                io_latency.append(float(latency.group(1)))
    return values, io_rate, io_latency


def turn(logs):
    values = defaultdict(list)
    for log in runs_of(logs, "turn"):
        quickest = None
        for line in open(log, errors="replace"):
            m = re.search(r"child: turn [\d.]+ seventh [\d.]+ step [\d.]+ ms, calibration ([\d.]+) ms", line)
            if m:
                c = float(m.group(1))
                quickest = c if quickest is None else min(quickest, c)
            m = re.match(r"\s+(\S+\.pdf) p(\d+) (turn|seventh|step) ([\d.]+) ms, band [^,]+, (.*?): (.*)$", line)
            if m and not re.search(r"load [\d.]+ against", m.group(6)):
                page, number, figure, value = m.group(1), m.group(2), m.group(3), m.group(4)
                values[(f"{page} {number}", f"{figure}_ms")].append(float(value))
        if quickest is not None:
            values[("", "calibration_ms")].append(quickest)
    return values


def toolkit(logs):
    values = defaultdict(list)
    for log in runs_of(logs, "toolkit"):
        window, kept = "", True
        for line in open(log, errors="replace"):
            m = re.match(r"toolkit launch: (quorra-\w+) ", line)
            if m:
                window = m.group(1)
            if re.match(r"toolkit launch: load [\d.]+ before", line):
                kept = "NOT JUDGED" not in line
            m = re.match(r"toolkit launch: (\w+_ms) ([\d.]+), of ", line)
            if m and kept:
                values[(window, m.group(1))].append(float(m.group(2)))
    return values


def places(value):
    return 2 if value < 10 else 1


def outward(low, high):
    lo_scale, hi_scale = 10 ** places(low), 10 ** places(high)
    return math.floor(low * lo_scale) / lo_scale, math.ceil(high * hi_scale) / hi_scale


def fmt(value):
    return f"{value:.{places(value)}f}"


def rule(samples):
    return outward(min(samples) * 0.85, max(samples) * 1.20)


def io_shape(samples):
    return outward(min(samples) * 5 / 6, statistics.median(samples) * 2.5)


def render(template, values, special, print_):
    """One machine file's text from the template's rows and keys, and the figures it could not take."""
    missing, text = [], []
    header, identity = "", ""
    for line in open(template):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("[["):
            header, identity = stripped, ""
            text += ["", stripped]
            continue
        key, _, value = (part.strip() for part in stripped.partition("="))
        if key in ("path", "name"):
            identity = value.strip('"')
            text.append(stripped)
            continue
        if key == "page":
            identity = f"{identity} {value}"
            text.append(stripped)
            continue
        if key == "machine":
            text.append(f'machine = "{print_}"')
            continue
        if key.endswith("_lowest_ms") or key.endswith("_raised_mhz"):
            continue  # a split by the device's clock level is taken by hand (ADR 2402), not here
        if value == "none":
            text.append(stripped)
            continue
        # The template's rows name a document by its path; the turn gate's lines by its file name.
        row = identity if header != "[[page]]" else os.path.basename(identity.split(" ")[0]) + " " + identity.split(" ")[1]
        if (row, key) in special:
            samples, band, how = special[(row, key)]
        else:
            samples = values.get((row, key), [])
            band, how = (rule(samples), "ADR 1969's rule") if samples else (None, "")
        if band is None:
            missing.append(f"{row or 'the file'}: {key}")
            continue
        shown = ", ".join(f"{s:g}" for s in samples)
        text.append(f"# {how} over {len(samples)} kept reading(s): {shown}")
        text.append(f"{key} = {fmt(band[0])} .. {fmt(band[1])}")
    head = "# This machine's clock bands, written by tools/bands.sh retake (ADR 2662).\n"
    return head + "\n".join(text).lstrip("\n") + "\n", missing


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--logs", required=True)
    parser.add_argument("--template", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    print_ = open(os.path.join(args.logs, "fingerprint")).read().strip()
    missing, files = [], {}
    taken = launch(args.logs)
    values, rate, latency = taken
    special = {}
    if rate:
        special[("", "io_ms")] = (rate, io_shape(rate), "ADR 2660's disk shape")
    if latency:
        special[("", "io_latency_ms")] = (latency, io_shape(latency), "ADR 2660's disk shape")
    for (row, field), samples in list(values.items()):
        if field == "first_page_ms" and samples:
            special[(row, "cold_first_page_ms")] = (samples, rule(samples), "ADR 2158: the row's warm band, ADR 1969's rule")
    for gate, (vals, spec) in {
        "launch-path": (values, special),
        "turn-path": (turn(args.logs), {}),
        "toolkit-launch": (toolkit(args.logs), {}),
    }.items():
        if not vals:
            missing.append(f"{gate}: no run")
            continue
        text, lacking = render(os.path.join(args.template, f"{gate}.toml"), vals, spec, print_)
        files[gate] = text
        missing += [f"{gate}: {m}" for m in lacking]
    if missing:
        # Nothing is written unless everything is: a directory the harness finds is a whole one.
        print("bands: not written, no kept reading of " + "; ".join(missing), file=sys.stderr)
        sys.exit(1)
    os.makedirs(args.out, exist_ok=True)
    for gate, text in files.items():
        with open(os.path.join(args.out, f"{gate}.toml"), "w") as f:
            f.write(text)


if __name__ == "__main__":
    main()
