# 3594 — A movie this program shows is played in its annotation and told Table 307

Session 2223. Status: accepted. Context: RFC 0015 section 6 round 14; `doc/questions/A1006`;
ADRs 2382, 3538, 3545, 3585, 3593; ISO 32000-2 §12.5.6.17, §12.6.4.10, §13.4 Tables 306 and 307.

## Decision

1. **Which movies.** `pdf_model::media::movie` now takes the `Processor`. For a host whose player
   shows video (`Players::AlsoVideoHere`), a movie whose Table 306 `/F` is an embedded file of an
   MP4 `video/` type is `played_here`: it goes over as a rendition's clip does, associated with
   the movie annotation, so `Event::Media`'s screen is the annotation and every window that draws
   a clip over its screen annotation draws this one over `/Rect` (`Query::Screen` reads any
   annotation's rectangle). Every other movie, and every movie for another host, is handed on.
2. **Table 307, an action's entries over the activation dictionary's** (§12.6.4.10).
   `/Start` and `/Duration` in the table's three forms — an integer, an eight-byte big-endian
   string, an array with a scale — in the movie's `mvhd` time scale where no scale is stated
   (`container::movie_time_scale`, which decodes the embedded file's header only then) become
   §13.2.4.3's section. `/Volume` is the level, silence below zero (ADR 3341's reading).
   `/ShowControls` sets `Playing::controls` (ADR 3595). `/Mode`: `Once` one pass, `Repeat` a count
   of zero; `Open`, "Play and leave the movie controller bar open", is read as an infinite
   duration — the last picture held, with the bar where it is shown, until the movie is stopped —
   **a reading**, since the table says nothing of the picture. `/FWScale` is a Table 295 floating
   window of `(n ÷ d) × /Aspect` pixels on the monitor holding the document window, with a title
   bar and close (ADR 3587); `/FWPosition` places it at the nearest of `/P`'s nine points, said
   where it lies between.
3. **Set aside, each said.** `Palindrome`'s backward half: the worker decodes forward, so the
   movie repeats forward. `/Rate` other than 1: the pictures follow the sound device's clock,
   which runs at the recording's rate. `/Synchronous` true: the window stays the reader's, and
   Escape dismisses the movie. `/Rotate`. `/FWScale` where the host opens no window, or no
   `/Aspect` is stated: played in `/Rect`.
4. **Operations.** §12.6.4.10's `Stop`, `Pause` and `Resume` become `MediaRequest::Operate` on the
   movie annotation, which reaches a movie this program plays and is "no effect" otherwise.

## Consequences

`Playing` carries `controls`, on the confined wire as one more boolean. Table 306's `/Poster true`
stays owed (`doc/todo/65`): the frame is the decoder's, which is not in the renderer.
