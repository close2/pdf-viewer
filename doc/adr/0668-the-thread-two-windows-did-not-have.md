# ADR 0668 — The thread two windows did not have

Status: accepted, 2026-08-25. Session 754. Takes the item `doc/todo/30` opened in the
seven-hundred-and-forty-ninth — both native windows rasterising inside their `Event::NeedsRender`
arm — and, with it, ADR 0657's interrupt policy in the form a tier-1 host can state exactly.
Cites no clause: this is `CLAUDE.md` principle 3 against principle 2, and the ledger is untouched.

## What was owed, and what the briefing said not to build

ADR 0657 gave `viewer-ui`'s processor window a policy for `pdf_render::Interrupt` and named the
three hosts it could not reach. Two of them are the subject here:

> `viewer-gtk` and `viewer-qt` both call `self.rasterizer.rasterize(&request.list, request.target)`
> **inside** their `Event::NeedsRender` arm, on the toolkit's main thread. `pdf_render::Interrupt`
> is a flag *another* thread raises, so in those two there is no other thread to raise it and
> nothing for it to interrupt but the loop that would do the raising: a page written to draw for
> 27.6 s (ADR 0650) takes the window with it — no repaint, no key, no way to say stop.

`doc/todo/30` also says what **not** to build, and that half is load-bearing: not a watchdog. A
thread whose only job is to raise the flag after a fixed duration is the automatic deadline ADR
0657 §1 measured and refused, on a population that does not separate — median 2.2 ms, p99 73.9 ms,
slowest of 957 first pages 252.1 ms, against 27 600 ms for a 1567-byte fixture, so a deadline low
enough to catch the fixture refuses one legitimate first page in sixteen.

## 1. One arrangement in `viewer-host`, not two in two crates

`viewer_host::drawing` is the whole of it: a `Drawing` that holds a queue, one job in flight, and a
thread that is spawned by the first page that needs one. `viewer-gtk` and `viewer-qt` each lost
their `rasterize` call and gained `drawing.ask(request)` in its place.

That placement is this crate's own test applied one more time, and it is the same argument ADR 0473
made for `viewer_host::Clock` and ADR 0630 for `viewer_host::form::clicked`: **what is shared is the
decision and what is the toolkit's is only the event loop.** Three copies of a scheduling rule is
where two hosts stop agreeing about when a draw is worth finishing, and unlike a widget there would
be nothing on the screen to notice it by.

`viewer-host` gained a dependency on `render-cpu` for it, and the crate's own manifest comment
changed: it used to say "a vocabulary of values rather than a rasteriser: nothing here draws." Now
something does, on a thread, and the rasteriser is named rather than taken behind a parameter. ADR
0650's reason: `render-cpu` is the only backend in the tree that can be interrupted at all, because
the loop is ours and a hostile document arrives in it as data — the device backends were
deliberately given no such method rather than one they would ignore, so a `Rasterizer` parameter
here would be a choice between one implementation and one that silently could not stop.

## 2. The rule, and why a tier-1 host can state it exactly

`viewer-ui`'s policy is `stale::could_stand_in` — *would a finished picture of this frame be worth
anything?* — and it is a judgement about pixels, because that host composes Table 29's whole
arrangement into one window raster and reprojects it. A tier-1 host holds no such picture. What it
holds is a *token*, and that turns the same question into one with a provable answer:

> `viewer_core::Viewer` keeps one outstanding request per page of the arrangement and **drops** a
> `Command::RenderReady` whose token is not the one outstanding (`Viewer::rendered`). So a draw
> whose token the viewer no longer holds cannot change a pixel however long it runs.

Two things take a token away, and `Drawing` acts on both:

- **A newer request for the same page** — a resize, a zoom, a re-interpretation. `Viewer::schedule`
  writes the new `Pending` over that page's old one *as it issues the event*, so the old token is
  already dead when the host sees the new request. `Drawing::ask` raises the interrupt for it and
  needs nothing from the host to know.
