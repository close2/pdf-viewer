# 2250 — GTK draws the page at the surface's fractional scale, on the surface's pixel grid

Status: accepted. Session 1708, 2026-10-23.

## Context

The owner reads under KDE Plasma on Wayland, and `~/.config/kwinoutputconfig.json` (readable from
the agent account) holds his outputs at 1.5, 1.6, 1.7 and 1.75 as well as 1. Neither toolkit window
chooses a backend (`grep` finds no `GDK_BACKEND` or `QT_QPA_PLATFORM` in either crate), and no
desktop entry starts one, so launched from his shell GTK 4 and Qt 6 both take Wayland. Every
hardware trial of batches 106–108 forced X11 (`tools/zoom-probe-drive.py` sets `GDK_BACKEND=x11`,
`QT_QPA_PLATFORM=xcb` and removes `WAYLAND_DISPLAY`), which is why the one real-adapter trial saw
Qt at 1.59375 — Xwayland's `Xft.dpi` of 153 over 96 — and GTK at 1.

The owner's loop (his word of 2026-10-23 admits display and UI questions) confirmed it on his
session: `wayland`, none of `QT_QPA_PLATFORM`, `GDK_BACKEND`, `QT_SCALE_FACTOR`, `GDK_SCALE` set,
the panel `eDP-1` at 2880×1800 and scale 1.6; HEAD's `quorra-gtk` on `GdkWaylandDisplay` at
scale 2, this tree's at 1.6, and `quorra-qt` on `wayland` at 1.6.

On Wayland at 1.6, `gtk_widget_get_scale_factor` is 2 — GTK rounds the fractional scale *up* —
while `GdkSurface:scale` (GTK 4.12) is 1.6. `viewer-gtk` sized the page raster by the whole number,
so GSK drew a 1238-pixel source into 990 device pixels: the blur the owner reports, by
construction. Measured headless under a nested KWin set to each scale (ADR 2251): HEAD's source is
shown at 0.6243, 0.7494, 0.7992, 0.8479 and 0.8739 of its size at 1.25, 1.5, 1.6, 1.7 and 1.75, and
1:1 byte for byte only at 1 and 2.

## Decision

1. **The scale is the surface's own** (`window::surface_scale`): `GdkSurface:scale` read by its
   property name, so the `v4_10` floor stays (`Cargo.toml`); a GTK without the property gives its
   whole factor, which is then what it draws at. `Host::scale` is an `f64`, and the surface's
   `notify::scale` measures the viewport again where the scale changes without the size.
2. **The page area's device size is the pixels it covers whole** (`window::device_extent`): from
   the first device pixel at or after its top-left edge to the last before its bottom-right one.
   381 logical pixels at 1.25 begin at 476.25; GSK clips the area to whole pixels, and a raster of
   the rounded product (774) lost a column to that clip.
3. **A finished frame lands on the surface's pixel grid** (`stand_in::on_the_surface_grid`): its
   whole-pixel origin (ADR 1865) is moved by the fraction the page area begins into a pixel. Without
   it the source was shown at size at 1.6 but sampled between pixels: 96.77 % of pixels identical,
   a largest channel difference of 102. A stand-in is scaled anyway and keeps its fractional place.

With all three the same instrument reads 1:1, every byte identical, at 1, 1.25, 1.5, 1.6, 1.7, 1.75
and 2. The pointer, the scroll step and the zoom anchor already multiplied by `Host::scale`, so
they now speak the device pixels the core rasters in, which is the brief's anchor in device pixels.

## Costs and limits

The page raster at 1.6 is 0.64 of the pixels it was at 2: less to draw, not more. Two widget
queries (`compute_point`, `surface_transform`) per placement and per resize. The grid alignment
assumes the page area keeps its place on the surface between a resize and the next refresh; a
sidebar toggled without a refresh would leave the fraction stale until the next frame. Qt was
already right on Wayland — `devicePixelRatioF` is 1.6 there — though its first viewport arrives at
2 before the compositor states 1.6 (the round's report). Deep-zoom drift is not attributed here:
no Wayland compositor accepts synthetic input from an unprivileged client.
