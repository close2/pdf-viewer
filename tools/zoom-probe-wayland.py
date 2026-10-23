#!/usr/bin/env python3
"""Native-Wayland source-to-output map for the GTK window at fractional output scales (ADR 2251).

A nested, headless KWin (`kwin_wayland --virtual`) on a private socket and session bus is set to
each scale with `kscreen-doctor`, exactly as Plasma sets a fractional output; `quorra-gtk` runs on
it with no X11 display at all, dumps its settled page raster (`PDFVIEWER_ZOOM_CAPTURE_DIR`, ADR
2240), and KWin's own `ScreenShot2` interface hands back the output's pixels. The verdict per scale
is the independent map of ADR 2241 taken one step further, to the compositor: the page's shown
width against the source's, and where they agree, the byte comparison of the whole page.

Run as `python3 -I -B tools/zoom-probe-wayland.py --binary <quorra-gtk> --pdf <one-page.pdf>
--output <new dir> [--scale 1.6 ...]`; `--rescale 1.6:1.25` instead changes the output's scale under
one open window and passes only where no resize reached the core, which is the scale hook alone. It judges no latency and no zoom input — a Wayland
compositor admits no synthetic input from an unprivileged client — and needs `kwin_wayland`,
`kscreen-doctor`, `dbus-run-session` and python-dbus, refusing at once where one is missing.
"""
import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import threading
import time
import zlib
import struct
from pathlib import Path

import numpy as np

DEADLINE = 600


def write_png(path, rgb):
    height, width, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[row].tobytes() for row in range(height))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def capture(output_name):
    """The output's pixels through KWin's ScreenShot2, at the output's native resolution."""
    import dbus  # imported here so that a missing module is the refusal below, not a traceback

    proxy = dbus.SessionBus().get_object("org.kde.KWin", "/org/kde/KWin/ScreenShot2")
    reader, writer = os.pipe()
    received = []

    def drain():
        with os.fdopen(reader, "rb") as source:
            received.append(source.read())

    thread = threading.Thread(target=drain)
    thread.start()
    meta = dbus.Interface(proxy, "org.kde.KWin.ScreenShot2").CaptureScreen(
        output_name, {"native-resolution": True, "include-cursor": False}, dbus.types.UnixFd(writer))
    os.close(writer)
    thread.join(timeout=20)
    width, height, stride = int(meta["width"]), int(meta["height"]), int(meta["stride"])
    if int(meta["format"]) not in (4, 5, 6):  # QImage RGB32, ARGB32, ARGB32_Premultiplied
        raise RuntimeError(f"unexpected screenshot format {meta['format']}")
    data = np.frombuffer(received[0], dtype=np.uint8)[: stride * height].reshape(height, stride)
    bgra = data[:, : width * 4].reshape(height, width, 4)
    return np.ascontiguousarray(bgra[..., [2, 1, 0]])


def page_columns(shot):
    """The page's columns in the output: the longest run of columns that are mostly near-white."""
    white = np.all(shot >= 250, axis=2)
    columns = np.where(white.sum(axis=0) > 0.4 * shot.shape[0])[0]
    if columns.size == 0:
        return None
    runs = np.split(columns, np.where(np.diff(columns) != 1)[0] + 1)
    run = max(runs, key=len)
    return int(run[0]), int(run[-1]) + 1


