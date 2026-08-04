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

### 1. `Document::open` — **taken in the two-hundred-and-seventy-sixth session, 41% off**

§7.5's trailer and cross-reference table, for 101 318 objects. Localised with
`crates/pdf-syntax/examples/callgrind_open.rs` and fixed the same round: **40% of it was one
searched `BTreeMap` insert per entry**, re-deciding §7.5.6's "most recent copy" rule 101 318 times
for a rule that is a property of the whole file. One stable sort and one bulk build instead —
130.7 M instructions per open to 76.6 M, and the launch timeline's `document open` step from
+27.8 ms to +21.0. ADR 0180, with two allocation fixes beside it.

**What is left of this item is the design question, and it is still open.** §7.5.8's
cross-reference streams are a compressed table, and a processor that wants object 12 must find its
entry — which does not require materialising the other 101 317. Whether this tree can defer that is
a question about `xref.rs`'s `Option<Location>` map (ADR 0100), and the answer changes what
`was_recovered` and the writer can promise. The measured floor now: 76.6 M instructions, of which
inflating the two cross-reference streams is 18 M and nothing can remove it, so the remaining
ceiling on this route is roughly a further 40%.

### 2. `Outline::read` — 3.35 to 6.61 ms for **988 items**

**This entry said "6.716 ms for 38 items" when it was written, and 38 is `items.len()`** — the top
level, which for a book is its chapters. 988 items at 3.4 to 6.7 µs apiece is one indirect object
fetch and a text-string decode each, which is proportionate; the example prints both counts now.

So this is not a defect and the item is **eagerness** rather than cost: 3 to 7 ms of a launch is
spent reading a panel nobody has opened. §12.3.4's thumbnails are already deferred to the first
time their tab is shown, with the argument written down; the outline is not, and the reason it is
not is a real one — `Open::around` needs it for the title bar's section (`Outline::section_at`)
before the first frame. **The question to settle is whether the section is worth 3 to 7 ms at
launch**, or whether it should arrive on the frame after the page.

### 3. `signature::signatures` — **taken in the two-hundred-and-seventy-seventh session**

§12.8's field walk found nothing and charged 1.681 ms for it, on every launch of every document
with a form. The empty answer was reachable without it and the *standard says so*: §12.7.3's
Table 225 bit 1 exists so that a processor need not "scan the entire document for the presence of
signature fields", and Table 224 defaults the entry to 0. **1.681 ms → 0.017 ms**, counted over
the corpus before it was trusted, and it corrected a ledger row that had called the entry
"signature behaviour". ADR 0181.

### 4. The graphics device — 40 to 46 ms, and the backend set is not the lever

ADR 0179's table: restricting the wgpu instance to Vulkan moves the cost from instance creation
into `request_adapter` and the total does not move. What is left to try is **overlap**: instance
creation needs no window, and the launch path has 9 ms of document work and 8 ms of event-loop
work sitting in front of the window that the device waits for. Hoisting `wgpu::Instance::new` onto
a thread started at `main`'s first line would hide up to ~20 ms of it.

**That needs quorra's agreement**, because `Device::for_surface` creates the instance itself:
either an `Options::instance` or a `Device::for_surface_with(instance, …)`. **Asked for, with the
measurement, in `doc/QUORRA_FEEDBACK.md` §8.2** — `bring_up overlap` puts it at 44.4–50.0 ms one
after the other against 22.9–28.9 both at once — along with §8.1's field split, because the one
number quorra reports as `adapter_enumeration` is three steps with different causes. What is
*not* asked for is a backend knob: §8.3 records having measured it and found the total invariant.

`request_adapter` cannot be hoisted either way — it takes the surface — so the honest claim is
"up to the instance's share" and not "up to the bring-up's".

## What is deliberately not here

**The first present (54 to 68 ms) is not on this list yet**, because nothing has taken it apart.
It contains the first frame's pipeline waits, the first buffer allocations and the page's own
rasterisation, and `render-quorra/examples/frame_race.rs` measures the last of those in isolation
at a fraction of it. A number nobody has split is not an item; it is the next measurement.
