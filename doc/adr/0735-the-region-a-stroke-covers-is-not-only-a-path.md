# ADR 0735 — The region a stroke covers is not only a path

Status: accepted, 2026-08-28. Session 802. Closes the half of §8.7.3 that ADR 0028's rule had
kept unimplemented since the fifty-third session, and that ADR 0463 made audible on both routes
in the six-hundred-and-thirtieth. Does **not** amend ADR 0028: no crate that builds a display
list expands a stroke after this change either, which is the point.

## The subject

`SCN` may install a tiling pattern as the stroking colour, and §8.7.2 says what that means in one
sentence:

> All patterns shall be treated as colours; a Pattern colour space shall be established with the
> CS or cs operator just like other colour spaces, and a particular pattern shall be installed as
> the current colour with the SCN or scn operator

The `scn` half has been drawn since the fifty-third session — a path's fill and a glyph's fill
alike. The `SCN` half was **reported** and not drawn, on the path route from the fifty-third and
on §9.3.6's stroking text modes from the six-hundred-and-thirtieth (ADR 0463), and one corpus
document does it: `scorecard_reduced.pdf` states a dotted leader as a stroke whose colour is a
cell of dots, and this reader drew nothing there.

## Why the instrument chose it

The corpus gate has printed the composition of its `incomplete` list since ADR 0730, with every
report placed under a mechanism and one of three classes — the file's defect, neither one's, this
reader's. **The class this reader owns held exactly two mechanisms**, and the other is
`doc/todo/22`'s Arabic free-text value, read and priced and pinned. So this was the only unpinned
item the gate attributes to this program, and it was named by the gate rather than chosen.

## The reason that had stood, and what was wrong with it

The ledger's §8.7.3 row is unusually good evidence, because the six-hundred-and-thirty-second
session re-derived its blocker rather than repeating it:

- a stroke's cell would be replayed across the stroked **outline**;
- the outline is the backends' to compute (ADR 0028) — `render-cpu` through
  `tiny_skia::PixmapMut::stroke_path`, `render-gpu` through `vello`, `render-quorra` through
  `kurbo::stroke`;
- `pdf-model` and `pdf-render` depend on neither `kurbo` nor a rasteriser, so computing the
  outline here would be a **fourth** expander in the one crate whose whole point is that it has
  none.

Every one of those three sentences is still true. The conclusion does not follow from them,
because they are all about one construction: *the outline as a path*. **The region a stroke covers
is not only a path.** §11.5.2 gives a second way to state a region that no crate has to construct:

> The mask value at each point shall then be derived from the alpha of the group.

A group whose one element is the stroke itself, taken for its alpha, **is** the stroke's region —
including the coverage a rasteriser gives its anti-aliased edge, because it is the same rasteriser
giving it. And a `SoftMask` in this tree already carries a command list rather than a raster,
precisely so that each backend evaluates it at device resolution with its own machinery. So the
shape travels as a `Command::Stroke`, and each backend expands it **once, with the expander it
already has**. ADR 0028's rule is not amended, relaxed or worked around; it is what makes the
construction work.

§11.6.4.2 is the clause that says the two halves multiply, and it says it about this exact object:

> For objects painted with a tiling pattern (8.7.3, "Tiling patterns") or a shading pattern
> (8.7.4, "Shading patterns"), the shape shall be further constrained by the objects that define
> the pattern (see 11.6.7, "Patterns and transparency").

The mark's own shape is the mask; the objects that define the pattern are the tiles it is put on.

## What was built

`content::pattern::Tiled` is the new parameter of `Interpreter::tile`, and it carries the *one*
thing a fill and a stroke differ in — which region the cells are cut to:

- `Tiled::Fill(rule)` is the existing behaviour, unchanged in every particular: a `Clip` over the
  path under that fill rule, and the tiles carry it.
- `Tiled::Stroke(&Stroke)` builds a `SoftMaskKind::Alpha` mask over one opaque `Command::Stroke`
  of the same path with the same parameters, and the tiles carry the graphics state's own clip
  instead. §11.6.5.1 makes the colour irrelevant — "[t]he colours of the constituent objects shall
  be ignored" — so the element is painted white and nothing reads it.

Three smaller things follow and each is a correction rather than a side effect:

- **The tile span is the stroke's reach, not the path's.** `pdf_render::stroked_bounds` already
  answers that tightly, and it is asked in *device* space — where §8.4.3.2 resolves a zero width to
  one device pixel — with its answer then mapped into the pattern's, because a pattern unit is not
  a device pixel.
- **The alpha constant is `CA` and not `ca`.** §11.6.4.4 puts them on different operators and a
  pattern is the colour of one mark; `tile` took `state.fill_alpha` unconditionally before, which
  was right when only a fill could reach it.
- **§11.7.5.2's sixth condition fails through a shape mask.** The tiles are multiplied by a
  coverage below 1.0 wherever the outline is anti-aliased, so the marks inside such a cell are not
  opaque and the overprint condition cannot be met through them. It is now a fifth term in the
  `inside` test rather than an assumption the comment above it stated.

The two groups are two because a command has one mask slot and the two masks are different
quantities — §11.6.4.1's second and third sources of shape and opacity. The inner one carries the
object's shape; the outer one, which existed before and is unchanged, carries the state's own
alpha, blend mode and soft mask, and is still elided where §11.4.4's NOTE 5 makes it a no-op.

## What it costs

One soft-mask group per patterned stroke, which each backend evaluates into a buffer at device
resolution. That is the price of not having a stroke expander here, and it is paid only by a mark
whose colour is a tiling pattern — one document in the pdf.js corpus. The cheaper construction is
the one ADR 0028 forbids, and the choice between them is not close: a fourth expander would have
to agree with three others about caps, joins, mitre conversion and dashes, forever, and every
disagreement would be a page drawn differently by the backend that did not build it.

## What was measured

Every gate in `doc/todo/02` §2, before and after, against a pristine baseline run in the same
worktree.

- **The corpus gate**: `scorecard_reduced.pdf` leaves the incomplete list, and the class the gate
  attributes to *this reader* falls from two mechanisms to one.
- **The oracle**: every verdict count identical — agrees, contradicted, ambiguous, our geometry,
  reference geometry, not comparable, no render — with one page moving from *incomplete* to
  *complete* **inside** the agreeing set, which is `scorecard_reduced.pdf` and is the only line
  that differs.
- **quorra's cross-backend gate**: the differing list is the same set and the same figures, so the
  construction draws the same picture on the third rasteriser. The Vello backend has no corpus
  gate (`doc/todo/02` §2's map) and consumes a `Group` with a mask as it already did; nothing was
  added to any backend.
- **`doc/todo/00` step 7**, our ink minus the lightest live reference's over every ambiguous page:
  the head reproduces entry for entry, `issue16038.pdf` −5.642 then `issue12295.pdf` −2.362, and
  the page this round changed is not in that population because it *agrees*.
- **The page itself** (trap 1): rendered before and after at 2× and looked at. Before, blank where
  the leader belongs; after, the dotted rule. `poppler` draws the same leader, which is evidence
  that §8.7.2 was read right and is not the reason it was read.

## What this does not close

§8.7.3 stays `partial`, and its reason is now its subclause's: Table 74's `/TilingType` is unread,
which §8.7.3.1's own row has said all along. The parent row said something else for seven hundred
and fifty sessions.
