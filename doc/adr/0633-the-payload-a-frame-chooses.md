# ADR 0633 — The payload a frame chooses, and the tier that could not be per page

Status: accepted, 2026-08-25. Session 736. Wires ADR 0626's codec into ADR 0607's boundary, which
is what `doc/todo/15`'s road B has been waiting on. Cites no clause: this is `CLAUDE.md` principle
3's boundary against principle 2's frame, and the ledger is untouched.

## What was owed, and the order the three blockers actually stand in

ADR 0626 §6 declined the wiring for three reasons and `doc/todo/15` carried them. They are not
independent, and taking them in the order they block each other is most of this decision:

1. **which host rasterises** — a design question whose answer decides what the reply carries;
2. **it breaks every consumer of `Reply::Frame`** — a consequence of 1, not a separate question;
3. **it bumps `MAGIC`** — a consequence of 2, and the last thing to do rather than the first.

## 1. `viewer-confined` takes no rasteriser, and the reason is the constraint 724 measured

ADR 0607's own sentence — "`render-quorra` is that translator and `viewer-ui` is already on it" —
points at the host rather than at this crate, and the measurement behind it is what makes that
more than a preference: **a process holding a graphics device cannot be confined**, at any
ordering, because it dies on its first `ioctl` and cannot install Landlock under this crate's
descriptor ceiling. So the device is the host's by necessity, and the crate that speaks for the
confined process has no business owning a rasteriser.

What follows is the shape of the reply. `Reply::Frame` carries a `Vec<Framed>`, and `Framed` now
carries a `Payload`:

- `Payload::Raster(Raster)` — the pixels, as before;
- `Payload::List { list: Arc<DisplayList>, target: TargetSpec }` — the marks, for whoever holds a
  device.

**The `TargetSpec` crosses rather than being rebuilt.** A host could in principle rebuild one from
the list's `page_size` and a scale, and it would be wrong the moment a tile offset or the y flip
differed — which is `doc/traps/the-interactive-loop.md`'s trap 12a, a doc comment that claimed the
display list's space *was* the raster's. Thirty-two bytes buys the question never being asked.

**The `Arc` is the point of the arm and not an optimisation.** The host keeps it, so a zoom or a
scroll is a new `TargetSpec` over marks it already holds — the property
`zooming_rasterises_again_without_interpreting_again` already asserts in process, now available
across a confinement.

## 2. The bound this arm needed, which is 719's finding in a new place

Every other length on this boundary is checked against bytes the sender actually wrote. A raster's
dimensions are compared with its samples; a display list's tables precede the identifiers that
index them. So a claim costs the sender what it costs the reader, and a nine-byte message cannot
ask the host for a gigabyte.

**A render target is the exception, and it had to be made one.** It is two `u32`s that become
however many pixels the *host* asks its allocator for when it draws, out of a frame under two
hundred bytes long. That is exactly the shape the seven-hundred-and-nineteenth session found on
the raster arm — a subverted worker claiming 2 GiB and a host asking for it — arriving somewhere
new. `decode_list_payload` therefore refuses:

- a dimension of zero, which is a target nothing can be drawn into;
- a dimension past `pdf_render::MAX_EXTENT`, which is `TargetSpec::for_page`'s own `f32`
  precision limit and applies whatever the tier;
- a pixel count past `viewer_core::MAX_PIXELS`, which is the budget a **tier-1** host's render
  request is held to — and the confined worker is one, so this refuses exactly what an honest
  worker cannot have produced.

`fuzz/fuzz_targets/confined_wire.rs` asserts all three of anything that decodes at all, beside the
raster invariant it already had, and the target's own header says why the middle one is different
from the others.

What is *not* refused is a non-finite number in the transform, deliberately and for ADR 0626 §7's
reason: the three places in this tree that ask that question are in `pdf-render`, where a device
decision belongs, and a fourth answer in a codec would be a fourth place the confined path and the
in-process path could differ.

## 3. The choice is made where both sizes are known, and one of them needs no rasteriser

`Event::NeedsRender` is that place. The list is in the request; the raster's price is the target's
pixel count times four, which is arithmetic — and is exactly the number the decoder checks a
crossing raster against, so the two sides of the comparison are one statement rather than two.