- **The page leaving Table 29's arrangement** — a page turn under `SinglePage`, where the whole
  `OnScreen` entry goes and the outstanding request with it. That one is `Drawing::superseded`,
  because whether the arrangement still shows a page is `Query::PageGeometry`'s answer and
  `viewer-host` holds no viewer to ask it. Each host reads that answer and hands over the boolean;
  the *decision* stays in one place.

Neither reads a clock and neither asks what a page costs, which is ADR 0657 §1's whole point carried
over. What is new is that this form of the rule is **exact** rather than a judgement: it abandons
only work whose answer the viewer has already declined to accept.

## 3. An abandoned draw is answered to nobody — trap 20, in its second host

`viewer_core::Rendered::Failed` sets a page's `shown` to the request's own target and revision, so
the scheduler stops asking for it — correctly, because a rasteriser that refused this page at this
size will refuse it again. Saying it about a draw a host merely *chose* not to finish freezes the
page.

So `Finished::outcome` is an `Option<Rendered>` and `None` means nothing is owed. Both hosts write
the same two lines, and the shape puts the rule where the call site can see it:

```rust
let Some(rendered) = finished.outcome else { continue };
queue.push_back(Command::RenderReady { token: finished.request.token, rendered });
```

**Abandoning cannot freeze a page here, and that is provable rather than hoped for.** In the first
case the viewer has already replaced the token, so the answer would have been dropped anyway. In the
second the page has lost its place in the arrangement, so nothing is outstanding and coming back to
it rebuilds the entry, interprets it again and asks again.

There is one case where `Query::PageGeometry` answers `Answer::None` for another reason — a page
whose *re-interpretation* has newly failed, where `OnScreen::interpreted` is cleared and not
replaced. Abandoning there costs nothing either, and the reasoning is written down rather than left
implicit: this host answers nothing, so `viewer_core` neither records the page as shown nor drops
the raster it is already holding, and the window goes on showing the last true picture of that page
until an interpretation succeeds and bumps the revision, which issues a fresh request. That is
strictly better than the alternative, which is the freeze trap 20 names.

## 4. A queue, which is where this differs from `viewer-ui`'s composer

That host draws the arrangement as **one** frame, so a superseded frame is simply not drawn and one
slot is enough. A tier-1 host is asked for each page separately and **owes an answer for every one
of them**: `Viewer::schedule` skips a page whose outstanding request matches the target and
revision, so a request dropped on the floor is a page that never draws again.

So `Drawing` keeps a `VecDeque`, drains it one job at a time, and replaces rather than appends where
a queued request names a page already queued. `every_page_of_an_arrangement_is_answered` is the test
for the first half and `a_queued_request_is_replaced_rather_than_drawn_and_thrown_away` for the
second.

## 5. How a finished page reaches the window: a pull, because one toolkit cannot be pushed to

`viewer-qt`'s C++ owns the `Host` for the life of `QApplication::exec` and **Rust never calls a Qt
object** — the property that keeps that crate to one hand-written `unsafe` token (ADR 0246, and the
flag-plus-getter shape ADRs 0470, 0519 and 0526 record). There is no path from a drawing thread into
`QApplication::exec` that would not cost a second one. GTK could be woken through a file descriptor
on its main context; Qt cannot, and `doc/todo/30`'s "one arrangement in three hosts" is the reason
that asymmetry was not taken.

So `Drawing::interval()` answers how long to wait and each toolkit arms its own one-shot:
`glib::timeout_add_local_once` and a `QTimer`. That is exactly `viewer_host::Clock`'s shape and
`viewer-qt`'s accessibility drain's — *the interval is pulled, so this side owns the timer and Rust
owns the decision* — and it is now the third instance of the same three lines in `window.cpp`.

**It answers `None` while nothing is being drawn**, so a window showing a drawn page wakes for this
exactly never, which is `CLAUDE.md` principle 2's rule about a wakeup with nothing behind it.

`Drawing::POLL` is one millisecond, and the number is bounded from both sides rather than picked.
From below: a poll is one `try_recv` on an empty channel and the timer does not exist at rest. From
above: the interval is added to every page's latency and the median first page of `doc/pdf.js` draws
in about two milliseconds — so a poll at one refresh period, the obvious alternative, would be most
of a median page turn again. §7 has what it actually cost.

