# 2331 — The four departures read against the exact measure, and the patch rows against the chords

Status: accepted for integration. Session 1748, 2026-10-25, the clause slot of batch one hundred and
fourteen. Clauses: ISO 32000-2 §10.7.4, §8.7.4.5.7, §8.7.4.5.8, Table 52, §10.7.2. Every row stays
`departed`; the notes are rewritten to what is. Context: ADRs 0025, 0308, 1082, 1217, 1560, 1622,
2122, 2170, 2252, 2268, 2280, 2281, 2330; `doc/questions/A559`.

## 1. The question, and the hypothesis that did not hold

§10.7.4's first sentence named four departures — the one pixel, (1) a partly covered pixel painted at
its area, (2) the area sentence, (3) a reduced image averaged — written before ADRs 2122, 2252, 2268
and 2281 made a folding stroke's measure exact. The brief's hypothesis was that the exact measure
lifts two. Read against the code path that now does the work (`render_cpu::area`,
`area::union`, `stroke::folding`, `scan::level_of`), it lifts none: the measure is departure (1)'s
own construction made exact, and the other three are not on its path. The one pixel is lifted, by a
reading rather than by the measure (ADR 2330).

## 2. Each departure

- **(1) Anti-aliasing stands.** "[P]ainting any pixel whose half-open square region intersects the
  shape" is a whole pixel; the measure paints the area the shape covers there, exactly for every
  shape short of the declines ADR 2280 names, and `level_of` rounds it to the nearest of 255 levels,
  so a pixel covered under half a level is painted nothing. That is the departure ADRs 1082 and 1560
  decided and priced, and the measure changed its accuracy, not its kind.
- **(2) The area sentence stands, narrowed.** "The area covered by painted pixels shall always be at
  least as large as the area of the original shape." Read as the coverage a mark deposits, the
  measure now meets it to within three named residues: `level_of`'s rounding, half a level per
  partly covered pixel in either direction; a folding stroke's chords at a twentieth, inscribed, at
  most `√2/20` of a rim pixel (ADR 2252); and the marks the walk or a cost guard leaves to the
  library's sixteenths (ADR 2280). Read as the pixels painted, it fails with (1). A `shall` met on
  one reading of a word the standard does not define stays `departed` (ADR 1622 §1). The seam two
  marks leave in a shared pixel (ADR 0308) is §11.3.7.3's union of two objects, not this sentence.
- **(3) Image reduction stands.** `Image::area_averaged` is no part of the measure; ADR 0025's
  reason, several samples sharing one device pixel, is unchanged.
- **The one pixel is lifted** (ADR 2330), with a line across the axes beside it; what replaces it as
  a departure is the subpath that retraces itself off any one line, ADR 2330 §4. The fifth, quorra's
  sampled lane past ten times magnification (ADR 0556), is untouched.

## 3. The patch rows

§8.7.4.5.7 and §8.7.4.5.8 are `departed` on Table 52's flatness — "[t]he precision with which curves
shall be rendered on the output device", initial value 1.0 (ADR 2170) — for the one kind of patch
`pdf-colour` still tessellates at `mesh::PATCH_STEPS`, whose colours are converted between its
corners in a space no backend holds. The folding work's flattening is a stroke's: `FOLD_FLATNESS`,
chords at a twentieth of a device pixel, chosen under §10.7.2's permission (`doc/questions/A559`),
finer than Table 52's 1.0 and in `render-cpu`, which no patch reaches. So it lifts neither row. Lifting
them is a colour grid carried beside the control net through `pdf_render::SurfacePatch` to every
backend, so that the silhouette is derived in device pixels while the colours stay converted where
they are; no document this tree holds takes the branch, which is the row's own census. Both stand,
that cost recorded. One sentence of §8.7.4.5.7's note was stale and is corrected: `render_cpu::area`
flattens a fill's curve at a 256th of a device pixel, and a folding stroke's pieces at a twentieth.
