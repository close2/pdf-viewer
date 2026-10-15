# 1554 — The confined window draws a page past the bound, and the workers close what they inherited

Slot 3 of batch eighty-eight, 2026-10-15, a host round: ADR 1921 §3's build, and (A430 said yes)
ADR 1907's option 1. ADRs 1944, 1945; no question; no row moved.

**Premise** held; the new test found a fourth refusal it did not name, the host's decoder refusing a
list target past the bound. **Hypothesis** held: the list fits (past the bound a raster is over a
gibibyte), so the marks were the cheaper build and no worker draws a window (ADR 1944 §1).

**Built.** ADR 1944: the command crosses; the worker sends a page past the bound as marks within
`MAX_PIXELS × 4` bytes or refuses it in the viewer's words, drawing no raster past the bound (a turn
included); the host admits such a target only after `Listed`; the processor screen (`--cpu`, a
refused frame) draws the window and its margin. Step 82 judges the confined window as the toolkits.
ADR 1945: each worker's first statement is `close_range(3, ~0, 0)`, one lift each, pinned by
`pdf-sandbox/tests/inherited_descriptors.rs`; `viewer-qt`'s tree sweep could not see a lift with a
`reason` (trap 13) and now names six crates; the GLib report is drafted in ADR 1945 §3.

**Proof nothing moved below the bound**: `Marks::decide`'s `min` is the identity there, the decoder
changes only after `Listed`, windows only past the bound; ten notches (never past it) settled in
24.2 s at HEAD and 24.5–24.6 s here, 0 refusals each. No `pdf-render`, `render-cpu` or `raster`
file changed, so `raster_golden` and the arms cannot move; the close changes no byte a worker reads
or writes. Planted back, the close's absence fails both new tests (the second at its 10 s ceiling).

**Measured** (one `--clock` hold, `scratchpad/r1554/hold.sh`, Xvfb, HEAD export against this one,
two runs): eighteen notches into the witness, HEAD 58.9–68.2 s with four refusals and the surround,
this 63.7–67.5 s with none and the page drawn (looked at); host high-water 6.08–6.43 GB against
4.71–5.86 GB. `--cpu` (exploratory): 1 201 × 1 501 windows in 139–309 ms. HEAD's first ten-notch
run (39.9 s) overlapped this round's tests, set aside. **Unfinished**: a list over a gibibyte past
the bound is refused, `--cpu` stands in no scaled window (ADR 1944 §4); GLib's report is the owner's.

**Files.** `viewer-confined` (manifest, `lib`, `protocol`, `worker`, its bin, `tests/confined.rs`),
`viewer-host/src/oversize.rs`, `quorra-confined{.rs,/screen.rs}`, `pdf-sandbox` (manifest, worker,
new test), `pdf-script-worker` (manifest's libc hunk, worker), `viewer-qt/tests/unsafe_position.rs`,
`tools/drive-windows.sh`, `doc/{ui-boundary,state-of-play}.md`, `doc/todo/30`, A430 (copied), ADRs.

**Gates.** `rustfmt --check` 0. Clippy `-D warnings --all-targets` 0 on every crate touched
(`viewer-host` without a neighbour's failing example). Nextest 0: 456, 192, 26 passed. `cargo test
-p conformance` 0; `state.sh docs` 0, `confined` 0 (0 killed). Drive, the export's release build:
`quorra-confined` whole 22 works, 1 not offered, 0 wrong; steps 82 and 84 works in all four windows
(84: 8 of 8). Clock hold 0, 626 s; `launch_path` 31 banded, 0 outside. Duration 8 800 s.
