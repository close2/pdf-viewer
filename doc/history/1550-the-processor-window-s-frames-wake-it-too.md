# 1550 — The processor window's frames wake it too, and the owner's display is measured

Slot 5 of batch eighty-seven, 2026-10-14, a loop round under the owner's switch to code improvement.
ADR 1936; no question; no ledger row moves.

**Premise.** Held: the composing thread sent `Finished::Frame` and `Finished::Partial` and woke
nothing. The cadence half did not hold as a task: the window's one `Cadence` already serves `--cpu`,
so `may_present` and the look's present (ADR 1926 §3) were in force there at HEAD.
**Hypothesis.** Held: the composer's frames gain up to one period. The second half failed. The
owner's session was open from the orchestrator's word on, and `xdotool search --onlyvisible --name .`
read 0 on the open session too. Only a control window the loop script opened told open from locked.

**Built.** The composing thread wakes the loop after each frame and each look, through a closure over
the window's proxy given to `Composer::new`. The test `a_finished_frame_wakes_the_loop_after_it_is_sent`
fails at its 60 s bound with the frame's wake taken out (planted and reverted).

**Measured.** Clock hold 1, under Xvfb at 60 Hz with `--cpu`, HEAD export against this round's, three
runs an arm. Key to frame, as medians: AN001 37 → 30 ms, WTPDF 21 → 17 ms, ISO 32000-2 38.5 → 36.5 ms.
Every run's first present was 3–11 ms earlier. The owner's loop then wrote "locked": no visible
windows and no visible control window. Clock hold 2 came after the owner unlocked, through the loop
at 120 Hz. It ran all four scripts in order, and each was pressed: `tmp/r1545/owner.sh`,
`tmp/r1550/turns.sh`, `tmp/r1531/owner.sh` (its line now `-u WAYLAND_DISPLAY -u WAYLAND_SOCKET`) and
`tmp/r1528/owner.sh`. None found the session locked again. ADR 1926's owed device figure, as medians:
AN001 12 → 9 ms, WTPDF 14 → 7.5 ms. This round's `--cpu` turns: AN001 48 → 44 ms, WTPDF 30.5 → 24 ms,
and ISO 32000-2 46 → 50.5 ms, which overlaps and is not judged. Round 1531: four zoom runs pressed,
logs in `tmp/r1531/owner-*-zoom.log`. Round 1528: in Qt, HEAD's witness drew no frame of the last
notch in 60 s and said 19–20 refusals after 30 pointer moves. That round's binary drew it 0.14–0.16 s
after the notch and said none. GTK drew no frame of it in 60 s in either arm.

**Files.** `crates/viewer-ui/src/bin/quorra/{composer,surface}.rs` (surface: the one constructor
call); `doc/todo/36`; ADR 1936; this record; outside the tree, `tmp/r1531/owner.sh` (the line and its
header comment), `tmp/r1550/{owner,turns}.sh` and `tmp/r1550/{head,mine}/quorra`.

**Gates.** rustfmt clean on both files. `clippy -p viewer-ui --all-targets -D warnings` exit 0.
`nextest -p viewer-ui` exit 0, 190 passed. `cargo test -p conformance` exit 0, 436 passed. Mid-round
it failed 1, `every_quotation…`, on slot 3's `xfa.rs`/`notes.rs`. Clock holds: 149 s exit 0, and
the owner's 825 s exit 0. Drive of `quorra` and `quorra-confined` (HEAD's script, both arms): exit 0,
109 works and 5 not offered in each, no step moved. `quorra-confined` digest 0b118ff3796b825a in
both arms. Duration 6 041 s.
