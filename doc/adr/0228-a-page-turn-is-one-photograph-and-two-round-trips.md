# ADR 0228 — A page turn is one photograph and two round trips

Status: accepted, 2026-08-08 (session 391).

Supersedes nothing. Answers two of the three findings ADR 0227's new instrument produced and
written up in `doc/todo/45-where-a-frame-goes.md`; the third — quorra's `encode` — is a report
rather than a change here and is `doc/QUORRA_FEEDBACK.md` §13.

The witness throughout is the project owner's own `tmp/windows/NorthAmerican.30MB.pdf`, 65 pages,
30 MB, driven through 38 page turns by `xdotool` under `Xvfb` at 800×1000 on `llvmpipe`. **That is
not the owner's Intel UHD through DX12**, so the ratios are about shape and the absolute figures
are this machine's.

## 1. The `scene` stage was bimodal, and it was neither of the two suspects

`doc/todo/45` named two candidates in `render-quorra/src/scene.rs`: `Image::area_averaged`, and the
row reversal that copies a sampled shading's grid one pixel at a time (§8.9.5 puts the first row at
unit y = 1). **The second never runs on this document at all** — the probe counted zero sampled
shadings over the whole session — so it was ruled out by measurement rather than by argument.

The first is the whole of it, and the probe leaves no room to doubt which:

```text
PROBE scene 19.40 | area 18.45 (1 hit, 0 miss) | upload 0.00 (1) | samp 0.00 (0) | seg 0.54 (117) | mesh 0.00 (0)
PROBE scene 12.93 | area 12.14 (1 hit, 0 miss) | upload 0.00 (1) | samp 0.00 (0) | seg 0.40 (119) | mesh 0.00 (0)
PROBE scene 1.09 | area 0.00 (0 hit, 0 miss) | upload 0.00 (0) | samp 0.00 (0) | seg 0.47 (148) | mesh 0.00 (0)
```

(`area` is `Image::area_averaged`, `samp` the shading row reversal, `seg` the outline uploads that
miss the cache, `mesh` a shading rasterised on the host — a throwaway probe, removed before the
commit, whose whole content was seven `Instant` pairs and six counters.)

Every expensive frame is one image being averaged down and nothing else; every cheap frame has no
image on it. The pages, with the reduction each asks for:

| page image | reduced to | cost |
|---|---|---|
| 2700×3450 (page one) | 900×1150 | 38.2 ms |
| 1374×2362 | 458×788 | 14.8 ms |
| 2100×1448 | 700×483 | 13.2 ms |
| 1374×1374 | 458×458 | 8.5–11.3 ms |

**So the cost is per *source sample* and the display list's command count cannot see it**, which
is exactly why a 388-command page cost sixteen times a 3675-command one. That is the finding, and
it generalises past this function: a cost paid per resource is invisible in every instrument that
counts commands.

## 2. What was wrong inside it, and what was merely slow

Two things, and separating them matters because only one is a defect.

**A defect: `Image::area_averaged` could panic.** It called `Self::reduction` *before*
`is_consistent()`, and `reduction` clamps a ratio into `1.0 ..= width` with `f32::clamp`, which
panics outright when its minimum exceeds its maximum — which a zero-width image makes it do.
`render-cpu` and `render-gpu` both ask `is_consistent()` before calling, and `render-quorra` does
not, so the only thing standing between a public method and a panic was the habits of two of its
three callers. The check is first now and `an_image_with_no_samples_is_not_reduced_and_does_not_panic`
is the guard.

**Merely slow: two things, and both are measured rather than assumed.**

`Bands::at` is two 64-bit divisions, and it was being asked once per *output cell* for the column
bands — which are the same for every row of the image. Computing them once costs one `Vec` the
width of the reduced image.

And the rest is a loop over every source sample that one core walks. **Each output cell is a pure
function of its own block of the source**, the blocks are disjoint and tile the grid exactly, so
which thread computes which row cannot change a byte — the same property that made `pdf-model`'s
colour conversion divisible (ADR 0147), and precisely the property a rasterisation does **not**
have, which is the whole of ADR 0138. Dividing the output rows across rayon is therefore exact by
construction and not by luck.

