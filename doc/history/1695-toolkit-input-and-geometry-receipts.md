# 1695 — Toolkit input and geometry receipts

Session 1695, 2026-10-22. Host diagnostics; ADRs 2224/2225.

**Change.** Opt-in structured receipts distinguish toolkit callbacks, host suppression and
core commands. Qt now records every wheel entry before modifier branching, its branch and
busy/accepted outcome. A synchronous C++ span contains one Rust accumulator receipt without
extending the bridge ABI. Raw Qt motion distinguishes moves from drags. GTK records motion
before borrowing its host, then acceptance, measuring suppression, busy borrow or host absence.
Input behavior, accumulation and event acceptance remain unchanged.

Geometry names actual GTK Overlay/Qt PageArea bounds in client logical space, widget and
surface scales, backend identity and explicitly derived device extents. Accepted resize calls
have independent core-viewport receipts. GTK Fixed natural allocation is labelled separately.
Unavailable mappings remain null. Qt accepted motion means the host handler returned; a core
Pointer trace is still needed to prove dispatch. None of these receipts establishes scanout.

**Gates.** Scoped package formatting passed. All-target GTK/Qt Clippy with warnings denied
passed, exit 0, 4 s, peak 0.84 GiB. Earlier failed cast/lint attempts remain in the scratch report.
The C++ build still reports generated-bridge maybe-uninitialized warnings, not owned diagnostics.
Full toolkit nextest passed: 57 tests, 2 skipped, including the owned-Xvfb allocation and
zero-test rejection witnesses. Tests plus immutable debug-bin copy exited 0, 287 s, 1.42 GiB.
CPU-only Xvfb input rehearsals passed for both hosts at scale one and two: fresh motion pairs,
independent third-point mapping and four Zoom commands each. Qt also passed four callback/span
checks. Holds: 6 s/0.29 GiB and 5 s/0.44 GiB, exit 0. An absent requested page was rejected before
input; the expected-refusal control exited 0, 25 s/0.25 GiB. Final integration remains pending.

**Owed.** The owner's Qt lag, GTK blur and deep-zoom drift remain unattributed. The first missing
Zoom cannot be called a missing wheel callback from the old Ctrl-only trace. Root's separate
instrument round owns fresh receipt association, calibration controls and the actual display
protocol. Programmatic native-backend zoom would not establish physical wheel delivery.
No hardware result, native Wayland reproduction, matched pixel comparison or speedup is claimed.
Total round wall time and tool-call count are unavailable; quoted hold figures are measured.
