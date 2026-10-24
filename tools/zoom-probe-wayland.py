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

`--drift` instead drives the zoom anchor with real compositor input (ADR 2275): the nested KWin is
this probe's own, and its `org.kde.KWin.EIS.RemoteDesktop` hands this process a libei sender, so
the pointer, Control and the wheel reach the window through the compositor exactly as a person's
do (`libei.so.1` is then needed too). The window shows a page this probe writes — one red square —
the pointer is put on the square, and `--notches` Control+wheel notches go in and as many come
back out; after each, the settled output's square is found again and its offset from the pointer
is the drift, in device pixels. `--binary` is either window, `--pdf` is not read.

`--touchpad` drives the same page with the travel a touchpad reports — smooth axis deltas in logical
pixels, `ei_device_scroll_delta`, which KWin hands a window as a continuous axis with no discrete
value — and reads off the output how far twelve events of ten pixels moved the page (ADR 2291). It
passes where the page moved as far as the travel, ten logical pixels an event, rather than the
notch of 48 GTK's `DISCRETE` controller counts ten such pixels into. EIS states no axis source, so
this is a continuous source and not a finger: the toolkits' finger and continuous paths are one
path in both windows (`gdk::ScrollUnit::Surface`; Qt's `MouseEventSynthesizedBySystem`), and what is
not driven is the finger's own stop and kinetic events, which neither window reads.

`--wheel` drives the same page through the same input and reads the wheel's count off the output
alone, so a window built before any receipt existed is measured as it is (ADR 2274): how many zoom
steps four Control+wheel notches make (the square's area grows by 1.25 squared a step), how far one
notch scrolls the page, and after which of sixteen eighths of a notch — what a high-resolution
wheel sends — the page moved. It passes where the window counts as `viewer_host::DiscreteWheel`
and `viewer_host::SCROLL_NOTCH` say: four steps, 48 logical pixels a notch, a move on the
fourth and the twelfth eighth.
"""
import math
import ctypes
import select
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


class Eis:
    """A libei sender on the nested KWin's EIS socket, through ctypes: the pointer, a key, a wheel."""

    POINTER_ABSOLUTE, KEYBOARD, SCROLL = 1 << 1, 1 << 2, 1 << 4
    SEAT_ADDED, DEVICE_ADDED, DEVICE_RESUMED, DISCONNECT = 3, 5, 8, 2

    def __init__(self):
        import dbus

        self.lib = ctypes.CDLL("libei.so.1")
        lib = self.lib
        for name, result in (("ei_new_sender", ctypes.c_void_p), ("ei_get_event", ctypes.c_void_p),
                             ("ei_event_get_seat", ctypes.c_void_p), ("ei_event_get_device", ctypes.c_void_p),
                             ("ei_device_ref", ctypes.c_void_p), ("ei_now", ctypes.c_uint64),
                             ("ei_event_get_type", ctypes.c_int), ("ei_get_fd", ctypes.c_int),
                             ("ei_setup_backend_fd", ctypes.c_int), ("ei_device_has_capability", ctypes.c_bool)):
            getattr(lib, name).restype = result
        lib.ei_event_get_type.argtypes = [ctypes.c_void_p]
        lib.ei_event_get_seat.argtypes = lib.ei_event_get_device.argtypes = [ctypes.c_void_p]
        lib.ei_event_unref.argtypes = lib.ei_device_ref.argtypes = [ctypes.c_void_p]
        lib.ei_device_has_capability.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.ei_device_start_emulating.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
        lib.ei_device_pointer_motion_absolute.argtypes = [ctypes.c_void_p, ctypes.c_double, ctypes.c_double]
        lib.ei_device_keyboard_key.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_bool]
        lib.ei_device_scroll_discrete.argtypes = [ctypes.c_void_p, ctypes.c_int32, ctypes.c_int32]
        lib.ei_device_scroll_delta.argtypes = [ctypes.c_void_p, ctypes.c_double, ctypes.c_double]
        lib.ei_device_frame.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
        lib.ei_get_fd.argtypes = lib.ei_dispatch.argtypes = lib.ei_now.argtypes = [ctypes.c_void_p]
        lib.ei_get_event.argtypes = [ctypes.c_void_p]
        lib.ei_setup_backend_fd.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.ei_configure_name.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        proxy = dbus.SessionBus().get_object("org.kde.KWin", "/org/kde/KWin/EIS/RemoteDesktop")
        handed, _cookie = dbus.Interface(proxy, "org.kde.KWin.EIS.RemoteDesktop").connectToEIS(63)
        self.ei = lib.ei_new_sender(None)
        lib.ei_configure_name(self.ei, b"zoom-probe-wayland")
        if lib.ei_setup_backend_fd(self.ei, handed.take()) != 0:
            raise RuntimeError("libei refused KWin's EIS descriptor")
        self.devices, self.resumed, self.sequence = [], 0, 1
        for _ in range(100):
            self.pump(50)
            if self.resumed >= 3:
                break
        if not self.resumed:
            raise RuntimeError("the nested KWin resumed no EIS device")

    def pump(self, ms):
        select.select([self.lib.ei_get_fd(self.ei)], [], [], ms / 1000)
        self.lib.ei_dispatch(self.ei)
        while event := self.lib.ei_get_event(self.ei):
            kind = self.lib.ei_event_get_type(event)
            if kind == self.SEAT_ADDED:
                self.lib.ei_seat_bind_capabilities(ctypes.c_void_p(self.lib.ei_event_get_seat(event)),
                                                   *[ctypes.c_int(1 << bit) for bit in range(6)], None)
            elif kind == self.DEVICE_ADDED:
                self.devices.append(self.lib.ei_device_ref(self.lib.ei_event_get_device(event)))
            elif kind == self.DEVICE_RESUMED:
                self.lib.ei_device_start_emulating(self.lib.ei_event_get_device(event), self.sequence)
                self.sequence += 1
                self.resumed += 1
            elif kind == self.DISCONNECT:
                raise RuntimeError("the nested KWin disconnected the EIS sender")
            self.lib.ei_event_unref(event)

    def _device(self, capability):
        for device in self.devices:
            if self.lib.ei_device_has_capability(device, capability):
                return device
        raise RuntimeError(f"no EIS device with capability {capability}")

    def _frame(self, device):
        self.lib.ei_device_frame(device, self.lib.ei_now(self.ei))
        self.pump(0)

    def move(self, x, y):
        """The pointer to `(x, y)` in the compositor's logical pixels."""
        device = self._device(self.POINTER_ABSOLUTE)
        self.lib.ei_device_pointer_motion_absolute(device, x, y)
        self._frame(device)

    def key(self, code, down):
        device = self._device(self.KEYBOARD)
        self.lib.ei_device_keyboard_key(device, code, down)
        self._frame(device)

    def wheel(self, value120):
        """One wheel event, in `wl_pointer.axis_value120`'s units: 120 a notch towards the person."""
        device = self._device(self.SCROLL)
        self.lib.ei_device_scroll_discrete(device, 0, value120)
        self._frame(device)


def marker_pdf(path):
    """One US Letter page, white but for a red 2 pt square centred at (200, 500)."""
    content = b"1 0 0 rg 199 499 2 2 re f"
    bodies = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
              b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>",
              b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream"]
    data, offsets = bytearray(b"%PDF-1.7\n"), []
    for number, body in enumerate(bodies, 1):
        offsets.append(len(data))
        data += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    table = len(data)
    data += b"xref\n0 %d\n0000000000 65535 f \n" % (len(bodies) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offsets)
    data += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(bodies) + 1, table)
    Path(path).write_bytes(bytes(data))


