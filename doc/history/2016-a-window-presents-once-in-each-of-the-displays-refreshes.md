# 2016 — A window presents once in each of the display's refreshes, counted by the display

Session 2016, a host round on the owner's request, after 2015 on the same branch and machine: the
owner's macOS notebook, an M5 Pro with a 120 Hz panel. ADR 2849.

**The premise.** The owner: scrolling and pinching do not feel like a well-written Mac program.
Held, and the cause was not the rate: `quorra`'s cadence was 120 Hz as the surface stated it, but
its timer's phase was the last present's and an input's moment, so presents fell anywhere in the
display's refresh — 63 % of intervals at one refresh, the rest smeared over 10–15 ms, every frame
fast enough. The touchpad was measured at about 120 Hz on its own clock.

**Built.** `viewer-display-link`: a count of the display's refreshes and a wake, macOS 14's
`NSView` `CADisplayLink` through `objc2`, `Unavailable` everywhere else. `quorra`'s `Cadence`
takes it in `resumed`; while it runs, one present a refresh, an owed frame asks the link for the
next refresh, and every present keeps the link counting. The trace says what paces the frames and,
at exit, the period the link stated and the refreshes it saw. The crate map has the crate.

**Measured.** Five builds, each one person's scrolling and pinching through `--trace=frames,window`
(the table is ADR 2849's): the timer 63 % of intervals at one refresh; the first link build 23 %
with the loop spinning 183 472 times; a refresh-only gate 0 %; any refresh since the last present
68–86 % across four runs, the link pausing mid-gesture; that and every present counted as activity
80 %, 10 of 610 inputs refused. The owner: it feels good.

**Tests.** `quorra`'s cadence test of the one-present-a-refresh rule beside the timer's six. The
link itself has no test: it needs a window on a display, which no test here has.

**Gates.** Run once for both rounds, as 2015 records them: Homebrew's Rust 1.99 without rustup, not the pinned 1.97.1, on `main` at
6dc0bc74 and compared with it by name, `doc/pdf.js` and the Arlington model linked from the owner's
checkout on both sides. `cargo fmt --check` on the seven crates: exit 0. Clippy `--no-deps
--all-targets` on `viewer-core`, `-confined`, `-ffi`, `-host`, `-ui`, `-display-link`: exit 0, no
finding on a changed line (117 warnings, 1.99-only lints in unchanged code). `cargo test
--no-fail-fast`: `viewer-core` and `-device` exit 101, the same 67 failures as `main`, 579 passed
against 578; `viewer-ffi`, `-host`, `-confined`, `-ui`, `-display-link` exit 101, the same 54, 602
passed against 594 (fixtures and worker binaries this machine lacks). `conformance`: exit 101, the
same 33 (no `doc/md` here). GTK, Qt, .NET, and Wayland and Windows: not built. CI's `check` then failed the
non-macOS stub on an unfulfilled `expect`; that half is now an uninhabited type, linted here as Linux.

**Left.** Wayland (`pre_present_notify` and the frame callback) and Windows (`DwmFlush` on a
thread) as second implementations of the same interface; macOS before 14; a gate that drives
synthetic input at a stated rate and phase, so that the figures above become a band.
