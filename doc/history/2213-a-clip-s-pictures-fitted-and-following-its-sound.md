# 2213 — A clip's pictures fitted by Table 291's `/F` and following its sound

Round 2213, RFC 0015 section 6 round 9 and §13.2.5's `/F`. ADRs 3553–3556; ADR 3526 amended.

## What changed
- ADR 3526 carries the owner's A1027 (2026-11-24): `wide` and `safe_arch` stand in the worker's
  scalar build, no upstream request, no fork.
- `/F`: `pdf_model::media::Fit` in `Playing`, across the confined wire; `viewer_host::video::placed`
  fits meet, slice, fill and hidden in `quorra`'s device and processor windows and GTK; 5 is meet;
  3 is MH-non-viable and BE-set-aside (no scrolling interface). The device layer had mapped texel
  space as a unit square — one sample stretched over the rectangle — and now uses the sample affine.
- Timing: the worker reads `ctts`, `stss` and the video `elst`; a picture is presented at its
  composition time through the edit list (`client::video::Timing`).
- Sync: the helper reports `Progress::Heard` with each `Played`; `MediaClock::heard_time`;
  `Player::listening` and `VideoPlayer::play_beside` slave the pictures to the sound as heard.
- Resume: `decode_mp4_video_from` begins at the last sync sample at or before the paused place.

## Measured
`AUDIO_IN_USE_ROUTE=loopback AUDIO_IN_USE_ONLY=av tools/audio-in-use.sh`, 120 s Main-profile clip:
slaved median −8.0 ms (−31.5 to 46.6), drift +3.2 ms, 0 dropped; control on the monotonic clock
−113.5 ms, drift −14.2 ms (ADR 3556).

## Ledger
§13.2.5 stays `partial`, now on `/F 3`'s scrolling interface alone; its note, tests and code list
restated, §13.2's aggregate note and `doc/todo/65` with it.

**Gates.** Over `pdf-model`, `pdf-media-worker`, `quorra-sound`, `viewer-confined`, `viewer-host`,
`viewer-device`, `viewer-ui`, `viewer-gtk`, `viewer-qt`, `render-raster`: fmt 0; clippy `-D warnings
--all-targets` 0; `cargo build -p pdf-sandbox -p pdf-media-worker -p quorra-sound --bins` 0; nextest
`--lib --bins --tests` under `xvfb-run -a` 4140 of 4141, the one `render-raster`'s `gpu_lock`
`a_processor_s_adapter_never_waits` under load, passing alone; rustdoc 0; fuzz `cargo check --bins` 0;
`cargo test -p conformance`: names and gates fixed, then the record's length, `--test records` 0.

## Left, in order
1. A scrolling interface over a screen annotation for `/F 3`.
2. §13.4's poster from the movie file and Table 307's in-place play.
3. RFC 0015 section 6 rounds 7–8 (Qt, C ABI), 10–12.

**Trap.** An empty exported variable is still present to `var_os`: a switch is set or unset.
