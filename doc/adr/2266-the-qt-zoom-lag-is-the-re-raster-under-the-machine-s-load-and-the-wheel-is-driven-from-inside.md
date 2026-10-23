# 2266 — The Qt zoom lag on the owner's display is the re-raster under the machine's load, and the wheel is driven from inside the program

Status: accepted. Session 1716, 2026-10-23.

## Context

ADR 2251 measured `quorra-qt`'s Control+wheel chain on an Xvfb with `--cpu` and found no lag, and
left the owner's report to the device lane or to the compositor. A Wayland compositor admits no
synthetic pointer input from an unprivileged client, and `tools/zoom-probe-drive.py` reached the
owner's windows only through Xwayland, so no trial had driven the window the owner uses.

## Decision

1. **The wheel is driven from inside the program.** With `PDFVIEWER_ZOOM_PROBE=1`,
   `PDFVIEWER_ZOOM_DRIVE=notches=<n>,every=<ms>,after=<ms>[,angle=<eighths>][,x=<f>,y=<f>]` makes
   `PageArea` post Control+wheel `QWheelEvent`s to itself, starting `after` ms past its first frame;
   `angle=15` sends a notch as a high-resolution wheel does, a negative `notches` zooms out. The
   events go through the same handler, accumulator and dispatch as the compositor's; they bypass
   only the compositor's delivery, which is exactly the boundary this separates. A malformed value
   is refused by name. `tools/zoom-probe-drive.py` keeps its XTest drive as an explicit `--x11` arm
   and refuses without it.
2. **The attribution, from one loop run** on the owner's session (Wayland, `eDP-1` 2880×1800 at
   1.6 and 120 Hz, AMD Radeon 890M on RADV), every window activated by its pid through a KWin
   script and confirmed the active one before its notches, the ISO document at page 20, eight
   notches 133 ms apart. Per notch, from the wheel callback:
   - **felt** (the first paint, the scaled stand-in of ADR 1843): 3.6 to 8.9 ms in the quiet run,
     11 to 40 ms in the others; its terms are the core's dispatch (0.1 to 9.7 ms), the copy into a
     `QImage` (0.1 to 10.0 ms, up to 19.6 MB), the wait for the paint (0.4 to 13.2 ms) and the
     scaled paint (1.3 to 17.4 ms);
   - **settled** (the first paint at the new magnification): 11 to 23 ms in the quiet runs, 26 to
     111 ms in the others; 36 ms after the last of eight notches 16 ms apart;
   - **the dominant term is the re-raster per notch**, and in it the processor's half: on the
     device lane the scene encode (1.3 to 70.3 ms) and the read back (2.1 to 16.2 ms) against an
     execute of 0.2 to 0.4 ms, and past the window the processor's window raster (9.1 to 72.0 ms).
     The same binary on the same page read encodes of 6.2 and 52.7 ms in two arms minutes apart,
     with the one-minute load average at 30 to 44 on 24 threads from the agent rounds' bare builds.
3. **No product change for the lag.** The hypothesis that a notch waits for a whole re-raster
   before anything shows did not hold: the stand-in is already shown within a frame or a few.
   What stretches is the processor's share of the re-raster under load, which is `raster`'s encode
   and the machine's, not the Qt window. The one Qt-owned term, the per-notch copy of an unchanged
   raster into a new `QImage`, is at most 10.0 ms at load and 2.8 ms quiet; removed entirely it
   leaves the loaded felt figure above a 120 Hz frame (8.3 ms), so it is not built on this ADR.

4. **A window on the owner's display is captured by its identity**: `tools/zoom-probe-activate.py`'s
   KWin script activates it by pid and reports the active window's pid back over D-Bus, and only then does `spectacle` take the
   active window. The GTK window's page so captured matched its settled source 1:1, every byte. KWin's
   window capture is not exact by construction: on a nested KWin whose screen showed a window 1:1 it
   read 96.73 % identical, so an exact reading through it is a fact about that capture, and the
   screen comparison stays the nested compositor's (ADR 2251).

## Costs

The drive is about 70 lines of C++ in the release binary behind the probe's own variable, read once.
A posted event is not spontaneous and carries no compositor timestamp, so delivery latency is not
measured by it. The figures are one session's: a quiet machine would read nearer the quiet run.