def red_centroid(shot):
    """The red square's centre in the output, in device pixels, and how many pixels it covers."""
    red = (shot[..., 0] > 200) & (shot[..., 1] < 90) & (shot[..., 2] < 90)
    rows, columns = np.nonzero(red)
    if rows.size == 0:
        return None
    return float(columns.mean()) + 0.5, float(rows.mean()) + 0.5, int(rows.size)


def settled(output_name, deadline=8.0):
    """The output once two captures 300 ms apart agree, or the last one at the deadline."""
    end = time.monotonic() + deadline
    shot = capture(output_name)
    while time.monotonic() < end:
        time.sleep(0.3)
        again = capture(output_name)
        if np.array_equal(again, shot):
            return again
        shot = again
    return shot


def placements(log):
    """Each `zoom-geometry` receipt a window wrote after a zoom step: where the core put the page."""
    found = []
    for line in log.read_text(errors="replace").splitlines():
        if "zoom-geometry " in line:
            found.append(json.loads(line.split("zoom-geometry ", 1)[1]))
    return found


def drifted(config, client, scale):
    """The point of the page under the pointer, followed through `--notches` steps in and back.

    Three numbers a step: `dx`, `dy`, the marker's offset from the pointer on the output, which is
    what a person sees; `core_dx`, `core_dy`, the offset the core's own placement predicts, from the
    window's `zoom-geometry` receipt, which is the anchor's arithmetic alone; and `shown_dx`,
    `shown_dy`, the output against that prediction, less the first step's, which is the window's
    placement of the frame alone."""
    pixels = config.output / f"drift-{config.binary.name}-{scale:g}"
    pixels.mkdir()
    document = pixels / "marker.pdf"
    marker_pdf(document)
    environment = dict(client, PDFVIEWER_ZOOM_PROBE="1", QT_QPA_PLATFORM="wayland", GDK_BACKEND="wayland")
    viewer = subprocess.Popen([str(config.binary.resolve()), str(document.resolve())], env=environment,
                              stdout=(pixels / "viewer.log").open("wb"), stderr=subprocess.STDOUT,
                              start_new_session=True)
    steps = []
    try:
        found = None
        for _ in range(60):
            time.sleep(0.5)
            if viewer.poll() is not None:
                raise RuntimeError(f"viewer exited {viewer.returncode}")
            found = red_centroid(settled("Virtual-0", 3.0))
            if found:
                break
        if found is None:
            return dict(binary=config.binary.name, scale=scale, verdict="no marker on the output")
        sender = Eis()
        # A whole logical pixel, which is where a compositor's pointer stands after an absolute move.
        pointer = (round(found[0] / scale), round(found[1] / scale))
        sender.move(pointer[0] + 4, pointer[1] + 4)  # a motion GTK reads, then the place itself
        sender.pump(200)
        sender.move(*pointer)
        sender.pump(300)
        at = (pointer[0] * scale, pointer[1] * scale)
        sender.key(29, True)  # KEY_LEFTCTRL
        sender.pump(200)
        plan = [-120] * config.notches + [120] * config.notches
        for index, value120 in enumerate([0] + plan):
            if value120:
                sender.wheel(value120)
            shot = settled("Virtual-0")
            centre = red_centroid(shot)
            level = sum(1 if v < 0 else -1 for v in plan[:index])
            row = dict(step=index, level=level)
            if centre is None:
                row["marker"] = "not on the output"
            else:
                row.update(dx=round(centre[0] - at[0], 3), dy=round(centre[1] - at[1], 3), covered=centre[2])
            placed = placements(pixels / "viewer.log")
            if index and len(placed) >= index:
                last = placed[index - 1]
                # The marker is centred on (200, 500) in default user space, y up (`marker_pdf`).
                predicted = (last["origin_x"] + 200 * last["scale"],
                             last["origin_y"] + (last["page_height"] - 500) * last["scale"])
                row.update(core_dx=round(predicted[0] - last["anchor_x"], 3),
                           core_dy=round(predicted[1] - last["anchor_y"], 3), scale=last["scale"])
                if centre is not None:
                    row["_shown"] = (centre[0] - predicted[0], centre[1] - predicted[1])
            steps.append(row)
            write_png(pixels / f"step-{index:02d}.png", shot)
            print(json.dumps(row), flush=True)
        sender.key(29, False)
        sender.pump(100)
    finally:
        os.killpg(viewer.pid, signal.SIGTERM)
        viewer.wait(timeout=10)
    shown = [row for row in steps if "_shown" in row]
    for row in shown:
        first = shown[0]["_shown"]
        row["shown_dx"], row["shown_dy"] = (round(row["_shown"][0] - first[0], 3), round(row["_shown"][1] - first[1], 3))
    for row in steps:
        row.pop("_shown", None)
    # The verdict is read where the page is far wider and taller than the viewport, so nothing
    # centres it: the marker's offset from the anchor there, in user space units, against what one
    # device pixel is worth on the fitted page, which is how far from the marker a pointer standing
    # on a whole logical pixel can be put. And ten steps out have to land where the gesture began.
    held = [row for row in steps if "core_dx" in row]
    deepest = max(held, key=lambda row: row["level"], default=None)
    fitted = held[0]["scale"] / 1.25 if held else None
    offset = None if deepest is None else max(abs(deepest["core_dx"]), abs(deepest["core_dy"])) / deepest["scale"]
    first, back = steps[0], steps[-1]
    returned = "dx" in first and "dx" in back and max(abs(back["dx"] - first["dx"]), abs(back["dy"] - first["dy"])) <= 1.0
    stays = offset is not None and offset <= 1 / fitted and returned
    return dict(binary=config.binary.name, scale=scale, pointer_device=[round(v, 3) for v in at], steps=steps,
                deepest_offset_units=None if offset is None else round(offset, 4),
                one_fitted_pixel_units=None if fitted is None else round(1 / fitted, 4), returned=returned,
                verdict="the point stays" if stays else "the point moves")


