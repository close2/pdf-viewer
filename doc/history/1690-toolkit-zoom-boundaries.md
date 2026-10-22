# 1690 — Toolkit zoom boundaries

Session 1690, 2026-10-22. Host investigation; ADRs 2214/2215.

**Change.** An independent GTK allocation witness proves a scale-two defect: a 200×160 texture
occupies 400×320 physical pixels despite a smaller size request. GTK now allocates texture-native
dimensions and supplies device-to-logical scaling in every placement transform. Finished origins
retain device-pixel rounding; previews retain fractional movement. The automatic Linux witness
checks fifteen reused placements at scales 1, 2 and 1.5; a wrong-filter control rejects zero tests.
CI explicitly supplies Xvfb, xdpyinfo and GNU timeout. Other native platforms remain unverified.
Opt-in Qt/GTK traces distinguish input, image retrieval/copy, host follow-up and actual painting.

**Gates.** Scoped warnings-denied all-target clippy for viewer-host/GTK/Qt and unit suites passed:
GTK 10, Qt 23; exit 0, 95 s, 1.29 GiB. Immutable release copy passed, exit 0, 249 s, 3.79 GiB;
source comparison stayed equal. Later Linux-only test cfg changes no release behavior. YAML
and scoped formatting checks passed. CPU software smoke7 passed, exit 0, 43 s, 0.29 GiB:
each toolkit delivered 32 Zoom and six Scroll commands. Earlier failures remain recorded.

**Display investigation.** Root-owned GPU/display runs use immutable copies and exclusive holds.
Initial setup, synthetic-input and coordinate failures were rejected. Separate unmaximize requests
corrected a harness error; repeated xdotool options had removed only the last named state.
Probe7 matched Qt's 900×700 viewport at DPR 1.59375 but delivered cumulative 3/7/19/31 zooms
instead of 4/8/20/32; six pans arrived. Settling, focus checks and longer pulses left one Zoom
missing; non-Ctrl wheel delivery is unobserved. Invalid: exit 1, 95 s hold, 173 s including queue.
Qt first-paint medians for the four phases were 1.94/1.27/2.09/1.85 ms, from 3/4/12/12 events;
these are incomplete Xwayland observations at nominal 120/55 ms cadence, not a speedup verdict.
GTK matched 900×700 at DPR 1 but failed pointer calibration before gestures: two samples reused
one old motion event. A future probe must require a new motion receipt after each requested move.

**Interpretation.** Native-size freshness cannot require every image to be unscaled: a completed
opaque oversized frame can cover a scaled retained underlay. The original conservative parser
and its verdicts remain preserved; its native-latency aggregate is not a settled-frame verdict.
Fixed allocation can exceed its clipped Overlay without viewport drift. Paint is not scanout.

**Owed.** The owner's Qt lag, GTK blur and deep-zoom drift remain unattributed. No Qt performance
fix is asserted; GTK's observed DPR 1 does not exercise the proved scale-two fix. Next work needs
widget-to-window geometry, fresh motion receipts, exact Qt/XI2 input diagnosis, within-toolkit
pixel/anchor comparisons and native Wayland reproduction. Cross-toolkit pixels require normalized
DPR/zoom. Final toolkit gate passes; launch follow-up: ADR 2209. Total round wall time and
tool-call count are unavailable; individual hold durations above are measured.
