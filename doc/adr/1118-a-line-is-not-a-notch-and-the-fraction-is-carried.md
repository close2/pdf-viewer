# ADR 1118 — A line is not a notch, and the fraction is carried

Status: accepted, 2026-09-15. Session 1104. On the trace the project owner took on 2026-09-15 at 21:50
(`tmp/trace.txt.gz`). Amends the quantisation half of ADR 0166, whose anchor half is untouched.

## What the trace said

Seven separate Ctrl-held gestures, **579 `MouseWheel` events carrying 146.4 lines of travel, and one
`zoom` line in 4 844**. The trace prints every window event and prints a `zoom` only where a command is
dispatched, so the defect was legible in the file from the moment it was written: 578 events reached
`App::wheel` and left no trace after it. Two of the file's own headline figures are that silence —
`p99 3513.3 ms` and `max 6713.3 ms` between presents are the spans in which the owner was cranking the
wheel and the window had nothing new to show.

## The assumption that was wrong

`MouseScrollDelta::LineDelta` was read as *a notch*, and a notch as *a step*:

```rust
MouseScrollDelta::LineDelta(_, lines) => { self.pinch = 0.0; lines.trunc() }
```

`winit`'s X11 backend divides an XInput2 smooth-scroll valuator by that axis's increment, so a
high-resolution wheel reports a **fraction** of a line per event. On the owner's device the quantum was
about a thirty-seventh of a line, and `trunc()` of a thirty-seventh is zero — once per event, with the
accumulator *reset* on the way past, so nothing was carried either. The one step that did fire came from
the single event in the file whose delta reached 1.0.

The scroll path above it was never affected: it multiplies by sixteen and a fraction of a line is a
fraction of a row, which is exactly right. Only the magnification asked for whole units.

## The decision

**One accumulator, counted in lines, and the fraction is carried rather than truncated** — which is what
the pixel arm has always done, and the arms now differ only in what converts a device's travel into
lines. `WHEEL_ZOOM_PIXELS` is that conversion for a touchpad, so fifty pixels stays one step; a classic
wheel's whole notch is still one step on the event that carries it.

Rejected: *round instead of truncate* (a half-notch flick would zoom, and every sub-quantum jitter with
it); *a step per event regardless of size* (a high-resolution wheel would zoom thirty-seven times a
notch); *a threshold in lines of the device's own quantum* (nothing reports it, and a bound fitted to one
mouse is a bound fitted to one mouse).

The carry is **not** reset when Ctrl is released or when the direction reverses. A reversal cancels in
the accumulator, which is what a person doing it means, and a residue surviving a gesture is the
behaviour the pixel arm has had since it was written.

An accumulator can be poisoned permanently by one value where a per-event truncation could not, so a
non-finite delta is ignored rather than added in, and the ±64 clamp `ZOOM_RANGE` justifies now takes the
whole of what it clamped out of the carry instead of banking it.

## What it cost

`crates/viewer-ui/src/bin/quorra/sidebar.rs`: the arm becomes a free `zoom_steps` function with four
tests, one of which replays the trace's own quantum and prints the before figure beside the after.
`App::pinch` becomes `App::zoom_carry` because the field is no longer pixels and no longer a touchpad's.
