# 2360 — `quorra`'s turn leaves its loop thread, and a spread and a `/Dur` advance are handed out

Status: implemented. Session 1763, 2026-10-26, the host slot of batch one hundred and sixteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses: ISO
32000-2 §12.4.4.1 (`/Dur`) and Table 29 (`/PageLayout`'s two-page values); no row moves. Context:
ADRs 1871, 1957, 2328, 2344; traps 154, 263.

## 1. What was left

ADR 2344 took a turn's read off the toolkit thread in `quorra-gtk`, `quorra-qt` and the C window,
and named three reads still on a loop thread: `quorra`'s own `App::turn`, a turn whose arrangement
brings on more than one unread page (`Viewer::anticipating` answered `None` and the host read them
in place), and §12.4.4.1's advance, which turns the page inside `Command::Tick`, a command none of
the windows sent through the turn.

## 2. Decision

1. **One hand-out, every unread page.** `Viewer::read_out` hands out the page the view stands at
   and, after it in page order, every other unread page of the arrangement. The first is the one
   foretold and drawn beside its read; the others are read after it on the same thread, outside
   the drawing's hand-over, and `Viewer::anticipated` takes them all in under the same checks. No
   `settle` reads any of them while the read is out, and `Query::PageGeometry` places each. A page
   holding §12.5.3's replacement is still read in place, because its read is not the first read.
   `anticipating`, which page one's launch uses, is unchanged.
2. **A tick is a turn where it turned the page.** `viewer_host::loading::may_turn` names `GoTo`
   and `Tick`. A tick is handled with its read left to the window. If its events say
   `PageChanged`, which is §12.4.4.1's `/Dur` or §12.4.4.2's node stepping to another page, its
   read is handed out as a key's is. Otherwise `read_the_rest` ends it at once, so a script timer's
   re-read of a page stays where `Viewer::handle` made it (ADR 1957). **A tick now waits behind a
   read that is out**, where ADR 2344 handled one at once: a tick that turns would hand out a
   second read beside the first, which is what section 2.4 of that ADR exists to stop. The
   presentation pump sends its tick through `NativeWindow::through_the_viewer`, the one place a
   toolkit window hands the viewer a command.
3. **`quorra`'s `App::turn` is `NativeWindow::turn`'s construction**, reusing `Viewer::read_out`
   and `Ahead::read_out`. `pump` holds a second turn and every command after it, in order.
   `Ahead::read_out` takes a closure the reading thread calls when the read ends, however it ends,
   after a flag `Turning::ended` reads. `quorra` passes its event-loop proxy's wake, and the
   toolkit windows pass nothing, because they poll at `drawing::POLL`. `about_to_wait` takes the
   read in (`App::take_the_turn`). It hands the drawing to the renderer first and the commands that
   waited after it. While the read is out the loop does nothing else and waits to be woken.
4. **`quorra` draws no frame while the read is out.** Its arrangement is built from the render
   requests the core has made, and for a page being read there is none, so a frame would put the
   window's ground where the page is. `App::redraw_requested` declines while the read is out. The
   window keeps the picture it last presented, and the read taken in asks for the redraw. Nothing
   is dropped: the presentation's clock carries the real elapsed time at its next tick (trap 263).

## 3. Measured

Drive step 82's fixture in `quorra`: two pages of six hundred thousand fills each, `Right` and then
`Left`. The step is `82-turn-off-the-loop`, new in `tools/drive-windows.sh`. The binaries alternated
three times. HEAD with only this round's key trace added (`key … arrives`) handled the key's command
1.280–1.337 s after the key, because the read held the handler, and its frame came 1.331–1.398 s after
the key. This tree handled the command 0.000–0.001 s after the key. The read was taken in off the
event-loop thread 1.308–1.700 s after the key and the frame came 1.364–1.764 s after it; every
settled photograph held page one's red block and none of page two's blue. The frame is up to 0.37 s
later, which is trap 154's cost again (ADR 2344 section 3): a read off the thread the system
placed may land on a slower core. It is taken because the loop answers while it is paid.

§12.4.4.1's `/Dur` on the drive's three one-second slides, traced in each window: pages two and
three were each read off the loop thread, in `quorra`, `quorra-gtk` and `quorra-qt`. The core's
tests `a_turn_that_brings_on_two_unread_pages_hands_both_out` (Table 29's `TwoPageLeft`, pages
three and four) and `a_tick_that_advances_the_page_hands_its_read_out` hold the two hand-outs to
what the command handled whole asks for. The first fails with the set-up asked for no other page.

## 4. What it leaves

- `quorra` shows the page being left while the turned page is read, not its retained picture of
  the page arriving. Its retained pages (ADR 0443) are placed under a frame's arrangement, which
  holds a page only with its display list, and both surfaces would need a stand-in drawn from a
  placement with no list. That is its own change.
- `quorra-confined` reads in its worker process (ADR 1827) and is untouched.
