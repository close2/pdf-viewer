# 3609 — A movie's poster is its first picture, decoded beside the window

Session 2227. Status: accepted. Context: RFC 0015 section 6; `doc/questions/A1006`; ADRs 1561,
3545, 3554, 3594; ISO 32000-2 §13.4 Table 306 `/Poster`.

## Decision

1. **The page renderer still draws no `/Poster true`.** Table 306: "If it is the boolean value
   true , the poster image shall be retrieved from the movie file". The renderer is confined and
   decodes no video, so `appearance::movie_poster` keeps refusing it, now saying that a window
   that shows video draws it.
2. **The viewer raises it.** For a host whose player shows video (`Players::AlsoVideoHere`), a
   movie annotation on a page that comes onto the screen (the arrivals Table 197's `/PV` is raised
   for), or already on it when the host says it shows video, whose `/Poster` is `true` and whose
   movie this program plays (an embedded MP4 file of `video/` type, `pdf_model::media::poster_movie`)
   raises `Event::Poster` with the movie file and the fit it plays with, **once for each annotation
   while its document is open**. Any other `/Poster true` is said once, as a note. Table 189's `/A`
   is about a click and is not read. No reader's level stands in front of it: the file never
   leaves this program, and a poster is what the page shows.
3. **The window decodes it.** `viewer_host::video::Posters` sends the file to a media worker of its
   own, on a thread started at the first poster, takes the first picture the worker hands on (the
   first presented once the edit list is applied, ADR 3554), stops the decode there, and converts
   it on that thread with `render-cpu`'s oracle conversion. A worker of its own rather than the
   player's, so a poster asked for while a movie plays does not wait for its pass. Held: at most
   `HELD_BYTES` (eight 1080-line pictures); one past it is said and not drawn; a document's go when
   it closes. A refusal says the poster is not shown and the rectangle shows what the page draws.
4. **Drawn while the movie is not playing**, fitted as the movie is: `Pictures::posters` leaves out
   the one whose movie is in hand. `quorra`: an overlay under the clip's picture, on either kind of
   window. GTK: a widget per poster, its texture made when the part shown changes. Qt: a frame
   after the pages. The C ABI window: `quorra_window_picture` and `_copy` hand the first poster
   placed where no clip's picture is shown — one at a time, which is all those calls hold — and
   `QUORRA_EVENT_POSTER` hands a bare viewer's caller the file.

## Consequences

While a poster is asked for and not answered, `Pictures::next_look` looks every `DECODING_LOOK`
as well as at the posters' thread's wake: in GTK's test the wake alone never brought a look, which
this round did not explain, and the timer costs nothing once the poster is answered.
§13.4 has no residue left on the poster. A second poster on the screen at once reaches a C host
only once the first leaves, until an index-taking picture call is added.
