# 1177 — A measurement is traced, and Control means something

The HOST-UI round of batch twenty-seven.

## What was found

**§12.9's refusal had expired and no gate could see it.** The row said *`partial` because nothing takes the two points*, and the whole arithmetic was here — Tables 265 to 268, the five formatting
steps, the worked example as their test. What was missing was a hand.

**Table 267's `/S` disagrees with its own `/CYX` cell, and the specific sentence wins.** `/S` names
all three scale factors; `/CYX` says it is for "calculations (distance, area, and angle)" and that
"[o]ther calculations (change in x , change in y , and slope) shall not require this value". So a
slope is a `/Y` unit over an `/X` unit, and the clause's own plot of temperature against time, which
has no distance and no area, has a gradient.

**§12.9.2's empty string is not a measurement.** `format` answers an absent array with `""`; that
became a slope of nothing beside the word *slope*, and the end-to-end test caught what reading had
not.

**§12.10's blocker was two blockers.** The host half is gone; the projection needs the EPSG
registry or an ISO 19162 string, which §12.10.3 names as outside this standard, and nothing
interpolates between registration points because the clause states no function between them. **And
the brief's table numbers were two out** (268/269/270 where the standard has 266/267/268), while
`measurement.rs`, the module it thought might be new, exists.

## What was built

`Measure::angle`, `Measure::slope`, `Rectilinear::gradient_axes`, `Viewport::unit_square`,
`Geospatial::within_bounds` and `Viewports::traced` in `pdf-model`; `Query::Measure` and
`Answer::Measured` in the core, with no `Command` and no `Event`; `viewer_host::measuring` — the
mode, the wording, §12.10's refusal said out loud; `Key::M`; the gesture and the sentence in all
three windows; `quorra_measure` with a `MeasurePart` selector; a query kind, an answer kind and a
boxed `Reply` on the wire. And `viewer_host::Modifiers` with `ctrl` beside `shift`,
`ctrl_meaning` holding the four conventional bindings this program's operations already earn, and an
unbound Control meaning **nothing** rather than the unmodified row — threaded through the GTK
modifier read, the Qt `cxx` signature and both C++ call sites, and winit's state, each at a seam a
test reaches without a display.

**Left owed**: a projected coordinate's latitude; a rubber band, since no window draws the path it
is measuring, only the answer; and no window driven by hand — compiled and tested, not watched.
