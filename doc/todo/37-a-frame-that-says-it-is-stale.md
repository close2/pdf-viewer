# A frame that says it is stale — the one window it does not cover yet

Status: **built for the window with a graphics device** (ADR 0378, extended by ADR 0383, rule 5
corrected by ADR 0384 and the base's lifetime by ADR 0385, each after the owner found it did not
fire on a real one), which is every run without `--cpu`. A view change whose last frame was slow
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

## What is left

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
