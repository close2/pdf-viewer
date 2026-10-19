# 1582 — A refused lane is heard on both sides, and the chooser keeps its place

Slot 6 of batch ninety-two, 2026-10-19, a host round under the owner's switch to code improvement.
ADRs 2000 and 2001; Q477 asks the owner one thing; no ledger row moves.

**Premise.** (1) It held. Under ADR 1984's three plants, `tracemonkey.pdf#zoom=1500` draws on `Gpu`,
the idle retained picture is refused under `Cpu`, and the first `Cpu` frame fourteen notches out
asked the device again. (2) It held for Qt alone; `quorra`'s card and GTK already start on a
`Choosing`'s indices. (3) The hypothesis, the witness stays on the list, failed on the figures.

**Built.** ADR 2000: the render thread tells the window a lane the retained layer heard first
(`Finished::Lane`). Its own record (`RefusedLanes::said`) answers a job asked before the window read
it. ADR 2001 §1: `Choosing::starting_on` starts Ctrl+Shift+A's chooser on the held choice in three
windows, and the Qt bridge carries the two indices. Drive step 88 gains `88-archive-again-held`.

**Measured, not built** (ADR 2001 §2). Two clock holds on the witness in `quorra-confined`, the list
against the worker's window. Twenty notches took 46.4–48.4 s against 40.7–41.2 s (one run 58.5 s). A
press's next frame took 166–180 ms against 9–10 ms. Every device frame on the list was inside `WARN`,
so moving them to the processor needs the owner's reading of ADR 1831 (Q477).

**Proof that nothing else moved.** `Finished::Lane` is sent, and the record answers, only after the
retained layer hears a lane refused. Plants in one binary, HEAD's routing the control, two runs each:
refused asks at fit 1 in both, zoomed out 2 at HEAD and 1 here. The drive of the four windows: 320
works, 0 wrong, 10 not offered. The eight witness runs' final photographs differ by 0 pixels. `cargo
tree -p render-raster` holds none of the four crates touched, so no corpus arm can see the change.

**Files.** `crates/viewer-ui/src/bin/quorra/{renderer.rs,window.rs}` and `renderer/{lanes,retained,
thread,window,gesture}.rs`; `crates/viewer-host/src/{archiving,native_window}.rs`; `crates/viewer-qt/
src/{bridge.rs,host/archiving.rs}`, `cpp/window.cpp`; `tools/drive-windows.sh` (three hunks in
a patch in the round's scratch); ADRs 2000, 2001; `doc/questions/Q477`.

**Gates.** rustfmt `--check` on ten Rust files: 0. From the export, `-D warnings` clippy
`--all-targets` over viewer-host, viewer-ui, viewer-gtk, viewer-qt: 0. Nextest over them: 525 passed,
3 skipped. One earlier GPU test timed out under contention and then passed alone in 0.17 s. Each new
test fails when its fix is planted out (trap 13). Step 88 in three windows: 39 works, 0 wrong. `cargo
test -p conformance`: 440 passed; then only `records` failed, on 1579's 41-line record. `tools/
state.sh --round 1582 docs`: 0. Clock holds: 330 s and 392 s, exit 0. About 7 400 s.

**Unfinished.** The gpu and compute arms: their binary, for a digest against `arms-1577`, queued all
round. `doc/state-of-play.md` (slot 5's) owes ADRs 2000 and 2001.
