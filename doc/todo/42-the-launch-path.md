# The launch path: 145 ms to the first frame, and what is on it

Status: **open**, measured in the two-hundred-and-seventy-fourth session.
Priority: 42 — performance, measured and priced, not yet taken
Corpus: every document; the two costs that scale do so with the *document*, not with page one
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`Launch`), `crates/pdf-model/examples/open_cost.rs`,
`crates/render-quorra/examples/bring_up.rs`, ADR 0179

## Why this is a todo and not a caveat

`CLAUDE.md`'s startup section states two rules this path breaks:

> **Nothing eager.** No system font enumeration, no full page-tree walk, no configuration or
> recent-file scanning, no thumbnail generation on the launch path.

> **Incremental parsing.** Opening a document reads the trailer and the objects page one needs —
> not the whole file. **A 500-page document must open no slower than a 5-page one.**

Measured: **27.8 ms against 0.84 ms**, on 1023 pages against 5. The rule was written down and
never instrumented, which is the same shape as every claim this project has found stale — with the
difference that this one is about a number, so the instrument settles it.

## The four items, in the order the timeline ranks them

### 1. `Document::open` — 22.2 ms on ISO 32000-2, 0.20 ms on a 5-page document

§7.5's trailer and cross-reference table, for 101 318 objects. Reading the trailer is O(1); what
costs is everything done to the xref sections after finding them. **Not yet localised** — the
next step is `callgrind_interpret`'s sibling: a `callgrind` run stopping at `Document::open`, which
says whether the cost is inflating the xref *streams*, building the entry map, or the recovery
scan deciding it is not needed.

The clause leaves room: §7.5.8's cross-reference streams are a compressed table, and a processor
that wants object 12 must find its entry, which does not require materialising the other 101 317.
Whether *this* tree can defer that is a question about `xref.rs`'s `Option<Location>` map (ADR
0100), and the answer changes what `was_recovered` and the writer can promise — so it is a design
question, not a micro-optimisation.

### 2. `Outline::read` — 6.716 ms for **38 items**

0.18 ms per outline item, on a document whose whole page tree walks in 0.486 ms. The cost is
therefore not the reading; something inside resolving an item is doing work proportional to the
document. **The suspect is the destination**: §12.3.3's item carries a `/Dest` or an `/A`, and
turning either into a page *index* means finding that page in the tree — per item.

This is the cheapest of the four to take and the one with a clause behind it (§12.3.3, §12.3.2.2's
named destinations, §7.9.6's name trees).

### 3. `signature::signatures` — 1.681 ms on a document with **no signatures**

§12.8's walk finds nothing and charges for it, on every launch of every document. Whatever it
walks — the AcroForm's field tree, most likely — the empty answer should be reachable without it.

### 4. The graphics device — 40 to 46 ms, and the backend set is not the lever

ADR 0179's table: restricting the wgpu instance to Vulkan moves the cost from instance creation
into `request_adapter` and the total does not move. What is left to try is **overlap**: instance
creation needs no window, and the launch path has 9 ms of document work and 8 ms of event-loop
work sitting in front of the window that the device waits for. Hoisting `wgpu::Instance::new` onto
a thread started at `main`'s first line would hide up to ~20 ms of it.

**That needs quorra's agreement**, because `Device::for_surface` creates the instance itself:
either an `Options::instance` or a `Device::for_surface_with(instance, …)`. It belongs in
`doc/QUORRA_FEEDBACK.md` with a measurement, and the measurement is the one in ADR 0179's second
table — including the part that says *`request_adapter` cannot be hoisted*, because it takes the
surface, so the honest claim is "up to the instance's share" and not "up to the bring-up's".

## What is deliberately not here

**The first present (54 to 68 ms) is not on this list yet**, because nothing has taken it apart.
It contains the first frame's pipeline waits, the first buffer allocations and the page's own
rasterisation, and `render-quorra/examples/frame_race.rs` measures the last of those in isolation
at a fraction of it. A number nobody has split is not an item; it is the next measurement.
