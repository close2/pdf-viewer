# ADR 0213 — One mark, described by two cells

Status: accepted, 2026-08-07. Session 374. The half ADR 0155 measured and left:
`issue16038.pdf`'s second square, 13% under the ink its own geometry states.

## The figure

`/pgfpat22` is a tiling pattern whose `/BBox` is `[0 0 2.98883 2.98883]`, whose steps are both
2.98883, and whose whole content stream is

```text
q 0.3985 w 0.0 0.0 m 2.98883 0.0 l 0.0 2.98883 m 2.98883 2.98883 l S Q
```

— a rule along the *bottom* edge of the box and another along the *top*, each 0.3985 wide and each
therefore half inside the box and half outside it. Table 74's clip keeps the inside half of each,
so the rule at every multiple of 2.98883 is assembled out of two cells: the lower half from the
cell below it and the upper half from the cell above. This is how a producer draws a continuous
rule out of a repeating figure, and `/pgfpat21` on the same page is the same rules at the other
phase — one rule through the middle of a cell that spans it — which is what makes the page its own
conformance test. **The two squares state the same ink and must weigh the same.**

They did not. Interior coverage against the geometry's own answer, measured over eight whole
periods inside each square with `examples/render_at`:

```text
              1×      2×      4×      8×
left        0.983   0.985   0.967   1.002
right       0.858   0.771   0.901   0.951
```

The right square is short at every scale and worst where the rule is thinnest relative to the
pixel. The arithmetic is exact: two halves each covering `a` of the boundary pixel are painted one
after the other and composite as `1 − (1−a)(1−b)`, which for `a = b = 0.266` is 0.462 where the
rule's own width says 0.531 — 87%, and 0.1159 against 0.1333 is 87%.

## The clause answers whether two cells' contributions compose or add

This was the round's open question and it is not open. §11.6.2:

> Single graphics objects, as defined in 8.2, "Graphics objects", shall be treated as elementary
> objects for transparency compositing purposes … Portions of an object shall not be composited
> with one another, even if they are described in a way that would seem to cause overlaps

and §11.6.7 is what makes the whole tiling *one object's paint* rather than a stack of many:

> When the pattern is later used to paint a graphics object, the colour, shape, and opacity values
> resulting from the evaluation of the pattern definition shall be used as the object's source
> colour ( 𝐶𝑠 ), object shape ( f j ), and object opacity ( qi ) in the transparency compositing
> formulas.

The pattern is *evaluated* to a shape, and that shape is then the object's. So the tiles are
portions of one object and §11.6.2 forbids compositing them with one another outright. §8.7.3.1's
"figures on adjacent tiles should not overlap" is what then makes the shape's area the sum of the
tiles' areas rather than something needing a union computed; and §10.7.4's "[t]he area covered by
painted pixels shall always be at least as large as the area of the original shape" is the floor
the 87% fell through.

§11.6.7's NOTE 2 — "[i]n a raster-based implementation of tiling, it is advisable to treat all
tiles as a single transparency group. This avoids artifacts due to multiple marking of pixels along
the boundaries between adjacent tiles" — is the same advice informally, and this tree has built
that group since the hundred-and-seventeenth session. It does not fix this, because compositing
*inside* a group is still compositing: two elements of shape 0.266 union to 0.462 there as
anywhere else. **The advice names the artefact and does not remove it**, which is worth writing
down, because the note is the obvious place to look and it is a dead end.

`doc/todo/11` said the fix was "rasterising a tiling's coverage once rather than cell by cell", the
cells accumulating into one coverage buffer. That would work and it is not what was built.

## What was built instead: the two halves are one mark, so draw it once

The cell states the rule **twice**, a whole `/YStep` apart. Those two statements are not two marks
of the tiling — they are the same mark, described from two cells: cell *k*'s top rule lands exactly
where cell *k+1*'s bottom rule does. Keeping one of them and taking the box clip off it draws that
mark once, whole, with a single anti-aliased outline. The painted set of points is identical; the
number of commands painting it goes from two to one; and there is no buffer, no accumulation, no
snapping to the grid and no new vocabulary in the display list.

