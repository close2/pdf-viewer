# 0292 — The interpolation is on the parameter, and the streaks were the function called early

**Status.** Accepted.
**Context.** §8.7.4.5.5's mesh rows were `partial` on a note about a subdivision that had not
existed since the forty-third session. Reading the clause to correct the note found a `shall` the
row had never carried, and the code had never obeyed: a mesh shading that states a `/Function`
interpolates the *parameter* and calls the function afterwards. This tree called the function at
each vertex and interpolated the colours it returned.

## The sentence, twice

ISO 32000-2 §8.7.4.5.5:

> If the shading dictionary contains a Function entry, the colour data for each vertex shall be
> specified by a single parametric value t rather than by n separate colour components. All linear
> interpolation within the triangle mesh shall be done using the t values. After interpolation, the
> results shall be passed to the function(s) specified in the Function entry to determine the
> colour at each point.

And Table 81's own `/Function` row states the same order independently, which is worth having
because a reader who missed the paragraph would still have to answer the table:

> The designated function(s) shall be called with each interpolated value of the parametric
> variable to determine the actual colour at each point.

The two orders agree **exactly where the function is a straight line** and nowhere else. `f(a)`
interpolated to `f(b)` is a chord; `f` of the interpolated parameter is the curve. Nothing about
the resulting page says which was drawn, no report fires, and both are plausible pictures — which
is trap 1's shape with the metric not merely silent but structurally unable to speak.

## What was done

**The parameter crosses into the display list.** `pdf_render::Corners` is what a mesh triangle's
corners carry, and it is an enum because the distinction cannot be recovered from a colour once one
has been computed:

- `Corners::Colours([Color; 3])` — the mesh states components and no `/Function`.
- `Corners::Parameters([f32; 3])` — §8.7.4.5.5's `t`, as a fraction of the range `/Decode` gives
  it, with `ShadingKind::Mesh`'s `ramp: Option<Ramp>` beside the triangles.

**The function crosses as a `Ramp`,** which is the same answer this tree already gives for an axial
or a radial shading and for the same reason: a display list holds no PDF functions. It is sampled
*across* its own breakpoints at §10.7.3's resolution — `Ramp::sample_across_at` — so a type 3
stitching function's steps survive the sampling. The two call sites now share one construction,
`pdf_model::shading::breakpoints_over`, because the mapping from a function's own domain onto the
ramp's unit interval is a rule that would otherwise drift in two places.

**`MeshRaster::build` reads the ramp at the interpolated parameter**, one barycentric mix per
device pixel, which is where the clause's interpolation already lived (ADR 0051). A parametric mesh
handed no ramp is **refused** rather than painted with its parameters read as colours — trap 5,
and the invariant `ShadingKind::Mesh` states is what makes the refusal unreachable from this tree.

**The reader is generic over what a vertex carries and nothing else.** `pdf_model::mesh`'s trait
`Corner` has four methods — read one, mix two, hand three to the display list, and a placeholder
for a patch slot the stream is about to fill — and every other line of the reader is unchanged:
the flags, the lattice rows, the patches and their shared edges are the same code for both. That is
the shape of the clause, which makes `/Function` a choice about *what is interpolated* rather than
about a format.

**Four public methods went with it.** `Triangle::is_flat`, `is_subpixel`, `average_colour` and
`subdivide` were the subdivision's API and had been called by nothing but their own tests since the
forty-third session, when `MeshRaster` replaced it. A public item with no caller is a claim about
what a crate offers, and two ledger rows were still making it — §10.7.3's cited `is_subpixel` as
what bounds a mesh's geometry, and §8.7.4.5.5's described the whole subdivision as current. Both
are corrected here, and so is the one sentence in `doc/performance.md` that named the method: what
it measured stands and where it went is now beside it. Four stale comments in the two backends went
the same way, one of them still saying "the caller subdivides and fills them".

## What the page says, which is the whole argument

`issue6231_1.pdf` is the corpus's one lattice mesh and its `/Function` is a **type 3 stitching
function**, so it is the one corpus witness to this order. Page 1 at scale 2, before against after,
the same crop:

- **Before**: the plotted surface carries a lattice of yellow streaks running diagonally across it
  — one per row of the mesh, which is exactly what a chord between two function values per triangle
  produces when the function bends inside the triangle.
- **After**: one smooth surface, the yellow band where the stitching function puts it.

449 pixels of the page differ, by up to **48 of 255**. Over the crop the distance to `poppler`'s
render at the same resolution falls **1.067 → 0.617** of a level per pixel: the picture we now draw
is `poppler`'s, which is evidence that the clause was read right rather than a target that was
matched — the reading came first and the render agrees with it.

