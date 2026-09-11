# 0961 — The bomb bound for meshes was counted in our own triangles, and said nothing

Status: accepted. Session 959 — recorded as 945 when written, which is ADR 0963.
Context: `crates/pdf-model/src/mesh.rs`, ISO 32000-2 §8.7.4.5.5 to §8.7.4.5.8 and §10.7.3,
`doc/todo/01`'s `partial` rows, `doc/traps/instruments-and-reports.md` trap 5 and trap 11.

## How it was found, and the false start is the point

`doc/todo/01`'s twenty-fourth sweep put §8.7.4.5.8 at the head of its third rank — a `partial`
row whose whole debt rests on a sentence with no modal verb in it. Session 939 had already read
that row and kept it, naming the debt exactly: the tessellation's fineness is `mesh::PATCH_STEPS`,
the constant 10, rather than anything derived from §10.7.3's smoothness tolerance (ADR 0919).
The row is checkable, so it was checked: every corpus page that paints with a mesh, rendered at
`PATCH_STEPS` 10 and at 60.

The first run said the difference was enormous — `personwithdog.pdf` at a mean **6.81 of 255**,
5.39% of pixels differing, structural similarity 0.925 — and the picture at 60 was a person and a
dog with pieces missing. Which is trap 1 doing its job and `doc/habits.md`'s *attribute by removing
the suspect* catching an A/B that moved two things: the finer tessellation had run the same
document into `mesh::MAX_TRIANGLES`, and the **bound** was what cut the drawing, not the fineness.
Lifting the bound and repeating the A/B puts that page at a mean **0.0029**.

The false start is the finding. The bound is real, it is necessary, and **it was stopping a
document's mesh part-way in silence**.

## What the bound is, and what was wrong with it

> A mesh stream is compressed, so a few kilobytes can describe an unbounded number of patches.

That comment has been right since it was written. §10.7.3 licenses the bound in as many words —
"Each output device may have internal limits on the maximum and minimum tolerances attainable" —
and it is the same sentence `shading::MAX_FUNCTION_CELLS` already rests on. Three things were
wrong around it.

1. **It said nothing.** `while triangles.len() < MAX_TRIANGLES` stopped, the remaining patches
   were dropped, the page was drawn with what had been read, and `Interpretation::is_complete`
   returned true. Every other bound reached inside `pdf_model::content` — `max_clips`, the image
   bounds, the group bounds, the four in `pattern.rs`, the four in `run.rs` — is already raised as
   `Unsupported::LimitReached`, and `crates/pdf-model/tests/corpus.rs` already has a row for that
   variant ("a bound this program set"). This one had no route to a report at all: `mesh::read`
   returned `(Vec<Triangle>, Option<Ramp>)` and there was nowhere for the fact to sit.
2. **It is counted in *this program's* triangles rather than in the document's patches.** A type 6
   or 7 patch becomes `PATCH_STEPS`² cells and two triangles a cell, so what a document is allowed
   is `MAX_TRIANGLES / (2 · PATCH_STEPS²)` — a patch budget set by a constant that is nothing to
   do with the document, and one that *falls* as the tessellation gets finer. Nothing anywhere in
   the tree said so, and the two constants sit eleven lines apart.
3. **Neither constant had a measurement under it.** `PATCH_STEPS`' comment claimed ten steps "puts
   the error of a patch spanning a whole page well under a pixel", which is a claim with no scale
   in it: the surface is evaluated in the shading's own space and the triangles are transformed
   afterwards, so a device sees the chord error multiplied by the magnification.

## What was measured

`crates/pdf-model/examples/mesh_triangle_census` interprets every page of
`doc/pdf.js/test/pdfs`, the four `doc/corpora/` submodules and `corpus-cache/openpreserve` — 1516
files — and counts the triangles of every mesh the *display list* carries. Off the display list
rather than off the objects, and that is not a detail: §8.6.5.1 resolves a `/ColorSpace` stated as
a name through "the resource dictionary in force", the space decides how many components a vertex
carries and therefore how the bit stream divides, and the dictionary in force belongs to a page or
to a form XObject inside it. A first draft scanned objects and could not build nine of the
corpus's meshes for exactly that reason — the largest documents among them.

