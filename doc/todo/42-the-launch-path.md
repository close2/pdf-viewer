# The launch path: 110 to 119 ms to the first frame, and what is left on it

Status: **open**, measured in the two-hundred-and-seventy-fourth session; four of its five items
are closed and the fifth is quorra's (§9 of `doc/QUORRA_FEEDBACK.md`).
Priority: 42 — performance, measured and priced, not yet taken
Corpus: every document; the two costs that scale do so with the *document*, not with page one
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`Launch`), `crates/pdf-model/examples/open_cost.rs`,
`crates/render-quorra/examples/bring_up.rs`, `first_frame.rs`, ADRs 0179, 0180, 0181, 0182

## Why this is a todo and not a caveat

`CLAUDE.md`'s startup section states two rules this path breaks:

> **Nothing eager.** No system font enumeration, no full page-tree walk, no configuration or
> recent-file scanning, no thumbnail generation on the launch path.

> **Incremental parsing.** Opening a document reads the trailer and the objects page one needs —
> not the whole file. **A 500-page document must open no slower than a 5-page one.**

Measured when this file was written: **27.8 ms against 0.84 ms**, on 1023 pages against 5. The
rule had been written down and never instrumented, which is the same shape as every claim this
project has found stale — with the difference that this one is about a number, so the instrument
settles it.

**And the instrument now says the rule holds, by a route nobody had considered.** 41% of the open
went in ADR 0180; what was left went *beside* the window in ADR 0182 and is joined after a device
bring-up that costs 13 to 19 ms. Measured in the two-hundred-and-eighty-ninth session, three runs
each:

```text
                         ISO 32000-2, 1023 pages    PDF20_AN001-BPC, 5 pages
document joined            +5.2 to +5.6 ms            +2.6 to +5.4 ms
process start → frame      110 to 119 ms              105 to 124 ms
```

**The two documents cost the launch the same**, and what the join measures is the handshake rather
than the file. That is the rule satisfied where a person can see it. What is *not* satisfied is the
rule read as a statement about `Document::open` itself — 10 to 13 ms against 0.2 — and item 1 below
keeps that question open, now correctly labelled as a question about the function rather than about
the launch.

## The five items, in the order the timeline ranked them

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

### 1a. …and what was left of it is now *beside* the window rather than in front of it

**Taken in the two-hundred-and-eighty-first session** (ADR 0182). Nothing the window needs — an
event loop, a window, a graphics device — depends on the document, and the document depends on
none of them, so `main` opens it on a thread of its own and `resumed` joins that thread *after*
the presenter exists. `document joined` now lands 3 to 6 ms after `graphics device`, where
`document open` used to be a step of 21 to 28. `viewer-core`'s rule 4 is kept: the core is made on
that thread and moved back, single-threaded throughout.

**So items 1, 3 and 5's second bullet are the only launch costs still on this list that are ours**,
and item 1's is the design question rather than a number.

### 2. `Outline::read` — **closed by ADR 0182's structure, in the two-hundred-and-eighty-ninth**

The item was *eagerness*: 3 to 7 ms of a launch spent reading a panel nobody had opened, on a
document tree whose §12.3.4 thumbnails are deferred with an argument and whose outline is not.
**It costs the launch nothing now** — it happens on the thread that opens the document, which
finishes while the main thread is still bringing the graphics device up, and the join costs 5 ms
whether the outline has 988 items or 5. Deferring it would now buy zero milliseconds and cost the
title bar its §12.3.3 section on the first frame.

What is recorded rather than removed: the reason it is *allowed* to be eager is that something
slower runs beside it. If the device ever became free, this would be back.

### 2a. What the item said before, and it was wrong twice

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

### 4. The graphics device — **taken in the two-hundred-and-eighty-eighth session, 30 ms off**

ADR 0179's table said restricting the wgpu instance to Vulkan moves the cost from instance
creation into `request_adapter` and the total does not move, and that what was left to try was
**overlap** — an instance needs no window. That needed quorra's agreement, because
`Device::for_surface` created the instance itself.

**It agreed** (`doc/QUORRA_FEEDBACK.md` §8, answered at `7d5dafb`, their ADR 0014): five startup
fields where there were three, and `create_instance` + `for_surface_with_instance` so a host can
make the instance early. `main` now spawns *two* threads at its first line — one for the document,
one for the instance — and `resumed` joins the instance before building the presenter and the
document after it, because the device needs the first and not the second.

```text
graphics instance   +0.006 to +2.6 ms   (hidden behind EventLoop::new)
graphics device     +13.2 to +19.2 ms   (was 33.4 to 45.1)
start → first frame   110 to 119 ms     (was 145 to 152)
```

**What is left of this item is not ours**: `surface 0.03 ms, adapter 5.3, device 8.4` is what
bring-up now blocks for, and quorra's own headless numbers are 3.2–4.4 for adapter selection
against our 5.3–6.8 with a `compatible_surface` — the difference is the surface, and it is the
reason §8.1 asked for the field split.

### 5. The first frame pays ~12 ms of first-use allocation, and it is **not** the shaders

Measured in the two-hundred-and-eightieth session on the machine's real adapter, headless
(`crates/render-quorra/examples/first_frame.rs`): frame 1 costs 18.2 ms and frames 2 to 10 cost
3.7 to 5.1, and the difference is roughly fixed across scales — 13.3 ms at 1×, 14.3 at 2×, 18.1
at 4×.

**Sleeping between bring-up and the first render changes nothing** (16.05 / 15.26 / 16.65 ms after
0, 300 and 1000 ms), and the background thread reports its pipelines compiled in 5.3 to 5.7 ms —
so what the first frame pays for is device resource creation, not warmth. Two consequences:

- `CLAUDE.md`'s "nothing on the launch path waits for warmth" **costs nothing here**, and a
  `wait_until_warm` would buy zero milliseconds while hiding the twelve that matter.
- The ask is in `doc/QUORRA_FEEDBACK.md` §9: warm the *allocations* on the same background thread
  that already warms the shaders. Nothing about the API changes and ~12 ms comes off every cold
  launch of every host.

**And it re-scales the whole timeline.** ADR 0179's 145 ms is `lavapipe` under `Xvfb`, where the
first present is 54 to 68 ms because llvmpipe is drawing the page on the processor. On the real
adapter the same steps are bring-up 33 to 43, interpretation ~5 and a first frame of ~18, so a
launch on this machine's own GPU should be **75 to 90 ms**. Nobody has run it — `AI` has no X
authority cookie for the user's display, so the window half of that number is the user's to
measure (ADR 0126).

## What is deliberately not here

**Nothing, since item 5.** This section used to say the first present was not yet an item because
nobody had split it; it is split now, and the half that is ours is item 5's second bullet.
