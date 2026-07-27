# ADR 0010 — A command draws into the rows its clip admits, not into the page

Status: accepted, 2026-07-27.

## Context

`bug1721218_reduced.pdf` is an 825 kB file with an ordinary 612×792 page. Opening it took
49 seconds: 0.4 s to interpret and **48.7 s to rasterise**, holding about 1.7 GB while it
did. The corpus gate found it on its first run and it has been the largest single
performance defect in the tree since.

The page references **3576 distinct clips** across 7050 commands — a clip used by two
commands on average, so the mask cache that exists for repeated use was barely helping.
`MaskCache` built a `tiny_skia::Mask` the size of the whole page for each one and kept
every one of them, which explained the memory outright.

## The measurement contradicted the diagnosis

The previous handover recorded the cause as the page-sized masks, and the arithmetic was
persuasive: 3576 clips × a 485 kB mask is 1.7 GB, which is exactly what the process held.

`callgrind` says otherwise. Of 75.7 G instructions for one page:

| | share |
|---|---|
| `tiny_skia::pipeline::lowp::gradient` | **78.9%** |
| `pdf_model::function::Function::parse` | 4.9% |
| `pdf_model::function::Function::eval` | 2.9% |
| `tiny_skia::Mask::intersect_path` | 2.5% |
| `tiny_skia::Mask::fill_path` | 1.4% |

Mask building — the diagnosed cause — is under 4% of the run. Four fifths of it is the
raster pipeline's **gradient stage**: this page is a stack of thousands of shading fills,
each covering a large area and each clipped down to a sliver. The shading was being
evaluated per pixel across everything the *path* covered, and the clip mask then threw
almost all of it away.

The masks were still a real defect, but a memory one. Had the mask been made cheaper and
nothing else, the page would have kept nearly all of its 48 seconds.

Measuring the clip population made the shape of the fix obvious. Over those 3576 clips:

| | |
|---|---|
| mean clip width | 1.6% of the page |
| mean clip height | 1.2% of the page |
| mean clip area | 0.92% of the page |
| chains that are axis-aligned rectangles | 63 of 3576 |
| distinct clip *geometries* | 3523 of 3576 |

So neither of the two obvious special cases would have helped: the clips are not
rectangles, and they are not repeats of one another. What they are is *small*.

## Decision

**Every command draws into a horizontal band of the target — the rows its clip can admit
— rather than into the page.** An unclipped command's band is the whole page, so there is
one code path and no special case.

The band is derived from the intersection of the clip chain's device-space bounds,
rounded outward. Its mask is built in the same band, so a mask costs the band rather than
the page too. The page-to-device transform carries the band's row offset, so geometry,
paints and images all move together.

### Rows, and not a rectangle

The clips are as narrow as they are short, so a rectangle would be about 26% cheaper here
than a band. It is not available: a pixmap's rows are contiguous in memory and its columns
are not. `tiny_skia::PixmapMut::from_bytes` can borrow a run of rows as a pixmap in its
own right, and `tiny-skia` exposes no sub-rectangle view — `subpixmap` is private. A
rectangle would mean copying the destination into a scratch pixmap, drawing, and copying
back per command, which trades a bounded win for an unbounded amount of copying and a new
class of compositing bug.

A band is also *never worse* than the page, needs no fallback, and changes nothing about
what is drawn. That is worth more than the last 26%.

### The mask cache gets a budget

`MASK_BUDGET` caps the masks held at 32 MiB, dropping the oldest first and never the one
in hand. A document names as many distinct clips as it likes, so an unbounded cache is a
memory-exhaustion vector regardless of how small each mask now is — principle 3 asks for
an explicit bound, not merely a smaller unbounded one. Eviction in build order rather than
by recency is deliberate: clips are used in runs, so the two orders coincide closely
enough that an active clip is rebuilt at most once per run, and build order costs one
`VecDeque` instead of a recency index.

## Consequences

Measured on the file that motivated this, at 612×792:

| | before | after |
|---|---|---|
| rasterise | 48.7 s | 0.24 s |
| peak resident memory | ~1.7 GB | 53 MB |
| masks held | 1.73 GB (3576 page-sized) | 25.5 MB (3576 banded) |

Nothing here depends on the clips being small. A page whose clips cover it entirely gets
bands the size of the page and pays exactly what it paid before.

### It is not byte-identical, and the reason is worth stating

Folding the band's row offset into the transform is exact arithmetic in principle and
different rounding in practice: `a·x + c·y + (f − top)` is not bit-for-bit
`(a·x + c·y + f) − top`. So an antialiased edge can land on the other side of a
supersample boundary and one pixel changes.

Over the 974-document pdf.js corpus plus the 14 specification PDFs, **36 of 988 pages
change, and every change is that**:

| | |
|---|---|
| pages that change at all | 36 of 988 |
| largest share of a page's channels affected | 0.18%, every one of them by ±1 |
| largest single-channel change | 33, on 111 pixels of one page |
| differing pixels lying on a high-contrast boundary | 100% (median local contrast 88, against 0 for the page at large) |

That last row is the measurement that identifies these as antialiased edges rather than a
displacement: shifted output would move interiors as well as edges, and shifting the whole
page by one row in any direction makes the match *worse*, not better.

The alternative — leaving the drawing surface at the page and bounding only the mask —
would have preserved the last bit at the cost of nearly all of the speed, since it is the
paint evaluation and not the mask that dominates. Trading a hundred antialiased pixels for
a 200× render is worth it, and saying so out loud is the point of writing it down.

### Where this was found, and what it found first

The first corpus comparison of this change reported 19 pages differing, two of them
drastically — 18% of one page's channels, by up to 133 levels. That was not the band. It
was the band *perturbing* two pre-existing defects in how paints were positioned, which
had been invisible because the arithmetic happened to cancel at a scale of 1.0: a shading
mirrored about the page's centre line, and an image whose pattern was transformed twice
and drew a photograph as a single flat colour. Both are fixed in the two commits preceding
this one, and each has tests derived from the specification rather than from a renderer.

An optimisation that changes the output is a defect *or a discovery*, and the way to tell
them apart is to explain every changed pixel rather than to accept a plausible total.

### What this does not fix

The gradient stage is still 80% of what remains, because a 256-sample `Ramp` becomes a
256-stop gradient and `tiny-skia`'s gradient stage scans its stops per pixel batch. The
page is fast enough now that this is no longer the most valuable thing to look at, but it
is the next thing to look at if shading-heavy pages need to be faster still — and the
answer would be to give the rasteriser fewer stops, not to give the display list a coarser
ramp.
