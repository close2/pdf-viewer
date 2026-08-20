# 0457 — What a stand-in costs, and what a picture is of

Status: accepted.
Session: the six-hundred-and-twenty-second.
Subject: `doc/todo/37`'s two remaining items. The processor's window gets a stand-in, which
reintroduces the premise ADR 0391 deleted rule 4 for and therefore owes rule 4 back with an
argument; and a page's retained picture stops being identified by the address of the commands that
drew it, which is what a `SinglePage` page turn had been failing on since ADR 0443.

## Item 1 — the processor's window, and rule 4 comes back for it alone

### What was absent

`--cpu`, and any machine whose graphics device will not come up, showed the previous view's pixels
unmoved for the whole of a slow frame with nothing saying why. `doc/todo/37` argued it was the
*smaller* of the two pieces of work and said exactly why: `viewer_ui::software::SoftwareSurface`
presents a raster the processor produced, so the host already **has** the pixels of the frame on the
screen. There is no capture to arrange and no readback to price — the two things that cost the
device path three ADRs (0383, 0384, 0385).

### The shape, and why it is not the device path's shape

The device path is *adopt, ask, place*: the render is on a thread of its own and the presenting
thread issues three textured quads while it runs. The processor's window has no other thread. Its
frame is drawn by `App::present` on the event thread, and the whole of what a stand-in can be there
is: **resample, present, then draw the true frame and present that, in one call, before it
returns.**

That order is a decision and the alternatives were considered:

- **Stand in on this tick and draw the true frame on the next.** Rejected. The frame is then later
  by the resample *and* a whole refresh, and worse, nothing makes the next tick draw the true one —
  rule 5 would say "misses" again and the loop would stand in for ever, which is rule 1 broken by
  construction. It would need a second mechanism to force alternation, and a mechanism that exists
  to stop a livelock is a design saying it went wrong earlier.
- **Move the processor's render to a thread of its own**, so that the device path's arrangement
  holds on both. That is a larger change than this item — it is `doc/todo/36`'s work done a second
  time for the other surface — and it is not needed to answer the person: the window answers within
  one resample of the gesture either way. It stays open and `doc/todo/37` carries it.

**Rule 1 is met sooner than the clock could meet it**, so `MustFollow` gains a second discharge,
`drawn_in_the_same_frame`. The obligation is unchanged and still cannot be dropped; what changes is
that the frame replacing this stand-in is already being drawn when the value is consumed. Arming the
cadence instead would ask for one more full processor frame per view change, which is exactly the
cost rule 4 exists to refuse.

### Rule 4, and the argument ADR 0391 said a round doing this would owe

ADR 0391 deleted rule 4, and it was right for the window it was written about: a reprojection there
is three textured quads issued by the thread holding the surface while the render is on another, so
what it costs the frame it stands in for is **nothing rather than a fraction**, and a bound on
nothing is not a bound. `doc/todo/37` warned in as many words that *"a resample on the processor
really would cost the frame it stands in for, so a round that builds that path is reintroducing the
premise and owes an argument for whatever it puts back."*

The premise is back, exactly. So the rule is back, in the form ADR 0384 re-grounded it in and not in
the form ADR 0391 deleted:

> **standing in must buy at least one refresh: `resample + period ≤ frame`.**

No constant, and the display's own unit. **Unmeasured permits**, because drawing one is the only
thing that can produce the first number — which is the defect ADR 0384 found in rule 5 and this is
the same trap one surface over.

**It is one rule asked of two arrangements rather than two rules**, and that is what `Standing` is:
`Beside` for the window whose render is on another thread, `InFrontOf` for the window whose render is
this very call. `Stale::affordable` answers `Ok` for `Beside` without reading anything, so the device
path is untouched — no gate returns to the surface ADR 0391 removed it from.

### What a processor-side resample costs

**The absolute number is the machine's and is deliberately not in the code.** `Stale::resampling`
holds whatever the run measured, so a faster machine changes the policy's answer rather than
falsifying a constant. On this machine, under `Xvfb`, with three parallel rounds running their gates
at a load average over 70 on 24 cores, an 800 × 1000 resample plus its copy to the window measured
between 35 + 41 ms and 195 + 70 ms across a session's view changes, against processor frames of 24
to 390 ms — and the trace shows the policy doing both things it should: standing in where the frame
is 220 ms and refusing where it is 35.