`pdf_render::repeat::repeated_subpaths` is the rule and `without_subpaths` carries it out;
`Interpreter::plan_repeated_marks` asks the first where `unclip_redundant_cell` (ADR 0155) has
already failed, and `fold_repeated_marks` applies the second to every cell. The two clip questions
are one question at two strengths: "does the box cut anything" and "is what it cuts described again
a step away".

### What has to be true, and it is an argument rather than a heuristic

Write *S* for the marks one command of the cell states, *R* for what survives folding, *B* for the
box and *L* for the lattice the steps generate. The clipped tiling paints `M = ⋃ᵥ ((S ∩ B) + v)`;
the folded one paints `P = ⋃ᵥ (R + v)`. Because *B* + *L* covers the plane once, every point `p` has
exactly one `v` with `p − v ∈ B`, and `p ∈ M` exactly when `p − v ∈ S`. So `P = M` follows from one
condition:

> for every kept subpath `s` and every `u ∈ L` whose copy `s + u` reaches into *B*, the command
> already states `s + u`.

Any `p ∈ P` lies in some `s + w`; its `p − v` lies in `s + (w − v)` and in *B*; so `s + (w − v)` is
stated and `p − v ∈ S`. On `/pgfpat22` the only such `u` is one step up, and the top rule is
exactly it.

Two further conditions are about the raster rather than the set, and each buys a specific thing:

- **`R` fits inside one step in each axis.** Then no two sites' copies of it overlap, so "drawn
  once" is a fact. It also settles a fill's winding without a second check: every dropped subpath is
  `K + v` for a kept `K` and a nonzero `v`, so it lies in a box that meets the kept ones' box in no
  area at all, and a subpath that cannot reach a point was never deciding whether that point is
  inside it.
- **`R` stays within half a step of *B*.** The tile span is computed from the *box* before any cell
  has run, and a folded mark hangs outside its box. `span`'s own `floor`/`ceil` slack covers an
  overhang of less than a step — `floor(a + b) ≤ ceil(a)` for `0 ≤ b < 1` — so half a step is
  comfortably inside what is already drawn and no ring of extra cells is needed.

### Four decisions

- **It is decided after the cell is drawn, from the first cell, and applied to every cell.** ADR
  0155's reason unchanged: a cell's marks are not known until its content stream has run, and
  running it twice would double the readback, the text layer, §14.8.2's artifact spans and §9.3.8's
  overlap bookkeeping. The commands carry their geometry and name their clip, so only the path and
  the clip's *name* change. `Command::path_mut` is new and exists for this, beside `set_clip`,
  which exists for ADR 0155.
- **The answer is a plan of indices, decided once, and every cell follows it.** The first draft
  re-derived the fold from each cell's own commands, on the argument that every cell states the same
  figure and would therefore get the same answer. **It did not, and the reason is the one worth
  recording.** The comparison runs in pattern space, which the caller reaches through an inverted
  matrix, so it needs a tolerance; with an exact one the first draft folded **180 of 1296 tiles** —
  the ones near the pattern's origin, where the subtraction comes out exact, and none further out,
  where an `f32`'s own neighbours at x ≈ 65 are already 4 × 10⁻⁶ apart. **Half a folded tiling is a
  worse picture than none**, and it is nearly invisible in a count: the page's ink went 0.1197 to
  0.1205 of an expected 0.1333, which reads as "the change did nothing" rather than as a defect.
  Widening the tolerance moved the threshold rather than removing it — the noise scales with the
  coordinate and the tolerance does not — so the fold is now `repeated_subpaths` once per pattern
  and `without_subpaths` per cell, and consistency is structural rather than numerical. The guard is
  that the command at each planned position still draws a path with the number of subpaths the
  answer counted; a cell that does not is *reported*, and nothing in the interpreter can produce
  one.
- **One tolerance, a ten-thousandth of a step**, and it now decides only the first cell. What it
  admits is a mark moved by 10⁻⁴ of a tile: 0.0003 of a point here, two hundredths of a device pixel
  at this viewer's 6400% clamp. What it costs is that a pattern whose first tile happens to sit very
  far from its own origin may not fold at all — a missed improvement rather than a wrong picture,
  which is the direction this has to err in.
