# 3562 — Qt shows a clip's pictures through the frame sink the pages cross

Session 2215. Status: accepted. Context: RFC 0015 section 6 round 7; ADRs 3329, 3417, 3545, 3553,
3555; A979 (the unsafe allowlist); ISO 32000-2 §13.2.1, §13.2.6.1 Table 294, §12.6.4.14 Table 218.

## Decision

1. **Who says it.** Qt's launch makes its `Hearing` with `of_a_showing_window`, as GTK's does, so
   the core is told `ShowsVideoHere` and a clip in a screen annotation plays in the window rather
   than going to this machine's player.
2. **The pictures cross the frame sink, and nothing else crosses.** `crates/viewer-qt/src/bridge.rs`
   has one hand-written `unsafe extern "C++"` block (`tests/unsafe_position.rs`), and its
   `append_frame` already copies a checked RGBA raster into a `QImage` C++ owns and paints at a
   rectangle. `Host::collect_frames` appends the picture after the pages, so it is painted over them:
   the picture `viewer_host::video::Pictures` converted on the processor (ADR 3545's reason: this
   window draws pages as images too), fitted by Table 291's `/F` with `Pictures::placed` (ADR
   3553), shown over the bounding box of the part's quadrilateral — GTK's construction — and placed
   from `Query::Screen` at each paint, so it follows a scroll or a zoom. The part `/F` shows is
   copied out only where it is not the whole picture. No bridge function, no C++ line and no
   `unsafe` token are added.
3. **No wake.** A wake from the decoding thread into `QApplication::exec` would cost a second
   hand-written `unsafe` token, `plays_sounds`'s reason (ADR 3329). The timer `drawing_wait` arms
   takes the clip's next time instead, and `drawing_pump` looks at the pictures each turn. For a
   window no wake reaches, `VideoPlayer::next_due_unwoken` (through `Pictures::next_look_unwoken`)
   answers `next_due`, or — while a pass runs and the decoding thread has queued nothing —
   `video::DECODING_LOOK`, 4 ms, a quarter of a 60-per-second picture's interval: a picture is up at
   most that late. A paused or stopped clip answers `None`, and the timer stops.
4. **The keys and the close.** Escape stops the clip and takes its picture down, Space resumes a
   held or paused clip — each beside the sound, and a silent clip's pictures take the key alone, as
   in GTK. Table 218's stop, pause and resume reach it through the shared `NativeWindow` route,
   which calls `look_at_the_pictures`. Closing the clip's document stops it; closing the last ends
   the player.

## Cost

A clip running with nothing queued wakes the window every 4 ms until its first picture is decoded:
a channel poll per turn, against the second `unsafe` token the alternative costs. The picture is
copied once a paint by the sink, as every page is.

## Tests

`crates/viewer-qt/src/host/tests/shown_here.rs`, GTK's `shown_here` in this window: a clip plays to
its end, each picture one of the fixture's samples converted, placed inside the screen annotation's
box, the timer armed throughout and stopped at the end; a pause holds the picture, Table 218's
resume and Space show it again, Escape stops it before its end, a stop with nothing associated says
so, and the document's close ends the clip and takes its picture down.
