# ADR 2215 — Toolkit zoom observation separates delivery and paint

Status: **assessment** at the merge that took it, `0eb4d278`: it records no decision a later round
must not re-litigate, as `doc/reviews/1704-batches-104-to-108-against-the-principles.md` finding 5
reads it (line added by ADR 2260).

A frame copied into a toolkit image is not yet a frame painted by the widget. Qt schedules an
update, performs host follow-up work, and later enters its paint callback. GTK similarly submits
textures before its frame-clock callback. Diagnose zoom latency with those boundaries visible;
do not attribute the entire interval to a copy or to accessibility without measurements.

Opt-in `PDFVIEWER_ZOOM_PROBE` diagnostics record wheel anchors, device scale, placement dimensions,
frame retrieval/copy, host follow-up and actual toolkit paint callbacks. Qt's retrieval/copy
interval includes Rust frame getters and geometry queries. Its paint-end is not compositor
scanout. GTK reports available presentation feedback separately; a zero timestamp is unavailable.
Normal execution does not install the additional GTK frame observer.

The current Qt wheel receipt is inside the Ctrl-modifier branch, not an unconditional census
of wheel events. A missing Zoom command therefore does not establish missing toolkit delivery:
a wheel arriving without Ctrl is unobserved by that receipt. Further input diagnosis needs an
entry receipt before modifier branching and an accepted/rejected receipt at the host busy guard.

A display probe must verify actual core Zoom and Scroll commands, not successful input-tool
exit codes. Synthetic window events and XTest events can behave differently. Input coordinates
must be calibrated from observed core pointer positions independently of resize coordinates;
fractional Xwayland scaling can give them different units. Verify the final content anchor,
record requested and actual geometry, and mark unmatched viewports incomparable. Compositor
maximization is distinct from a widget's minimum-size constraint.

A completed render notification alone does not prove accepted, settled pixels: its token may
be stale, and a cache hit may need no new completion. Associate the last native-size placement
batch with the actual toolkit paint callback. This distinguishes a scaled stand-in from a
native-size frame, but does not prove correct page coordinates or compositor presentation.
Requests superseded by a later wheel event cannot be assigned the later frame's latency.
Keep raw events and identify those requests separately.

The real-display protocol owns only its spawned viewers and their windows, bounds every wait,
and releases only the Ctrl key it injected. Immutable binaries carry source and byte manifests.
X11/Xwayland observation does not substitute for a native Wayland reproduction. Settled-pixel,
anchor and performance comparisons require matching document, page, zoom, viewport and display
conditions; exploratory observations remain labelled when those conditions are absent.
