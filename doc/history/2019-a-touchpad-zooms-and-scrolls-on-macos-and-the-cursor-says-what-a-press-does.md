# 2019 — A touchpad zooms and scrolls on macOS, and the cursor says what a press does

Session 2019, a host round on the owner's request: macOS trackpad gestures and a text cursor in
`quorra`. ADR 2856. Run on the owner's macOS notebook, in a worktree beside the checkout another
session held.

**The premise.** The owner: text is selectable but the pointer does not change over it, and the
touchpad does not zoom. Held, and wider than stated: `Query::LinkAt` was the boundary's only
pointer-speed question, `quorra` read Control and never ⌘ although ADR 2048 §2.5 says the
platform's command modifier, two-finger scrolling sent `dx: 0.0`, and no window read winit's
`PinchGesture` or `DoubleTapGesture`.

**Built.** `Query::TextAt` → `Answer::OverText` (inside a glyph's box, the interpreted page only);
`viewer_host::Under` (link before text) in `quorra`, `quorra-gtk` and `quorra-qt`;
`quorra_text_at` in C and declared for .NET; kind 39 both ways on the confined pipe. `quorra`: ⌘ is
the command modifier on macOS, scrolling takes both axes, a pinch is `viewer_host::pinch`'s factor
through `ZoomTravel::commands`, Smart zoom is `viewer_host::SmartZoom` (width, then back).
`viewer_host::magnification` is the one reading of the magnification showing.

**Tests.** `viewer-core` `the_text_under_a_point_is_where_the_selection_draws_it` (points along
the first selected run of `basicapi.pdf`, the viewport corner); `viewer-host` gesture tests (a
pinch's factor and its refusals, both taps, a doubled page, a zoom between taps, the range's end);
`viewer-confined` round trips; `quorra`'s modifier test on the command modifier. **Plants**: an
`over_text` that always answers false fails the first test on its run count, and one that answers
true within 1000 units fails it on the corner.

**Gates.** Homebrew's Rust 1.99 without rustup, not the pinned 1.97.1, on `main` at
6dc0bc74 and compared with it by name, `doc/pdf.js` and the Arlington model linked from the owner's
checkout on both sides. `cargo fmt --check` on the seven crates: exit 0. Clippy `--no-deps
--all-targets` on `viewer-core`, `-confined`, `-ffi`, `-host`, `-ui`, `-display-link`: exit 0, no
finding on a changed line (117 warnings, 1.99-only lints in unchanged code). `cargo test
--no-fail-fast`: `viewer-core` and `-device` exit 101, the same 67 failures as `main`, 579 passed
against 578; `viewer-ffi`, `-host`, `-confined`, `-ui`, `-display-link` exit 101, the same 54, 602
passed against 594 (fixtures and worker binaries this machine lacks). `conformance`: exit 101, the
same 33 (no `doc/md` here). GTK, Qt, .NET: not compiled; CI builds them.

**Left.** The .NET window's cursor (it reads only `quorra_window_pointer`'s link flag); a native
macOS window for three-finger swipes and the elastic edge, which winit 0.30 does not report.
