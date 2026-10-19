# 1595 — The confined processor screen draws a window where the toolkit windows do

Slot 4 of batch ninety-four, 2026-10-19, a host round under the owner's switch to code improvement.
ADR 2026; 2027 and Q490 not used. No ledger row moves: the change cites no clause.

**Premise.** Held: `Screen::take` and `Screen::fall_back` windowed a marks page only past
`MAX_PIXELS` (ADR 2016 §3). **The hypothesis did not**: `Turning::draws_a_window_of` is the
worker's question about a page that crosses as pixels; of a page whose marks cross the worker asks
nothing, and cannot know the processor draws it — `--cpu` and a refused frame are the window's. So
`worker.rs` is untouched and the screen asks `viewer_host::oversize::worth_a_window`, the toolkits'
own function, of the marks it holds: no frame, no greeting. Nor did "nothing changes for a page
under the rule" hold of the witness: its marks cross at the two notches before the bound, where
HEAD drew 600 MB and 935 MB rasters of it on the processor.

**Built.** `screen::drawn_as_windows`: past the bound always, under it where the toolkits' rule
holds at the window's extent, asked once for a drawing (one held whole, drawing, refused or as
windows keeps its answer), whole behind no extent; `fall_back` asks the same. The window says a
whole page in the toolkits' words and ends a window's line with its page. Step
`82-zoomed-window-processor` judges `--cpu` on the drive's A1 page.

**Files.** `crates/viewer-ui/src/bin/quorra-confined.rs` and `quorra-confined/screen.rs`,
`tools/drive-windows.sh`, ADR 2026, `doc/todo/30-a-native-host.md`, `doc/state-of-play.md`.

**Measured** (ADR 2026 §3). One `--clock` hold, 282 s, twenty notches: the witness 37.0 and 33.5 s
at HEAD against 35.9 and 22.9 s, inside the hold's noise; the A1 page 6.0–6.1 s both. Resident
high-water, a run of its own: the A1 page at fourteen notches 934 980 kB at HEAD, 32 016 kB here;
the witness at twenty 3 663 588 kB against 2 479 604 kB. Final photographs 0 pixels from HEAD's.

**Gates.** `rustfmt --check --edition 2024` on the two Rust files: 0. `-D warnings` clippy `-p
viewer-ui --all-targets` in the round's export: 0; `cargo nextest run -p viewer-ui` there: 204
passed, 2 skipped. Two plants (trap 13): the old rule fails two of the three new tests, a rule
asked every frame the third. `tools/state.sh --round 1595 confined`: exit 0, killed 0. The drive's
confined steps from the export on Xvfb `:195`: 18 works, 0 wrong, 10 not offered (script steps);
HEAD's `82-zoomed-window-processor`, the control: wrong, its first window at notch fourteen.
`cargo test -p conformance --no-fail-fast`: 58 binaries, 440 passed, 2 failed (`records`, on
1592's and 1597's records). `tools/state.sh --round 1595 docs`: exit 101 on slot 6's `raster-gpu`
mid-edit (`Limits::max_resident_bytes`); its rustdoc line in the export: exit 0. About 4 600 s.

**Unfinished.** A resize that changes no target keeps a drawing's answer, as the toolkits' does.