`protocol::Marks` carries the answer from there to the moment a host asks `Query::Frame`, and it
has three properties that are not tidiness:

- **It keeps the greatest token per page.** `viewer-core` asked for one page twice keeps the
  *later* answer; the worker performs the two in whichever order its own queue pops them. A store
  that took the last write could hold the marks of a render the viewer threw away, and the host
  would draw an interpretation nothing on the screen was of.
- **It compares dimensions before it hands anything over.** A raster the viewer is holding and a
  target this store recorded describe the same render only if they are the same size; where they
  disagree the pixels — which the viewer holds *now* — are what crosses.
- **It is bounded by the pages on the screen**, asked of the viewer after every command rather
  than deduced. A confined process runs under an address-space ceiling, and one encoded list per
  page a reader ever scrolled past is a slow leak in the one process that must not have one.

## 4. The tier that could not be per page, which is why the worker still rasterises

**This is the round's finding and it is the honest limit on what was built.**

A worker that skipped the rasterisation for a page crossing as a list would be a *tier-2* host of
`viewer-core`, and `viewer-core` has one tier per **viewer**, not one per page:
`Rendered::Presented` sets `holds_rasters` false for the whole viewer, `Query::Frame` then answers
`Answer::None` for every page including the ones that must cross as pixels, and — the part that
decides it — `raster_budget()` becomes `u64::MAX`, so `MAX_PIXELS` stops bounding what the
confined process is asked to draw. Inside a confinement an unbounded raster is not a refusal; it
is the abort ADR 0597 spent a round turning back into a sentence.

So the worker stays tier 1 and draws the page either way. What that costs is one CPU
rasterisation of pixels that are not sent, on the pages that cross as a list — and the shape of
the fix is a `viewer-core` change with a name: an outcome that says *the host took the request's
own list*, which holds the page's place without giving up the budget. `doc/todo/15` states it as
what remains.

It is worth being clear about what this does and does not cost, because it is not a wash. Under
the boundary as it now stands the host receives marks and draws them itself, which is what a
tier-2 host on a confinement needs and what did not exist at all before; the child's redundant
raster is CPU inside a confined process, and the saving is bytes and latency on every frame.

## 5. The encoder stops at the number it is being compared against

ADR 0626 encoded a list in full and then compared it, with the cost written down as "one buffer,
on the four percent of pages where the answer is pixels". That was the right call for a codec
exercised once per corpus run. **On the frame path that buffer is paid per render**, and the
population that pays it is exactly the population that gets nothing for it.

So `crossing` hands the encoder the raster's own size and the encoder stops when it passes it.
Two checks in `write_list` cover both shapes of an oversized list and neither is a second
statement of the format: the body is already written, so its length is a fact; and the samples
table is not written yet, but interning holds `Arc`s rather than copies, so summing the slices it
is about to write is a *lower bound* on this format's output whatever the format later decides to
write around them.

`Uncodable::TooLarge` is the refusal, `RasterReason::Larger`'s figure becomes a lower bound where
it fires and says so, and `encode` — which the seeder and `examples/list_over_the_wire` use — passes
no budget and is still the exact price.

## 6. What it measures, both arms in one sitting

`examples/confined_page`, one machine, **load average 1.5 to 2.4** — the neighbouring rounds had
finished, which is worth saying because ADR 0607's own transport figure was taken at load 12 and
the two are being compared. `examples/list_over_the_wire` gained a `crossing_ms` column so that
the *choice* is timed apart from the exact encoding.

**Both arms, one sitting** — four documents, four processes, one run of one binary, nothing else
on the machine:

| page | crosses as | payload | crossed in |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1, 849×1200 | marks | **35 580 B** against 4 075 200 B of pixels | **0.074 ms** |
| ISO 32000-2 p1, 849×1200 | marks | **1 011 568 B** against 4 075 200 B | **0.778 ms** |
| `scan-bad.pdf` p1, 900×1165 | pixels | 4 194 000 B | **2.758 ms** |
| `issue12841_reduced.pdf` p1, 900×1165 | pixels | 4 194 000 B | **3.312 ms** |