def verdict(source_meta, source, shot):
    alpha = source[..., 3:4].astype(np.float64) / 255
    rgb = np.round(source[..., :3] * alpha + 255 * (1 - alpha)).astype(np.int16)
    shot = shot.astype(np.int16)
    found = page_columns(shot)
    result = dict(source=[source_meta["width"], source_meta["height"]])
    if found is None:
        return dict(result, verdict="no page found in the output")
    x0, x1 = found
    result["shown_width"] = x1 - x0
    if x1 - x0 != source_meta["width"]:
        return dict(result, verdict="resampled", ratio=round((x1 - x0) / source_meta["width"], 4))
    band = rgb[min(200, rgb.shape[0] // 4):][:400]
    top = min(200, rgb.shape[0] // 4)
    best = None
    for offset in range(-shot.shape[0], shot.shape[0]):
        row = top + offset
        if row < 0 or row + band.shape[0] > shot.shape[0]:
            continue
        same = int((np.abs(shot[row:row + band.shape[0], x0:x1] - band).max(axis=2) == 0).sum())
        if best is None or same > best[0]:
            best = (same, offset)
    if best is None:
        return dict(result, verdict="page taller than the output")
    offset = best[1]
    y0, y1 = max(0, offset), min(shot.shape[0], offset + rgb.shape[0])
    difference = np.abs(shot[y0:y1, x0:x1] - rgb[y0 - offset:y1 - offset]).max(axis=2)
    return dict(result, rows=y1 - y0, identical=round(float((difference == 0).mean()), 6),
                max_difference=int(difference.max()),
                verdict="1:1 exact" if difference.max() == 0 else "shown at size, not byte-identical")


def rescaled(config, client):
    """The scale hook alone: the output's scale changed under one open window whose logical size stays,
    so a resize cannot be what re-measured the viewport; the core-viewport records count that."""
    first, second = (float(part) for part in config.rescale.split(":"))
    subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{first:g}"], env=client, check=True,
                   capture_output=True, timeout=20)
    pixels = config.output / f"rescale-{first:g}-{second:g}"
    pixels.mkdir()
    (pixels / "request").write_text("1\n")
    log = pixels / "viewer.log"
    viewer = subprocess.Popen([str(config.binary.resolve()), str(config.pdf.resolve())],
                              env=dict(client, PDFVIEWER_ZOOM_PROBE="1", PDFVIEWER_ZOOM_CAPTURE_DIR=str(pixels.resolve())),
                              stdout=log.open("wb"), stderr=subprocess.STDOUT, start_new_session=True)

    def settled(n):
        for _ in range(100):
            if (pixels / f"capture-{n}.json").exists() or viewer.poll() is not None:
                break
            time.sleep(0.2)
        time.sleep(2)  # the dump is taken as the frame is placed; its paint lands a tick later
        if not (pixels / f"capture-{n}.json").exists():
            return None
        meta = json.loads((pixels / f"capture-{n}.json").read_text())
        source = np.fromfile(pixels / f"capture-{n}.rgba", dtype=np.uint8).reshape(meta["height"], meta["width"], 4)
        shot = capture("Virtual-0")
        write_png(pixels / f"output-{n}.png", shot)
        return verdict(meta, source, shot)

    def resizes():
        return sum(1 for line in log.read_text(errors="replace").splitlines() if '"kind":"core-viewport"' in line)

    try:
        before = settled(1)
        counted = resizes()
        (pixels / "request").write_text("2\n")
        subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{second:g}"], env=client, check=True,
                       capture_output=True, timeout=20)
        after = settled(2)
        moved = resizes() - counted
    finally:
        os.killpg(viewer.pid, signal.SIGTERM)
        viewer.wait(timeout=10)
    exact = all(v is not None and v.get("verdict") == "1:1 exact" for v in (before, after))
    return dict(rescale=config.rescale, before=before, after=after, resizes_after_the_change=moved,
                verdict="1:1 exact" if exact and moved == 0 else "the hook is not what this shows")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--pdf", type=Path, required=True, help="a one-page document: the dump wants a single frame")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scale", type=float, action="append", help="default: 1 1.25 1.5 1.6 1.7 1.75 2")
    parser.add_argument("--rescale", metavar="FROM:TO",
                        help="instead: one window opened at FROM, the output set to TO under it; passes only if "
                             "no resize reached the core and the page is 1:1 at both, which is the surface's "
                             "scale hook alone (ADR 2250)")
    config = parser.parse_args()
    for tool in ("kwin_wayland", "kscreen-doctor", "dbus-run-session"):
        if shutil.which(tool) is None:
            parser.error(f"{tool} is not installed; this probe needs a nested KWin")
    try:
        import dbus  # noqa: F401
    except ImportError:
        parser.error("python-dbus is not installed; the output is read through KWin's ScreenShot2")
    scales = config.scale or [1, 1.25, 1.5, 1.6, 1.7, 1.75, 2]
    config.output.mkdir(parents=True, exist_ok=False)
    runtime = config.output / "runtime"
    runtime.mkdir(mode=0o700)
    socket = f"zoom-probe-{os.getpid()}"
    bus_file = runtime / "bus"
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(RuntimeError("finite probe deadline")))
    signal.alarm(DEADLINE)
    base = {k: v for k, v in os.environ.items() if k not in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY")}
    base.update(XDG_RUNTIME_DIR=str(runtime.resolve()), KWIN_SCREENSHOT_NO_PERMISSION_CHECKS="1")
    compositor = subprocess.Popen(
        ["dbus-run-session", "--", "sh", "-c",
         f'echo "$DBUS_SESSION_BUS_ADDRESS" > "{bus_file}"; exec kwin_wayland --virtual --no-lockscreen '
         f'--no-global-shortcuts --no-kactivities --socket {socket} --width 1600 --height 1000'],
        env=base, stdout=(config.output / "kwin.log").open("wb"), stderr=subprocess.STDOUT, start_new_session=True)
    results = []
    try:
        for _ in range(100):
            if (runtime / socket).exists() and bus_file.exists() and bus_file.read_text().strip():
                break
            time.sleep(0.2)
        else:
            raise RuntimeError("the nested compositor did not come up")
        client = dict(base, WAYLAND_DISPLAY=socket, DBUS_SESSION_BUS_ADDRESS=bus_file.read_text().strip())
        os.environ["DBUS_SESSION_BUS_ADDRESS"] = client["DBUS_SESSION_BUS_ADDRESS"]
        if config.rescale:
            results.append(rescaled(config, client))
            print(json.dumps(results[-1]), flush=True)
            scales = []
        for scale in scales:
            subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{scale:g}"], env=client, check=True,
                           capture_output=True, timeout=20)
            pixels = config.output / f"scale-{scale:g}"
            pixels.mkdir()
            (pixels / "request").write_text("1\n")
            log = (pixels / "viewer.log").open("wb")
            viewer = subprocess.Popen([str(config.binary.resolve()), str(config.pdf.resolve())],
                                      env=dict(client, PDFVIEWER_ZOOM_PROBE="1", PDFVIEWER_ZOOM_CAPTURE_DIR=str(pixels.resolve())),
                                      stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                for _ in range(100):
                    if (pixels / "capture-1.json").exists() or viewer.poll() is not None:
                        break
                    time.sleep(0.2)
                time.sleep(2)  # the dump is taken as the frame is placed; its paint lands a tick later
                if viewer.poll() is not None:
                    raise RuntimeError(f"viewer exited {viewer.returncode} at scale {scale:g}")
                if not (pixels / "capture-1.json").exists():
                    results.append(dict(scale=scale, verdict="no settled single-frame dump"))
                    continue
                shot = capture("Virtual-0")
                write_png(pixels / "output.png", shot)
            finally:
                os.killpg(viewer.pid, signal.SIGTERM)
                viewer.wait(timeout=10)
            meta = json.loads((pixels / "capture-1.json").read_text())
            source = np.fromfile(pixels / "capture-1.rgba", dtype=np.uint8).reshape(meta["height"], meta["width"], 4)
            results.append(dict(scale=scale, **verdict(meta, source, shot)))
            print(json.dumps(results[-1]), flush=True)
    finally:
        os.killpg(compositor.pid, signal.SIGTERM)
        compositor.wait(timeout=20)
        signal.alarm(0)
    (config.output / "report.json").write_text(json.dumps(results, indent=1) + "\n")
    return 0 if all(r.get("verdict") == "1:1 exact" for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
