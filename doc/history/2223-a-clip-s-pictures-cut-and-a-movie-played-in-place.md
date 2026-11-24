# 2223 — A clip's pictures cut, and a movie played in place

Round 2223. ADRs 3593, 3594, 3595.

## What changed
- `viewer_host::video::timeline` (ADR 3593): §13.2.4.3's section and Table 291's `/D` and `/RC`
  cut a clip's pictures as `sound::passes` cuts its sound, on the same clock: each pass decodes
  from the section's start, a picture past the section's end or the pass's length is cut and the
  decode stopped, a longer duration holds the last picture, a fractional count cuts the last
  pass, and the next pass begins where the last ended. `Shown::cut` and its caveat are gone.
- `pdf_model::media::movie` takes the `Processor` (ADR 3594): for a host that shows video, an
  embedded MP4 `video/` movie is played here in the movie annotation's `/Rect`, told Table 307
  (`/Start`, `/Duration` in the `mvhd` scale or a stated one, `/Volume`, `/ShowControls`, `/Mode`
  Once, Repeat and Open, `/FWScale`, `/FWPosition`); Palindrome's backward half, `/Rate`,
  `/Synchronous` true and `/Rotate` are said. §12.6.4.10's Stop, Pause, Resume operate on it.
- `viewer_host::video::controller` (ADR 3595): the bar drawn on the processor — button, track,
  m:ss — over the rectangle's bottom in `quorra`, GTK and Qt; `quorra_window_controller` hands a
  C host the three facts. `Playing::controls` crosses the confined wire.

## Tests
`viewer-host`: five in `video/timeline.rs` (one runs `sound::passes` beside the pictures, every
pass end compared), four in `video/tests.rs`, four in `video/controller.rs`, one through the real
worker in `played_video.rs`; `pdf-model` two; `viewer-core`'s stop/pause/resume test expects
operations. Not exercised: a beep at a cut, the bar under a person's eye, a pointer on the bar.

## Ledger
§13.2.6.3: the excluded sentence replaced by the timing built; code and tests. §13.2.5 stays
`partial` (`/F 3`'s scrolling interface). §13.4 stays `partial`, on `/Poster true` alone.
§12.5.6.17, §12.5.6.18: notes. 59 sentences of 59 rows restated where they cited the playing of
video as excluded: 3D rows on 3D alone, rich media and §13.5 on their engines, eight implemented
rows on the player that now plays.

**Gates.** Over `pdf-model`, `pdf-media-worker`, `viewer-core`, `viewer-confined`, `viewer-host`,
`viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`: fmt 0; clippy `-D warnings --all-targets` 0;
`cargo build -p pdf-sandbox -p pdf-media-worker --bins` 0; nextest under `xvfb-run -a` 4686 of
4686, none against the manifest; rustdoc 0; fuzz `cargo check --bins` 0; conformance 0.

## Left, in order
1. Table 306's `/Poster true`, a host-side poster from the movie's first picture (`doc/todo/65`).
2. A pointer press routed to the bar, and Table 291's `/C` true with it. 3. A loopback beep at a cut.
