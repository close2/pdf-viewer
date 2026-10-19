# 1566 — A refused lane is asked once, and two cold-open bands are re-taken

Slot 3 of batch ninety, 2026-10-15, a pixels round under the owner's switch to code improvement.
ADRs 1968 and 1969; no question; no ledger row moves.

**Premise.** It held: a pipeline refusal was asked again at every frame. **Hypothesis.** It held for
the arms, and a refused lane's frames cost the processor's draw alone. It failed in the window: a
planted frame's chrome went back under the refused lane and was refused at every frame, at HEAD as
well. That chrome is now drawn (ADR 1968 §2).

**Built.** A refused compute pipeline is kept as `Kept::Lane` for the window's device's life. Every
other pipeline refusal joins the frame memory (`Kept::Frame`). A frame of a refused lane goes to the
processor unasked, and its picture goes back under `upload_lane`. The trace names the lane
(`Landed::not_asked`). The two cold-open bands are the launch file's rule over three HEAD runs:
`bug1815476.pdf` 0.35 .. 0.56 ms, `opt_demo.pdf` 0.14 .. 0.28 ms (ADR 1969).

**Proof that nothing moved.** Four device arms from `tools/batch.sh export 1566 --as mine` matched
HEAD's arms by page digest: gpu and compute, 968 pages at 1x and 965 at 4x, 0 differing.
The control, HEAD's gpu-1x against its cpu-1x, read 106. Unplanted, every frame was the device's.

**Drive.** Under my own Xvfb `:66`, `--coverage compute`, on `tracemonkey.pdf` and
`PDF20_AN001-BPC.pdf`, with ADR 1962's plant in two exports only. HEAD asked the device 8 and 9
times (157.5 ms and 52.1 ms in all). It said `this page could not be drawn` 9 times on the second.
Mine asked once (8.5 ms and 7.5 ms), drew the chrome, and said the note once. Before the upload
change, my planted captures matched HEAD's with 0 pixels differing.

**Files.** `crates/viewer-ui/src/bin/quorra/renderer.rs`, `renderer/{device,lanes,window,gesture}.rs`,
`surface.rs` (the trace line), `doc/checks/launch-path.toml`, `doc/state-of-play.md`, ADRs 1968, 1969.

**Gates.** rustfmt clean on the six Rust files. From the export, clippy `-D warnings --all-targets
-p viewer-ui` exit 0 and nextest `-p viewer-ui` exit 0, 194 passed, 2 skipped. With the lane's
keeping planted out, the new test failed (trap 13). The four arms exit 0. One `--clock` hold, 119 s,
load 1.4 to 2.7, its output whole in the round's scratch (`hold.log`. `launch_path` on HEAD's export
exit 0 three times. On mine it exited 0 twice and 101 once, on two warm opens and `opt_demo.pdf`'s
0.407 ms, with the counted figures in band. `frame_budget` exit 0. `tools/state.sh docs` exit 0.
`cargo test -p conformance --no-fail-fast` exit 0.

**Unfinished.** The retained layer draws its whole-page pictures under `Cpu`. On a device where
raster's compute assist is on, each still asks the device once after the lane is refused, and then
takes the processor; nobody waits for them (`retained.rs`).
