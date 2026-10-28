# 2510 — A toolkit window holds the picture it had for one refresh after a zoom, and stands in only where the frame has not landed

Status: implemented. Session 1838, batch one hundred and twenty-four. Context: `doc/questions/A657`
("Q657 agree"), which adopts `doc/questions/Q657`'s recommendation and so re-opens the choice ADR 1843
made; ADRs 0384 and 0391 (`quorra` stands in only where its frame is predicted, then observed, to
miss the refresh), 1843 (a toolkit window scales the last frame into the extent the page has now),
2408 (the zoom lever this follows), 2416 (a column stands in from its last frame page by page);
`doc/todo/37`'s rule 5. Code: `crates/viewer-host/src/stand_in.rs` (`Hold`), `native_window.rs`
(`hold_the_pictures`, `mind_the_hold`), the three `frames()` call sites. Drive: `tools/drive-windows.sh`,
step `zoom_stand_in`, verdicts `82-zoom-stand-in` and `82-zoom-held`.

## 1. The rule

For one display period after a zoom command, a toolkit window paints each page whose zoomed frame has
not landed **as it was**: its own pixels, at their own size, at the origin they had when the command was
taken. That is the old picture of the page, which is what Q657 asks for, and it is not ADR 1843's defect,
which was old pixels at the *new* geometry. **The choice is per page**: in a column whose pages land
at different times, a page whose frame landed inside the period is painted at the new geometry beside
neighbours still held at the old one, and on a zoom in the two can overlap. That mixed arrangement lasts
at most one period, and it is what each page's own frame landing means; holding every page until the last
landed would hold a sharp frame back for a slower neighbour, and `quorra` shows a landed frame at once too. At the first look after the period it paints the scaled
stand-in ADR 1843 decided, and it asks for one repaint at that moment so the change is not left to the
next frame. A frame that lands inside the period replaces a sharp picture, not a blurred one.

## 2. What the decision rests on

1. **The geometry is recorded before the command.** `Hold::zoomed` runs in `through_the_viewer` ahead
   of `Command::Zoom`, because afterwards `Query::PageGeometry` answers the new extent and the one the
   pictures were shown at is gone. Only a frame that already has the extent its page has is recorded
   (an earlier stand-in is not a picture to hold).
2. **The period runs from the first command no frame has answered.** A second notch inside the wait
   does not restart it and does not replace the record, so a gesture whose frames never land inside a
   refresh stands in after one period and stays there, as `quorra` does, rather than showing the
   picture from before the gesture for as long as it lasts.
3. **A page holds only the frame it was recorded with.** Pixels of another size (a frame that landed
   for an intermediate magnification) are not the recorded ones and are scaled. The record is a vector
   sorted by `(document, page)` and found by bisection, so a 5 000-page document costs a logarithmic
   look per page (`stand_in.rs`'s test of that size).
4. **Retained pages stay out of it.** `Retained::fill` adds nothing beside a held picture: a page the
   hold is painting needs no 512-pixel reduction, and one that arrives with no pixels at all has no
   picture to hold, so it is blank for the period and gets its retained picture at the lapse, together
   with the scaled stand-ins of the pages that were held.
5. **Only a zoom command starts a hold.** `through_the_viewer` takes it on `Command::Zoom`; a fit
   (`Zoom::FitPage`, `FitWidth`, `FitHeight`) is a `Command::Zoom` and is held like a notch, because it
   changes the magnification the reader asked about. A resize or a rotation is another command and
   stands in at once as ADR 1843 decided: the reader did not ask to change the magnification, and a
   hold there is a latency with nothing bought by it.
6. **The hold is a function of the clock the window passes it.** `Hold` reads no clock of its own;
   every method takes the `Instant` it is asked at, so its tests run with none and the window's
   hosts pass `Instant::now()`.

## 3. What it costs

One refresh of latency before a stand-in appears on a notch whose frame misses, which is the price
`quorra` pays by design (Q657). The window polls its drawing interval while a look is owed
(`WindowState::drawing_interval` includes the time left of the period) and queries the viewer in
`mind_the_hold` only then. The held picture is a wrong picture shown on purpose under `doc/todo/37`'s
rules 1 and 3: the frame that lands replaces it, and the `frames` topic says it is held (`holds its …`)
as it says a stand-in (`stands in at …`).

## 4. What the drive proves

`zoom_stand_in` runs two documents under both toolkit windows. On a page whose frame misses
(`drive-zoom-slow.pdf`) the trace must carry `holds its` before `stands in at` (`82-zoom-stand-in`). On a
page whose frame lands inside the period (`drive-zoom-fast.pdf`, up to five notches, each retried) the
trace must carry no `stands in at` at all (`82-zoom-held`). The first proves the hold ends and the
stand-in follows; the second proves the hold removes the blurred refresh where it was the only thing a
fast frame had to show. The unit tests in `stand_in.rs` prove the rule without a clock: the period's
bounds, the second notch, a changed size, retained pages beside a held one, and the 5 000-page
bisection. `viewer-ffi`'s `hold_tests.rs` proves it through a real window: the pictures are held after a
zoom command went through the viewer, which they could not be if the record were taken after the
command, nothing is owed inside the period, and at its end exactly one repaint is owed and the pages are
painted as stand-ins.
