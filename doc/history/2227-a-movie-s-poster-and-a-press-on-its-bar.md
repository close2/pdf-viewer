# 2227 — A movie's poster, and a press on its bar

Round 2227. ADRs 3609, 3610, 3611, 3612.

## What changed
- Table 306's `/Poster true` (ADR 3609): for a host that shows video, the viewer raises
  `Event::Poster` once per movie annotation on a page that comes onto the screen (or is on it when
  the host says it shows video) whose movie this program plays (`pdf_model::media::poster_movie`);
  any other is said. `viewer_host::video::Posters` decodes the first presented picture in a worker
  of its own and converts it on the processor; `quorra` (overlay), GTK (widget), Qt (frame) draw
  it in `/Rect` while the movie is not in hand, and a C host reads it through
  `quorra_window_picture` (one at a time). The page renderer's refusal now says who draws it.
- The event crosses the confined wire (kind 32) and the C ABI (`QUORRA_EVENT_POSTER` 33, count 34,
  the movie through `quorra_events_bytes`) (ADR 3611).
- Table 291's `/C` true is the controller bar over a clip with pictures; a clip without pictures
  keeps the MH-non-viable / BE-set-aside judgement (`uncontrolled`). A press on the bar is the bar's
  in `quorra`'s document and floating windows, GTK's document and floating windows, Qt's window and
  the C ABI window through `quorra_window_pointer`; the floating windows draw the bar (ADR 3610).
- `quorra`'s floating window is driven through winit's pumped loop on X11 (ADR 3612); CI runs
  `viewer-ui` under `xvfb-run`.

## Tests
`pdf-model` two (`/C`, `poster_movie`); `viewer-core` one; `viewer-confined` the round trip;
`viewer-host` one (`controller::at`); `quorra` two (poster and bar; the running loop); GTK, Qt and
the C ABI one each, all through the real worker. Not exercised: a person's eye on the poster, a
second poster through the C ABI, a press on GTK's floating window's bar.

## Ledger
§13.4 `partial` → `implemented`. §13.2.5 stays `partial` (`/F 3`); its note says `/C` is honoured.

**Gates.** Over the nine crates: fmt 0; clippy `-D warnings --all-targets` 0; bins 0; nextest
under `xvfb-run -a` 4695 of 4695 (an earlier full run lost the running-loop test to
`XOpenDisplayFailed` under load, so it runs as a retried child); none against the manifest;
rustdoc 0; fuzz `cargo check --bins` 0; conformance 0.

## Left, in order
1. An index-taking picture call, so a C host is handed every poster on the screen.
2. A press on the bar's track (seeking). 3. A loopback beep at a cut.