The other two corpus documents with a parametric mesh, `coons-allflags-withfunction.pdf` and
`tensor-allflags-withfunction.pdf`, move by **one level at most**: their functions are near enough
to linear over a patch that the two orders very nearly coincide, which is the clause's own
condition and is the reason nothing had ever caught this.

`mesh_shading_empty.pdf` is **byte-identical**, and that is the finding rather than an absence of
one: its two meshes state colours per vertex and no `/Function`, so this clause cannot reach the
page the §8.7.4.5.5 row had spent four hundred sessions naming.

## The population, counted rather than assumed

`pdf-model/examples/mesh_census` walks the corpus: **10 documents state a mesh shading, 3 of them
one with a `/Function`** — the three named above — over 28 mesh shadings in all. The by-space
tally is what §8.7.4.4's separate departure turns on, and re-running the census corrected a claim
this round inherited: the 3 meshes in a `DeviceN` space are **2 in `personwithdog.pdf` and 1 in
`bug1703683_page2_reduced.pdf`**, not three in the first, and all three tint transforms are type 4
functions. A number nobody re-ran is the previous round's.

## What it costs, measured

- **The oracle**: **every verdict count identical and every per-page line identical**, before
  against after, over 1794 pages — 905 agrees, 68 contradicted, 786 ambiguous, 1 our geometry, 2
  reference geometry, 14 not comparable, 18 no render. `issue6231_1.pdf` **agreed before and
  agrees now**: the streaks were a few hundred pixels of a plotted surface on a page of axes and
  labels, which is inside the tolerance a page of that content gets. **So the instrument that
  judges this tree against three others could not see this defect at all**, which is trap 1's
  sentence with a number attached — the picture found it and no metric would have. The two
  patch fixtures' lines do not move to two decimal places either (worst mean 0.24 on both).
  The *before* half was taken by stashing the round and re-running the gate.
- **The corpus gate**: 974 documents, and every `incomplete`, `locked`, `unusable` and
  `encryption` line identical before and after — this change adds no report and removes none,
  because the order was wrong rather than the feature absent.
- **`doc/todo/00`'s step-7 ink sweep**: **all 786 rows byte-identical**, before against after,
  which is the honest result rather than a shrug: a difference of at most one level over a few
  hundred pixels of two fixtures is under the third decimal of a page-wide mean. The head is
  unmoved and every name on it is diagnosed — `issue16038.pdf` −5.734, `issue12295.pdf` −2.823,
  `issue14297.pdf` −1.145, `issue7821.pdf` −1.000, then nothing past −0.839 — over the 747 rows on
  documents this tree calls complete.
- **The cross-backend gates**: `cpu_and_gpu_agree_on_a_parametric_mesh_shading` is new, and it is
  trap 2's rule one field over — every mesh scene in the suite carried `Corners::Colours`, so the
  ramp a backend reads was a parameter no scene had ever varied. The quorra corpus gate is
  unchanged.
- **The workspace suite**: 1634 tests, all passing.

## What discriminates, which is the part a passing test does not show

Both new tests were run against the old order, by restoring it in `Triangle::paint` alone:

- `pdf_render`'s `a_parametric_mesh_interpolates_the_parameter_and_not_the_colour` — a square-law
  ramp, the simplest function that tells the orders apart. Fails at 134 where the clause gives 70.
- `pdf_model`'s `a_mesh_with_a_function_interpolates_the_parameter` — a whole PDF with a type 2
  function of `/N 2`. Fails at 129 where the clause gives 65, and the message says both numbers.

`a_lattice_mesh_triangulates_between_its_rows` was checked the same way, by emitting one triangle
per lattice cell instead of two: it fails on the fourth corner, which is the only corner a single
triangle does not reach.

## What was considered and not done

- **Carrying the function itself to the backends.** It is the only construction with no sampling
  error at all, and it is refused for the reason `pdf-render` exists: a display list that held PDF
  functions would put colour management and function evaluation inside both rasterisers, where the
  two could disagree. The ramp is bounded by §10.7.3, which is the clause that decides how fine a
  sampling has to be.
- **Keeping colours at the corners and subdividing the triangles until the chord and the curve
  agree.** That is the construction the forty-third session removed for its lattice and its bias,
  and it would have to come back finer rather than coarser. The ramp costs one lookup per pixel
  against a subdivision that costs triangles.
- **Reporting a parametric mesh instead of drawing one**, until it was drawn right. Refused: the
  order was wrong, not absent, and the three pages drew something close to correct. A report there
  would have taken three pages off the oracle's judged set to say something the ledger already said.
