# 1838 — The toolkit windows hold their picture one refresh before standing in

Slot 7 of batch one hundred and twenty-four, 2026-10-27, a host slot (Sonnet, high), written by the round's
second agent after a handover; ADRs 2510 and 2511 used, Q704 left unused. I worked in `scratchpad/r1838/`.
No ledger row moves: A657 is the owner's ratified build of Q657, not a clause.
**Premise** (a deadline one period after the command, asked at paint): held, with two additions the code needed.
The deadline alone cannot say *what* to paint before it, so `Hold` records each page's pixels' size and origin
*before* the viewer handles the command (afterwards `Query::PageGeometry` answers the new extent), and a window
that paints only on an event would never stand in at the lapse, so `mind_the_hold` asks for one repaint when the
period ends with a frame still owed. The period runs from the first command no frame has answered, a page holds
only the frame it was recorded with, `Retained::fill` adds nothing beside a held picture, and the records are
sorted and found by bisection (a test at 5 000 pages).
**Built.** `viewer_host::stand_in::Hold` and `Placed::held`; `WindowState::hold`, `hold_the_pictures` on
`Command::Zoom`, `mind_the_hold`; the three `frames()` call sites through `hold.placed` with `holds its …` on the
`frames` topic; the period per host (GTK monitor refresh at the zoom, Qt `QScreen::refreshRate` told with the
placement, the Windows window `GetDeviceCaps` on the primary screen through the new ABI function
`quorra_window_refresh`, a sixtieth of a second where none is stated; ADR 2511). The drive's `zoom_stand_in` is two
checks: `82-zoom-stand-in` (the slow page: `holds its` before `stands in at`) and `82-zoom-held` (a fast page,
`drive-zoom-fast.pdf`, up to five notches: no `stands in at`).
**Drive under Xvfb** (:141, mine, removed after): `82-zoom-stand-in` works on `quorra-gtk` (red block 247x246 held
then 248x248 landed) and `quorra-qt` (252x251, then 252x252); `82-zoom-held` works on both, on notch 1 and notch 2.
Under Xvfb the load average was 80 to 220 and the frames are llvmpipe's, so the figures show the rule and not a speed.
**For CI's `window` job** (the orchestrator dispatches it): `dotnet build` of `Quorra.App` green with
`DisplayRefresh.cs`, `QuorraWindow.Refresh` and `Native.quorra_window_refresh`, and the published window starting and
reading its title; its drive step `zoom_stand_in` should show both verdicts `works`. Nothing here ran on Windows.
**Found, not fixed:** `tools/dotnet-check.sh` builds `Quorra.App` and passes on this host, but it runs none of it, so
`DisplayRefresh.Millihertz` is unexercised until CI; a window moved to a second display on Windows holds for the
primary's period (ADR 2511 section 3); the ABI header test's counts (`361`, `342`) are written in two files and move
by hand.
**Gates.** `cargo test -p viewer-host --lib stand_in` exit 0, 18 passed; `cargo test --no-fail-fast -p viewer-ffi
-p viewer-host -p viewer-gtk -p viewer-qt` exit 0, 460 passed, 0 failed (the two header counts for the new symbol were
red once, fixed); `cargo clippy -p viewer-host -p viewer-gtk -p viewer-qt -p viewer-ffi --all-targets -- -D warnings`
exit 0; `cargo fmt --check` on the four exit 0; rustdoc `-D warnings` on the four exit 0; the Qt C++ compiled in
`cargo build -p viewer-qt` exit 0; the drive above 4 works, 0 wrong; `tools/dotnet-check.sh` exit 0;
`cargo run -p conformance --bin cited` exit 0; `cargo test -p conformance --no-fail-fast` exit 0, all passed, no red of mine or anyone's. The Opus validator's six findings (an unproved lapse repaint, three false ADR sentences, an owner's quotation filed under the wrong source, an untested claim) were fixed by me; `hold_tests.rs` in `viewer-ffi` is the new test. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on an unproved lapse repaint and five ADR sentences, and PASS on the hold's behaviour, the drive and the gates; the round added the test that plants the repaint away and corrected the sentences.
