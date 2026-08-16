# ADR 0230 — A frame is two pages and a fraction

Status: accepted, 2026-08-08. Session 393.

## Context

ISO 32000-2 §12.4.4's presentation has been *read* since the seventieth session and *advanced*
since the hundred-and-fiftieth: `Command::Tick { millis }` is how a state machine with no clock is
told the time, and `Event::Transition` carries Table 164's style, duration and direction (ADR
0135). That ADR's own closing section said what was missing, and said it would stay missing until
a host wanted it:

> The transition is **named, not played**. […] a sequence of frames over 0.7 seconds is exactly
> the thing a crate with no clock cannot own.

Two hundred and forty-three sessions later nothing drew one, and the handover still opened its
"what it still does not do" paragraph with it. This round made one host draw them.

## The two constraints, which between them decide the design

- **`viewer-core` has no clock and may not grow one** (rule 3). Time arrives as a command.
- **A host draws through `pdf-render` display lists**, because that is what makes the sidebar, the
  selection and the caret work on *both* backends. `CLAUDE.md` keeps the CPU rasteriser as the
  correctness oracle and as the frame a graphics device refuses; an animation only the device
  could draw would have quietly cost the second of those.

So the round's whole design question is where to cut between the two, and the cut is:

**The shape of a frame is the core's; when to ask for one is the host's.**

`viewer_core::transition::frame(&Transition, viewport, progress) -> Option<Frame>` is a pure
function of a fraction. `Frame::draw(viewport, outgoing, incoming) -> DisplayList` turns it into
at most two `Command::Image`s with a clip apiece. Nothing in the crate knows what a second is, and
the same fraction produces the same frame in a test with no display as in a window.

## What the standard states here, and what it does not

Table 164 states which styles exist, what `/Dm`, `/M`, `/Di`, `/SS` and `/B` mean for each, and
that `/D` is "[t]he duration of the transition effect, in seconds". §12.4.4.1 states that a
transition is one *to* a page. **It states not one word about what a frame in the middle looks
like** — no timing curve, no line count for `Blinds`, no band width for `Glitter`, no dissolve
pattern — so most of this round is a documented choice in the manner ADR 0211 recorded a caret's
colour and ADR 0225 its placement. The choices, in full:

- **Progress is linear in time.** A host divides elapsed time by `/D`. Nothing reads a curve into
  a clause that states a duration.
- **A sweep reveals what it has passed over.** Table 164's own verbs are "sweep across the screen,
  revealing the new page" and "slides on to the screen … covering the old page", so the swept or
  covered area shows the page moved to and the rest shows the page being left. This is a reading
  of the table's sentences rather than an invention, and it is the only part of the module that
  claims to be.
- **The clock is held while a transition is drawn.** §12.4.4.1's own EXAMPLE describes "a page to
  be displayed for 5 seconds" whose 3.5-second transition happens "[b]efore the page is
  displayed", so the transition precedes the display duration rather than spending it.

## Which styles, and what the others now cost

**Seven of the twelve**: `Wipe`, `Split`, `Box`, `Cover`, `Uncover`, `Push` and `Fade`. The line
is not "the easy ones" — it is *the ones whose frame is determined by Table 164's own words*. Each
is a placement of two pages plus a rectangular region and a constant alpha, and none of them needs
a quantity the standard does not give.

~~The other five are named and **reported by name** rather than silently drawn as a cut~~ — **four
of the other five are**, which the paragraph under the table below says in as many words and this
sentence did not until the five-hundred-and-fifty-third session. Reporting rather than silently
drawing a cut is trap 5's rule everywhere else in this tree and matters most exactly here: the page
that arrives looks right, and only the file knows it asked for an effect. *(Two ledger rows were
written from the wrong half of this ADR and said five: §12.4's and §12.6.4.15's, corrected in the
same round that corrected this. An ADR holding both answers at once is what the seventh failure
shape looks like inside one document.)*

| style | the quantity Table 164 does not state |
|---|---|
| `Blinds` | how many "[m]ultiple lines, evenly spaced across the screen" there are |
| `Glitter` | how wide "a wide band" is, and what a dissolve looks like inside it |
| `Dissolve` | what "dissolves gradually" does to a pixel between two pages |
| `Fly` | what "[c]hanges" are — the flown object is the *difference* between two pages |
| `R` | nothing: the table defines it as the cut, "no special transition effect" |

