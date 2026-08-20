# A frame that says it is stale — the one window it does not cover yet

Status: **built for the window with a graphics device** (ADR 0378, extended by ADR 0383, rule 5
corrected by ADR 0384, the base's lifetime by ADR 0385, and **the retained low-resolution page by
ADR 0443**, each after the owner found it did not fire on a real one), which is every run without
`--cpu`. A view change whose last frame was slow
now shows the pixels already on the screen, moved to where the new view puts them, and the real
frame replaces it — the frame line says `approximated`, the summary counts them **and counts what
was refused**, and `crates/viewer-ui/src/bin/pdf-viewer/stale.rs` carries the five rules with the
thing that enforces each.
Priority: 37 — one surface of two.
Witness: `tmp/Entwurf.pdf` — **not in the repository**, so no test may name that path. The costs
this file used to quote are ADR 0378's and both have since moved; ADR 0383 measured them again on
the same witness, and the *reason* they moved is worth more than either pair of numbers.

**Two of the five rules changed shape in the five-hundred-and-forty-eighth session**, and neither
weakened. Rule 1 no longer refuses a second reprojection outright — `doc/todo/36`'s owner allows
one explicitly — it refuses to draw the view already on the screen a second time, which is a
question about *which* view rather than about whether one is showing; the loop still cannot come to
rest on one. And rule 4's "it costs the real frame nothing" gained a second mechanism: the readback
is taken **once per real frame** and every reprojection after the first resamples what it captured.
ADR 0383 has both.

**Rules 5 and 4 both changed in the five-hundred-and-forty-ninth, and those were defects rather
than refinements — the same one at two scales.**

Rule 5 said `SHARE` × a *measured* reprojection cost, with an assumed 51 ms standing in — so a bar
of 510 ms until a reprojection had been drawn, and a reprojection was drawn only above the bar. The
project owner ran it on a real graphics device and reported *"I don't have the impression that
reprojection works"*: fifteen presents, frames of 80 to 438 ms, not one reprojection. **A
self-calibrating threshold whose own gate blocked its only sample.**

Rule 4 then became the binding constraint, still at a tenth, and the owner ran the fix and reported
the same sentence again. A tenth of a real device's frame is less than what a readback costs on it:
reprojections of 6 to 16 ms refused against frames of 58 to 156, six view changes of fifteen
showing nothing.

Both are now the display's own unit and neither has a constant in it. **Rule 5**: a *miss* is a
frame that does not land inside the cadence's period. **Rule 4**: standing in must buy at least one
refresh, `reprojection + period ≤ frame`, and *unmeasured permits*, because the first reprojection
is the only thing that can ever produce the number. **And rule 3 now reaches rule 4's refusal**,
which is the other half of why the owner had to write twice: `Stale::plan` returns a `Plan` rather
than an `Option`, and a refusal that is a judgement about two measurements prints both of them.
ADR 0384 has the traces, the A/B and the secondary defects it turned up.

