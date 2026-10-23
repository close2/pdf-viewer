# 2274 — A wheel's notch is a notch in both windows, a click zooms a step and less than a click zooms in proportion

Status: implemented. Session 1720, 2026-10-23.

## Context

The owner asked whether the wheel behaves differently under Qt, with coarser steps. It did. On a
nested KWin 6.7.5 at 1.6, with the pointer, Control and the wheel sent through KWin's own
`org.kde.KWin.EIS.RemoteDesktop` (libei), HEAD's `quorra-qt` received a notch as an angle of 120
eighths **and a pixel delta of 15** (source `MouseEventNotSynthesized`, no phase), and its handler
preferred the pixels: 15 logical pixels a notch where `quorra-gtk` moved 48, and under Control 15
pixels at `ZOOM_PIXELS`' 50, one zoom step every three and a third notches where GTK took one. A
high-resolution wheel's eighths moved Qt's page 1 to 2 pixels each; GTK's `DISCRETE` controller
moved it a whole notch on the fourth and the twelfth. The cause is the toolkit's: QtWayland 6.10
withheld the pixel delta for a wheel source (`hasPixelDelta`), and 6.12, the version here, reports
the compositor's axis value for every source. Every earlier trial ran X11, where a wheel carries no
pixel delta, or posted events without one (ADR 2266), so none could see it.

The owner's own receipts, taken with his mouse in `quorra-qt` under `PDFVIEWER_ZOOM_PROBE`, are the
same case: 46 Control+wheel events, 32 in and 14 out, every one `source` 0 and no phase, every one
an angle of ±120 — whole notches, not a high-resolution wheel — and a pixel delta of ±15, and every
branch record `zoom` with `pixels` true and an amount of 15. That is 0.3 of a step a notch: three
notches in four did nothing at all, which is what he described as the wheel "simply ignored". No
outcome record in them says the host was busy. His words, 2026-10-23: "I like the idea of having
butter-smooth zooming if the mouse supports it, so please only change it to 'click'-event based
zoomings if necessary."

## Decision

1. **One rule picks the unit**: `viewer_host::WheelTravel::of` counts a wheel in notches whatever
   else the event carries, and a touchpad's gesture in pixels. Qt marks a gesture by a scroll phase
   or by `MouseEventSynthesizedBySystem`, which QtWayland gives the finger and continuous sources;
   the C++ handler asks the rule across the bridge (`wheel_in_pixels`).
2. **A scroll counts whole notches**: `viewer_host::DiscreteWheel` is GTK 4's discrete count taken
   whole (the wheel-unit branch of `gtkeventcontrollerscroll.c` in 4.24.1): carried, spent once it
   reaches half a notch, one at least, and thrown away on a reversal. Qt's scroll reads it; GTK's
   `DISCRETE` controller is the same count. A choice to agree with the other window's toolkit,
   since the standard says nothing about a wheel.
3. **One distance**: `viewer_host::SCROLL_NOTCH`, 48 logical pixels, read by both windows; Qt's
   `kScrollStep` is gone and the distance is computed on the Rust side.
4. **A zoom is a step a click and a proportion between** (`viewer_host::ZoomTravel`), built
   because the owner asked for it where a device reports less than a click. A whole notch is one
   `Zoom::In` or `Zoom::Out`, exactly as before for a notched mouse like his; a part of one — a
   high-resolution wheel's eighth — is a `Zoom::Scale` of the magnification showing by
   `viewer_core::ZOOM_STEP` to the power of the part, about the pointer, at once; a touchpad's
   pixels are the same proportion at `ZOOM_PIXELS` a step. Eight eighths are one click's
   magnification, and nothing is carried, so nothing is lost or held back. GTK zooms on a second
   scroll controller without `DISCRETE`, since its one `DISCRETE` controller would hand over only
   whole notches; GTK runs every controller of a widget on each event, and the two each take their
   half (Control or not). `ZOOM_STEP` is public so that a host states the proportion in the
   core's own step.

Read off the output by `tools/zoom-probe-wayland.py --wheel` at 1.6, so a window with no receipt is
measured as it is: HEAD's Qt window made one zoom step of four Control notches, scrolled 24 device
pixels a notch and moved on every one of sixteen eighths; this tree's makes four, scrolls 76 and
moves on the fourth and the twelfth — what GTK's window does at HEAD and here (four, 77, the fourth
and the twelfth; the square's centre is read to the pixel). Sixteen Control eighths now magnify
each window by sixteen equal factors of 1.0283, two clicks' worth.

## Costs of the proportion

A high-resolution wheel asks for a re-raster per event rather than per click: on the nested KWin
Qt queued 35 frames for sixteen eighths against 10 for four clicks, about the same per event. ADR
1843's stand-in shows each step's scaled frame before its raster lands, and ADR 2266 found the
re-raster the dominant term under load, so a fast spin of such a wheel may show the stand-in longer.
A notched wheel's cost is unchanged.

## Not decided here

A touchpad: GTK's `DISCRETE` controller turns ten surface pixels into a notch of 48, Qt now counts
the finger's pixels as before. EIS states no axis source, so a finger's events cannot be sent to the
nested compositor, and this is left as it was. Under X11, Qt 6.12's XInput 2 path states a pixel
delta only for a driver whose increment exceeds 15 and libinput's is 15, so a touchpad there reaches
both windows as fractions of a notch and both count them whole, as GTK's controller always has. KWin's
scroll speed scales a wheel's angle and its axis value alike, so it moves both windows alike.

## Costs

One bridge function and a carry in Qt's host; about 150 lines of `viewer-host` with their tests. The
probe's receipts (`wheel-entry` for GTK, `scroll-dispatch` for Qt) are written only under
`PDFVIEWER_ZOOM_PROBE`.
