# ADR 0128 — 5933 fills of 107 outlines: what our own GPU backend would be for

Status: accepted, 2026-08-01. The decision is *not yet, and by measurement* — the plan at the end
is the decision, not the preamble to one.

## The question

Asked by the project owner immediately after ADR 0127, and worth more than that defect was:

> I am thinking about replacing vello. If I understood correctly, we currently need to rebuild a
> frame when zooming. Would creating our own GPU rendering backend give us the possibility to
> implement this in a way, where the GPU renders the whole page? Would writing our own backend
> open some good ways to optimize something, we couldn't easily do otherwise?

Three questions: what a zoom costs, whether a page can live on the device, and what a backend of
our own would be *for*. They are answered below in that order, and the third is the only one whose
answer is a reason to write one.

## What a zoom step costs today

Measured on the 890M, ISO 32000-2 at 1132×1601, through the tier-1 path:

| | page 5 | page 6 |
|---|---|---|
| interpret the content stream | 13.4 ms | 3.4 ms |
| encode the display list into a Vello scene | 1.2 ms | 1.2 ms |
| the whole GPU pass, readback included | 30.7 ms | 29.2 ms |

**Interpretation is already cached** — `Open::interpreted` keeps the display list and a zoom
reuses it, so the first row is paid once per page and not per zoom. The encode is 1.2 ms. So the
premise in the question is half right: a frame *is* rebuilt, but almost none of the rebuilding is
ours.

What the third row contains is the real answer: **Vello keeps nothing between frames.** Every
frame re-uploads the scene and re-derives flattening, binning, tiling and coarse rasterisation
from scratch. The readback in that row is tier 1's alone — the window does not read pixels back —
but the pipeline before it is the same one the window runs.

## The number that decides the argument

Page 6 of ISO 32000-2, counted from its display list:

| | |
|---|---|
| fill commands | **5933** |
| path commands across them | 57 077 |
| **distinct outlines** | **107** |
| distinct outline *and* scale | 115 |

`Command::Fill` holds an `Arc<Path>`, and every occurrence of a letter shares one outline — that
sharing is why a dense page's display list is small, and it has been there since the display list
was designed. **Vello re-flattens and re-tiles all 5933 every frame**, because a `Scene` is a flat
encoding with no notion that two draws are the same shape.

A glyph atlas rasterises 115 small coverage bitmaps and draws 5933 textured quads. That is the
single largest optimisation available to this program, and it is not reachable from outside Vello:
its own README lists glyph caching among the work outstanding. It would also **dissolve ADR 0127's
cliff** rather than band around it, since tile records would then scale with distinct glyphs
instead of with occurrences.

## "Render the whole page once" is a trade, not a capability

Rendering above the window's resolution and sampling down is available to any backend, including
this one today. It costs 116 MB for A4 at 4× of this size, it still re-renders once the zoom
passes what was rasterised, and below that it is visibly softer than rasterising at the target
resolution. It is not a reason to write a backend.

Scaling the last frame while the real one renders behind it is the cheap fix for *perceived*
zoom latency, and the owner has judged it **ugly but acceptable for now**. Recorded as exactly
that: a stopgap whose replacement is the resident scene below, not a design we mean to keep.

## Can an IR go to the device and stay there?

Yes, with one boundary that is not negotiable and one honest limit.

**This is already the shape of Vello**, which inherits it from piet-gpu: a scene is encoded into
buffers and compute stages do flattening, binning, tiling and painting. Our display list is
already the IR the question asks about. What is missing is not the idea, it is **residency** —
upload once per page, not once per frame.

**The parser stays on the CPU, and that is principle 3 rather than performance.** A content stream
is variable-length tokenisation over filters, font programs and image codecs: sequential,
data-dependent, and *untrusted*. `#![forbid(unsafe_code)]` and the confined decoders are this
project's answer to that input, and neither reaches a shader. The IR is the boundary precisely
because it is validated and fixed in shape by the time anything writes it to a buffer.

What would live on the device: segment arrays in **page space**, per-draw paint, clip and mask
indices, a glyph atlas, an image atlas, baked shading tables. The view transform is a uniform.

What still re-runs per frame: flattening — its tolerance depends on scale, so a zoom does change
it — then binning, tiling and fine. All of it parallel, all of it over resident data, with no
upload and no CPU encode. **That is the honest limit**: "the GPU renders the whole page" means a
zoom costs one pass over data already there, single-digit milliseconds, not nothing.

**And not one program.** The pipeline needs prefix sums and barriers between stages, so it is
several dispatches — recorded once and resubmitted, which is itself a saving Vello does not take.

**Dynamic memory is the thing to design first.** ADR 0127 is what bolting it on afterwards looks
like. Two candidates, and the choice belongs in the spike: size the buffers on the CPU from
statistics the IR lets us compute exactly, or count on the device and dispatch indirectly — with
banding as the fallback rather than as the mechanism.

