# 2251 — A display fact is measured on a nested KWin at the owner's scale, not on Xwayland

Status: accepted. Session 1708, 2026-10-23.

## Context

Three batches of instrumentation (ADRs 2214–2241) drove X11 windows: `tools/zoom-probe-drive.py`
forces `GDK_BACKEND=x11` and `QT_QPA_PLATFORM=xcb` so that XTest can move the pointer. The owner's
windows run on Wayland (ADR 2250), where the toolkits read a different scale, so those trials could
not see the GTK blur, and a clean X11 result was a sentence about Xwayland.

## Decision

A question about what the owner's display shows is first reproduced on a **nested, headless
KWin** — `kwin_wayland --virtual` on a private socket, runtime directory and session bus — with
the output set by `kscreen-doctor output.Virtual-0.scale.<s>`, the same path Plasma's settings take.
The window runs with no `DISPLAY`; the output's pixels come from KWin's own
`org.kde.KWin.ScreenShot2.CaptureScreen` at native resolution, permitted for the nested instance by
`KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1`. `tools/zoom-probe-wayland.py` is that instrument for the
GTK window: per scale it compares the settled source dump (ADR 2240) with the output, and exits
non-zero unless every scale is 1:1. It is calibrated both ways: HEAD reads `resampled` at the five
fractional scales and exact at 1 and 2, and ADR 2250's tree reads exact at all seven.

`kwin_wayland --virtual --scale <s>` is not the owner's construction: it multiplies the mode and
leaves the output's scale at 1 (logical 2560 for `--width 1600 --scale 1.6`), so GTK reads 1.

## The Qt wheel chain, measured before anything was touched

On an owned Xvfb at `QT_SCALE_FACTOR=1.6`, `--cpu`, the ISO document at page 20, eight Control+wheel
notches about 133 ms apart: each notch re-rasters the page — nine rasters of 8.25 to 16.06 ms, the
dominant term — and the settled frame is painted 10.1 to 25.4 ms after the wheel callback; the
scaled stand-in is painted 3.3 to 8.0 ms after it; one paint costs 0.23 to 2.63 ms, and each frame
is painted twice. Twelve notches 12 ms apart drew 4 rasters, a stand-in within 2.2 to 13.7 ms of
every notch, and the settled frame 20.7 ms after the last. On this machine's processor the chain
shows no lag a person could see; the owner's lag is then in what this instrument cannot reach — the
device lane his window takes by default, or the wheel's own delivery on Wayland (a high-resolution
wheel can send parts of a notch, which `viewer_host::ZoomWheel` accumulates). The probe's
`PDFVIEWER_ZOOM_PROBE` read is taken once per process, as GTK's and the C++ side's already were; it
was read on every wheel step.

## Costs

The nested compositor needs `kwin_wayland`, `kscreen-doctor` and python-dbus, all present on the
owner's Arch machine; elsewhere the probe refuses by name. Synthetic input is not available on it,
so a zoom, a drift or a latency is still Xvfb's or the owner's loop's.