Two optimisations were tried and **only one was kept**, which is `CLAUDE.md`'s rule about
optimisation applied honestly rather than as a preamble:

- **Removing `f32::floor` and `f32::round` was kept.** `x86-64` without `SSE4.1` has no rounding
  instruction, so those are `libm` calls — six per pixel, 4.8 million per window. Timed
  **alternately in one process**, 25 samples each and the best of them, the form with the calls is
  1.3 to 1.8 times dearer: 48.3 against 35.3 on a scroll, 55.4 against 41.8 at the identity, 157.1
  against 86.1 on a zoom. Interleaving is why that ratio is trustworthy on a machine whose absolute
  figures are not.
- **Fixed-point interpolation was rejected.** Weights quantised to 1/256 and the channels
  interpolated in `u32` measured 30.4 against 33.3, 30.4 against 26.8 and 37.8 against 33.9 — no
  gain outside the noise — and it cost two levels of 255 of accuracy and a paragraph of
  justification. An optimisation that cannot show its number does not go in.
- Row-stepping the inverse affine is in, and it is *not* claimed as a measurement: the loop with the
  sampling taken out of it is 0.13 ms of a resample that is tens, so what it saves is inside the
  noise. It is there because it is also shorter to read.

### Rule 2 and rule 3 are unchanged, and where the pixels live

The resample is `crate::stale::Canvas`, a private module of the binary — never
`viewer_ui::software`, which is a *library* that `viewer-confined`'s worker and the software-surface
tests link to. The base is the raster `compose_pages` has just produced, adopted in the same breath
as `Stale::settled` records what it is a picture of: ADR 0385's defect was a base and its record
updated by two different events, and one call site is what makes it unrepresentable.

Rule 3 gained a distinction rather than a word. This tick presents **twice**, so a frame line saying
`approximated` would describe a picture that is no longer on the window; `Stages::stood_in` is beside
`Stages::approximated` rather than in it, the frame line says `after approximated <ms>`, and the
summary counts the stand-in as its own present and its own approximation — which keeps
`doc/todo/36`'s rule 6 the two separate claims it is meant to be.

### What the processor's window still does not have

The retained low-resolution pages. They are drawn by a render thread that is idle, and this window
has none — so a page turn there is still `Refusal::AnotherPage`, said out loud, exactly as it was.
`doc/todo/37` carries it.

## Item 2 — what identifies a page's picture

### The defect, which was an identity rather than a mechanism

Session 608 found it and declined to fix it in passing: a retained picture is keyed by the
`Arc<DisplayList>` it was drawn from, and `viewer_core::open` keeps "one entry rather than a cache of
them", so a page turn drops the outgoing page's interpretation and returning to it produces a **new
`Arc` over the same commands**. Every `SinglePage` page turn printed `another page — nothing to
show`. 608 wrote that this "needs a decision about superseded interpretations rather than a key
changed in passing".

### The decision

**A page's picture is identified by the page it is of and the state of the ink it was interpreted
against — `(document, page, ink)` — and not by the address of the commands.**

The `Arc` answers *which interpretation*, and that is the right question for the **base**: the sharp
layer is a picture of one arrangement at one placement and `depicts` must be exact about it. It is
the wrong question for a **retained page**, which is a picture of a *page*. So the base keeps the
address and the retained layer takes the identity. `stale::Picture` is that identity and
`stale::Placed` is what carries it beside the commands and the placement, so that the two cannot be
paired up wrongly.

**What may be kept**: a picture whose `(document, page, ink)` matches an entry of the arrangement
being asked for. A page returned to is interpreted again, gets a new address and the same identity,
and its picture is found. That is the whole of what item 2 buys, and it is what a `SinglePage` page
turn had been failing on.

**What must be dropped**: every picture of a superseded ink — all of them, not the page that
changed. That is `viewer_core::open::Open::stale`'s own argument for dropping every readback rather
than one: *"a layer switched or a value typed is a change to the state, and the state is what every
cached page was interpreted against"*. `Proxies::keep` evicts them when the first picture of the new
ink arrives, because a picture nobody may show is not worth one of the slots the memory bound
allots.

**Why superseded ink may not stand in, which is the question 608 actually asked.** A stand-in is a
wrong picture drawn deliberately, and the licence for that is rule 3: it says what it is. Blur says
*approximation* by itself and nobody mistakes it for the page. Stale ink says nothing at all — a
§8.11 layer switched off goes on being drawn, sharp and legible, and the person cannot tell that from
the switch not working. That is the plausible-instead-of-true failure `CLAUDE.md`'s first principle
exists to prevent, and it is a different *kind* of wrong from blur. So the ink is part of the
identity and not a refinement of it.

