# 3538 — A clip's pictures are timed on the host and kept on the device: the sink's shared half

Session 2209. Status: accepted, the windows' wiring open. Context: RFC 0015 section 6 rounds 5 and
6; ADRs 3342, 3349, 3407, 3433, 3513, 3527, 3529; ISO 32000-2 §13.2.2, §13.2.5, §13.2.6.1 Table
294, §12.6.4.14 Table 218.

## Decision

1. **The core judges a video clip for this program's player only where the host says it shows
   video.** `pdf_model::media::Players::AlsoVideoHere` admits an embedded MP4 clip whose `/CT` is
   `video/` (`container::is_mp4_video`) beside the sound clips; `viewer_core::Command::ShowsVideoHere`
   sets it together with `PlaysHere`, and the chosen clip crosses as `Recording::Mp4Video`. A host
   that never sends it (the C ABI, Qt, and today every window) hands video on as before. The
   confined wire carries the command (tag 56) and the recording (kind 4).
2. **Where.** `Query::Screen(annotation)` answers the screen annotation's `/Rect` as a window
   quadrilateral, only while a placed page carries it (Table 294's "rectangle occupied by the screen
   annotation"); the C ABI answers it as `quorra_screen_quad`, which round 7's sink will use.
3. **When.** `MediaWorker::decode_mp4_video_timed` hands each picture on with its sample's
   decoding time (ISO/IEC 14496-12 8.6.1.2); no `ctts` and no video edit list is read, so that is
   the presentation time only where the track states none or only reorders. `viewer_host::video`'s
   `VideoPlayer` decodes on a thread of its own, at most `AHEAD` (3) pictures ahead, and the window's
   `due(now)` hands on the newest picture whose time has come on Part 1's `MediaClock` (run as its
   freeze runs, on the monotonic clock), counting each one it passes over as dropped late; it never
   waits. Stop, pause and resume act as on a sound (ADR 3433); a resume decodes from the start and
   shows nothing before the paused place. `/A` false holds the clip; `/RC` repeats whole passes.
4. **Converted where it is drawn.** `render_raster::VideoSink` runs ADR 3529's pass and copies the
   output buffer into an `Rgba8Unorm` texture it keeps; nothing is read back.
   `viewer_device::renderer::Window` takes the sink at its split, with the medium, and draws the
   texture as a layer over the quadrilateral, under the chrome (`show_video`, `place_video`,
   `clear_video`). A window on the processor converts with `viewer_host::video::on_the_processor`,
   `render-cpu`'s oracle. `VideoSink::read_back` exists for the oracle comparison only; read back,
   the kept texture equals `convert_picture`'s pixels.

## Approximate until §6 round 9, and said

The pictures' clock and the sound's device clock start together and are not locked; `/D` and a
§13.2.4.3 section govern the sound and not the pictures (the sentence says so); a resume costs the
decoding of what was shown.

## Open

`quorra` and GTK do not yet send `ShowsVideoHere`, pull `due`, or draw the layer: the wiring is
left to the next round, in the order `doc/history/2209-h264-held-and-the-frame-sinks-shared-half.md`
lists it. Until then no window's behaviour
changes.
