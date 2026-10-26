# 1763 — `quorra`'s turn leaves its loop thread, a spread and a `/Dur` advance are handed out, and step 66 was stale

Session 1763, slot 4 of batch 116, a host round; 2026-10-26. ADRs 2360, 2361; Q626 unused. No row moves.

**Premise.** I worked in `scratchpad/r1763/`. Held: `App::turn` read on `quorra`'s loop thread (HEAD plus
a key trace handled the key 1.280–1.337 s late); `anticipating` answered `None` for a spread's two unread
pages; `/Dur`'s tick went to `Viewer::handle` everywhere. Held: one hand-out serves the four windows, and
step 66 was stale (ADR 2076 changed the confined window's sentence, not the step). Not held: `quorra`'s
key-to-retained-picture under a frame — its retained pages need a list to be placed, so while the read
is out it keeps the page being left and draws no frame (ADR 2360 §2.4).

**Change.** `read_out` hands out every unread page, the first drawn beside its read; `anticipated` takes
them all. `viewer_host::loading::{may_turn, turned}`: a tick goes through the turn, is handed out where it
turned the page and waits behind a read that is out; the presentation pump's tick goes through
`through_the_viewer`. `Ahead::read_out` takes an `ended` wake, flag first. `quorra`: `App::turn` hands
out, `pump` holds what comes behind, `take_the_turn` lands it from `about_to_wait`'s `waiting_for_a_read`
(the opening's wait moved in, no removed line lost), `redraw_requested` declines while out, `pressed`
traces `key … arrives`. Drive: `82-turn-off-the-loop`; `turn_timeline` reads `quorra`; 66 greps the window.

**Measured.** Step 82's fixture in `quorra`, three alternations at loads 4.6 to 8.1: HEAD plus the key
trace, command 1.280–1.337 s after the key and frame 1.331–1.398 s; this tree, command 0.000–0.001 s,
read in 1.308–1.700 s, frame 1.364–1.764 s, settled on page one's red with no blue. `/Dur`'s one-second
slides, traced: pages two and three read off the loop thread in `quorra`, `quorra-gtk`, `quorra-qt`.

**Gates.** My export (HEAD plus my 13 paths): `cargo fmt --check` on `viewer-core`, `viewer-host`,
`viewer-ui` exit 0; warnings-denied clippy `--all-targets` on those and `viewer-gtk`, `viewer-qt`,
`viewer-ffi` exit 0; `cargo nextest run` on the six 994 passed, exit 0; rustdoc `-D warnings
--document-private-items` on the three exit 0; the two-page test fails with its set-up planted out.
Drive, dev binaries, four windows, steps `drive`, 49, 60, 66, 82, 83, 97–100, 106: 158 works, 2 wrong
(GTK's 20 and 24, after a 30.8 s step 23), 16 not offered; GTK's `drive` again: 38 works, 0 wrong. 66
works in all four; `82-turn-off-the-loop` 4 of 4 here, HEAD 0 of 3. Two `--clock` holds, loads 1.80 to
2.87, no render-node holder: `t2-toolkit_launch` exit 0 and 0; `t2-launch_path` HEAD 0, 0, 0, mine 101,
101, 0 — `opt_demo.pdf`'s `script_spawn_ms` 0.895 (HEAD 0.897 that hold) and `warm_open_ms` 0.143 (HEAD
0.141, 0.150), judged on mine and set aside by the child's calibration on HEAD; first pages 26.3–40.5 ms
against 28.0–39.2 ms. Six arms not owed: no `raster/` path, 0 viewer crates in `render-raster`'s tree.
`tools/state.sh docs` not run (the rustdoc above instead). Worktree `cargo test -p conformance
--no-fail-fast` 464 passed, exit 0. Every Xvfb stopped, no lock left.

**Unfinished.** `quorra`'s retained picture of the page arriving while it is read (ADR 2360 §4).