**And the base's lifetime was the third, found in the five-hundred-and-fiftieth session — the same
shape a third time.** The owner's trace carried `no reprojection: the device has no retained encode
to replay` twice in twenty-four presents, and each time the frame before it was a rendering that
repacked its glyph atlas. The pixels of the rendering *before* it had been read back and were
this host's own `Arc<[u8]>` — and were destroyed by the frame that replaced them, because the base
was a field of `Settled`. So the window showed nothing for want of a **capture**, while what a
reprojection actually needs was in memory. A base is now `Stale`'s and carries the page and the
placement it is of, a refusal to *capture* is no longer read as a refusal to *draw*, and
`Stale::composed` reads the placement off the base it draws rather than off the last frame record —
which makes "compose, do not chain" unrepresentable to get wrong rather than merely enforced.

**Rule 3 is finished in the same round.** Every path that declines a reprojection was audited
against the owner's three words — *impossible*, *unwise*, *unnecessary* — five were unnecessary and
all five were the same mistake; the rest now print which of the two remaining kinds they are, and
the summary carries the count. ADR 0385 has the table and the argument for the one answer that is
deliberately silent.

## Two of the five rules changed again in the five-hundred-and-fifty-sixth session (ADR 0391)

*The render moved to a thread of its own. That is `doc/todo/36`'s item rather than this one, but it
reaches two rules here and the sections below were written before it.*

- **Rule 4 is deleted.** Its premise was that a reprojection ran on the event thread and pushed the
  real frame back by exactly what it cost, so it had to *buy* a refresh. A reprojection is now
  three textured quads issued by the thread holding the surface while the render runs on another,
  so what it costs the frame it stands in for is nothing rather than a fraction — and a bound on a
  cost that is structurally zero is not a bound. `Stale::measured`, `Stale::affordable` and
  `Refusal::TooDear` are gone with it. What a stand-in costs is still reported on the frame line;
  what is gone is a gate reading it. **This is why the paragraph below about a processor path
  owing rule 4 a measurement no longer binds** — but that paragraph's *other* half still does, and
  reads more sharply now: a resample on the processor really would cost the frame it stands in
  for, so a round that builds that path is reintroducing the premise and owes an argument for
  whatever it puts back.
- **Rule 5 gained a second instrument.** A miss is still a frame that does not land inside one
  refresh, and it is now *observed* as well as predicted: a render still being drawn when the next
  tick comes round has missed that refresh, whatever the last one cost. The prediction is what
  answers the first tick of a view change, where there is nothing yet to observe.
- **The base is a texture rather than a readback**, so `Base` is gone as a type and `Settled` is
  what it was — see the note below about `Base::of` being fed from `SoftwareSurface`, which a
  processor path would now write as a second producer of *rasters* rather than of bases.
- **And `Refusal::NoDevice` is deleted.** The processor's window no longer reaches `Stale` at all:
  it draws to completion and presents, as it always did, and asks the stand-in policy nothing. So
  the sentence at the foot of this section — "a round that builds this path deletes both lines" —
  has had one of its two lines deleted already, for a different reason.

## A sixth refusal arrived with Table 29's column — 607

`Refusal::Rearranged`, and it is worth knowing because it is the first refusal in this file that is
about the *arrangement* rather than about the frame. The window presents its pages as **one texture
under one placement**, so a reprojection is defensible only where one affine is true of every page
in the picture. `stale::one_placement` composes each page's own `settled⁻¹ ∘ asked` — matched by the
`Arc`'s address — and answers only where they all agree.

- A **scroll** moves every page by the same distance, so they agree exactly and a column reprojects
  as a single page always did. This is the common gesture and it is the one that works.
- A **zoom** does not, and the reason is `viewer_core::layout`'s own documented choice: the gap
  between rows is stated in *logical* pixels and does not scale with the magnification, while the
  pages either side of it do. So a placement read off the first page would move the second to
  somewhere it is not — `GAP × (1 − k)` per gap, two pixels at one zoom step — and the window waits
  for the real frame and says why.

**It was exact rather than tolerant, and that was the defect** — a scroll's placements differ in
the last bits of an `f32` inverse, so the sharp layer was refused for a share of a column's ordinary
view changes (ADR 0444, and the section at the head of this file). What replaced the exactness is a
bound with a derivation rather than a threshold: half a device pixel, at the worst corner of the
picture. What would remove the refusal rather than bound it is a placement *per page* — the presenter putting up
one textured quad per page instead of one for the frame — and that is a change to
`crate::renderer`'s three layers rather than to the policy. Nobody has asked for it; a zoom in a
column shows the previous frame unmoved for one render, which is what every view change did before
ADR 0378.

## What is left

**The base's comparison is no longer on this list** — the six-hundred-and-ninth session took it, and
the two items below with the identity above them are what remains.

**The processor's window**: `--cpu`, and a machine whose graphics device would not come up. There
is no retained encode to replay there, so the device path's mechanism does not apply — the attempt
is made once, refused, and never repeated, which is why nothing on that path is wrong today, only
absent.

It is a **smaller** piece of work than the device path was, and for a reason worth writing down:
`viewer_ui::software::SoftwareSurface` presents a raster the processor produced, so the host
already **has** the pixels of the frame on the screen. There is no capture to arrange and no
readback to price. What it needs is a resample of one window of RGBA under the same
`new ∘ old⁻¹` affine — on the processor, where every other pixel of that path is already produced —
and the same policy object deciding when.

Three things bind it, and none of them is new:

- **The five rules are the same five**, and they are already enforced by `Stale` and by
  `MustFollow`; what a processor path adds is a second producer of pixels, not a second policy.
- **Rule 2 is still structural**: the resample belongs in `stale.rs`, a private module of the
  binary, and not in `viewer_ui::software`, which is a *library* and is what
  `viewer-confined`'s worker and the software-surface tests link to.
- **Rule 4 needs its own measurement.** A processor-side resample of 800×1000 is not free, and the
  check is what it actually costs plus a refresh, rather than what the device path measured.
  `Stale::affordable` already takes whatever the run measures, so the code needs nothing; the round
  that builds it owes the number. **Rule 5 needs nothing from it at all** — a miss is a miss on any
  surface, which is one thing ADR 0384's re-grounding bought that was not the point of it.
- **And the base is already the right shape for it** — more so since ADR 0385, which moved it out
  of `Settled` and gave it the page and placement it is a picture of. A software surface has those
  pixels without a readback at all, so what a processor path adds is `Base::of` being fed from
  `SoftwareSurface` rather than from a capture. The composition, the re-basing, the refusal
  vocabulary and the clock are all shared and none of them knows which surface it is on.
- **Its refusal already says which kind it is**, and it is the one `Refusal::NoDevice` names: today
  the processor's window declines once, says *impossible*, and every view change after it says that
  no pixels are held and none can be read back. A round that builds this path deletes both lines
  rather than adding one.

## What is deliberately not here

Not progressive rendering (`doc/todo/16`'s road C), not a page turn — nothing about the outgoing
page's pixels says anything true about the incoming one — and not §12.4.4's transitions, which are
already a picture of two pages moving. ADR 0378 has each argument.

## The retained page was built in the six-hundred-and-eighth session — ADR 0443

**Everything the two sections below asked for exists**, in `crate::stale`'s `Proxies` and
`crate::renderer`'s idle turn: a whole page at `PROXY_EDGE` pixels along its longer side, drawn on
the render thread when nothing else is asked of it, placed under the base by the same composition,
one placement per page. The extent is `--proxy-pages N`, the host's flag, defaulting to
`PROXY_PAGES`; both constants carry the measurement that chose them and ADR 0443 has the tables.
The two sections below are kept because their *argument* is what the code rests on; four of their
claims have been corrected by running it and the corrections are here.

- **"A page turn and a `GoTo` have no base at all" is true only under `SinglePage`.** Both
  specification documents open in `OneColumn`, where the incoming arrangement shares a page with
  the outgoing one, so the base carries — and what the retained layer adds there is *the other page
  of the pair*, which used to vanish for the length of the render. The frame line says which:
  `approximated, over a retained page`.
- **The `SinglePage` page turn is still not answered, and the reason is the identity.** A retained
  picture is keyed by the `Arc<DisplayList>` it was drawn from, and `viewer_core::open` keeps "one
  entry rather than a cache of them" — so a page turn drops the outgoing page's interpretation and
  returning to it produces a new `Arc` with no picture held. Every such turn prints `another page —
  nothing to show`. **What is owed is a decision rather than a mechanism**: whether a stand-in may
  be a picture of a *superseded* interpretation of the same page. That is a different kind of wrong
  from blur — a layer toggled off would still be drawn for one frame — and it needs the argument
  rule 2's siblings all got, not a key changed in passing.
- **The scale was free in time and the file's illustration was the wrong axis.** "An eighth of
  device scale" assumed the cost was pixels; it is the display-list walk, flat over a sixty-fourfold
  range of raster sizes. What binds is memory, and the second thing the ladder showed: a proxy scale
  that followed the zoom would put a new glyph size in quorra's atlas at every step, and a repack
  costs the next real frame its whole geometry.
- ~~**`Refusal::Rearranged` fires on scrolls.**~~ **Taken in the six-hundred-and-ninth session**
  (ADR 0444), in the order this entry asked for: the bound comes from the geometry — the four
  corners of the picture, because the difference of two affines is convex and attains its maximum at
  a vertex — and half a device pixel is the raster's own quantisation, the largest distance a point
  can move without leaving the pixel it is a sample of. The measurement was taken *after*, and what
  it found is a separation rather than a threshold: a scroll's placements differ by 0 to 0.000183 px
  and a zoom step's by 1.25 to 2.75 px, four orders of magnitude apart with the bound between them.
  `stale::AGREEMENT` carries the derivation and both tables; the frame line prints the disagreement
  absorbed and the refusal prints the one refused, so the next round re-measures without
  instrumenting anything.

## A second thing is left, asked for by the owner on 2026-08-20: pixels for the area that was not there

**The module's own doc names this gap and has since it was written**: *"a scroll reveals an edge the
old raster has no pixels for, and anything the new view would draw that the old one did not is
simply absent."* So a reprojection answers a zoom **in** well and a zoom **out** or a scroll badly —
the base covers the old view and nothing covers the margin that has just appeared. Today that margin
shows nothing at all. The owner:

> Another thing we should consider: keep a version of the page (possibly low resolution) so that we
> can reproject onto it, when zooming out, this would allow us to display something onto the newly
> appearing area.

**Why the construction is a good one rather than merely a filler**, and this is the argument to keep:
its error is smallest exactly where it is needed. A whole-page proxy at a small fraction of device
scale is close to the *right* resolution when the view is zoomed out, which is when the margin
appears; it would be badly blurred zoomed in, which is when the base already covers the view. The two
sources are complementary rather than competing, so a reprojection becomes two layers — the proxy
underneath, covering the page, and the sharp base over it, covering what was already on the screen.

What binds it, none of it new:

- **Nothing on the launch path.** `CLAUDE.md`'s startup rules forbid anything eager before page one,
  so the proxy is produced *after* the first real frame — on the render thread, when it is otherwise
  idle — and never in front of it. A round that measures a launch regression here has built it wrong.
- **The five rules are still the five.** Rule 2 above all: the proxy and its resample belong in
  `stale.rs`, a private module of the binary, so that no gate, oracle or harness can link to a
  stand-in. Rule 1 is unchanged — a proxy is still never the last word.
- **Rule 3 has to grow one distinction.** "Approximated from the last frame" and "approximated from a
  low-resolution page" are *different amounts of wrong*, and a frame line that says only
  `approximated` for both has stopped saying what it is showing. The trace names which source filled
  which region, or the rule is weaker than it was.
- **The cost is not memory.** A page at an eighth of device scale is a couple of hundred kilobytes;
  what has to be priced is the render that makes it and where it is taken from — a crop of a real
  frame will not do at high zoom, because the frame is then a crop of the page rather than the whole
  of it.

**It pays somewhere this file has not been looking**, which is the argument for taking it above the
processor path: a page turn and a `GoTo` have **no base at all**, so they show nothing whatever the
device does. A retained page is the only thing that can stand in there.

Open, and for the round to settle rather than for this file to guess: whether the proxy is one page
or a small window of neighbouring pages, and what scale earns its keep.

### The proxy's extent is configurable — the owner, 2026-08-20

This file left one thing open for the round to settle: *"whether the proxy is one page or a small
window of neighbouring pages, and what scale earns its keep"*. The first half is answered and it is
answered by not choosing:

> regarding the question if we should have only one page or a small window of neighbouring pages.
> Make it configurable.

**Where the configuration lives is decided by rule 2 and is not free.** The stand-in is a private
module of a binary so that no gate, oracle or harness can link to one, so the neighbour count may
**not** become a `Command`, a field of a boundary type, or anything below `crates/viewer-ui/src/bin`
— that would make a wrong picture visible to the tree that judges pictures, which is the one thing
rule 2 exists to prevent. It is the **host's** setting: a flag today, and whatever a host's own
configuration becomes later, the same way `--cpu`, `--backend` and `--no-sandbox` are the host's.
`doc/todo/30`'s "all three hosts stay level" applies to it as it does to a feature.

Two things a round still owes, and being configurable does not excuse either. **A default is a
decision**, so it is chosen from a measurement — what a neighbour costs to draw and what it saves on
a page turn — and written down as one rather than picked. And **the scale is still open**, which was
the other half of the sentence: an eighth was this file's illustration, not a finding.

*Both were taken in the six-hundred-and-eighth session and both are in the constants' own doc
comments, with the tables: `crate::stale::PROXY_EDGE` and `crate::stale::PROXY_PAGES`. The section
at the head of this file says which of the four claims above the measurement contradicted.*
