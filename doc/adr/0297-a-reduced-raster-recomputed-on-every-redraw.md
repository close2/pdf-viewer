# ADR 0297 — A reduced raster recomputed on every redraw, and the entry nobody could reach

Status: accepted, 2026-08-13. Session 462. Closes item 2's leftover in
[`doc/todo/45`](../todo/45-where-a-frame-goes.md), which ADR 0228 opened and priced as needing a
witness. Amends `render-quorra`'s cache module comment; does not amend ADR 0025, whose departure
this keeps rather than changes.

## The question

ADR 0228 divided `Image::area_averaged` across rayon and hoisted its column bands, taking a
2700×3450 image reduced threefold from 22.4 ms to 2.9, and left one sentence behind:

> the reduced raster is still *transient*, recomputed on every frame that draws the image. On this
> witness that costs nothing — each page is drawn once — but a scroll, a selection, a caret blink
> or a resize redraws the same page […] What it needs before it is worth building is a witness: a
> redraw-heavy session measured with `--trace=frames`, which nothing in this project has taken.

Nothing had taken it because ADR 0228's own witness is 38 *page turns*, and a page turn draws each
page once. This session took the redraw witness instead.

## The measurement

`tmp/windows/NorthAmerican.30MB.pdf` — the owner's 65-page scan, one 2700×3450 photograph a page —
under `Xvfb` at 1200×1500 on `llvmpipe`, `--trace=frames`, in an 800×1000 window. Two `+` to put
the page past the window's height, then twenty `Down`, which is twenty redraws of **the same page
at the same scale**: same samples, same placement, same reduction.

```sh
Xvfb :78 -screen 0 1200x1500x24 &
DISPLAY=:78 ./target/pdf-viewer --trace=frames tmp/windows/NorthAmerican.30MB.pdf
DISPLAY=:78 xdotool key --delay 400 plus plus
for i in $(seq 20); do DISPLAY=:78 xdotool key --delay 250 Down; done
```

A scroll step cost **12.7 to 16.8 ms**, of which `scene` was **8.5 to 9.8** on a display list of
**one command**. A scratch build — two `Instant`s round the call, not committed — attributed it:

```text
SCRATCH source 2700x3450 ptr 0x7f571e6f0020 averaged Some((1350, 1725)) in 8.969 ms
SCRATCH upload 0.002 ms
trace: frame p1 1cmd presented 15.6 | host 0.0 scene 9.8 device 6.4 settle 0.0 | 1 up, 0 culled
```

Three things in it, and the third is what made a cache possible at all:

- **The averaging is the whole of `scene`.** 8.5–9.8 ms of an 8.5–9.8 ms translation.
- **The upload is not the cost.** 0.002 ms to hand it over, and the summary's `transfer` row is a
  0.8 ms median — this is host processor time and one pass over the *source's* 9.3 M samples, which
  is what ADR 0228 says the work is proportional to.
- **The source `Arc` is stable across redraws and changes on a page turn.** The same pointer for
  every one of the twenty steps; a different one after `Right` and back. A display list is rebuilt
  per page and not per frame, so there is an identity to key on and it is exactly a page's.

## The decision

**Keep the reduced raster in `render-quorra`'s resource cache, keyed by the source image's `Arc`
identity together with the two reduction factors.**

Exactness is the same statement ADR 0025's departure already rests on: every output byte is a
function of the source samples and the two factors. Both are in the key, so a hit is the raster a
miss would have produced, and nothing else in the frame changes.

Two things had to be true and neither was free.

**1. A backend has to name the reduction before it pays for it.** `pdf_render::Image::reduction`
is new and is the split: it answers the factors, the reduced dimensions and the filter — two vector
lengths and two divisions — where `area_averaged` carries the reduction out. `area_averaged` is
written in terms of it, so every refusal is one function's and the two cannot disagree; a unit test
asks them both over seven placements spanning both regimes of both axes and compares every field.

