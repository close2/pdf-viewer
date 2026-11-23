# Round 2207 — a video picture converted by its own VUI, on the device and on the processor

RFC 0015 section 6, round 4: YUV to RGB, in the `audit` worktree.

- `pdf-media-worker::vui` walks each sequence parameter set to its VUI (H.264 7.3.2.1.1 with its
  scaling lists, E.1.1), and each access unit's first slice to its sets; `video::Picture` carries a
  `Signal` (range, primaries, transfer, matrix, siting), Annex E's inferences where absent. A set
  cut inside its VUI is refused. The picture item's header grows five bytes; wire version 2.
- `pdf_render::video`: `Signal` → `Conversion` (Table E-5's pairs for 1, 4, 5, 6, 7, 9; both
  ranges; Figure E-1's six sitings), one affine formula over exact integers. Unspecified matrix:
  BT.601 to 576 lines, BT.709 above — a choice, flagged `matrix_stated: false`. Refused by name:
  0, 8, 10 to 14, siting above 5. Primaries and transfer carried, not applied (ADR 3529).
- `render_cpu::video::convert_picture`: the oracle, `f64`, rows on rayon.
- `QuorraRasterizer::convert_picture`: `video.wgsl`, three `R8Uint` planes, levels taken in
  integers into a storage buffer; compiled on the first picture; `last_video_cost`.
- Tests: the oracle against Annex E's equations written out (`render-cpu/tests/video_conversion.rs`);
  the device against the oracle (`render-raster/tests/video_conversion.rs`); the VUI through the
  session and the ring, Baseline and High with a scaling list (`pdf-media-worker/tests/video.rs`).

**Comparison (RX 580, RADV).** A channel may differ by one level only within 10⁻³ of a half level:
hand-built planes 13 450 536 channels, 67 at a tie; H.264.1, 84 streams, 788 pictures,
129 415 248 channels, 201 at a tie. A siting planted wrong in the shader failed the test. None of
those streams' first pictures states a colour description. The H.264.1 walk is unchanged: 67
EXACT, 52 handed off, 16 otherwise.

**Timing, 1920×1080, median of 40, under the lock.** Device 5.5–5.9 ms to upload, convert and
finish, 31.6 ms more to read back; processor 12.1 ms (default pool), 114 ms (one thread).

**Vello** (`render-gpu`) takes no video. **Ledger**: no row moved; clause 13's video rows stay
`out-of-scope` until a window plays video.

**Gates.** Over `pdf-render render-cpu render-raster pdf-media-worker viewer-host viewer-qt`:
fmt 0; clippy `-D warnings --all-targets` 0; `cargo build -p pdf-sandbox --bins` 0; nextest
`--lib --bins --tests` (`xvfb-run -a`) 1327 passed, 0 failed — `expected-failures-current.json`
empty, difference none (a first run's `gpu_lock` timing failure at load 45 passed on re-run);
rustdoc `-D warnings --document-private-items` (four crates) 0; `fuzz` `cargo check --bins` 0;
`cargo test -p conformance --no-fail-fast` 0.

**Left.** The sink per window (rounds 5 to 8); primaries and transfer onto the display; the
constant-luminance and `YCgCo` matrices.