The bottom two rows are the *control*, and they are what makes the top two mean something: the
raster arm still exists, still runs, and is still what a scan takes — 2.8 to 3.3 ms for four
megabytes, which is ADR 0607's 3.915 ms at load 12 reproduced on a quiet machine. Against that,
the sparse page's frame is **0.074 ms** and the densest first page in this tree is **0.778 ms**.
A list would have been 8.7× and 20.7× its raster on those two scans and the comparison says so,
which is ADR 0607's decision working in both directions rather than a display list winning by
fiat.

**What the codec costs, which is the other half**, same sitting:

| page | choice (`crossing_ms`) | exact encode | decode |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 0.029 ms | 0.015 ms | 0.025 ms |
| ISO 32000-2 p1 | 0.216 ms | 0.202 ms | 0.182 ms |
| `scan-bad.pdf` | **0.004 ms** | 6.601 ms | 15.648 ms |
| `issue12841_reduced.pdf` | **0.006 ms** | 15.967 ms | 35.496 ms |
| `function_based_shading.pdf` | 0.008 ms | — | — |
| `issue16263.pdf` | 0.006 ms | — | — |

The codec costs about 0.05 ms on a sparse page against roughly 2.7 ms of transport removed, and
about 0.4 ms on the dense one against about 2.0 ms removed. Net in both, and net by a wide margin
on the page a window actually shows. **And section 5's stop is worth what it looked like it would
be**: the two scans choose in four and six *microseconds* where finishing would have cost 6.6 and
16.0 milliseconds and 33.7 MB and 80.1 MB of transient allocation, per render, inside a process
under a four-gibibyte ceiling.

`crossing_ms` under `encode_ms` on the two pages that cross as lists is the same work twice, warm
the second time; the column is there for the two rows where the numbers are three orders of
magnitude apart.

## 7. The refusals stay by name, on the pages ADR 0626 named

`the_two_deferred_producers_reach_the_raster_arm_by_name` asserts the *variant* rather than the
direction, on all four corpus pages: `function_based_shading.pdf` and
`function_based_shading_cmyk.pdf` reach `Uncodable::DeferredColours` (§8.7.4.5.2's type 1
shading), `issue16263.pdf` and `issue19517.pdf` reach `Uncodable::DeferredImage` (§11.6.5.2's
soft-mask image on a grid of its own). Trap 5 is why it is by variant: a page that quietly went
blank satisfies any assertion that only counts arms.

`a_scanned_page_crosses_the_confinement_as_pixels` is the other arm end to end, through a real
worker, and `a_sparse_page_crosses_the_confinement_as_marks_rather_than_as_pixels` is what says
the wiring is switched on at all — every other comparison in that file goes through a helper that
draws the marks where marks are what crossed, and is therefore true of either arm. That helper is
also the strongest assertion in the round: **the page a host draws from a list it did not
interpret is byte-identical to the page the unconfined viewer draws**, which is what
`the_confined_process_draws_the_page_this_one_would_have` has always said and now says across the
new arm.

## 8. `MAGIC` moved once

`PDFVCF03` → `PDFVCF04`. ADR 0626 deliberately left it where it was, because nothing it added
crossed in a frame and bumping would have refused a worker speaking the same protocol. Something
new crosses now.

**And bumping it found that a second implementation had gone stale unnoticed.**
`fuzz/seed_confined_wire.py` — the hand-written frame layer whose disagreeing with the Rust is
half its value — asserted `PDFVCF02`, one behind. So it had been refusing to run, and
`confined_wire`'s corpus had been empty, since whenever `03` landed. The assertion is right and a
*copy* of the constant is not: the seeder now reads `MAGIC` out of `protocol.rs`, which is
`CLAUDE.md`'s rule about a fact that can be counted applied to a constant in another language.
Re-seeded, the target is clean at 1 500 000 runs.

## What would change this decision

- A `viewer-core` outcome that is per page rather than per viewer, which would let the confined
  process stop drawing what it does not send (section 4).
- A page population where the list is routinely larger than the raster, which is ADR 0607's own
  condition and whose instrument is `examples/list_over_the_wire`.
- A host that wants the *reason* a page crossed as pixels. Nothing on the wire carries it today
  and nothing would draw it; the reason is named in the tree and in the tests instead.
