# 1747 — The C title carries its notes, a presentation holds `quorra`'s cards, and step 82 waits for the read

Session 1747, slot 3 of batch 114, a host round; 2026-10-25. ADRs 2328, 2329; no `Q`. No row moves.

**Premise.** I worked in `scratchpad/r1747/`. Held: the C window's title had no `title_notes`;
`quorra`'s cards were put up over a presentation; GTK's `82-retained-page` failed on reruns; the C
driver printed none of the three titles. Not held: step 82 did not lose its Left key, and the race
was not the camera starting late. GTK's key trace and a stack taken mid-wait show the key reaching
the window's controller every time and the turn back reading page one on the toolkit thread (ADR
1886, trap 154) for 4.9 s and 9.7 s in two probes, then the retained picture painted for about 0.7 s
before its frame; the step's 2 s ceiling and its "rasterised" check (said 0.55 s before
`RenderReady`) are the race. `quorra-confined` presents nothing, so only `quorra` had cards to hold.

**Change.** `Window::title` appends `title_notes()` (ADR 2328); `drive_a_window.c` places the fit
fixture with `positions = 0`, clicks a presenting deck's link under `--links=ask` and answers after
Escape, and draws the union's strokes as `--declined`, planted and not; the C driver test holds each
line. `quorra` holds a question, a script's question and the password prompt while full screen in
the pump's words (now `presentation_pump::{QUESTION,SCRIPT,PASSWORD}_WAITS`) and puts them up one at
a time from `about_to_wait` (ADR 2329); step 106 runs in `quorra`. Step 82 waits for page two's
frame and for the sentence under 30 s, counts a photograph late only after `RenderReady`, and in GTK
reads the window's `raster-paint` record for page one's first paint after the turn.

**Measured.** HEAD's step 82 in GTK 0 of 6 at loads 8.4 to 12.5, this one 4 of 4 at 11.5 to 17.6
and Qt 2 of 2; the paint reader names a retained paint, a frame and none on a recorded log; the
notes planted out of the title, the C program fails at its uncentred step.

**Gates.** On my export (HEAD plus this round's ten paths): `rustfmt --check` per file 0; `cargo
fmt --check` for `viewer-ffi`, `viewer-host`, `viewer-ui` 0; warnings-denied clippy `--all-targets`
on the three 0 (101 twice on the C test's length and two borrows, fixed); warnings-denied rustdoc
0; `cargo nextest run` 74 of 74, 300 of 300 and 137 of 137, the C driver among them. The drive,
dev binaries, steps `drive`, 90, 46–48, 97–100, 105, 106, 89 and 82 in four windows: 173 works,
1 wrong, 13 not offered; the wrong is Qt's `01-center-window` (0,0 when read, twice, at loads 30
to 40), whose code and placement decision this round does not touch. `t2-toolkit_launch`, one
`--clock` hold on the export at load 31.2: exit 0 and 0, with ten of fourteen ratchets past their
high edge at that load (no quiet arm taken; this round changes neither window's launch). In the
worktree `cargo test -p conformance --no-fail-fast` 459 passed, one failure in `round_numbers`,
round 1744's new example's first line; `--test evidence --test adr_status --test records` 2, 2 and
2 of 3 passed, the third round 1748's record. Every Xvfb stopped with TERM, no lock left.

**Unfinished.** The turn's read still freezes the window for its length (trap 154's question).