`crates/pdf-render/examples/area_bench.rs` is the instrument, best of 100 runs a size, and it
asserts byte-identity on every case it times rather than only reporting a clock:

| 3× reduction of | before | hoisted only | and divided |
|---|---|---|---|
| 1374×1374 | 4.57 ms | 3.50 ms | **0.65 ms** |
| 2100×1448 | 7.34 ms | 5.99 ms | **0.86 ms** |
| 2700×3450 | 22.39 ms | 19.81 ms | **2.93 ms** |

**The threshold is measured and is deliberately above the crossover.** `PARALLEL_FLOOR` is 65 536
source samples:

| source samples | serial | divided regardless |
|---|---|---|
| 4 096 | **0.008 ms** (mean 0.010) | 0.012 (0.031) |
| 16 384 | 0.052 (0.054) | **0.026** (0.043) |
| 65 536 | 0.207 (0.216) | **0.035** (0.065) |
| 262 144 | 0.459 (0.522) | **0.088** (0.149) |

The division starts winning at the second row and the floor sits at the third, because a floor
belongs where the division is worth *having* rather than where it first becomes faster: below it
the whole reduction costs a fifth of a millisecond, and `render-cpu` calls this from inside its own
rayon strips, where the pool is already saturated and this benchmark cannot see it.

**What it costs in readability**, which principle 2's rule requires stating: one `Vec` of column
bands, one branch on a threshold, and a `rayon` dependency in `pdf-render` — a crate that had two.
The dependency is already in `pdf-model`, `render-cpu` and `viewer-confined`, so it adds nothing to
the tree and no thread to a process that did not already have the pool.

**What it does not cost is a pixel.** The oracle is unmoved in all seven buckets (859 agreeing, 67
contradicted on complete pages, 786 ambiguous), the two backends' own gate is 920 agree / 36 differ
/ 1 refused, the corpus is 974 documents with 70 incomplete, and text extraction is 99.2%.

## 3. The accessibility publication was 2 ms and §14.7's tree was not what cost it

`doc/todo/45` predicted this cost would be either `Query::AccessibilityTree` walking §14.7 or
`App::place_window` asking winit where the window is, and said one more `Instant` could tell them
apart. It could, and it was the second — by an order of magnitude:

| | per page turn |
|---|---|
| `Query::Reports` | 0.001 ms |
| `Query::AccessibilityTree` | 0.13–0.25 ms |
| `Bridge::publish` | 0.008–0.022 ms |
| **`App::place_window`** | **1.8–3.2 ms** |

`place_window` is two synchronous X11 round trips — winit's `outer_position` and `inner_position`
— and **a window's position on the screen does not change when a page turns.** So the fix is not
the one the todo proposed. It is asked where the answer can have changed: when the bridge comes up,
on `WindowEvent::Moved`, and on `WindowEvent::Resized`, which moves the inner rectangle inside the
outer one.

**And the platform question is answered in the crate that knows, but not by the function that
already existed.** `Bridge::shortfall()` says whether this build has an adapter at all;
`Bridge::wants_window_bounds()` says whether the adapter it has needs to be told where the window
is. Those are the same answer today and **will stop being the same answer**: AT-SPI reports node
bounds in screen coordinates, which is why `accesskit_unix` needs this, while AccessKit's Windows
and macOS adapters take a window *handle* and let the platform do that arithmetic. When
`doc/todo/31` wires those in, `shortfall` becomes `None` there while `wants_window_bounds` stays
`false` — and a host that had asked the first question would start paying for a position nobody
wants. `a_build_with_no_bridge_wants_no_window_bounds` guards the one direction that is not going
to change.

Measured over the same 38 page turns, three runs each: **2.32 / 2.35 / 2.36 ms mean and 92.8 / 94.1
/ 94.3 ms over the session, to 0.26 / 0.31 / 0.27 ms mean and 10.3 / 12.4 / 10.7 ms.** What is left
is §14.7's tree, which is the part that is about the page.

## 4. The `elsewhere` row is a bound and not a duration, and this is a retraction

