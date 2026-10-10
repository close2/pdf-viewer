# 1492 — A refused page is paid once

Slot 3 of batch seventy-eight, 2026-10-10, a loop round on the project owner's "improve performance",
zooming especially. ADR 1821. No ledger row: the owner's target, not a clause.

**Premise, held, and three costs it did not name.** `draw` asked the device, raster built and priced
the scene, then `compose_pages` drew the arrangement uninterruptibly. Unnamed: the device was asked
again every frame (1.30–1.45 s an attempt; 2.3 s cold), the sharp pass asked it after every fallback
frame (its prediction read the raster upload, 3–5 ms), and the retained page asked it after every
frame and never once got one ("0 of 0 retained"). A gesture ran notch one's 10.0 s frame to its end.

**Hypothesis, held**: a processor raster lands as the page texture and `Stale::settled` takes it, so
the stand-in reprojects it already; what was missing was not asking again, and stopping. **Built.**
`Window::refusals` (pictures + linear part, 32 kept, budget/list refusals only) sends a known
refusal straight to the processor; the job carries an `Interrupt`, honoured before and during
`compose_pages_interruptibly`; `Window::superseded` raises it when the magnification moves, a
departure from ADR 0657's composer rule argued in the ADR; no sharp pass and no retained-page retry
after a refusal; `Window::overlong`/`abandon` put `mind_a_long_draw`'s warning and Escape on it; the
note is said when the title changes. The settled view after ten notches: AE 0 between arms, seen.

**The figure** — ten `plus` notches 300 ms apart on the witness, last notch to the first real frame
after it (`scratchpad/r1492/zoom.sh`, `analyse.sh`; arms HEAD `24f758fd…` and HEAD + this change
exported apart `42bc0b09…`, final `35a71eb6…` differing only in where the note is printed). **Xvfb
(llvmpipe), `--clock`: before 10.94, 18.91, 11.08 s; after 3.15, 3.91 s.** **Owner's display (890M,
loop, inside one clock hold, keys pressed only while the window was active): before 9.96, 9.43 s;
after 3.65, 3.75 s.** Of the 3.65 s: 1.3 s finishing an attempt of a magnification already left,
1.3 s the device asked at the last one, 1.2 s the processor.

**Unfinished.** The device is still asked once per new magnification, uninterruptibly (predicting a
refusal before the scene is built is raster's); a refused page's retained picture is not drawn on
the processor; no drive step, since no fixture of the drive's reaches 256 MiB of scene in its time.

**Gates.** `rustfmt --check --edition 2024` on the four files: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p viewer-ui --all-targets`: exit 0. `cargo nextest run -p viewer-ui`: 174 passed, 2 skipped;
the new `a_frame_the_device_refused_before…` fails with the sharp suppression planted out (trap 13).
`cargo test -p conformance`: 430 passed, 0 failed. `launch_path` (`--clock`, twice): exit 101, 31
banded, 1 then 2 outside — allocation high-water *under* its floor (22.36 MiB against 23.0, 11.37
against 11.5), on a shared tree carrying siblings' `pdf-render`, `render-cpu` and `raster-gpu` edits;
the test compiles nothing of `bin/quorra`. `tools/drive-windows.sh` whole (`--tree 12`, 1 223 s
held): exit 0, 270 works, 0 wrong. Queues: 961, 25, 383, 976 and 2 435 s. Duration in the report.
