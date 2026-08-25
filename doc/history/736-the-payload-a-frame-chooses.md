# 736 — The payload a frame chooses, and the tier that could not be per page

ADR 0626's codec wired into ADR 0607's boundary: a frame now carries either the pixels or the
marks, chosen per page in the confined process by comparing two byte counts. ADR 0633.
Date 2026-08-25.

## The three blockers, settled in the order they block each other

ADR 0626 section 6 gave three reasons for not wiring the codec in, and `doc/todo/15` carried them.
They are not independent:

**Which host rasterises** is the design question and it decides the rest. ADR 0607's own sentence
points at `viewer-ui`, and the measurement behind it is what makes that more than a preference —
a process holding a graphics device cannot be confined at all, so the device is the host's by
necessity. `viewer-confined` therefore takes **no rasteriser**, and `Framed::payload` is either
`Payload::Raster(Raster)` or `Payload::List { list: Arc<DisplayList>, target: TargetSpec }`.

**Breaking every consumer** follows from that and was small: three of them, all in this tree.
`viewer-ui` does not use this boundary yet, which is why the round could be taken at all.

**`MAGIC`** follows from that, and moved once: `PDFVCF03` → `PDFVCF04`.

The `TargetSpec` crosses rather than being rebuilt from a page size and a scale — thirty-two bytes
so that trap 12a's question is never asked — and the `Arc` is the arm's whole point, because a
host that keeps it re-rasterises a zoom without asking the child anything.

## The bound this arm needed, which is 719's finding somewhere new

Every other length on this boundary costs the sender what it costs the reader: a raster's
dimensions are checked against its samples, a display list's tables precede the identifiers that
index them. **A render target is two `u32`s that become the host's own allocation**, out of a
frame under two hundred bytes long — which is exactly the shape 719 found on the raster arm.
`decode_list_payload` refuses a dimension of zero, a dimension past `pdf_render::MAX_EXTENT`, and
a pixel count past `viewer_core::MAX_PIXELS`, the last of which is the budget a *tier-1* render
request is held to and so refuses precisely what an honest worker cannot have produced.
`confined_wire` asserts all three of anything that decodes.

Not refused: a non-finite transform, for ADR 0626 section 7's reason — the confined path must draw
the page the in-process path draws, and a fourth answer to that question in a codec would be a
fourth place they could differ.

## The finding: `viewer-core` has one tier per viewer and it needs one per page

The confined worker **still draws every page**, including the ones it ships as marks, and this is
where the round stopped rather than pushing on.

A worker that skipped the render would be answering `Rendered::Presented`, which is a statement
about the *viewer*: it sets `holds_rasters` false for all pages at once, so `Query::Frame` goes
silent about the pages that must still cross as pixels, and `raster_budget()` becomes `u64::MAX`
— so `MAX_PIXELS` stops bounding what a **confined** process is asked to draw. Inside a
confinement an unbounded raster is not a refusal; it is the abort ADR 0597 spent a round turning
back into a sentence. Giving that up to save a rasterisation is the wrong trade, and principle 3
is why.

What removes it is a `viewer-core` outcome meaning *the host took the request's own list* — the
first thing in ten sessions on that boundary to need a change there rather than a message.
`doc/todo/15` and `doc/ui-boundary.md` both hold it.

## The other finding: an encoder that finishes to learn what it already knows

ADR 0626 encoded a list in full and then compared it, with the cost written down — right for a
codec run once per corpus, wrong the moment it is on the frame path, where **that buffer is paid
per render by exactly the pages that get nothing for it.** So `crossing` hands the encoder the
raster's size and the encoder stops at it. Two checks in `write_list` cover both shapes and
neither restates the format: the body is already written so its length is a fact, and interning
holds the samples' `Arc`s rather than copies so summing them is a lower bound on what the tables
are about to write.

`scan-bad.pdf` chooses in **4 µs** where finishing cost **6.4 ms and 33.7 MB**;
`issue12841_reduced.pdf` in **5 µs** against **15.5 ms and 80.1 MB**. Per render, inside a
four-gibibyte ceiling.

## And a second implementation that had gone stale unnoticed

Bumping `MAGIC` made `fuzz/seed_confined_wire.py` fail — asserting `PDFVCF02`, one behind. The
seeder is a hand-written frame layer whose *disagreeing* with the Rust is half its value, so it
had been refusing to run, and `confined_wire`'s corpus had been empty, since `03` landed. The
assertion is right and a *copy* of the constant is not: it now reads `MAGIC` out of `protocol.rs`.

## What the numbers say, both arms in one sitting

`examples/confined_page`, same worker, same viewport, load average 4.9 to 14.8 with three other
rounds building beside it — so the byte counts carry the argument and the milliseconds are
ranked rather than pinned. ADR 0633 section 6 has the two tables. The short of it: the sparse
page's frame went from ADR 0607's measured **3.915 ms** of pixels to **0.074 ms** of marks
(35 580 B against 4 075 200 B), ISO 32000-2's densest first page to **0.878 ms** (1 011 568 B),
and the two scans still cross as **pixels** in 3.2 and 4.4 ms, which is the decision working in
both directions. The codec costs about 0.06 ms on the sparse page and 0.4 ms on the dense one.

`examples/list_over_the_wire` gained a `crossing_ms` column so that the *choice* is timed apart
from the exact encoding, which is what makes the paragraph above checkable.

## What says the wiring is on

Every comparison in `tests/confined.rs` goes through one helper that draws the marks where marks
are what crossed, so all of them are true of either arm — which is what makes them good
comparisons and useless as evidence that anything changed. Three tests say it instead: a sparse
page crosses as marks, a scan crosses as pixels, and the two deferred producers reach the raster
arm **by variant** on the four pages ADR 0626 named. And the strongest one is the oldest, now
running across the new arm: the page a host draws from a list it did not interpret is
byte-identical to the page the unconfined viewer draws.

## The gates

Whole, on the briefing's own instruction: a change to what crosses the confinement can move any
page. Both workers built first (trap 10), and every changed crate's `src/lib.rs` touched before
the release build (trap 10b). The two fuzz runs were taken before the sequence rather than beside
it: `confined_wire` clean at 1 500 000 runs on a freshly seeded corpus, `display_list` clean at
25 550 340.

## Ledger

Untouched. This is `CLAUDE.md` principle 3 against principle 2 and cites no clause.