def finger(self, pixels):
    """One smooth axis event of `pixels` logical pixels towards the person, as a touchpad states
    its travel; EIS has no axis source to name a finger with."""
    device = self._device(self.SCROLL)
    self.lib.ei_device_scroll_delta(device, 0.0, pixels)
    self._frame(device)


Eis.finger = finger


def touchpad_travel(config, client, scale):
    """How far twelve smooth axis events of ten logical pixels move one window's page (see
    `--touchpad`)."""
    pixels = config.output / f"touchpad-{config.binary.name}-{scale:g}"
    pixels.mkdir()
    document = pixels / "marker.pdf"
    marker_pdf(document)
    environment = dict(client, QT_QPA_PLATFORM="wayland", GDK_BACKEND="wayland")
    viewer = subprocess.Popen([str(config.binary.resolve()), str(document.resolve())], env=environment,
                              stdout=(pixels / "viewer.log").open("wb"), stderr=subprocess.STDOUT,
                              start_new_session=True)
    events, each = 12, 10.0
    try:
        found = None
        for _ in range(60):
            time.sleep(0.5)
            if viewer.poll() is not None:
                raise RuntimeError(f"viewer exited {viewer.returncode}")
            found = red_centroid(settled("Virtual-0", 3.0))
            if found:
                break
        if found is None:
            return dict(binary=config.binary.name, scale=scale, verdict="no marker on the output")
        sender = Eis()
        pointer = (round(found[0] / scale), round(found[1] / scale))
        sender.move(pointer[0] + 4, pointer[1] + 4)
        sender.pump(200)
        sender.move(*pointer)
        sender.pump(300)
        # Two notches in first, so that the page is taller than the window and has room to move.
        sender.key(29, True)
        sender.pump(200)
        for _ in range(2):
            sender.wheel(-120)
            sender.pump(100)
        sender.key(29, False)
        sender.pump(200)
        before = red_centroid(settled("Virtual-0"))
        moves = []
        last = before
        for _ in range(events):
            sender.finger(each)
            sender.pump(60)
            now = red_centroid(settled("Virtual-0", 3.0))
            if now is None or last is None:
                break
            moves.append(round(last[1] - now[1], 2))
            last = now
    finally:
        os.killpg(viewer.pid, signal.SIGTERM)
        viewer.wait(timeout=10)
    moved = None if before is None or last is None else round(before[1] - last[1], 2)
    wanted = events * each * scale
    passes = moved is not None and abs(moved - wanted) <= 2.0
    return dict(binary=config.binary.name, scale=scale, events=events, logical_pixels_each=each,
                moved_device_pixels=moved, wanted_device_pixels=round(wanted, 2), each_move=moves,
                verdict="one touchpad" if passes else "another touchpad")


