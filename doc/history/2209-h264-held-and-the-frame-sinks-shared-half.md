# 2209 — H.264 held, and the frame sinks' shared half

Round 2209, RFC 0015 section 6 rounds 5 and 6 begun. ADRs 3537, 3538. Handed over.

## What changed
- Rec. ITU-T H.264 (V16) (06/2026) fetched into `doc/` (SHA-256 in ADR 3537). The VUI walk,
  E.2.1's inferences and Table E-5's pairs agree with it; two contradictions fixed with tests:
  Table E-5's codes 15–17 are defined, not reserved, and are refused by name; a picture-order cycle
  past 7.4.2.1.1's 255 is refused rather than read short.
- The sink's shared half (ADR 3538): `Players::AlsoVideoHere`, `Command::ShowsVideoHere`,
  `Recording::Mp4Video`, `Query::Screen` (C ABI `quorra_screen_quad`, header and C#), the confined
  wire; `decode_mp4_video_timed`; `viewer_host::video::VideoPlayer`; `render_raster::VideoSink`, a
  texture kept on the device; the `viewer-device` window's video layer. No window sends
  `ShowsVideoHere` yet, so no window's behaviour changed. No ledger row moved.

## Left, in order
1. `quorra`: send `ShowsVideoHere` beside `PlaysHere` (`quorra.rs`); a `VideoPlayer` field made at
   the first clip; `play` starts it for `Recording::Mp4Video` (the sound still goes to the sound
   player); `operate_media`, Escape and a closed document reach it; `about_to_wait` takes `due`,
   places it by `Query::Screen` plus the inset through `show_video`/`place_video`/`clear_video`,
   and folds `next_due` into its wait; the processor window draws `on_the_processor`'s raster.
2. GTK through `native_window`: the same, a cairo surface over the quad, a glib timeout at
   `next_due`; Qt and the C ABI keep handing video on (rounds 7–8).
3. An integration test playing `pdf-media-worker`'s I_PCM fixture through `VideoPlayer` to its end.
4. Ledger rows for §13.4's movie and Table 294 for video once 1–2 land.

**Gates.** fmt over the ten touched crates 0; clippy `-D warnings --all-targets` 0;
`cargo build -p pdf-sandbox -p pdf-media-worker --bins` 0; nextest `--lib --bins --tests` of the ten
under `xvfb-run -a`: 5224 passed, 4 failed (entry-point and query counts this round moved; fixed
and rerun); fuzz `cargo check --bins` 0; `cargo test -p conformance` rerun after the records' fix.

## Trap
A network stall paused the round for hours mid-edit; the wall clock is not the work's.
