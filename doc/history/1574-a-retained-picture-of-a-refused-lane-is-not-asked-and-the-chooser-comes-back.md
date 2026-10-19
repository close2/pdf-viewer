# 1574 — A retained picture of a refused lane is not asked, and the chooser comes back

Slot 4 of batch ninety-one, 2026-10-19, a pixels round under the owner's switch to code improvement.
ADRs 1984 and 1985; no question; no ledger row moves.

**Premise.** It held in part. A retained picture was asked of the device whatever lane had been
refused. But under `Cpu` the compute assist takes only a tile the atlas will not hold, so a 512 px
picture reaches the refused pipelines only when the atlas is full. With ADR 1962's plant alone, all
9 pictures of `tracemonkey.pdf` were asked and drawn. **Hypothesis.** It held: the frames' rule
(`lanes::remembered`) sorts the retained layer's refusal, and no settled pixel moved.

**Built.** The render thread keeps the refused lanes (`RefusedLanes`), learned from every frame's
carried or new refusal and from the retained layer's own first ask. A retained picture of a refused
lane goes to the processor unasked (ADR 1984). Ctrl+Shift+A puts the archive chooser up again in all
three windows. Escape keeps the held choice, and the sentence said on choosing names the key. The Qt
dialogue lays its rows out at its 560 px width before measuring them (ADR 1985).

**Proof that nothing moved.** Under the planted export, with HEAD's routing restored by a variable as
the control: retained asks 9 → 1 on `tracemonkey.pdf`, 5 and 6 → 1 on `PDF20_AN001-BPC.pdf`. Under
`--coverage` auto, 0 for both. Settled captures with and without the control: 0 pixels differ. One
taken 2 s after the turns differed by 931 pixels from the same build at 8 s, a view still settling.
Unplanted, the device drew all 9. The gpu and compute arms at 1x from the export matched HEAD's by
page digest, 968 pages each, 0 differing; the control, HEAD's gpu-1x against its cpu-1x, read 106.

**Files.** `crates/viewer-ui/src/bin/quorra/renderer/{retained,lanes,thread}.rs`,
`crates/viewer-ui/src/bin/quorra/window.rs`, `crates/viewer-host/src/{keys,archiving,native_window}.rs`,
`crates/viewer-gtk/src/host/input.rs`, `crates/viewer-qt/src/host/input.rs`,
`crates/viewer-qt/cpp/window.cpp`, `tools/drive-windows.sh` (step 88), `doc/state-of-play.md`,
ADRs 1984, 1985.

**Gates.** rustfmt clean on the nine Rust files. From the export: `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` over viewer-host, viewer-ui, viewer-gtk and viewer-qt exit 0. nextest over the
four exit 0, 523 passed, 3 skipped. With `learn` planted to keep nothing, the new test failed (trap
13). Drive step 88 in the three windows: 21 works, 0 wrong. HEAD's `quorra-qt` read `88-archive-again`
wrong. `cargo test -p conformance` exit 0, 437 passed. `tools/state.sh --round 1574 docs` exit 0.
The two arms exit 0. `frame_budget` in one `--clock` hold from the export exit 0, 105 s, load 3.24.

**Unfinished.** The window's own `Refusals` does not hear a refusal the retained layer met, so a
`Cpu` frame still asks once after it. The chooser starts on PDF/A-2b and `refuse-any-loss`, not
on the held choice. The bridge does not carry the indices.
