# ADR 0002 — CPU rasteriser first, GPU behind a trait

Status: accepted, 2026-07-26. The choice of CPU rasteriser library is **open**.

## Context

The project aims to be the fastest PDF viewer available, and the initial plan named
Vulkan as the renderer. Two facts complicate that:

1. Vulkan provides triangles and compute, not 2D vector graphics. PDF needs filled
   paths with winding rules, stroking, nested clips, soft masks, transparency groups,
   sixteen blend modes, seven shading types and antialiased text. Building that on raw
   Vulkan is a project comparable in size to the PDF work itself.
2. GPU rasterisation is not automatically faster. Text-heavy pages — the common case —
   are bound by glyph rasterisation and caching. Time-to-first-page is usually
   dominated by parsing, cross-reference resolution and font loading, not by rendering
   at all. GPU wins decisively on continuous zoom and pan, large vector artwork,
   high-DPI output and thumbnail grids.

## Decision

Define one `Rasterizer` trait in `pdf-render` over a resolved `DisplayList`, and
implement a CPU backend first. Add the GPU backend (Vello on wgpu) second, behind the
same trait. Neither backend is privileged: both translate from the project's own
display list.

## Consequences

The decisive benefit is a **same-scene oracle**. Both backends consume a
byte-identical `DisplayList`, so any difference in their output is a backend defect
rather than a difference in how the document was interpreted. That is a far tighter
test than comparing against another PDF viewer, where antialiasing, gamma and
subpixel positioning differ for entirely legitimate reasons. It gives the project a
correctness check that does not depend on any external renderer agreeing with us.

It also reaches a correct rendered page sooner, and keeps a working fallback for
machines with no usable GPU.

The cost is that some rendering work is done twice, and the two backends can drift in
feature coverage. Mitigated by requiring that unsupported content is reported as an
error rather than silently skipped, so the comparison harness sees a failure instead
of a plausible-looking wrong image.

## Resolved: `tiny-skia` is the CPU rasteriser

Decided 2026-07-26 in favour of **`tiny-skia` 0.12.0**.

Two candidates were considered:

- **`vello_cpu` 0.0.9** — shares Vello's scene model, so a reader sees one rendering
  model across both backends. But `0.0.9` is very early: the API will move, and
  feature coverage for soft masks and the non-separable blend modes is unverified.
- **`tiny-skia` 0.12.0** — a mature Skia raster-pipeline port, proven in production by
  `resvg`. Complete for the operations PDF needs, but a second rendering model.

The original argument for `vello_cpu` was a shared scene model making the backend swap
cheap. That argument proved weaker than it first appeared: because `pdf-render` defines
its own `DisplayList` and both backends translate *from* it, neither library's model is
native to the project, so the translation exists either way.

Since this backend's primary job is to be the trusted reference, maturity is worth more
than model sharing — a `0.0.9` dependency makes a poor oracle. `tiny-skia` is therefore
chosen.

A second consideration reinforces it. Because GPU device creation and pipeline
compilation cost tens to hundreds of milliseconds, page one renders on the CPU backend
while the GPU initialises on another thread (see `CLAUDE.md` principle 2). That puts the
CPU backend on the **startup critical path**, not merely in the test harness — which
raises the bar on its maturity and predictability further still.

Consequence to watch: `tiny-skia` covers a subset of Skia. Coverage of soft masks and the
four non-separable blend modes must be confirmed during Phase 5A, and anything missing is
implemented in `render-cpu` above `tiny-skia` rather than worked around silently.
