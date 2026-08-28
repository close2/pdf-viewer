# ADR 0713 — The first host on the confined boundary

Status: accepted, 2026-08-28. Session 775. Takes the step `doc/todo/15` has pointed at since the
tier change completed: an actual window whose pages arrive from the sandboxed worker process.
Cites no clause: this is `CLAUDE.md` principle 3's architecture reaching a person, and the ledger
is untouched.

## What was owed

Road B's machinery is complete and had no consumer. The transport carries every command, every
question and both payload arms (ADRs 0218, 0223, 0607, 0626, 0633); the worker draws nothing it
does not send (`Rendered::Listed`, ADR 0640); the host-side draw of the marks arm is stoppable
(`pdf_render::Interrupt`, ADR 0650) and the policy for stopping it decided (ADR 0657) — and ADR
0657's own survey ends with the sentence this round exists to retire: the boundary the mechanism
was built for is `viewer-confined`'s, *"and no host takes that road yet."* Every page a person
has ever looked at in this program was interpreted in the window's own process.

## Decision 1: a program of its own, `pdf-viewer-confined`, in `viewer-ui`

The host is a new binary rather than a flag on an existing window, and it lives in `viewer-ui`
rather than in a new crate or in `viewer-confined`. Three alternatives were each refused for a
reason worth keeping:

- **Not `pdf-viewer --confined`.** A mode flag on the flagship promises the flagship: fourteen
  thousand lines of chrome, panels, forms, selection and reprojection, all of it built around an
  in-process `Viewer` whose `Answer`s borrow from it and whose render loop runs on
  `NeedsRender`/`RenderReady` — two messages that deliberately never cross this boundary. A flag
  that delivered a fraction of that would be a placeholder path wearing an option; honestly
  moving the flagship is several rounds of work and is recorded in `doc/todo/15` as what remains.
- **Not a binary in `viewer-confined`.** A binary's dependencies are its crate's, so the window
  would put winit and softbuffer into the transport crate — and every future consumer of the
  boundary would carry a toolkit it does not want. The dependency runs the right way round as it
  is: `viewer-ui` (a host) depends on `viewer-confined` (a transport), and `pdf-viewer` names
  nothing from it, so the flagship's launch path links no transport.
- **Not a new crate.** `viewer-ui` already holds exactly the three things this window needs and
  nothing else does: a winit window, `viewer_ui::software`'s processor-only presentation, and
  `viewer-host` beside it. A crate for one binary would be a new part of the tree (ADR 0709's
  sweep population) bought for nothing.

**The cost of the placement is an instrument's, and it fired before it was paid.**
`tools/state.sh windows` counts what each window reaches by grepping the host *crates*, so a
second window inside `viewer-ui` was counted as the first: the section reported the tier-2 window
asking `Query::Frame` — the payload question that host's own reading row correctly says it never
asks — and the `SPENT` check fired on a reason that had not been spent. `names_in_code` excludes
the confined window's sources now, with the argument in its comment; the confined window becomes
a column of its own when there is a population of confined windows to rank, not before.

## Decision 2: the smallest complete host, with its scope stated

The window opens one document, shows Table 29's arrangement as the worker's viewer decides it,
turns pages, scrolls, zooms, reports what a page could not draw, and aborts. It has no panels, no
form controls, no selection, no find bar and no password prompt. Two rules make that scope honest
rather than a shortcut taken silently:

- **Everything outside the scope is refused by name.** A password-protected document gets a
  sentence naming the three windows that can prompt; a file the document asks for and a URI it
  wants resolved are declined in words; a page the rasteriser refuses keeps its refusal on
  standard error. Nothing falls back and nothing goes quiet (trap 5).
- **The level-hosts rule is not extended to this window, and that is argued rather than
  assumed.** `doc/todo/30`'s "all three hosts stay level" is the owner's decision about the three
  established windows, and it is about *chrome on the shared boundary* — a feature living in one
  host is a message nobody has tested. This window adds no message and no chrome; what it carries
  is the *other* boundary, which no established window is on at all. Extending levelness to a
  fourth window would make every UI feature cost a third more while forbidding the confinement to
  ship until a full-parity host exists — the attrition reading. The moment an established window
  moves onto the confined boundary, this program's reason to exist as a separate binary starts to
  expire, and that is the right way for it to go.

## Decision 3: `viewer_host::drawing` took a type parameter instead of being copied

The marks arm hands this window exactly the problem ADR 0668 solved for the native windows — an
unbounded draw on the toolkit's thread — and the arrangement there is the right one here: a queue
keyed by page, one job in flight, an interrupt raised where a newer request for the same page
arrives or the page leaves the arrangement, a poll that does not exist at rest, and the launch's
settle budget (ADR 0678). What this host cannot hold is the *request type*: a
`viewer_core::RenderRequest` carries a `RenderToken` only a `Viewer` can mint, and the viewer is
on the far side of a pipe.