## 6. The thread is not on the launch path until page one needs it

`CLAUDE.md` section 2 forbids anything eager before page one. `Drawing::new()` holds no thread; the
first `ask` spawns one. A window built and never given a document has spawned nothing, which
`no_thread_exists_until_a_page_is_asked_for` asserts against the private field.

That does not make the arrangement free at launch — page one *is* the first `ask` — and §7 is the
measurement rather than the assertion.

## 7. What it cost, measured — and the half that could not be measured here

Three numbers, and they are three different questions. All of them were taken under `Xvfb` on a
machine carrying three other rounds, so **749's rule applies throughout: a duration on a shared
machine measures the machine, and a ratio inside one run does not.**

**The handoff itself, in-run, with the toolkit's loop otherwise idle: 2.1 ms.** The frame line
prints what the drawing cost and what the host waited, so their difference is the whole of the
arrangement's overhead on that page and it is measured inside the same process. On the 1000-fill
amplification fixture, where the loop has nothing else to do for three to five seconds:

| | drew for | host waited | difference |
|---|---|---|---|
| `viewer-gtk` | 4842.41 ms | 4844.56 ms | **2.16 ms** |
| `viewer-qt` | 3085.99 ms | 3088.13 ms | **2.13 ms** |

Two toolkits agreeing to within 0.03 ms is what "only a thread and a channel" predicts, and it is
what one host could not have said.

**What the poll period is worth, on a page turn: median 6.9 ms at one millisecond against 15.0 ms
at one 60 Hz refresh.** The same difference, over four page turns per run and three runs of each
arm, alternating, from two release builds of this tree differing in that constant alone. The 16 ms
arm centres near half a period above the round trip, exactly as a poll predicts. **This is why
`POLL` is a millisecond**, and it is a measurement rather than the argument §5 makes.

**The launch path: not separable from the machine here, and at least +53 ms on it.** `opened` to
`first frame on the screen`, ten alternating pairs, load average 25 to 40:

| | best of ten | worst of ten |
|---|---|---|
| before | 9 ms | 100 ms |
| after | 62 ms | 448 ms |

The structural cost is **one extra trip through the toolkit's main loop** — page one is now asked
for, and answered on the next turn of the loop rather than inside the same one — plus one
`std::thread::Builder::spawn`. On an idle loop that trip is the 2.1 ms above. What the table shows
is that trip taken while the loop is competing for a core with three other rounds, and both arms
drift by an order of magnitude across the same sequence, which is what a loaded machine does to a
duration.

**So the honest statement is that the launch A/B is owed on a quiet machine**, and this ADR does not
claim the regression is 53 ms or that it is 2 ms. What it does claim is what the in-run numbers
support: the arrangement adds a channel round trip and a thread spawn, both of which are
milliseconds when the machine has a core to give.

## 8. What the screen said — trap 1's sentence, one directory over

Both windows, under `Xvfb`, on the 1000-fill fixture, with three zoom-ins 700 ms apart sent through
XTEST while the first draw was still running. `viewer-gtk`:

```
page 1 rasterised 509x659 in 4.842408428s, waited 4.844564672s
first frame on the screen at 5.53986505s
page 1 abandoned after 637x824 in 737.380473ms, waited 739.086147ms
page 1 abandoned after 796x1030 in 699.203883ms, waited 703.429619ms
```

`viewer-qt` says the same thing in its own sizes (`3.086 s`, then two draws abandoned after 702.6
and 702.7 ms). Three facts are in those four lines and none of them was true before this round: the
window took a key while a page was drawing; the zoom produced a new request; and the draw the new
request superseded stopped rather than running to a picture nobody would see.

## 9. What could not be checked here

- **A real display.** `Xvfb` throughout, `xdotool` driving XTEST rather than `--window`, which Qt
  ignores.
- **The launch path on a quiet machine**, which §7 says plainly rather than averaging over.
- **The C ABI**, which is the third host ADR 0657 named and is a different case: a C caller is
  *told* to move the request to a thread of its own, so the structure is already right and what is
  missing is an entry point to raise a flag with. `doc/todo/30` still carries it.