## And the whole document?

The question came back a minute later with the scope widened, and widening it is what makes it
answerable: 1023 pages of ISO 32000-2, interpreted over a 40-page sample and extrapolated.

| | |
|---|---|
| interpreting every page | **4.0 s** (3.9 ms each) |
| draw records, all pages | 2.2 million — about 70 MB at 32 bytes each |
| path commands *if outlines were expanded per occurrence* | 59.6 million, about 1.4 GB |
| decoded images | 60 MB, and unbounded in general |

Three things fall out of that table.

**The memory is not the obstacle.** Seventy megabytes of draw records plus deduplicated outlines is
an ordinary residency for a device with shared memory. The 1.4 GB row is the same document with
every glyph occurrence carrying its own copy of the outline — which is what a flat scene encoding
is, and is the atlas argument again in the document's units rather than the page's.

**The interpretation is.** Four seconds, and `CLAUDE.md` is explicit: "A 500-page document must
open no slower than a 5-page one", with nothing not needed for page one on the launch path. A GPU-resident *document* would mean parsing every content stream, decoding every image and
loading every font before anything appeared. That is not a graphics decision that a backend could
make differently; it is the eager-work rule, and it is decided.

**So the resident unit is a window, not a document.** Pages around the current one, interpreted and
uploaded on a background thread, evicted behind. That is what makes a page turn instant — it is the
same prefetch a viewer wants anyway — and it is bounded by a number we choose rather than by what
the file happens to contain. The whole document arrives eventually, for a person who reads it all,
and never on the launch path.

The one part that genuinely *is* whole-document and belongs on the device from the start is the
**glyph atlas**: a document sets its body text in a handful of faces, so the atlas fills during the
first page or two and is reused by all 1023. That is residency worth having, and it is the same
mechanism as everything else in this ADR.

## What our own backend would open, in order

1. **The glyph atlas.** 5933 → 115, above.
2. **Damage rendering.** A selection, a caret, a form field being typed into currently redraw the
   page. With tiles of our own they redraw the tiles that changed. Vello has no concept of a
   partially-changed scene.
3. **Persistent geometry with the transform as a uniform.** What makes zoom and scroll cheap for
   real, and what the question was reaching for.
4. **Progressive rendering** — a coarse pass immediately, refined after — which hides latency
   during a gesture instead of shortening it.
5. **Clause 11 conformance.** §11.6.6's overprint, non-isolated and knockout groups, a
   `/DeviceCMYK` blending space, shading types 4 to 7 on the device. Those are ledger rows open
   today *because* PDF's compositing model has to be squeezed through peniko's. Owning the
   compositor is the only way they close, and by principle 5 that ranks with speed rather than
   under it.
6. **Dynamic memory by construction**, per the section above.

## What it costs

Vello is the piet-gpu lineage — years of research — and still describes itself as alpha. A correct
2D rasteriser is conflation artifacts, antialiasing quality, clipping, groups, blend modes,
flattening tolerance and driver portability, and each of those is a place a page can go subtly
wrong rather than loudly wrong. This tree's GPU backend is an *accelerator* whose oracle is the CPU
backend; writing it ourselves puts correctness risk into the accelerated path, where only that
oracle can catch it. None of that is an argument against. It is the size of the thing, written
down before anyone starts.

## The decision

**Not now, not never, and by measurement** — in this order, because each step prices the next:

1. **Stale-frame zoom**, host-side. Days, no architectural commitment, and recorded as a stopgap.
2. **A glyph coverage cache in `render-cpu`.** The same insight, in the backend that is both the
   correctness oracle and the startup path. It prices the atlas before anyone writes a shader, and
   it speeds up the path page one already takes.
3. **A moving window of interpreted pages**, on a background thread, from the section above. It is
   worth having whichever backend wins — it is what makes a page turn instant — and it is the
   bounded form of the whole-document question.
4. **A spike**: our own backend over the display list — fills, clips, a glyph atlas — measured
   against Vello for speed and against the CPU backend for correctness, on the corpus. Third arm
   of the comparison: **`vello_hybrid`**, linebender's sparse-strips architecture, where strips are
   built on the CPU and rasterised by a fragment shader with no compute-buffer cliff to fall off.
   It is 0.0.x alpha, and it is the architecture they are moving to.

What would **not** justify the rewrite is ADR 0127's buffer cliff, which is handled, banded and
measured. The case for a backend of our own is the atlas, damage rendering and clause 11 — and it
is strong enough to test properly rather than to argue about.

## The lesson

**A dependency's defect and a dependency's limits are different arguments, and only one of them is
worth acting on.** The question "should we replace Vello" arrived out of a blank page, and the
blank page turned out to be the weakest reason to do it: a bug with a fix. The real case was
sitting in a number nobody had counted — 5933 draws of 107 shapes — and it would have been just as
true if page 6 had never gone black.
