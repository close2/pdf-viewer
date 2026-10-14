# 1547 — A zoomed toolkit page is drawn as its window where the window is half of it or less

Slot 2 of batch 87, 2026-10-14, a host round on ADR 1892 §4's build: ADR 1930, no Q, no row moved.

**Premise** held: both windows drew and copied the whole page under `MAX_PIXELS`. **Hypothesis**:
the path held, the bound did not — a window is drawn from its own culled list, and on the witness
one of nine tenths of the page landed 60–130 ms after the page drawn whole, one of two fifths
180–220 ms before it (`examples/window_against_whole`, exploratory); the bound is "the largest
window is half the page or less", ADR 1868's half for the same reason.

**Built** (ADR 1930). `oversize::oversized` takes the viewport; a window drawn for one request no
longer answers the next at the same target (a new ink was never drawn — planted back, the new test
fails); a notch back keeps the last window standing in until the whole frame lands; the first
window is a frame (report, open sequence, placing, face, first-frame line). Step 82's `band_first`
became `zoomed_window`; the band (ADR 1868) stays for pages drawn whole.

**Measured** (one `--clock` hold, `scratchpad/r1547/hold.sh`, Xvfb, two runs, HEAD export against
this round's, ten `=` notches into the witness). Notches 1–4: the same sizes in both arms, drawn
whole, 329–425 ms. Notches 5–10: GTK HEAD 393–948 ms, this 339–126 ms; Qt HEAD 366–930 ms, this
357–132 ms; the copy into the toolkit at notch 10 5.2–5.6 ms against 0.3–0.4 ms. Window pixels
against the page drawn whole: ≤ 2 levels in ≤ 1 920 pixels at notches 2–8; at 1.25¹⁰ 914 by more
than 8, at most 56, at stroke edges, either sign — the same drawing looked at (trap 1); ADR 1930 §4.

**The owner's GTK finding** (round 1528's owner logs): the fourteenth notch in GTK's 1046 × 1146
window is 14 075 × 9 942, under `MAX_PIXELS`, so the window path was not reached and the whole page
landed 15.3–15.4 s later; this round windows it from the fifth notch. Owner's display not taken.

**Proof nothing moved under the bound.** Notches 1–4 drawn whole at HEAD's sizes in both hosts;
`cargo tree -p render-raster -e normal,dev,build` names no `viewer-host`/`-gtk`/`-qt`, so the arms
cannot move (not re-run), and `quorra`'s launch path calls none of it.

**Files.** `viewer-host` `src/{oversize,drawing,native_window}.rs`, `tests/oversized_pages.rs`,
`examples/window_against_whole.rs`; `viewer-{gtk,qt}` `src/host.rs`, `src/host/{drawing,react,stand_in}.rs`;
`tools/drive-windows.sh`; ADR 1930; `doc/state-of-play.md`; `doc/todo/30`; this record.

**Gates.** `rustfmt --check --edition 2024` exit 0 on every file; `RUSTFLAGS="-D warnings" cargo
clippy -p viewer-host -p viewer-gtk -p viewer-qt --all-targets` exit 0; `cargo nextest run` of the
three 317 passed, exit 0; `cargo test -p conformance` exit 0. Drive of all four windows: 286 works,
2 wrong (the new step's grep, fixed; rerun works in GTK and Qt, HEAD's control wrong in both);
clock hold exit 0, 106 s. Six arms and `launch_path` not run (proof above). Duration 8 100 s.
