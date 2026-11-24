# 3578 — A window driven from C hands a clip's picture to its host as RGBA, on the host's timer

Session 2219. Status: accepted. Context: RFC 0015 section 6 round 8; ADRs 0247, 2168, 3217, 3538,
3545, 3553, 3555, 3562; A979 (the unsafe allowlist); ISO 32000-2 §13.2.1, §13.2.6.1 Table 294,
§12.6.4.14 Table 218.

## Decision

1. **The host says it draws pictures, and nothing else turns it on.** `quorra_settings_show_pictures`
   sets it on the settings a window opens under; `quorra_window_open_under` then opens with
   `Hearing::of_a_showing_window`, so the core is told `PlaysHere` and `ShowsVideoHere` where the
   sound helper is beside the host's program, and a clip in a screen annotation plays in the
   window. A clip's sound goes through the player its pictures follow (ADR 3555), so a host that
   draws pictures has its sounds played by this program too. Off in a new handle and in
   `quorra_window_open`: such a window hands every clip to its host's opener, as before (ADR 3217),
   because a window whose host draws no picture would show a clip nobody sees. Where the helper
   is missing the window hands clips on and says why (ADR 3417).
2. **What crosses is an RGBA buffer, never a texture handle.** `quorra_window_picture` answers the
   part Table 291's `/F` shows — its size, `QUORRA_FORMAT_RGBA8`, its byte count, the top-left
   corner of the bounding box of the screen annotation's quadrilateral and that box's extent, in
   device pixels of the viewport, placed from `Query::Screen` at each call (ADR 3553's fit) — and
   `quorra_window_picture_copy` copies it into a buffer the host owns. This window draws on the
   processor and holds no device, so it has no texture to give; and a device object crossing a C
   boundary would be one the host could free, resize or touch from another thread while the
   library still held it, with nothing in C to say so. The picture is the one
   `viewer_host::video::Pictures` converted on the processor, as Qt's is (ADR 3562).
3. **The event is a repaint bit.** `QUORRA_REPAINT_PICTURE` (8) in a `QUORRA_OWED_REPAINT` record:
   a picture came up, a new one is due, it moved with the pages, or it came down — read the
   picture again, or take it down where `quorra_window_picture` answers `QUORRA_NO_ANSWER`. A bit
   in an existing record rather than a new record kind, so `QUORRA_OWED_KIND_COUNT` and every
   compiled host stay as they are; an old host ignores a bit it does not know. The picture is not
   digested as a page's pixels are (ADR 2168): it changes up to sixty times a second and each new
   one is known new where it is taken, so the window counts the pictures it takes, and a repaint
   for a picture alone digests no page.
4. **Timed on the host's timer.** Nothing wakes a C host from the decoding thread — a callback into
   the caller is what ADR 0247 refused. `quorra_window_interval` takes the clip's next time,
   `Pictures::next_look_unwoken` (`VideoPlayer::next_due_unwoken`: the next picture's time, or 4
   ms while a pass runs with nothing queued), and `quorra_window_tick` looks. Escape and Space reach
   the clip as in Qt and GTK, Table 218's stop, pause and resume through the shared
   `NativeWindow` route, and closing the clip's document ends it.
5. **WinUI.** The tree holds the WinUI host (`hosts/windows/Quorra.App`): it now states the word,
   and on `QUORRA_REPAINT_PICTURE` reads the picture, converts it to premultiplied BGRA as it does
   a page, and shows it in one `Image` over the pages and under the marks and controls. The C# is
   compiled by CI's Windows job alone; nothing on this machine builds it.

## Cost

Three entry points (`tests/unsafe_position.rs`: 390 and 371), no `unsafe` block and no token
outside `src/abi.rs`'s positions. A clip running with nothing queued asks the host's timer for a
tick every 4 ms until its first picture is decoded, Qt's cost. The `/F` part is cut out twice where
it is not the whole picture — once for its facts, once for the copy — at the size of a picture.

## Tests

`crates/viewer-ffi/src/window/picture_tests.rs`: a clip plays to its end, each picture one of the
fixture's samples converted, inside the screen annotation's box, a `QUORRA_REPAINT_PICTURE` for
each change, a short buffer refused, the timer stopped at the end; a pause holds the picture,
Table 218's resume and Space show it again, Escape stops it, a stop with nothing associated says
so, and the document's close ends it; a host that did not say it draws pictures is handed the
clip. `c/drive_a_window.c` states the word and reads `QUORRA_NO_ANSWER` and `QUORRA_NULL_ARGUMENT`
through the C header.