- **The search is bounded at 64 subpaths per command.** It is quadratic, and with the plan it runs
  once per pattern rather than once per tile, so the bound is generous; it is still a deliberate
  shortcut with its cost written down — a cell whose one command draws more than 64 subpaths keeps
  the picture it has today.

## What it bought, measured

Interior coverage of the two squares against the geometry, same instrument, before and after:

```text
              1×               2×               4×               8×
left     0.983 → 0.983    0.985 → 0.985    0.967 → 0.967    1.002 → 1.002
right    0.858 → 0.989    0.771 → 0.971    0.901 → 0.972    0.951 → 1.003
```

The right square now lands on the left at every scale — 1.01, 0.99, 1.01, 1.00 of it — which is the
instrument that needs no reference at all: the two patterns state the same rules and now weigh the
same. Total ink over the page, `(1 − red) × area` against the 316.29 square points the geometry
states: **287.16 → 299.89** at 1×, **309.14 → 315.18** at 8×, and 313.84 at 24×. The picture agrees
with the arithmetic: at 8× the right square's rules used to be visibly paler than the left's, and
now the two squares are the same weight.

The gates say the change reaches this page and no other. Corpus 974 with **72** incomplete and the
same set; the oracle's seven buckets identical — 857 agreeing, 68 contradicted, 750 ambiguous, 0/2
geometry, 9 not comparable — with **exactly one page of 1794** whose numbers moved, this one: worst
mean 41.12 → **40.55**, worst tile 54.11 → **52.84**, structural similarity 0.3826 → **0.3935**, the
differing fraction unchanged at 26.03%. Text 99.2% and quorra unmoved. `doc/todo/00`'s step 7 over
all 786 ambiguous pages moves its head and nothing else: `issue16038.pdf` **−6.404 → −5.398**, with
every other entry unchanged to a thousandth.

**The page keeps the head of that ranking and will keep it**, which is worth stating rather than
leaving as a puzzle for the next round. The gap is our ink minus the *lightest* reference's, and on
this page every reference paints more than the geometry — `hayro` 139%, `mupdf` 115%, `poppler`
157%, `ghostscript` 299% — because a rule 0.4 of a device pixel wide is §10.7.4's whole pixel to an
aliasing renderer. We are at 95% of the geometry now instead of 91%, and no further correctness
would close the rest: closing it would mean painting more than the document asks for.

It costs something, on the page it fires on. `callgrind_interpret` over `issue16038.pdf` page 1:
**4 954 871 → 5 096 687** instructions, **+2.9%**, which is the fold's search once per pattern plus
`without_subpaths` rebuilding one path per cell. Nothing outside `Interpreter::tile` is touched, so
a page with no tiling pattern pays nothing; ISO 32000-2's page 101 interprets at 2 200.3 M, which is
recorded as today's figure rather than as a comparison — the last one in an ADR is session 188's
2 167.0 M, 186 sessions of interpreter ago.

## What it does not do

The general case remains: two *different* marks that abut across a box edge — a figure hanging out
of the top of the cell and a different one hanging out of the bottom, meeting at the boundary. The
clipped pair is then the right set of points and there is no repeat to fold, so drawing it without
a coverage accumulator would need the two pieces joined as geometry — and a boolean intersection of
path against box would bake a flattening resolution into a display list that deliberately has none.
**No corpus page names it**: `issue16038.pdf` was the family's only witness and its figure repeats,
so what is left is unwitnessed rather than measured and left. `doc/todo/11` keeps the item with
that as its remaining half rather than closing it.

The other residual is visible in the numbers above and is *not* this: both squares sit 1.5% to 3%
under the geometry at 2× and 4×, and that is the rules' **ends** meeting, cell column by cell
column, at a seam of exactly the same kind one axis over. It is smaller because a seam along a
rule's end is one pixel column per three, where the seam this ADR removed ran the whole length of
every rule.
