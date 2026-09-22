# 1199 — A named page is a button's appearance, and a stroke is cut as its outline

Date: 2026-09-22. Branch: `batch-1195-1200`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADRs: [1235](../adr/1235-a-named-page-becomes-a-buttons-appearance-and-a-second-file-is-a-hop.md), [1236](../adr/1236-a-stroke-is-cut-as-its-outline-and-a-bezier-at-its-roots.md).

## §12.7.8.3.2 — `/APRef`'s no-`/F` branch, built

Table 253 makes `/F` optional and states the default itself: absent, "the page resides in the
associated PDF file". Two branches, one host question. `named_page::page_as_form` turns a page this document names under §12.7.7 into the Table 93 form
§12.5.5 places — §7.8.2's concatenated `/Contents`, §7.7.3.4's inherited `/Resources`, §14.11.2.1's
crop box as the `/BBox`, Table 31's `/Rotate` as the `/Matrix` (whose translation §12.5.5's own
steps supply), §11.4.7's page group carried. It goes into the `Import::appearance` a stated `/AP`
fills — Table 249's own ranking rather than a second route — so it is drawn, saved and undone by
code that already existed. A page's `/Annots` are not composed in, and the import says so.

## §12.7.8.3.2 and §12.7.8.3.3 — the `/F` branch, named rather than built

Both rows keep one entry apiece and it is the same one. The copy exists (`forms_data::carry`) and
the conversion exists (`page_as_form`); what is missing is the **hop** — a second host question
raised while the first import is being applied, §12.6.4.4's suspended-walk shape `viewer_core`'s
`Purpose::TargetRoot` already has. `pdf-model` has no filesystem and must not acquire one.

## §12.5.6.23 — the two path refusals a geometry could lift

Both sentences ADR 1195 wrote were about this build rather than about the geometry, and the second
contained its answer: if a stroke's marks *are* the outline, cut the outline.

- A **Bézier** is split at roots: the depth along a segment is a polynomial whose Bernstein
  coefficients are its control points' own depths; closed form, split by de Casteljau.
- A **stroke** is expanded (dash first, §8.4.3.6), cut as a fill, painted `f` in the *stroking*
  colour — replayed under Table 74's non-stroking operator from the producer's own operand bytes
  inside a §8.4.2-balanced `q`/`Q`. §11.6.4.4's two alphas differing refuses the page.
- Admitted only where the expansion came back **polygonal**, asked of the output: an arc is an
  approximation, and approximating the producer's marks *outside* the region crosses the fence.

The stroke's cut is byte-identical outside the region at 150 dpi. The curve's is weaker by one
eight-bit step and says so: the rasteriser subdivides a sub-curve differently from that stretch of
the whole, so what is asserted is that a pixel painted whole or left untouched is identical and a
partial-coverage pixel differs by at most one step. Fixtures hand-built; the census stays zero.
`kurbo` joins `pdf-transform`'s dependencies; it is already `render-raster`'s.
