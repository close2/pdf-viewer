# 2323 — A toolkit notch with its window ahead settles in half the time, and is still past a frame

Status: accepted — a measurement of ADR 2322's build and what remains between a notch and one
120 Hz frame; no further product change. Session 1744, 2026-10-25. Context: ADRs 2314, 2315,
2322; `doc/todo/75` section 7.

## The figures

**Headless**, the machine's own adapter (Radeon 890M, RADV) under an Xvfb of the round's own at
`QT_SCALE_FACTOR=1.6`, page area 990 × 1595: `quorra-qt` on ISO 32000-2 page 20 from 100 %,
`PDFVIEWER_ZOOM_DRIVE=notches=8,every=400,after=3000`, HEAD's build against this round's in one
`--clock` hold, four passes interleaved, each arm started once the one-minute load was under 3
(2.1 to 2.9 throughout, a seven-round batch building beside it). Settled, as ADR 2314 defines it,
ms:

| notch | pages | HEAD | with the window ahead |
|---|---|---|---|
| 1 | 20, 21 | 30.0–34.4 | 15.8–28.3 |
| 2 | 20 | 20.6–23.4 | 9.0–11.8 |
| 3 | 20 | 22.2–29.3 | 6.6–10.8 |
| 5–8 | 20, a processor window | 14.8–26.5 | 13.6–28.4 |

From the fourth notch the page is past `oversize`'s bound and the processor draws the window, as
before; those notches are the same population in both trees. One notch of the eight measured headless at
the second and third is under 8.33 ms.

**On the owner's adapter**, one loop run in its own `--clock` hold: Wayland, `eDP-1` at 1.6, the
page area 990 × 1554, each window activated by its pid and confirmed the active one before its
notches, no capture; the same drive, HEAD's build and this round's interleaved three times at
loads of 1.3 to 2.4. Settled, ms: notch 1 22.7–24.3 against 15.5–19.7; notch 2 16.4–17.8 against
9.9–11.2; notch 3 20.1–23.1 against 7.6–9.6, two of three under 8.33; notches 5 to 8, the
processor's windows, 9.9–16.0 against 8.5–15.6. The window's device job there was 5.6 to 7.4 ms
at the second and third notches, its remainder 2.9 to 3.5, and 10.5 to 14.0 ms at the first.

**Where the rest goes**, from the headless runs' trace: the window's device job is 4.7 to 8.9 ms of
which the scene 0.3 to 0.5, encode 0.3 to 0.4, read back 1.5 to 2.4 and the device call's
remainder (submit and the queue) 1.8 to 5.8 — where `zoom_frame`'s replayed window spends 1.3 to
1.5, because between two notches' windows the page is drawn on `Cpu` and the compute lane walks
again rather than replaying its records (0.3 ms, so the walk is not the cost). Around it, about
0.6 ms from the wheel to the request, 0.5 to 3 ms from the window landing to the window's loop
taking it while it paints the stand-in, the copy into a `QImage` 0.45 ms and the paint 0.6 ms. The
first notch pays the compute lane's first use on this device: its window's remainder is 6.4 to
16.7 ms against 1.8 to 5.8 after.

## Decision

1. **ADR 2322 stays built**: it halves the settled notch where the device draws it and changes no
   frame at rest.
2. **What would take the notch under a frame is now named, and none of it is this tree's toolkit
   floor**: the compute lane's first use on a device that drew its launch on `Cpu` (raster's
   pipelines and residency for that lane), and the device call's remainder per window. Both are
   raster's to price; neither is built here.

## Costs

Single samples per pass at loads of 1.3 to 2.9 on a machine a seven-round batch shared.
