# 2291 — A finger scrolls by what it travelled in both windows, and what X11 cannot tell

Status: implemented. Session 1728, 2026-10-23, the host slot of batch one hundred and twelve.
Cites no clause: the standard says nothing about a wheel or a touchpad. Context: ADRs 1118, 2274,
2275; the owner's words of 2026-10-23 on the wheel.

## 1. What HEAD did, measured

ADR 2274 left the touchpad unfinished: GTK's scroll controller is built `DISCRETE`, and GTK 4
counts a surface-unit scroll into its discrete steps there, so a finger's ten pixels were a notch
of 48. Measured on the nested KWin at 1.6 with `tools/zoom-probe-wayland.py --touchpad` (twelve
smooth axis events of ten logical pixels, `ei_device_scroll_delta`; the page's travel read off
the output): `quorra-gtk` with HEAD's input moved 77 device pixels an event, a notch, until the
page's end; the premise held.

## 2. Decision

- **One distance**: `viewer_host::WheelTravel::distance` — a counted notch moves `SCROLL_NOTCH`, a
  pixel a pixel — is what both windows spend; Qt's `scrolled` and GTK's now call it, so a finger
  and a notch move the two pages alike.
- **GTK reads a finger's surface pixels on its smooth controller**: the `DISCRETE` controller is
  kept for wheels and passes over any event whose own unit is `gdk::ScrollUnit::Surface` (its
  `unit()` says `Wheel` for everything it counts, so the event is asked); the controller without
  `DISCRETE`, which already took Control's zoom, scrolls by the pixels as they come.
- **Qt is unchanged**: a gesture — a scroll phase, or `MouseEventSynthesizedBySystem`, which
  QtWayland gives the finger and continuous sources — was already counted in pixels by
  `WheelTravel::of` (ADR 2274).

After: `quorra-gtk` moved 16 device pixels an event, 192 for 192 wanted.

## 3. What was driven, and what was not

A finger was not. EIS states no axis source, so a finger cannot be sent to the nested compositor;
`/dev/uinput` is not open to the agent user, so no recorded `libinput` replay can be played; and
GTK's own test harness builds its scroll events with `gdk_scroll_event_new`, which no binding
exposes. What was driven is a **source-less continuous axis** — KWin's axis value with no
discrete value and no source. GTK reads it as `Surface` and spends its pixels. Qt 6.12 hands it on
as a wheel: no phase, source `MouseEventNotSynthesized`, an angle of 120 per ten pixels that
QtWayland synthesised from the pixel delta; `WheelTravel::of` counts that as a notch, 77 device
pixels an event, as HEAD did. Nothing on `QWheelEvent` tells such an axis from a wheel whose
compositor states a pixel delta, so Qt is not made to guess: a real finger states its source, which
both toolkits read — GTK as `Surface`, QtWayland as phases and `MouseEventSynthesizedBySystem` — and
then both windows spend its pixels. `viewer-host`'s
`a_finger_moves_the_page_as_far_as_it_went_and_a_notch_a_notch` holds the rule.

## 4. What X11 can and cannot tell

Under X11 a touchpad reaches a window through XInput 2's scroll valuators, in the driver's
increments — `libinput`'s is 15 — and not in pixels. GTK 4's X11 backend states every such event
in `ScrollUnit::Wheel`, as fractions of a notch; Qt 6.12's XInput 2 path states a pixel delta only
for a driver whose increment exceeds 15, so none for `libinput`. Neither toolkit can tell a
finger's distance there, only the device's name; so under X11 both windows count a touchpad as a
high-resolution wheel's fractions — whole notches for a scroll, a proportion for a zoom — alike,
and a finger that wants pixels wants Wayland.

## 5. Cost

One function in `viewer-host` and its test, one controller's branch in GTK; no pixel moves.
