# 1703 — GTK settled source and window observations

Session 1703, 2026-10-22. ADRs 2240/2241.

**Change.** Add opt-in completed-source provenance, refresh generation invalidation,
bounded source capture and paint/geometry joining. Extend the finite owned-window
probe with fit, two key zoom-outs and fit-return capture. Rendering decisions stay
in the existing path; no blur or deep-anchor fix is claimed.

**Gates.** All-target check passes (23 s, .60 GiB); final warnings-denied Clippy
passes (3 s hold, .77 GiB). Four observation library tests pass (44 s hold,
.43 GiB); the mistaken binary selection ran zero tests and is not accepted evidence.
Parser self-test passes ten methods, including 22 negative settled joins.
CPU rehearsal passes four captures (4 s hold, .29 GiB); the initial Xvfb lock-file
assumption failed before measurement and remains recorded. Independent visible-edge
mapping is 1:1 at all four states; every opaque page/interior ROI byte matches.

**Limits.** This is source-to-X11 backing evidence, not compositor scanout,
native Wayland, wheel latency or deep zoom drift. Instrumentation IO is outside
all latency claims. Final hardware results and combined gates are recorded in the
batch handover. Qt's separate one-query change establishes no overall latency gain.