The filter is in the `Reduction` and not derived from the factors, which is a real edge rather than
caution: an axis whose factor clamps to one is an axis that may be *magnified* while the other is
reduced, so `smoothed` is not implied by `(kx, ky)`. §8.9.5.3's rule is one function now, asked by
`is_smoothed` about an image's own grid and by `reduction` about a grid no `Image` exists for yet.

**2. The pin had to acquire an exit.** The cache pins the `Arc` it is keyed by, for the ABA reason
its module comment gives — and the key here is the *source's* address, so a reduced entry pins a
whole scanned page's 37 MB against a device-side budget that counts only the 9 MB it uploaded. On a
65-page scan that is the owner's stated "1 GB is definitely too much" reached by bookkeeping.

What settles it is a proof rather than a policy, and it applies to all three caches: **an entry
whose pin is the only reference left to that allocation can never be looked up again**, because no
display list can hold the address the pin holds. `ResourceCaches::drop_unreachable` releases those
after every frame, before the budget question, and an entry this frame used is kept whatever its
count says — `evict_settled`'s own rule, kept because a rule that costs nothing is cheaper than a
proof that it cannot be reached.

So the change needs **no new memory argument**: the entry's own bytes are the device's and were
already inside a stated budget, and its pin outlives the display list by exactly one frame.

**Deferred images stay transient**, and the reason is the same one that made this possible: an
`ImageSource::AtDeviceScale` produces its samples for this placement, so its `Arc` is this frame's
and there is no identity for a key. An entry would be a leak with a lookup on it.

## What it is worth

A/B in one sitting, the two release binaries built from the same tree with and without the change
and run alternately, three runs an arm, the recipe above:

| over 23 frames | before | after |
|---|---|---|
| median frame | 15.2 / 15.0 / 15.0 ms | **4.7 / 4.8 / 4.8 ms** |
| median scene | 8.9 / 8.9 / 8.9 ms | **0.0 / 0.0 / 0.0 ms** |
| sum, scene | 197.9 / 197.3 / 203.6 ms | **16.9 / 15.7 / 16.3 ms** |
| sum, frame | 359.6 / 358.4 / 369.3 ms | **155.3 / 159.4 / 158.9 ms** |
| resource uploads | 23 | **2** |

**A scroll step is 3.1× faster and the spreads are a tenth of the difference.** Two uploads rather
than twenty-three is the reduction being produced once per *magnification* — the initial fit and
the one the two `+` settle on — instead of once per frame, which is the claim `uploads` exists to
make (`FrameCost::uploads`, ADR 0228).

The residual `sum, scene` of 16 ms is the two misses and the page turns' own translation; the
median is zero because the median frame does no translation work at all.

## What did not move

- `render-quorra/tests/corpus.rs` at scale 1: **919 agree, 37 differ, 1 refused, 17 not
  comparable**, ratchets held.
- The same gate at **scale 4**, run before and after: `929 agree, 16 differ, 7 refused, 22 not
  comparable` and the same seven refusals by name, character for character. That run is the
  instrument that found the eviction problem this cache lives inside (533 pages refused, ADR 0156),
  and it is the one that would show a retained raster crowding the budget. It shows nothing.
- The oracle, the corpus gate, both text gates, dates, XMP, JPEG 2000 and the conformance checker.

## What this does not do

**The CPU backend still recomputes.** `render-cpu` and `render-gpu` call `area_averaged` per draw
as they did, and `pdf_render::Image::reduction` is available to both. The CPU backend is the
correctness oracle and the frame the device refuses, and `viewer-confined`'s worker rasterises with
it — so a confined host redrawing a scanned page still pays the 9 ms. It is left because those two
have no per-frame resource cache to hang an entry on and would each need their own bound, and
because the number above was taken in the window, which is the path a person waits on.
`doc/todo/45` carries it.

**And a retained *scene* is still the lever `encode` needs.** This removes the translation's cost
on an unchanged page; item 3's 3.86 µs a command is quorra's side of the same frame and is
untouched.
