# 0702 — The scene is built in page space, and the viewport carries the placement

**Status.** Accepted — stage two of quorra's ADR 0084, on the other side of the
boundary from their ADR 0085 / our ADR 0701, which unblocked it: with the stroke
width scene-space, a stroked scene stopped being untrue at other viewports, and the
question of *building* it placement-free became answerable.

## What changed

`render-quorra` used to bake each page's placement into every command it translated
(`Encoder::placed` composed the target transform in), so a zoom step was a new scene:
a full re-walk of the display list — 9–15 ms on the worst page — before quorra saw a
single command. Quorra's `Viewport` has taken a full affine since their ADR 0001;
nothing but this crate's own construction spent that.

Now a frame that is one page, over a medium, with no fallback raster and no overlays,
is built in **page space**: `Encoder::with_placement` composes the identity instead of
the target transform, and `FrameSlot::render` hands the placement to quorra as the
viewport affine. Three consequences, each deliberate:

- **The surround is not in the scene.** A page-space scene states nothing of the
  window; the presenter's own medium layer (ADR 0378's texel, composited `OVER` under
  the base) shows through wherever the scene left transparency. §11.4.7's 𝑊 stays in
  the scene, one rectangle at the page's own boundary, in page space.
- **§14.11.2.1's crop is stated unconditionally.** The old early-out asked "does any
  pixel of *this* target lie outside the boundary" — a question about the view — so a
  page-space scene answers it by always hanging the one clip rectangle.
- **Overlays keep the baked path.** They are stated in window pixels and a page-space
  viewport would move them. The windowed renderer already routes them down its chrome
  lane, so the page lane never sees one; the restriction exists for
  `rasterize_frame`'s callers.

## The scene survives the view change — when the build read no view

Building in page space is necessary but not sufficient: some translations *read* the
placement and write its answer into the scene. Every such site now marks the encoder
(`Encoder::consume_view`), and the ledger of them is the honest boundary of this
change:

| site | why it reads the view |
|---|---|
| `split_collapsed_fill` | §10.7.4's marks are sized and placed on this view's pixel grid |
| `split_degenerate`, dashed strokes | §8.5.3.2's dots likewise (dashes are cut in path space, but their zero-length marks are not worth telling apart yet) |
| anisotropic stroke expansion | expanded in device space |
| images | filter choice and pre-reduction are read off this view's scale |
| shading grids, sampled masks, mesh rasters | produced at this placement's grid |

A page-space build that fired none of these is **view-free**, and `Retained::draws`
then ignores the placement and the frame's extent entirely: a zoom, a scroll, a
resize, and the supersampling pass's 2× frame all reuse the scene as it stands —
quorra re-encodes under the new viewport affine and nothing on this side rebuilds,
re-walks or re-uploads anything. The relaxed key still compares what genuinely
reaches the pixels: the medium, the absence of a raster, the page count — and
`draws`'s identity and overlay checks run unchanged.

Everything else — multi-page arrangements, raster fallbacks, chrome, view-consuming
builds — rebuilds exactly as before, byte for byte. The mode admits; it never bends
a frame that does not qualify.

## Measured

The phase probe (this repository's `rasterize_frame`, 1200×900, alternating
placements so every frame is a zoom step, RADV 890M, `Coverage::Compute`):

- A dense text page (600 show ops): scene phase **0.0 ms from frame 1**, whole zoom
  step 4–9 ms. The scene is built once and survives every view change.
- The worst page (`doc/todo/44`, 58 009 fills): **still rebuilds** — ~20 of its fills
  are hairline-thin rulings that `split_collapsed_fill` collapses to §10.7.4 marks at
  any plausible zoom (measured firing identically at 0.55× and 2.4×), and one
  view-consuming command marks the whole scene. Its zoom step stays ~70–100 ms,
  scene 8–15 of it.

That finding converts a deferral into the named next step: ADR 0701 postponed moving
the degenerate/collapse splitting into quorra as "its own change", and this
measurement is why it is now the *blocking* change for the worst page — the collapse
must be resolved at encode, per placement, exactly as the stroke width now is, or the
page can never be view-free. The per-command ledger above is the work list.

## Held by

The sixty `render-quorra` tests and the full workspace, unmodified — the oracle
comparison is placement-for-placement identical because a page-space scene under a
placement viewport draws the same pixels the baked scene drew — plus the probe runs
above. Quorra is untouched by this stage; the pin stands.