`R` is therefore not reported and the other four are. What the four now cost is one function each
plus the number the standard withholds, written down as a choice: `Blinds` a line count,
`Glitter` a band width and a dither, `Dissolve` a per-pixel pattern (the one of the four the
display list's vocabulary does not already express, since a reveal is a list of rectangles), and
`Fly` a page *diff*, which is a different kind of problem and the reason it is last.

`/Di` is read as an angle and only the four quarter turns describe a rectangular sweep; 315 is
`Glitter`'s alone and the name `None` "is relevant only for the Fly transition", so a style that
needs a direction and is given one of those is reported rather than drawn at some nearby angle the
file did not ask for.

## What a host does, and what it costs

`viewer-ui` gained one key. `p` starts and stops driving the clock, which is the whole of what
"presentation mode" is here — ADR 0135 decided the core has no such state, so "is a presentation
running" is answered by whether something ticks. Between transitions the loop wakes ten times a
second (`ControlFlow::WaitUntil`); during one it polls, because that *is* an animation; and a
window reading a document waits for an event exactly as before.

Two rasters are taken per transition and none per frame: the page being left, as it was last
presented, and the page arriving, both drawn by `render-cpu` — the one rasteriser this host can
ask for *pixels* rather than for a present. Each crosses to the graphics device once, because
`pdf_render::Image` holds its samples behind an `Arc` and quorra's caches are keyed by that
pointer. Measured on this machine, an 800×1000 window under `Xvfb`:

| | starting one | a frame during one |
|---|---|---|
| lavapipe (software Vulkan) | **8.3 ms** for both pages | median **3.8 ms**, p90 5.1 — 433 frames in a 2-second wipe |
| `--cpu` (no driver at all) | **11.1 ms** | median **16.0 ms**, p90 24.0 — 112 frames |

A frame that re-rasterised both pages instead would have paid a page's interpretation sixty times
a second, which is the trade this makes deliberately.

## Two defects the window found that no unit test could

**The pages were animated against each other in the wrong order, and then not at all.**
`Viewer::handle` settles *after* the command that turned the page, so the events arrive as page
change, transition, render request — and the display list of the page being moved to is in the
last of the three. A host that began the transition when the event arrived rasterised the page
being *left* twice and cross-faded it with itself: 430 frames drawn, and a screen that never
changed. So the host arms the transition and begins it on the next `NeedsRender`, which is also
the clause's own order — a transition is one *to* a page.

**And every page was upside down.** A `Command::Image` draws "the unit square in user space,
with the image's *top* row at y = 1", because PDF's user space has y growing upward; a page raster
has its top row first. The frame therefore needs a negative y scale, and a page of flat colour
cannot see the difference — which is why the gate that catches it draws a page of two colours, one
half each, and why the fixture's white square is in a corner rather than the middle.

Both were found by running the program under `Xvfb` and reading the pixels back (ADR 0126), and
neither was reachable from any gate this tree had.

## What is checked

- **`viewer-core`**, nine unit tests over the shape: each style's geometry against Table 164's own
  sentence, the property that every shaped style begins on the page it is leaving and ends on the
  page it moved to, the four bands an inward `Box` reveals tiling what is outside it exactly, and
  the directions that are *not* a quarter turn being refused rather than rounded.
- **`viewer-core/tests/headless.rs`**, the tier-1 host: a two-page fixture ticked past its `/Dur`,
  the frame at a quarter and a half of the way through rasterised, and the pixels either side of
  the sweeping line read — plus a `/Blinds` reported by name with the page it was moving to.
- **`viewer-ui/tests/transition_frames.rs`**, the window's own backends: the same frame drawn by
  `render-cpu` and by quorra's headless software device, both against the closed form, and the two
  compared pixel for pixel — **0 of 28 800 differ**.
- **The window itself**, under `Xvfb` with `xdotool` and `xwd` (ADR 0126): `p`, then a capture
  every 0.4 s. The new page's edge reaches x = 0, 171, 335, 500, 668, 800 with the device and 6,
  166, 325, 499 without one, against an expected 160 px per 0.4 s of a two-second wipe across an
  800 px window — the excess being the capture's own time. And the fixture's second transition, a
  `Split` with `/Dm H` and `/M O`, photographed 0.9 s into its two seconds: down the column at
  x = 400 the window reads white to y 199, the page being left to y 266, **the page arriving from
  y 268 to y 731** and the page being left again to y 799 — a band centred on y 499.5 where the
  window's centre is 500, and 46.4% of the height where the clock says 45%. Frame *n* is between
  the two pages, in pixels read.

## What the corpus says, which is nothing

`pdf-model/examples/presentation_census` walks the page tree of all 964 openable corpus documents
and the 14 in `doc/` — 978 documents, 1971 pages — and **not one states a `/Trans`, a `/Dur` or a
`/PresSteps`.** ADR 0135 measured that over raw bytes in the seventieth session; this asks the
page tree, so a `/Trans` inside an object stream would have been counted. There is none.

So this is the specification track and says so, and the witness is hand-built:
`examples/presentation_fixture` writes the three-slide document the screenshots above are of. It
is a *fixture* and not a writer — the same construction every test in this tree makes in a string
literal, moved to a file because §12.4.4 is the one clause whose subject a person has to watch.

## The claim that was retired

Three places in this tree said, in three wordings, that **a transition is "an animation between
two pages, which a display list cannot express"** — the ledger's §12.4.4 row, `pdf_model::navigation`'s
module comment and ADR 0135 itself. It is false, and it was false when it was written: a frame of
one is two images, a clip apiece and a constant alpha, which is four commands of a vocabulary this
tree has had since its first rasteriser. What a display list cannot express is *time*, and that is
a much smaller claim — it is answered by handing the list a fraction, which is this ADR.

`CLAUDE.md`'s rule about a claim that a clause does not apply decaying is the same rule one layer
down: **"our vocabulary cannot say this" is a claim about the vocabulary, and it decays.** It had
stood for two hundred and forty-three sessions and cost the feature all of them.
