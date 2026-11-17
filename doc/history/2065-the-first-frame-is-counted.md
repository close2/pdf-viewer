# 2065 — The first frame is counted beside the open

Session 2065, a measure round alone in the audit worktree. No ledger row; ADR 2944. The slice is
round 2055's "Left" (ADR 2926): a counted first frame, banded, with a plant.

**The premise.** It held. `open_kinstructions` is the only instruction figure in `doc/checks/`, and
its child stops after `Command::Open` with no viewport. `quorra`'s sequence is as the brief said:
`open_document`, then `anticipate` on the document thread (through `viewer_host::Ahead`), then at
the join `receive` (whose `Opened` obeys Table 29's `/PageMode`) and the first `Resize` at
800 × 1000 logical less the panel. The gate's scale is 1.0.

**The figure (ADR 2944).** `frame_kinstructions`: a child, `count-first-frame`, runs the open,
`Viewer::anticipate`, the page mode's panel and that `Resize`, and stops at the `NeedsRender`
request; the device's encoding and draw are not counted. Three rows open a 300-pixel panel. Counted
as ADR 2666 counts the open, it was a lottery: the decode ahead of the `Do`s (ADR 1321) raced the
interpreter, the font-walk row read 66.7 to 91.8 million over twelve children (its images decoded
twice in the high ones), and idle rayon workers moved the 57-page row 6 %. With
`RAYON_NUM_THREADS=1` in the child the decode ahead declines and four children agree to 0.005 %.

**The bands.** This file's rule over four children of the gate's build: `PDF20_AN001-BPC`
10731 .. 10949, WTPDF 40789 .. 41617, ISO 32000-2 156099 .. 159254, `bug1815476` 64104 .. 65400,
`xfa_filled_imm1344e` 3204 .. 3269, `opt_demo` 3063 .. 3126. No other band moved. The first bands
came from a build with no `pdf-sandbox-worker` (trap 10): one of the font-walk row's images is
decoded there, and without it the page drew 1490 commands, not 1522, and counted 63549.0. Judged
run 1 failed on that row alone (64752.1), and the bands were retaken from the gate's own build.

**The plant.** A loop of a million `black_box` additions in `visibility_of`, after its no-viewport
return, applied as a patch and reversed. The gate exited 101 with all six `frame_kinstructions`
outside (each about 4000 thousand over: 14841.6, 45201.8, 161677.2, 68752.2, 7236.5, 7094.6) and
all six `open_kinstructions` inside, unchanged to 0.1 thousand.

**Gates.** fmt and `rustfmt --check` 0. clippy `-D warnings --all-targets -p viewer-ui` 0. nextest
`-p viewer-ui --lib --bins --tests` under `xvfb-run -a` 0 (143 passed, 1 skipped). `launch_path`'s
counted half: runs 2 and 3 exited 0, 36 figures banded, none outside, at loads of 42.68 and 29.82,
on the hardware adapter. `cargo test -p conformance --no-fail-fast` after `git add -N`: 101, from
this paragraph's placeholder and `bounded`'s self-test ("one sample cost 1040 ms"); after the fix the
whole run exited 0 (474 passed).

**Left.** Whether the decode ahead duplicates the interpreter's decode at native speed, as it did
under callgrind on the font-walk row, is unmeasured. The window's default size is still unargued.
