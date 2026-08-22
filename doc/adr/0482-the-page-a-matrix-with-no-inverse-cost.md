# ADR 0482 — The page a matrix with no inverse cost

Status: accepted, 2026-08-22. Session 653. Makes a mark stated under a noninvertible matrix cost
**itself** rather than the page it stands on, on all three backends, and names what it refused.
Closes the refusal half of `doc/todo/11` item 8; amends §8.3.4's and §10.7.4's ledger rows; adds
five crawled documents to `doc/checks/fixed-documents.toml`, four of which are ordinary
well-formed pages that this program drew as nothing at all.

## What §8.3.4 says a singular transform produces

The question item 8 asks first is what the *standard* says the mark is, and the answer is in one
NOTE that no code in this tree had ever cited — §8.3.4's third:

> When rendering graphics objects, it is sometimes necessary for a PDF reader to perform the
> inverse of a transformation -that is, to find the user space coordinates that correspond to a
> given pair of device space coordinates. Not all transformations are invertible, however. For
> example, if a matrix contains a, b, c, and d elements that are all zero, all user coordinates map
> to the same device coordinates and there is no unique inverse transformation. Such noninvertible
> transformations are not very useful and generally arise from unintended operations, such as
> scaling by 0. Use of a noninvertible matrix when painting graphics objects can result in
> unpredictable behaviour.

Three things follow, and the third is the decision.

**The mark has no area.** "[A]ll user coordinates map to the same device coordinates" is a
statement about the *geometry* and not only about the inverse: a singular matrix carries the plane
onto a line or onto a point. A page transform is invertible — `TargetSpec::for_page` is a positive
scale, a y flip and a translation — so a singular *command* transform makes the device transform
singular too, and the command's whole path lands on a line or a point at every scale on every
device. Whatever a renderer does there, it is not painting an area.

**The standard states nothing further about it.** "[C]an result in unpredictable behaviour" is the
clause declining to define the case, and it is a NOTE, so it is informative besides. There is no
`shall` to satisfy and no recovery prescribed.

**And it says nothing whatever about the page.** This is the sentence the round turns on. No clause
makes a neighbouring command's matrix a reason to stop drawing; §6.3.2.2 asks a rendering processor
for "the page contents as defined by the PDF specification" without an exception for one of them.
So the refusal §8.3.4 licenses is the *mark's*, and a renderer that makes it the raster's has
answered a question the standard did not ask — and thrown away every command that did draw, which
is the substitutive-versus-additive failure `doc/traps/instruments-and-reports.md` states and ADR
0106 is the precedent for.

### What §10.7.4 would still ask for, and why it is not built here

§10.7.4's "no shape ever disappears" is not satisfied by drawing nothing, and this ADR does not
claim it is. The clause's own construction for a shape with no area is a run of *whole* device
pixels — "a filling region is considered to intersect every pixel through which its boundary
passes, even if the interior of the filling region is empty" — and `pdf_render::split_collapsed_fill`
builds exactly that for a subpath flat in its **own** space (ADR 0154, ADR 0208). A path collapsed
by its **transform** instead reaches no such construction, and nothing in this tree builds one.

That is left open rather than taken, on three grounds and each is checkable. §8.3.4 NOTE 3 is the
standard declining to state the case at all, which is a weaker warrant than the one
`split_collapsed_fill` has. The *placement* would have to be answered as well as the extent —
§10.7.4's own flooring identifies a pixel from a point, and the collapsed image of a rotated or
sheared path is a staircase, which is the fallback `collapsed.rs` already declines to snap. And no
document on this disk offers a witness for what such a mark should look like: the four well-formed
ones state a single collapsed command among a page's two or three thousand, and the one that states
many is a garbage stream no reference agrees with either. `doc/todo/11` item 8 carries the residual
with the measurement beside it.

## Every site that refused, and what each does now

Item 8 named `render-cpu` and predicted `render-gpu`'s matching refusal. **Both were there and
there was a third**, which is trap 2's shape exactly — three libraries found three ways to refuse
one condition, and no gate could see any of them:

| backend | what it did | why |
|---|---|---|
| `render-cpu` | `CpuRasterError::UnsupportedPaint("singular transform …")` out of `rasterize` | `page_to_path` inverted the command's transform to place a paint, **before looking at the paint** |
| `render-gpu` | `GpuRasterError::UnsupportedPaint` out of `build` | `Spaces::new`, the same quantity for the same reason |
| `render-quorra` | `QuorraRasterError::Scene(InvalidStroke { width: 0.0 })` | it needs no inverse — and resolves a stroke's device width as `path_width × max_stretch`, which a collapsing transform makes zero |

