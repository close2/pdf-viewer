# A frame that says it is stale — the one window it does not cover yet

Status: **built for the window with a graphics device** (ADR 0378, extended by ADR 0383), which is
every run without `--cpu`. A view change whose last frame was slow now shows the pixels already on
the screen, moved to where the new view puts them, and the real frame replaces it — the frame line
says `approximated`, the summary counts them, and
`crates/viewer-ui/src/bin/pdf-viewer/stale.rs` carries the five rules with the thing that enforces
each.
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
  threshold is `SHARE` times what it actually costs rather than what the device path measured.
  `Stale::threshold` already takes whatever the run measures, so the code needs nothing; the round
  that builds it owes the number.
- **And the base is already the right shape for it.** `Stale::Settled` holds the last real frame's
  pixels since ADR 0383, captured once and resampled after that; a software surface has those
  pixels without a readback at all, so what a processor path adds there is `Base::of` being fed
  from `SoftwareSurface` rather than from a capture. The composition, the re-basing and the clock
  are all shared and none of them knows which surface it is on.

## What is deliberately not here

Not progressive rendering (`doc/todo/16`'s road C), not a page turn — nothing about the outgoing
page's pixels says anything true about the incoming one — and not §12.4.4's transitions, which are
already a picture of two pages moving. ADR 0378 has each argument.
