# 1755 — A page turn's read leaves the toolkit thread, and Qt's lost size is told

Session 1755, slot 4 of batch 115, a host round; 2026-10-26. ADRs 2344, 2345; no `Q`. No row moves.

**Premise.** I worked in `scratchpad/r1755/`. Held: `NativeWindow::turn` read on the toolkit thread
(HEAD plus a key trace, Qt's key handled 1.92 s and 2.46 s late, GTK's past the step's 2 s).
Hypothesis held: the read is on a thread of its own, the retained picture is said in the key's own
pass, step 82 passes at 2 s. Not held as asked: Qt's 0,0 was the window's own, never moved — a size
Qt delivered while `busy_` was set was dropped (7 of 25 HEAD launches). Step 88's predated ADR 2146.

**Change.** `Viewer::read_out` hands the turned page's read out owned (`ReadOut`; `Open::fonts` an
`Arc`); no `settle` reads that page meanwhile, `Query::PageGeometry` places it, a read under a
changed view is dropped. `Ahead::read_out` reads beside its drawer; `take_the_turn` takes it in on
the drawing poll; `through_the_viewer` holds a second turn and all after it. GTK, Qt and the C
window share it, `quorra` does not. Qt and GTK trace `key … arrives`; `place_the_window` traces its
decision; Qt's `resizedTo` defers a busy-time size (`PageArea::tellSize`). Drive: step 82 waits 2 s
and prints its key trace, step 01 waits for the move, 88's first half is `88-archive-substituted`.

**Measured.** Step 82, this tree: key to command and retained picture 0.000–0.001 s, read
0.45–4.52 s, frame 1.11–4.94 s; GTK's first paint after the turn is the retained picture. In the
hold, key to frame 1.11–3.32 s against HEAD's 1.09–1.61 s where HEAD's showed one (the 3.32 s a
3.21 s read on the new thread, trap 154's cost). Qt: 14 of 14 launches centred at 1000×905.

**Gates.** On my export (HEAD plus my 18 source paths): `cargo fmt --check` on `viewer-core`,
`viewer-host`, `viewer-gtk`, `viewer-qt`, `viewer-ffi` exit 0 each; warnings-denied clippy
`--all-targets` on the five exit 0; `cargo nextest run` on the five 855 passed, exit 0, the C
driver and `turn_tests` among them; rustdoc `-D warnings` on `viewer-core`, `viewer-host` exit 0;
both viewer-core tests fail with their check planted out. Drive, dev binaries, four windows, steps
`drive` (01–33), 49, 60, 75, 81, 82, 83, 88, 97, 105, 106: 172 works, 1 wrong (Qt's 01 before the
deferral), 14 not offered; `drive` re-driven three times in `quorra-qt` and `quorra` after it: 234
works, 0 wrong. `tools/state.sh --round 1755 docs` exit 101, its three unresolved links in a
sibling's new `quorra-confined/worker.rs`. One `--clock` hold, HEAD's export against mine, each arm
after the load gate at 1.85 to 2.91: `t2-toolkit_launch` exit 0 and 0, 14 ratchets in band each;
`t2-launch_path` exit 0 and 0, 59 banded, 0 and 1 outside (quorra's turn below its band, not
judged); step 82 three times each, HEAD 4 of 6 works, mine 6 of 6. The six arms are not owed: no
`raster/` path, 0 viewer crates in `render-raster`'s `cargo tree`. In the worktree `cargo test -p
conformance --no-fail-fast` 463 passed, 1 failed, `records` on round 1758's record, not mine.
Every Xvfb stopped with TERM, no lock left.

**Unfinished.** `quorra`'s turn still reads on its event-loop thread (ADR 2344 section 4).