The third is the one worth keeping. `render-quorra` positions a paint in **page** space
(`Encoder::placed`), so it never needed the inverse at all; that it refused anyway, one step along
and with a different error type, is what says the inverse was never the point. **The inverse is
`tiny-skia`'s and Vello's requirement rather than the clause's** — both apply a draw's transform to
its paint as well as to its shape, so each must undo it — and a decision two libraries impose on
two backends is precisely a decision that must not be taken in either of them.

Note also what `render-cpu` and `render-gpu` were refusing *for*: both computed the inverse for
every fill and stroke regardless of the paint, and a `Paint::Solid` reads it never. Every witness
the census found is a solid-coloured mark. **The whole measured population of this defect was pages
refused for a quantity nobody was going to read.**

So the condition is now stated once, in `pdf_render::paint_space`, with §8.3.4's NOTE under it. It
returns page→path where there is one — which the two library backends use — and `None` otherwise,
which all three read as *this mark is refused and the page is drawn*. `Command::Image` takes the
same guard for the same reason: it is a marking command, its unit square collapses with everything
else, and leaving it out would have made the rule true of two command kinds out of three.

## Where the report goes, which is the other half of item 8's question

Item 8 asked "which of `Rasterizer`'s errors are about a command and which about the target, and
whether the ones about a command belong on `Interpretation::unsupported`". The answer is that this
one is **neither**: it is a property of the *file*, decidable from the display list before any
target exists, so it never needed to be a rasteriser's error in the first place.

`pdf_render::DisplayList::noninvertible_marks` counts such commands, groups included; `interpret`
raises `Unsupported::NoninvertibleMatrix { commands }` once per page, which `viewer-core::describe`
words for a reader. That is the twelfth place this program reports while drawing, and it meets trap
5's test in both directions: the marks are absent whatever any backend does, so saying nothing would
leave a silence, and refusing the page throws away everything else, so the error was worse than the
silence.

**Counted from the finished list rather than at the six places a mark is pushed.** One walk cannot
miss a route — `path.rs`, `text.rs`, `image.rs`, `pattern.rs` and `annotations.rs` all push marks —
and the cost is one determinant per marking command against an interpretation that has already
parsed the stream.

## The population

`pdf-model/examples/singular_transform_census`, trap 11's shape: the condition is derived from the
clause (a marking command whose transform has no inverse) and the output is what it matched, in
three nested populations — every such mark, the fills and strokes that were the page-losing ones,
and the shadings that are the only paint an inverse actually positions.

The measurement is in `doc/history/653-*.md`; two shapes of it belong here because they are the
argument rather than the number. **It is rare, and where it lands it takes everything**: about one
document in thirteen thousand of the crawl loses a page, and the mark that costs it is usually a
single command among a page's two or three thousand. And **not one witness anywhere carries a
shading**, which is why the refusal `render-cpu`'s doc comment argued for — "the alternative is a
gradient placed somewhere arbitrary, which looks like a rendering rather than a failure" — was
correct about a case that does not occur and wrong about every case that does.

## What pins it

`render-quorra/tests/singular_transform.rs`, a hand-built pair on **all three** backends (trap 8):
one page with a mark that must survive and a command under a singular matrix beside it, and its
twin with that command removed. The two rasters must be **byte-identical**, which asserts both
halves at once — every surviving mark is still there, and the refused one deposited nothing — and
identity rather than a tolerance is derived rather than hoped for, from the zero-area argument
above.

Three matrices, because a determinant is the test and a per-entry test is not: §8.3.4's own example
of four zero elements, one axis scaled by zero (which still stretches the other), and a rank-one
matrix with no zero entry at all. Two paints, two painting operators and two scales, one of them
fractional. The control is the same command under an **invertible** matrix, which must mark the
page — without it the file passes on a backend that draws nothing under any transform, which is trap
2's fifth instance asked of the condition instead of the code.

Run against the defect first (trap 13): with `render-cpu`'s `?` restored, `examples/render_at`
aborts on `4605705.pdf`, `0546320.pdf`, `1407697.pdf` and `2883540.pdf`, and the pair test's first
scene fails.

## One thing the witness taught that the item did not predict

`4605705.pdf`'s singular matrix is not a `0` anybody wrote. It is

```text
a 2.8064233e22   b -4.296242e18   c -9.1778316e21   d 1.4049977e18
```

— a matrix of full rank whose determinant *cancels to zero in `f32`*, because `a·d` and `b·c` agree
to every bit the type has. So "singular" here is a property of the arithmetic as much as of the
file, which is a second reason not to build §10.7.4's run of pixels for this case on the strength of
one witness: the geometry such a matrix states is not recoverable from what the renderer can
compute about it. The three well-formed witnesses are the ordinary kind — `a` or `d` exactly zero.
