# 1964 — The RX 580's bands are re-taken whole on a quiet machine

Session 1964, a single measure round, PDF/A lane's worktree. ADR 2742.

**The premise.** Rounds 1958 and 1963 owed five clocks, and the 07:40 hang marker blocked all of them.
The owner removed it on 2026-11-09. Every run here is unpinned, each its own `--clock` hold. The other
two lanes' builds held the heavy-walk lock until 17:15, at one-minute loads of up to 323.

**Item 5, the hardware nextest** (`render-raster`, `raster-gpu`, `viewer-device`). The first run was
pinned to RADV and failed 416 of 1002 at load 63. 410 of those failures asked for llvmpipe, which the
pin had removed (the new habit). The run was repeated unpinned at load 23: 1002 of 1002 passed. The
hook logged 289 holds, the longest 3.9 s, and 283 bring-ups on the card and 709 on `Cpu`. It wrote no
slow-section line and no marker.

**Item 1, `launch_path` under round 1958's bands**: six runs at loads of 6.5 to 10.5. Runs 1, 2, 4,
5 and 6 exited 0. Run 3 exited 101: the five-page row's warm open read 1.300 ms against a floor of
1.63, with its 2201 k instructions, 88.5 KiB and 25 reads inside their bands. That is the machine
quicker than the loaded 2026-11-03 runs, not the program. `bring_up_ms` read 51.7 to 54.3.

**Item 2, `turn_path`**: one run at load 6.0, exit 0. All 33 figures were judged, and all were inside
their bands.

**Item 3, `toolkit-launch` unpinned**: three retake runs at loads of 5.9 to 6.3. `quorra-gtk`'s
`device_up_ms` read 74.9 to 77.9, over the pinned ceiling of 72.4. That is ADR 2730's ICD cost.

**Item 4, and decided (ADR 2742).** Under ADR 2741's rule, round 1958's logs give what ADR 2741
printed. The six quiet runs judged 347 of 348 figures, and `tools/bands.py` over them, with the three
toolkit runs, rewrites `epyc-7301-32-rx580-vulkan/launch-path.toml` and `toolkit-launch.toml` whole.
`bring_up_ms` moves to 43.9 .. 65.3, the five-page row's warm open to 1.10 .. 2.39, and the font-walk
row's first page to 82.8 .. 132.2. The toolkit's `device_up_ms` moves to 63.6 .. 93.5 (GTK) and
54.4 .. 83.3 (Qt). `turn-path.toml` is kept. Under the new bands, one judged run of each gate exited
0, `launch_path` at load 4.9 and `toolkit-launch` at 6.2.

**Unfinished.** `render-gpu`'s scripted hardware run (ADR 2675) is still owed. The RX 580's OpenGL and
lavapipe machine files were not re-taken.

**Gates.** The clocks above; `cargo test -p conformance` 0 (471 passed) after `git add -N`;
`tools/state.sh --round 1964 docs` 0.
