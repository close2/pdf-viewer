# 2553 — §10.7.5's grid fit is drawn in every backend at once, and on by default

Status: implemented. Session 1859, batch one hundred and twenty-six, building the owner's answer A714
(2026-10-28) to `doc/questions/Q714`, its option 1. Context: ISO 32000-2 §10.7.5, §8.4.3.2; ADRs 0085,
0848, 1445, 2022, 2530, 2531. Code: `raster/crates/raster-gpu/src/raster/grid_fit.rs`,
`raster-gpu/src/encode/{stroke.rs,line.rs,expansion.rs}`, `raster-gpu/src/shaders/line.wgsl`,
`crates/render-cpu/src/lib.rs`. Test: `crates/render-raster/tests/stroke_adjustment.rs`.

## 1. What was owed

> When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
> automatically be adjusted as necessary to produce lines of uniform thickness. The thickness shall be
> as near as possible to the requested line width -no more than half a pixel different.

ADR 2530 built that first requirement in `render-cpu` (`pdf_render::grid_fit`) and measured it; ADR
2531 kept it off because `raster`'s lanes resolve a stroke's width per placement at encode and applied
only the half-pixel promotion (ADR 0085), so the oracle would have stood alone. A714 chose option 1:
the fit at the encoder's four `resolve_width` call sites, agreement across the oracle and all three
lanes at eight placements, and the default turned on in every backend at once.

## 2. The construction in `raster`

`raster::grid_fit` restates `pdf_render::grid_fit` in the library, which depends on no PDF crate: the
same three bounds (`FIT_BELOW` 1.5 device pixels, a segment along an axis to 1/64 of a pixel, the two
stretches alike to 1/1024), each vertex moved across the segments it ends to `⌊v⌋ + 0.5`, the width
one device pixel. **The hypothesis held: the fit is a function of the stroke and the placement alone**,
the translation included, so it is asked where the composed transform is known, per placement. The
four sites:

- **`encode/stroke.rs`, the stroke arm**: the fit is asked before the cull, whose reach grows by the
  half pixel a vertex may move. A fitted stroke is expanded at its placement and pushed directly
  (`push_stroked`), past the per-outline shared expansion (ADR 1445) and the deferred parallel job,
  because both hold pieces made under the linear part and moved by the translation, and a rule fitted
  at one offset is a different set of pixels at another.
- **`encode/line.rs`, the line drawn as a stroke** (`resolve_width` at the expansion and at `pen`, the
  dot's): the same fit on the two-point path; a fitted round dot is one device pixel across.
- **`encode/line.rs`'s `PagePen::on_device` and `line.wgsl`'s `device_line`, the closed form**: the
  host's `fit_on_device` grows the ink box the plan is cut by, and the vertex stage moves the ends and
  sets the pen per view, by the same steps — a round pen under 1.5 pixels across on a line along a
  device axis — since that line is resolved on the device at every view (ADR 2022's ulp caveat holds).
  Roundness is asked of the pen the *stated* width makes, before the half-pixel promotion, because the
  promoted pen is a disc under any transform and `pdf_render::grid_fit` asks the transform: on
  `bug1743245.pdf`, whose matrix stretches its axes 0.125% apart, asking the promoted pen fitted the
  grid's 0.16-pixel rules in raster and not in the oracle, a mean error of 10.35 levels.
- **`render-raster`'s dashing** (`stroke.rs`, `fitted_for_dashing`): `render-cpu` fits the command's
  path at its stroker's entry and dashes the fitted path, while `render-raster` cuts dashes in path space
  before raster sees them. A dashed stroke the fit takes is therefore fitted there first, by
  `pdf_render::grid_fit` itself, so its dashes and the dots §8.5.3.2 makes of its zero-length dashes lie
  on the fitted row; `issue14297.pdf`'s dotted table rules were 0.229 levels from the oracle without it.
- **`encode/expansion.rs` is not a site, by design.** `Expansion::of` resolves the width under the
  linear part alone, which is exact for §8.4.3.2's zero and §10.7.5's promotion, both functions of the
  stretch; a stroke the fit takes never reaches it. Its comment says so.

`CpuRasterizer`'s `stroke_adjustment` now defaults to `true`; `with_stroke_adjustment(false)` remains
for the measurement's other arm. **ADR 2531's default is reversed by the owner's answer**; its argument
— both constructions meet the clause's bound and the choice is a reading of intent — stands as the
reason the owner was asked.

## 3. The evidence

**The test.** `render-raster/tests/stroke_adjustment.rs` draws three shapes at eight placements, an
eighth of a pixel apart: a 0.6-pixel rule under `/SA`, a stroked outline, and a rule promoted under
stretches 0.125% apart. The oracle draws the rule as one fully covered row at all eight and without
`/SA` at none of them; the CPU and GPU lanes are within 2 levels of the oracle on every shape at every
placement.

**Plants.** Each of the two lane defects the corpus gate found was put back and its own check went red:
the line lane asking roundness of the promoted pen drew the unequal-stretch shape 128 levels from the
oracle; dashes cut before the fit in `render-raster` put `issue14297.pdf` back at 0.229 levels.

**The two defects, before and after** (oracle against the lanes, mean levels): `bug1743245.pdf` 10.35 to
0.108; `issue14297.pdf` 0.229 to 0.0124, where HEAD's arm stood at 0.1233.

**The corpus.** `raster_golden`: 7 pages moved — `annotation-underline`, `issue14297`, `issue15629`,
`issue18032`, `issue21570`, `issue2761`, `stamps` — every one a page stroking under `/SA true`,
re-pinned. The `render-raster` corpus gate: 968 pages, 968 agree, 0 differ, 0 refused. Against HEAD's
six arms, each 1× arm (cpu, gpu, compute) moves exactly those seven digests and nothing else, and no 4×
arm moves one; the error against the reference falls or holds on every moved page but
`annotation-underline.pdf` in the cpu and gpu arms (0.0044 to 0.0046), and `issue14297.pdf` goes 0.1233
to 0.1160 there and 0.0201 to 0.0124 in the compute arm. The oracle (`--clock`, 1971 pages): agrees
1028, contradicted 47, ambiguous 837, HEAD's figures exactly; `issue14297.pdf` and `bug1743245.pdf` stay
ambiguous, as at HEAD. `fixed_documents`: none of the 77 rows moved. ADR 2530's own figures on page one
of the pdf.js documents stand as measured there.

## 4. What it costs

ADR 2530's cost is unchanged and now drawn: a fitted rule requested between half a pixel and one is up
to half a pixel heavier, inside the clause's bound; 2 seam pixels opened over 123 shared stretches on
page one of the pdf.js documents, none closed; no measurable draw time. In `raster` a fitted stroke
gives up the shared expansion and the deferred job, which only thin axis-aligned `/SA` strokes reach,
and a scene holding a dashed stroke the fit takes is true at its own view only, as a scene holding
§8.5.3.2's dots already is (ADR 0702), so a zoom step rebuilds it.
