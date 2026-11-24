# 3545 — `quorra` and GTK show a clip's pictures in its screen annotation

Session 2211. Status: accepted. Context: RFC 0015 section 6 rounds 5 and 6; ADRs 3297, 3329, 3433,
3529, 3538; ISO 32000-2 §13.2.1, §13.2.6.1 Table 294, §12.6.4.14 Table 218, §13.4.

## Decision

1. **Who says it.** `quorra` sends `Command::ShowsVideoHere` beside `PlaysHere` with the same
   answer. The toolkit windows share `native_window::Launch`, so the answer rides on
   `sound::Hearing`: a new `Hearing::HereWithPictures`, made by `Hearing::of_a_showing_window`,
   is GTK's; Qt keeps `of_a_playing_window` and the C ABI `HandsOn`, so both hand video on until
   their sinks are built (section 6 rounds 7 and 8). A variant rather than an eighth argument to
   `Launch::start`, which three hosts call.
2. **One description of a clip, read once.** `video::Shown::of(document, label, &SoundData)` makes
   the player's clip from an `Event::Media` sound whose recording is `Mp4Video`, and answers `None`
   for every other; both windows call it where they hand the sound to the sound player, so a
   clip's sound and its pictures start together and an operation reaches both.
3. **`quorra`.** `App::video` is made at the first clip with the loop's waker. `about_to_wait`
   puts the due picture up before the loop rests and folds `next_due` into the rest the clocks
   chose (`Wait` becomes `WaitUntil`, a sooner `WaitUntil` stays). On the device window the planes
   go to `show_video` and the layer moves with the page at every frame (`place_the_picture`, before
   the arrangement); on the processor window `on_the_processor`'s raster is the lowest overlay,
   placed by mapping §8.9.5's unit square onto the quadrilateral. Escape and Space reach the
   pictures beside the sound; a silent clip's pictures take either key alone. A closed document
   stops its clip.
4. **GTK.** `video::Pictures` (in `viewer-host`, shared by any toolkit window) converts on the
   processor: a GTK page is a texture GTK draws, so a device picture would be read back, which the
   oracle's conversion does not cost. The picture is a `GtkPicture` in the page layer, kept last,
   over the bounding box of `Query::Screen`'s quadrilateral, re-placed whenever the pages are. The
   decoding thread's wake invokes GTK's main context; a one-shot timeout at `next_look` shows a
   picture queued early and ends a pass; only the newest timeout acts (a counter), and a look that
   finds the window borrowed is retried 5 ms later rather than lost.
5. **The pass's end has a time of its own.** `VideoPlayer::next_due` answered only the next
   picture's time, so once a pass's decode had ended a window that slept until `next_due` was never
   woken to take the last picture down or start the next pass. It now answers the end, one interval
   after the last picture.
6. **Tests.** `pdf-media-worker`'s hand-built `I_PCM` stream moved to
   `tests/support/pcm_stream.rs` so that `viewer-host`, `quorra`'s headless window and GTK's
   harness play the same clip; `tools/conformance/test-support/showing.rs` is the document. Each
   picture is checked against the fixture's samples converted by `render-cpu`, never against the
   decoder's output.

## Ledger

§13.2.1 leaves `out-of-scope` for `implemented`: its list is of capabilities, every one a "may",
and the player it names may be "the interactive PDF processor itself", which it now is for video.
§13.1 stays out of scope on 3D artwork alone. §13.4 goes from `implemented` to `partial`: its
note rested a poster "retrieved from the movie file" and Table 307's activation on the video
exclusion, which no longer holds, so both are owed (`doc/todo/65` bucket 4).

## Costs

The GTK picture fills the quadrilateral's bounding box, so a page turned by a quarter shows an
upright picture there; the device and processor layers in `quorra` follow the quadrilateral.
Table 291's `/F` fit is not read: a picture fills the rectangle. The pictures run on the monotonic
clock beside the sound (round 9). No person has watched a clip through either window; the tests
run under a virtual display.