ADR 0227 named `elsewhere` — `Device::render` minus the three phases quorra reports — rather than
leaving a reader to subtract, on the argument that an unnamed remainder is where a cost hides. That
argument still holds. What was wrong is quieter: `execute` comes from **the adapter's own timestamp
queries** where they exist, and the summary says so in its own last line, while `device` is a host
`Instant` around the call. Subtracting a device clock from a host one does not leave a duration; it
leaves the acquire, the present and the readback *plus whatever the two clocks disagree by*.

So this is not a question that can be settled from here, and it is not an artefact of the *summing*
either — it is an artefact of summing quantities on two clocks, which is a fact about what the
boundary reports. The row stays, because the alternative is an unnamed remainder; the summary now
prints one line saying it is a bound; and the two ways out are quorra's, in
`doc/QUORRA_FEEDBACK.md` §13: report the acquire and the present as phases of their own, or say
which clock each phase is on.

## 5. The citation checker had a hole of exactly the shape it checks for

Found by writing this round's own comments: `QUORRA_FEEDBACK.md section 13` is the spelling this
tree uses, and the gate refused the first draft that wrote `§13`. It refused it in *one* of the two
places the draft wrote it, which is what made the hole visible.

`tools/conformance`'s `another_document` decides that a `§` belongs to some other document when the
word before it is an upper-case stem with a `.md` suffix. `doc/QUORRA_FEEDBACK.md` is not an
upper-case stem, because `doc/` is not upper case — so **every citation written with a path in
front of it passed the arm for the whole of its life**. There were eight in the tree and six of
them named `QUORRA_FEEDBACK.md`, which is the document the arm's own comment cites as the case it
exists to catch. Each was being checked against ISO 32000-2's clauses and passing by landing on
one, which is precisely the failure mode the arm's message describes.

One `rsplit('/')` closes it, with `a_path_in_front_of_a_project_documents_name_does_not_excuse_it`
as the guard and eight citations rewritten to "section N". The gate's citation count is 5095 →
**5133**, which is 46 this round added minus the eight it stopped miscounting.

**The lesson is the sweep's own**, one directory over: a test whose predicate is about *how a
string is spelled* is a test of how the author spelled it. This one asked for upper case and got
told about the path.

## 6. The before and after

Three runs of each, the same script, the same document, the same 39 frames — sums in milliseconds,
the rest medians and percentiles:

| | before | after |
|---|---|---|
| frame, sum | 1203.4 / 1242.3 / 1225.7 | **1071.6 / 1097.7 / 1074.0** |
| scene, sum | 208.4 / 219.2 / 210.1 | **71.4 / 71.4 / 68.3** |
| scene, p90 | 14.8 / 12.9 / 13.2 | **3.3 / 3.2 / 3.2** |
| scene, max | 23.9 / 40.7 / 29.1 | **6.3 / 7.0 / 6.4** |
| device, sum | 994.3 / 1022.4 / 1014.9 | 999.5 / 1025.7 / 1005.0 |
| attend, sum | 92.8 / 94.1 / 94.3 | **10.3 / 12.4 / 10.7** |

**The spread within each condition is smaller than the difference between them**, which is the only
thing that makes a wall-clock claim worth printing: `frame` varies by 39 ms inside the before and
26 ms inside the after, against 152 ms between them. `device` is unmoved, which is what says the
change reached what it was aimed at and nothing else — and it is also why §13 of the feedback
document could be written honestly.

## The lesson

**A cost paid per resource is invisible to every instrument that counts commands**, and this tree
had two such instruments before it had one that could see this: the corpus counts documents, the
oracle counts pages, and ADR 0227's own frame line prints the display list's command count first.
A 388-command page costing sixteen times a 3675-command one is the shape that says the denominator
is wrong, and it took a per-stage timer plus a per-*function* probe to find which resource it was.

The second lesson is smaller and is about predictions: `doc/todo/45` named two candidates for the
`scene` cost and one of them was right, and named two candidates for the accessibility cost and
**neither** was the one it proposed fixing. Writing the alternatives down was still what made both
answerable in one sitting — but the round that measures is the round that gets to say which.