def wheel_count(config, client, scale):
    """One window's count of a wheel, read off the output (see `--wheel`)."""
    pixels = config.output / f"wheel-{config.binary.name}-{scale:g}"
    pixels.mkdir()
    document = pixels / "marker.pdf"
    marker_pdf(document)
    environment = dict(client, QT_QPA_PLATFORM="wayland", GDK_BACKEND="wayland")
    viewer = subprocess.Popen([str(config.binary.resolve()), str(document.resolve())], env=environment,
                              stdout=(pixels / "viewer.log").open("wb"), stderr=subprocess.STDOUT,
                              start_new_session=True)
    try:
        found = None
        for _ in range(60):
            time.sleep(0.5)
            if viewer.poll() is not None:
                raise RuntimeError(f"viewer exited {viewer.returncode}")
            found = red_centroid(settled("Virtual-0", 3.0))
            if found:
                break
        if found is None:
            return dict(binary=config.binary.name, scale=scale, verdict="no marker on the output")
        sender = Eis()
        pointer = (round(found[0] / scale), round(found[1] / scale))
        sender.move(pointer[0] + 4, pointer[1] + 4)
        sender.pump(200)
        sender.move(*pointer)
        sender.pump(300)
        sender.key(29, True)
        sender.pump(200)
        # Four notches in first, so that the square is large enough for its area to count steps,
        # then four more measured; away from the person is in.
        for _ in range(4):
            sender.wheel(-120)
            sender.pump(100)
        start = red_centroid(settled("Virtual-0"))
        for _ in range(4):
            sender.wheel(-120)
            sender.pump(100)
        four = red_centroid(settled("Virtual-0"))
        steps = round(math.log(four[2] / start[2]) / math.log(1.5625)) if four and start else None
        sender.key(29, False)
        sender.pump(200)
        before = red_centroid(settled("Virtual-0"))
        sender.wheel(120)  # towards the person: the page moves up
        after = red_centroid(settled("Virtual-0"))
        notch = None if before is None or after is None else round(before[1] - after[1], 2)
        moved_at, last = [], after
        for eighth in range(1, 17):
            sender.wheel(15)
            now = red_centroid(settled("Virtual-0", 3.0))
            if now is None or last is None:
                break
            if abs(now[1] - last[1]) > 0.5:
                moved_at.append(eighth)
            last = now
    finally:
        os.killpg(viewer.pid, signal.SIGTERM)
        viewer.wait(timeout=10)
    wanted = 48 * scale
    passes = steps == 4 and notch is not None and abs(notch - wanted) <= 1.0 and moved_at == [4, 12]
    return dict(binary=config.binary.name, scale=scale, zoom_steps_of_four_notches=steps, notch_device_pixels=notch,
                notch_wanted=round(wanted, 2), eighths_that_moved=moved_at,
                verdict="one wheel" if passes else "another wheel")


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
    parser.add_argument("--pdf", type=Path, help="a one-page document: the dump wants a single frame (not with --drift)")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scale", type=float, action="append", help="default: 1 1.25 1.5 1.6 1.7 1.75 2")
    parser.add_argument("--rescale", metavar="FROM:TO",
                        help="instead: one window opened at FROM, the output set to TO under it; passes only if "
                             "no resize reached the core and the page is 1:1 at both, which is the surface's "
                             "scale hook alone (ADR 2250)")
    parser.add_argument("--drift", action="store_true",
                        help="instead: the zoom anchor driven in and back out through KWin's own EIS input, "
                             "at each --scale (default 1.6), the marker's offset from the pointer after each notch")
    parser.add_argument("--notches", type=int, default=10, help="with --drift: steps in, then as many out (10: 9.3x)")
    parser.add_argument("--touchpad", action="store_true",
                        help="instead: twelve smooth axis events of ten logical pixels, the page's travel read off the output")
    parser.add_argument("--wheel", action="store_true",
                        help="instead: one notch's zoom steps and scroll and sixteen eighths' moves, read off the output")
    config = parser.parse_args()
    if not (config.drift or config.wheel or config.touchpad) and config.pdf is None:
        parser.error("--pdf is needed unless --drift, --wheel or --touchpad")
    for tool in ("kwin_wayland", "kscreen-doctor", "dbus-run-session"):
        if shutil.which(tool) is None:
            parser.error(f"{tool} is not installed; this probe needs a nested KWin")
    try:
        import dbus  # noqa: F401
    except ImportError:
        parser.error("python-dbus is not installed; the output is read through KWin's ScreenShot2")
    scales = config.scale or ([1.6] if config.drift or config.wheel or config.touchpad
                              else [1, 1.25, 1.5, 1.6, 1.7, 1.75, 2])
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
        if config.drift:
            for scale in scales:
                subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{scale:g}"], env=client, check=True,
                               capture_output=True, timeout=20)
                results.append(drifted(config, client, scale))
            scales = []
        if config.touchpad:
            for scale in scales:
                subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{scale:g}"], env=client, check=True,
                               capture_output=True, timeout=20)
                results.append(touchpad_travel(config, client, scale))
                print(json.dumps(results[-1]), flush=True)
            scales = []
        if config.wheel:
            for scale in scales:
                subprocess.run(["kscreen-doctor", f"output.Virtual-0.scale.{scale:g}"], env=client, check=True,
                               capture_output=True, timeout=20)
                results.append(wheel_count(config, client, scale))
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
    passed = (("the point stays",) if config.drift else ("one wheel",) if config.wheel
              else ("one touchpad",) if config.touchpad else ("1:1 exact",))
    return 0 if results and all(r.get("verdict") in passed for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