```
40 mesh paints; the bound is 262144 triangles and 0 page(s) report it
    61000  23.27%  bug1703683_page2_reduced.pdf page 1
    38800  14.80%  issue13520.pdf page 1          (six of them)
    33200  12.66%  personwithdog.pdf page 1
    ...
    11800   4.50%  178360.pdf page 13
```

So **the largest mesh any corpus here paints with is 305 patches, 23.3% of the bound**, and no
page reports it today. The crawl is deliberately not in that denominator: interpreting every page
of 89 286 documents is a corpus walk rather than a census, and two other rounds are on this
machine. The walk was attempted under `tools/bounded.sh` and stopped at the data limit; that is a
fact about the attempt, not about the crawl.

And the fineness, measured on its own with the bound lifted so that only one thing moves — 10
against 60, every mesh page, at the page's own scale and at four times:

| | mean | worst tile 1× | worst tile 4× |
|---|---|---|---|
| `coons-allflags-withfunction.pdf` | 0.0511 | 0.92 | 1.48 |
| `tensor-allflags-withfunction.pdf` | 0.0485 | 1.45 | 9.18 |
| `issue18816.pdf` | 0.0212 | 2.02 | 10.18 |
| `personwithdog.pdf` | 0.0029 | 0.30 | 0.30 |
| `bug1703683_page2_reduced.pdf` | 0.0000 | 0.00 | 0.01 |

The mean does not move with the magnification and the worst tile does, because the departure is a
seam at each patch's boundary whose *width* does not shrink as the pixels arrive. That is what the
fixed fineness is worth: nothing for a page, visible under magnification — which is exactly the
quantity §10.7.3's tolerance is about, and why §8.7.4.5.7's and §8.7.4.5.8's rows stay `partial`
on it.

**The two constants are one decision.** 305 patches crosses `MAX_TRIANGLES` at a fineness of
**21**, so a session that derives `PATCH_STEPS` from §10.7.3 has to move the bound with it or
start dropping patches out of a real document.

## Decision

- `mesh::read` returns a `Mesh` carrying `truncated`, set where the bound stopped the reading
  **with a vertex, a row or a patch still in hand**. The test is taken *after* the next item has
  been read rather than before, so the flag says something was dropped rather than that a count
  was reached — trap 11, and the difference is a stream whose last patch lands exactly on the
  bound.
- `shading::Cache::build` and `shading::build` return a `Shaded { shading, truncated }`. The two
  callers are both inside the interpreter, which is the only place that can raise a report, and a
  bare `Shading` is what let this bound stay quiet.
- The interpreter raises `Unsupported::LimitReached { limit: "max_mesh_triangles" }` at both call
  sites. `note` is a set, so a pattern rebuilt at every mark says it once.
- `MAX_TRIANGLES` is public, so the census quotes the shipping constant instead of a copy of it,
  and its doc comment states the units and the relation to `PATCH_STEPS`.
- `PATCH_STEPS`' comment loses the unmeasured claim and carries the figures above.

## Calibration

`a_mesh_the_triangle_bound_cuts_short_is_reported` builds a strip of Coons patches out of Table
84's own bytes, one patch under the budget and two over it, and asserts both directions — a report
that fires on everything says nothing. Calibrated per trap 13 by planting `truncated = false`: the
test fails with `[]` for the report and passes when the plant is restored.

## What this does not change

No pixel moves: `mesh.rs` produces the same triangles in the same order, and the new report fires
on no page of any corpus this tree holds. The text-extraction gate is unchanged and
no verdict of this round's making moved, which is what makes this a report rather than a fix — the defect it closes is that
a page which *did* hit the bound would have looked like a page the producer meant to leave sparse.
