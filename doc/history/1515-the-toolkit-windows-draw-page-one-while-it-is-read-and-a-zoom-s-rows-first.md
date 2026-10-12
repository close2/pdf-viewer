# 1515 — The toolkit windows draw page one while it is read, and a zoom's rows first

Slot 1 of batch eighty-two, 2026-10-12, a host round on the owner's performance target; the
partial-row word yields to it. ADRs 1867, 1868; no question. No ledger row moves.

**Premises** held: both windows called `Viewer::anticipate`, `schedule` asks whole pages. The
fourth call was not one line: before the join a window's viewer has no geometry, so a look carries
the forecast origin (`Foreseen::page`, `Look::placed`). **Hypothesis (b) did not hold**: no protocol
change is needed (the host knows its rows; the drawing thread's row order is its own), and the band
as part of the frame lost — planner-cut edges cost 230–490 ms of one-thread planning beside a
533–783 ms whole draw, no sooner on screen. Built instead: the rows shown drawn first as a look, the
page whole after as the frame, byte for byte HEAD's (ADR 1868 §1). **Found by photograph, not
trace**: Qt's first build showed no look while its trace and step 81's ink test said one; fixed.

**Built.** Both windows: `Ahead`, `window_is`, the join at `GLIMPSE_AFTER`, the look until a frame,
`draw_ahead`; `Drawing::ask_showing`/`seen_drawn`, `band_of`/`draw_band`, a partial over a
stand-in, a band counted once; step 82's `band_first`; `examples/band_first`.

**Figures** (witness, HEAD → this, two runs, s). First look: Xvfb GTK 3.32, 3.46 → **0.565, 0.562**,
Qt 3.46, 2.50 → **0.532, 0.533**; owner's display GTK 2.53, 2.60 → **0.536, 0.538**, Qt 3.49, 3.23 →
**0.535, 0.533**. First frame: Xvfb GTK 3.38, 3.55 → 2.70, 3.20, Qt 3.60, 2.55 → 2.16, 3.11; owner's
GTK 2.58, 2.63 → 2.19, 2.19, Qt 3.63, 3.29 → 2.22, 3.22, inside the read's 2.0–3.1 s spread. A notch
showing under half the page, to its rows finished: Xvfb GTK 0.57–0.63 → **0.35–0.38**, Qt 0.59–0.74
→ **0.30–0.40**; owner's GTK 0.56–0.62 → **0.30–0.36**, Qt (scale 1.59) 0.69–0.82 → **0.36–0.40**;
the frame lands 0.29–0.42 s later; shallower notches 0.48–0.57 in both arms.

**Unfinished.** `quorra`, `quorra-confined` not driven (untouched; budget). The planner is half a
zoomed processor draw (236–331 of 574–699 ms), `render-cpu`'s lever. A toolkit turn reads on its
thread. **Files.** `crates/viewer-core/src/viewer.rs` (`Foreseen::page`), `crates/viewer-host/src/
{loading,drawing}.rs`, `crates/viewer-host/examples/band_first.rs` (new), `crates/viewer-{gtk,qt}/
src/host.rs`, `tools/drive-windows.sh`, `doc/todo/30`, `doc/state-of-play.md`, ADRs 1867–1868.

**Gates.** `rustfmt --check --edition 2024` (six files), `bash -n`: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy` on the four crates `--all-targets`: exit 0. `cargo nextest run` on them:
689 passed. `cargo test -p conformance --no-fail-fast`: exit 0, 432 passed. Hold 1 (queued 406 s,
held 380 s, exit 0): Xvfb A/B, `band_first`, steps 80–81 (8 works); `launch_path` exit 101, 5
outside — `bring_up_ms` 25.223 > 20.6 (slot 3's item), four `first_page_ms` 0.3–2.0 ms over;
`quorra` untouched. Holds 2, 3 (owner's display, photographs, Qt re-take): held 128, 206 s, exit 0.
Drive, both toolkit windows (`--tree 6`, held 740 s): 174 works, 1 wrong (Qt's bar sentence, fixed:
4 works re-driven); `band_first` 2 works, HEAD 2 wrong (the control). Duration 10387 s.
