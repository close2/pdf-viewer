# 1545 — A frame goes up as it lands, where its refresh has had no present

Slot 6 of batch eighty-six, 2026-10-14, a loop round under the owner's switch to code improvement.
ADR 1926; no question; no ledger row moves.

**Premise.** Held: the frame's send had no wake, and the sharp pass, a look and a scene each had one.
**Hypothesis.** Failed twice. First, the wake on its own moved nothing. Under Xvfb, AN001's turns read
21–22 ms with it and 17–18 ms at HEAD, and the frame was collected one period after the key in both.
`Cadence::owed` counted a tick that put nothing up as a present, so a redraw the wake asked for was
deferred. Second, `launch_path` and `turn_path` cannot see the change. The first leaves out winit's
loop and present, and the second is `render-raster`'s, which does not link `viewer-ui`.

**Built.** The wake after `Finished::Frame`. The cadence keeps the tick (`due`) apart from the first
moment a present may be made (`presentable`). Only a present moves `presentable`, `owed` moves only
the tick, and `redraw_requested` asks `may_present`. A look put up as built counts as a present. A new
test fails with the old `owed` put back.

**Measured.** One `--clock` hold under Xvfb (60 Hz floor, llvmpipe), on the six launch documents,
three runs an arm, alternating. Key to frame: AN001 21–26 ms (median 23) became 9–12 ms (median 11),
WTPDF 15–21 ms (median 18) became 8–17 ms (median 10), and ISO 32000-2's later turns went from a median
of 36.5 ms to 32.5 ms. Page one moved inside the spread (ADR 1926 §4). The confined window is untouched:
the `quorra-confined` and `pdf-view-worker` digests are the same at HEAD and here.

**Unfinished.** (b), the owner's display. The permission classifier refused the `LockedHint` probe and
the loop script that would unlock, so nothing was queued and no figure was taken. Round 1531's
`owner.sh` names a Wayland socket that does not exist. winit 0.30 panics on that (`NoCompositor`, its
own 10:58 logs), so re-queued unchanged it measures nothing. `tmp/r1545/{head,mine}/quorra` and
`tmp/r1545/owner.sh` are placed for the loop. The `--cpu` composing thread still wakes nothing.

**Files.** `crates/viewer-ui/src/bin/quorra/{cadence,surface}.rs`, `renderer/thread.rs`;
`doc/todo/36`; ADR 1926; this record.

**Gates.** rustfmt clean on the three files. `clippy -p viewer-ui --all-targets -D warnings` exit 0.
`nextest -p viewer-ui` exit 0, 189 passed. `cargo test -p conformance` failed 1 of 398: `names`, on
slot 3's `viewer-qt/src/host.rs:182`. One `--clock` hold, 298 s, exit 0. The drive of `quorra` and
`quorra-confined` exit 0, 107 works and 2 wrong. Against round 1542's HEAD drive, only 38 (both windows,
to `wrong`) and 82 (confined, to `works`) moved, both with sibling edits to the script (slots 2 and 3).
The confined binary is HEAD's. Duration 6 100 s.
