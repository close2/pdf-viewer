# 1649 — The idle build fills a window opened zoomed, so its steps keep runs

Slot 3 of batch one hundred and one, 2026-10-20, a "pixels" round: `doc/todo/75` item 3's first
remainder, round 1610's "Unfinished". ADR 2134; no Q file; no ledger row moves (no clause is cited).

**Premise, held**: a first frame that culls keeps no runs, so every step from a window opened zoomed
walks the witness's 4 398 932 lines again (ADR 2056). **Hypothesis, held in effect, not in size**: the
opened window now steps as the fit's window does, but raster had no walk that culls nothing at a view
that does not show it, so the build is raster's (`Before::Whole`, `encode_whole`, `prepare_whole`,
`Device::build_whole`) and `render-raster`'s (`build_whole` on the window renderer and the
rasterizer) beside the thread's thirty lines; no message is sent. **The brief's instrument was not the
figure's**: `window_against_whole` draws on the processor; `zoom_frame` gained `ZOOM_FRAME_IDLE`.

**Built**: the idle build is the frame walked again culling no line, drawn into a texture nobody
presents, its encode put in the held one's place; last in the idle turn (the sharp pass and the page
drawn small each replace the page's encode), under the step's lane, after the view has been still as
long as the frame took; declined once a scene where it keeps nothing, writes records (the record road
keeps no runs) or passes the residency budget. Planted back, raster's and the thread's tests fail.

**Measured** (one `--clock` hold, load 3.2–5.6, Radeon 890M, `Compute`, 4 and 24 threads, two
passes): a window opened at 2.4, step to 3.0 154.6–194.0 → 121.3–131.4 ms, to 3.6 143.8–174.8 →
118.3–124.7 ms, encode about halved, 84.2 → 18.8 MB with no line byte; the fit's steps in the same
hold 120.5–138.5 ms. **First frame unchanged**: digest `e0ff5b9a63ba9363` in both trees and from the
idle build, 531.4–574.4 against 532.2–554.9 ms; every step's digest HEAD's. The idle build 259.7–276.4 ms
read back included, while nobody waits.

**Files**: `raster-gpu`'s `src/{encode,retained}.rs`, `src/device/render.rs`,
`src/encode/{kept_lines,line,instance,encoded}.rs`, `tests/retained_replay.rs`; `render-raster`'s
`src/{lib,present}.rs`, `examples/zoom_frame.rs`, `tests/idle_build.rs` (new); `viewer-device`'s
`src/renderer/{thread,device}.rs`; ADR 2134; `doc/todo/75`.

**Gates.** `cargo fmt --check` 0 and `clippy -D warnings --all-targets` 0 (`raster-gpu`,
`render-raster`, `viewer-device`); nextest 933 passed, 6 skipped; rustdoc `-D warnings`, private items,
`raster-gpu` and `viewer-device` in the export, exit 0; the four device arms against `/home/AI/arms-1647/`
by page (968, 965, 968, 965), 0 differing, 0 one-sided, exit 0; `t2-raster_golden` 2 passed, exit 0; one
`--clock` hold exit 0 — `zoom_frame` two builds × 2 thread counts × 2 passes, the turn gate 33 of 33
inside in both trees, the launch gate 53 banded, 0 outside, 9 not judged (7 carve-out); `cargo test -p
conformance` 1 failed (`worker_features`: slot 5's `mozjpeg-rs` brings `bytemuck`), the rest passed. **Unfinished**: the vertex
stage still draws every kept line (0–4 ms); the record replay keeps no runs (ADR 2134 §5); not driven
under Xvfb. 6 900 s.
