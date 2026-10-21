# 2152 — Control and the wheel cross as travel in a unit, and the carry is the window's

Session 1658, RFC 0010 section 6's step N3a, second part. Status: **accepted** and **built** in
`crates/viewer-ffi/src/window/wheel.rs`, `Window::wheel_zoom`, `quorra_window_zoom_wheel`; the
WinUI window's call is compiled by CI only (ADR 2136). Context: ADRs 0247, 1118, 2088, 2104.

## Decision

`quorra_window` had no entry point for Control and the wheel, which magnify in the other three
windows (session 1650's finding). It gains one, beside `quorra_window_press`:

`quorra_window_zoom_wheel(window, travel, unit, x, y, &steps)` — `travel` is the wheel turned away
from the person in a `QUORRA_WHEEL_LINES` or `QUORRA_WHEEL_PIXELS` unit, `(x, y)` the pointer in
device pixels of the viewport, the point of the page that stays still, and `*steps` the whole steps
taken. The window holds a `viewer_host::ZoomWheel` and spends the travel through it, so the fraction
of a step a high-resolution wheel or a precision touchpad's pinch leaves is carried from one call to
the next, as `viewer-gtk` and `viewer-qt` carry it (ADR 1118). `WheelKind` is taken: a third unit is
`QUORRA_WRONG_KIND`, a travel or point that is not finite `QUORRA_NUMBER_OUT_OF_RANGE`, and nothing
moves for either.

## Why

The carry is the whole of what is hard here, and ADR 1118 measured why: a mouse whose quantum is a
thirty-seventh of a line produced one step from 146 lines of travel while each event was truncated
on its own. A fourth window counting a gesture its own way would be the copy `doc/todo/30` warns of,
and a caller cannot carry the fraction correctly without knowing what a step is worth in pixels. So
the travel crosses raw, in the unit the toolkit reports, and the step is the library's.

## Considered and refused

- **A step count from the caller** (`quorra_window_zoom(window, steps, x, y)`): every caller then
  carries the fraction itself, which is the trap ADR 1118 records, three times over.
- **An optional anchor** (a flag for "hold the centre"): the toolkit windows anchor at the pointer
  whenever they have one, and a wheel event always carries its pointer; a caller with none passes
  the viewport's middle, which is where the core holds it.
- **A callback, or a struct by value**: ADR 0247's two refusals, which this entry point needs neither
  of — six scalars and one nullable out-parameter.

## Costs

One entry point and two constants; the counts in `header_and_library_agree.rs` and
`unsafe_position.rs` move by one. `drive_a_window.c` drives it: three events of two fifths of a line
are one step and the page is drawn a quarter larger, sixty touchpad pixels back are one step out, and
the two refusals are statuses.