**Where the memory bound is: unchanged.** `PROXY_PAGES` pages of `PROXY_EDGE` pixels, both chosen
from ADR 0443's measurements. This decision changes the *key* rather than the extent — and it makes
the same bytes buy more, because returning to a page is now a hit instead of a redraw.

### Where the ink comes from, and why it crosses the boundary

`Open::stale` is already documented as "the one place that says what 'the ink is stale' means" and is
called by §8.11's layer switch, §12.5.5's appearance under the pointer, §12.7.5's field value,
§6.3.2.2's delegated widgets, §12.6.4's action outcomes and the edit log's replay — which is
`Command::Edit`, `Undo` and `Redo` between them. It now increments a counter, and
`RenderRequest::ink` carries it.

Two alternatives were rejected:

- **Deriving it in the host** by counting the commands it sends that could change an interpretation.
  It needs no boundary change and rule 2 would be trivially satisfied — but it puts the same
  knowledge in two places, and the host's copy goes silently wrong the day a command's meaning
  changes underneath it. `Open::stale` is where that question is already answered.
- **Caching interpretations in `viewer_core`**, so that returning to a page yields the same `Arc`.
  It would answer this and would also let `render-quorra` replay a retained encode (ADR 0351) — but
  a display list is very much larger than a proxy, so it spends memory in a place every host pays
  for, in order to answer a question only a stand-in asked. That is rule 2's direction of travel
  run backwards. It stays open on its own merits and not on this one.

**Rule 2 is not strained by the field**, and the distinction is worth stating because
`--proxy-pages` is deliberately *not* a boundary value: a count of stand-ins to keep is a knob that
exists only to make wrong pictures, and an ink counter is a fact about the document's interpretation
state that any host holding pixels needs. Nothing about a stand-in crosses; what crosses is what the
pixels are of.

## What running it said

Under `Xvfb`, `SinglePage`, `doc/ISO_32000-2_sponsored_EC3.pdf` on the graphics device: `Right`
to page 4 and `Left` back to page 3 both printed `approximated from a retained page`, `1 of 6
retained low-resolution page(s) stand in` — the return being the case that printed `another page —
nothing to show` before this session, because the page came back with a new address. Photographed
mid-render, the window held a third picture, different from both the outgoing page and the true
frame that replaced it.

On `--cpu`, the stand-in fires and rule 4 gates it: `resample 35.77 ms, present 76.64 ms` against a
frame of 224.3, drawn; `standing in would cost 184.5 ms on this thread and buy a 16.7 ms refresh,
against a frame of 126.7 ms` , refused, in the refusal's own words. What could not be photographed is
the processor's stand-in itself: a screen capture on this machine under that load takes longer than
the stand-in is on the window, so the burst captures show the window flip from the old view to the
true frame without catching what was between. What stands in for a photograph there is the
presenter's own report — the trace line is printed only after `SoftwareSurface::present` returned
`Ok` — and two unit tests over the resampled pixels: at the identity the picture is the frame itself,
and a scroll moves the rows and paints the window's medium in the strip it reveals.

## The spec-driven half, and it is the same discipline one clause over

§12.5.6.11's row said what was genuinely owed was *"a reader for `/Sy` and `/RD`, which this tree has
none of — no source names either key"*. `/Sy` is unread. `/RD` has been read since §12.5.6.8 was
implemented: `appearance::insets` takes it and applies it as the differences between `/Rect` and the
shape in it, left, top, right, bottom — Table 183's own order, and Table 177's and Table 180's.
Three tables state that entry in that order, two are read, and the row said none was. That is
`doc/habits.md`'s sixth shape: **grep for the entry rather than for the capability.**

What it cost was a sentence in the report a person reads. A caret fell to `construct`'s catch-all,
`Refusal::NotDerivable("its clause states no geometry")` — false of a table that states four numbers
of geometry and a `shall`-bearing code point — and it now has its own arm saying what §12.5.6.11
actually leaves unstated: the artwork of the caret, with Table 183's pilcrow "displayed along with
the caret" rather than instead of it. The behaviour and the refusal are unchanged; the test asserts
the sentence, because the sentence is the half a reader gets.
