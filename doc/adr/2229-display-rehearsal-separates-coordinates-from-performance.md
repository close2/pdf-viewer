# ADR 2229 — Display rehearsal separates coordinates from performance

Status: **accepted** at the merge that took it, `9cd2c665` (line added by ADR 2260).

The owner's ISO document is the primary witness for Qt zoom lag and GTK blur/deep-zoom drift.
A useful test must first establish that its input reached the intended viewport. A screenshot's
size, a GTK Fixed natural allocation or a requested backend does not establish that viewport.
The toolkit geometry receipts therefore name client/content logical bounds, widget/surface
scales, actual backend and accepted core device viewport separately (ADR2225).

The finite display driver launches one owned viewer and records its command and environment.
It requires the requested page to be retained, one visible native window with the viewer's PID,
and matching local-window geometry/viewport records. X input focus must belong to that window
or its verified child ancestry. Initial native/client size ratios choose safe stimulus guesses;
only fresh observed callback/core coordinates establish the calibration accepted by ADR2228.
The driver then requires four discrete Ctrl-wheel commands, retaining raw callback outcomes
where available. GTK's core commands do not claim an equivalent raw scroll callback census.

CPU rehearsal uses an owned Xvfb display, explicit CPU rendering and software toolkit graphics.
Qt's XI2-disable setting is recorded as a rehearsal condition. Such a result validates the input
instrument under those conditions; it is not a hardware, native-Wayland or latency measurement.
Scale-two rehearsal is a separate arm, and a missing-page negative must fail readiness before
any input phase. Every attempt retains its receipt and trace, including failures.

Only root may publish a reviewed owner-loop job, and only for GPU/display performance testing.
Builds, artifact preparation and analysis stay in the agent account. Hardware runs must use
immutable attested binaries, actual adapter/backend receipts and the exclusive measurement
hold. A probe terminates only its owned process and releases only the modifier it pressed.
A wall deadline, bounded reads and finite phases prevent an unattended test from continuing.

An input-and-geometry verdict is deliberately narrower than a symptom fix. Pixel comparisons
still need matched actual viewports, scale, current-frame coverage and opacity; timing needs
balanced immutable artifacts and quiet guards. X11/Xwayland evidence does not establish native
Wayland presentation. Qt lag, GTK sharpness and deep-zoom drift remain open until those specific
conditions are reproduced and measured; successful injected input alone does not settle them.
