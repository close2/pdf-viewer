# 2211 — `quorra` and GTK show a clip's pictures in its screen annotation

Round 2211, RFC 0015 section 6 rounds 5 and 6 finished. ADR 3545.

## What changed
- `quorra` sends `ShowsVideoHere`; GTK through `Hearing::HereWithPictures`; Qt, C ABI hand on.
- `video::Shown::of` turns an `Mp4Video` sound into the player's clip; both windows start it beside
  the sound, route Table 218's stop, pause and resume, Escape, Space and a closed document to it.
- `quorra`: `about_to_wait` puts the due picture up and rests no longer than `next_due`; the device
  window gets the planes as a kept texture moved with the page each frame, the processor window an
  overlay of the oracle's raster over the quadrilateral.
- GTK: `video::Pictures` converts on the processor; a `GtkPicture` over the rectangle, the
  decoding thread's wake and a newest-only glib timeout.
- `VideoPlayer`: `next_due` answers a pass's end; a picture taken with the end counts as dropped.
- The `I_PCM` fixture is `pdf-media-worker/tests/support/pcm_stream.rs`, the document `showing.rs`.

## Tests
`viewer-host/tests/played_video.rs`, GTK's `host/shown_here.rs`, `quorra`'s headless window and
`pictures::tests`: each picture against the fixture's samples converted; end, stop, pause, resume.

## Ledger
§13.2.1 `out-of-scope` → `implemented`. §13.4 and §13.2.5 `implemented` → `partial`, and §13.2
with them as an aggregate: the poster from the movie file and Table 307's activation rested on the
video exclusion, and Table 291's `/F` is not applied to pictures. §13.1 stays out of scope on 3D.
Notes of §13.2.2, §13.2.6.1 and §12.6.4.14 restated. `doc/todo/65` maps the three.

**Gates.** Over the brief's seven crates and `pdf-media-worker`: fmt 0; clippy `-D warnings
--all-targets` 0 on the second pass; `cargo build -p pdf-sandbox -p pdf-media-worker --bins` 0;
nextest `--lib --bins --tests` under `xvfb-run -a` 1704 of 1705, the one a GTK test of this round,
fixed and the four video crates rerun; rustdoc `-D warnings --document-private-items` of
`viewer-host`, `viewer-gtk`, `viewer-ui`, `pdf-media-worker` 0; fuzz `cargo check --bins` 0;
`cargo test -p conformance` rerun after the frontier and records fixes.

## Left, in order
1. Table 291's `/F` for pictures (fit, or non-viable in MH).
2. §13.4's poster from an MP4 movie file and Table 307's in-place play.
3. RFC 0015 section 6 rounds 7–8 (Qt, C ABI), 9 (A/V sync), 12 (controller bar).

## Trap
A test window's media level is `ask` unless set `open`: the clip waits on an unanswered question.