So the request is now the parameter — `trait DrawRequest` (page, list, target), implemented by
`RenderRequest` and by this window's `Draw` — and the arrangement is not. `Drawing<R = RenderRequest>`
means every existing field and call in the two native hosts compiles unchanged; `POLL` and
`SETTLE` moved from associated constants to module constants because an associated constant on a
generic type cannot be named without saying which `Drawing` it is of, and the numbers are the
same for every one of them. The alternative — a private copy of the queue in the new window — is
the second copy `viewer-host`'s own charter names as where two hosts stop agreeing.

Nothing about the abandonment rule changed, and trap 20's half holds without a viewer to freeze:
an abandoned draw is answered to nobody, and what replaces it is already queued or the page is
gone.

## Decision 4: what the screen shows while the marks are being drawn

`screen.rs` owns the gap between a frame arriving and its pixels existing, and three choices are
its whole content:

- **A finished draw lands only under an identity check** — the `Arc` of the list and the equality
  of the target, the same identity the wire format interns by. A draw can finish in the race
  window between the last command and a replacing `ask`, and pixels drawn *for* one target placed
  as another's is trap 12a's shape one instrument over. Calibrated by removing the check: the
  stale-draw test fails.
- **A scroll re-places pixels rather than re-drawing them.** The same list at the same target
  arriving with a new origin keeps what was drawn — which is the saving `Payload::List`'s own
  documentation promises the shared `Arc` for. Calibrated by removing the reuse: the moved-page
  test asks the thread to draw again and fails.
- **After the first frame, a new frame is presented only when every page has been answered** —
  with pixels or with a refusal — except when the window has changed size, where the stale
  picture is the wrong shape and a partial frame is the honest one. Before the first frame,
  partials present as they land, under the same one-refresh settle budget the native windows
  spend (ADR 0678). This is `doc/todo/37`'s show-what-it-had with nothing reprojected yet.

## Decision 5: Escape is the abort, and it is the pair `doc/todo/15` asked for

The owner's brief — *"warn the user and allow the user to abort, however don't block"* — needs an
input, and on this boundary the abort has two targets that the two mechanisms serve exactly
(ADRs 0241, 0650): the `Canceller` ends the worker with a kill the document cannot decline, which
covers the interpretation; and the interrupt takes back this side's drawing thread, which covers
the draw a kill does not reach. Escape does both, blocks on neither, and the window says what
happened in its title and on standard error. The same interrupt runs on the way out of `q` and
the close button, because the drawing thread is joined at exit and a hostile page would otherwise
hold the join for as long as it liked.

What this round deliberately did not build is the *warn* half as chrome — a bar saying "this page
is taking a while" — because `doc/todo/15` places that input in `viewer_host::keys`' table where
all three established windows get it at once, and this window is outside that table by Decision 2.

## Proof, driven under Xvfb on the real programs

`pdf-viewer-confined` and `pdf-view-worker`, release builds, on a 900×1100 virtual display; the
numbers are one run's own trace stamps, quoted as illustration rather than as a gate — the
machine was running a round beside them:

- **the marks arm**: `PDF20_AN001-BPC.pdf` — worker started and confined in **9.4 ms**, `frame: 1
  page(s), 1 as marks`, first frame presented at **0.126 s** after `main` with the window up at
  0.051. Arrow keys turn pages and the title follows `Event::PageChanged` (*page Cover (1 of 5)*,
  then *Copyright*, then *2 (3 of 5)*); `+` and the wheel re-present the same crossing at a new
  target and scroll it. `xwd` captures show the pages placed on the surround, magnified and
  scrolled.
- **the raster arm**: `personwithdog.pdf` — `frame: 1 page(s), 0 as marks`, the photo page drawn
  by the worker and blitted at its origin, on the screen at 0.123 s.
- **the abort**: the amplification fixture at four levels (ADR 0650's construction: 1567 bytes,
  ten thousand page-covering fills, the marks arm) crosses in milliseconds and draws on the
  drawing thread for tens of seconds. Escape kills the worker and raises the interrupt; the title
  carries the abort sentence, and the program then exits on `q` in **0.095 s** — which is the
  interrupt observed from outside, because an uninterrupted draw would have held the exit's
  thread join for as long as the fixture chose. **The first abort also found a defect this
  section exists to catch**: the cancelled worker stayed `<defunct>` beside the window, because a
  `Canceller` kills without reaping and only `Confined`'s drop reaps — so `stop` drops the handle
  now, and the re-run shows no zombie.
- **the refusal**: `issue6010_1.pdf` is refused with the sentence naming the three windows that
  prompt, in the title and on standard error.

The six new `screen.rs` tests run headless in the workspace suite, and each was calibrated
against an injected defect before being believed (trap 13): the blit's origin zeroed fails the
three placement tests, the identity check removed fails the stale-draw test, the reuse removed
fails the moved-page test, and the supersede removed fails the interrupt test.

## What this does not close

`doc/todo/15` keeps the remainder, now sharper for having a host to point at: the established
windows still interpret in-process, the warn-before-abort input is still owed to the three of
them through `viewer_host::keys`, and the confined window presents through the processor while
ADR 0607's whole argument is that the *device* is the host's — a `render-quorra` surface behind
this window is the natural next piece. The breach-as-refusal item (a fallible allocation on a
path this crate does not own) is untouched.
