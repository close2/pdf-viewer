# 1215 — The stages a conversion in has, and a matte undone before the conversion

Date: 2026-09-22. Branch: `batch-1213-1218`, worktree shared with five sibling rounds.
ADRs: [1267](../adr/1267-the-conversion-into-a-blending-space-is-carried-in-stages.md),
[1268](../adr/1268-a-matte-is-undone-before-the-colour-conversion.md).

## §11.3.4 — the cube into a parent's space, in three stages

ADR 1254 named the fix as the cube's *input* curves. They are half of it: the conversion in is
`E ∘ L ∘ D` — the device's decoding, a linear stage, the *space's own* encoding — and the unbounded
slope is at `E`, the **output** curve's place. All three now sit where `ColourCube` has a place for
them, so a space encoding its three components alike leaves the grid holding a linear map at two
samples an axis; where they differ the output curve is their pointwise maximum, whose inverse is the
flattest, and the grid carries the residues. Measured over the same 200 000 colours, the instrument
first reproducing ADR 1254's figures exactly: `/Gamma 2.2` 4.66 → **0.88**, `/Gamma 1`
0.09 → **0.0003**, `/Gamma [1.8 2.2 2.4]` 6.32 → **3.96**, a `CalGray` parent 1.70 → **0.0002**.
A table profile has no stage to separate and keeps the sampled grid. The row takes **`departed`**,
its one departure ADR 0790's route into a one-component blending space.

## §11.6.5.2 — both residues closed

A mask stating a one-component space Table 143 does not permit supplies its **samples**: the clause
reads one number per sample as the opacity, and a space says which colour a value denotes rather
than what the value is (§8.6.5.6's own sentence, ADR 1054's reading). The stream is handed on under
a substituted `DeviceGray`, sharing the same bytes, and the departure reported beside the drawing.
The `/Matte` is undone where the clause puts it, for **any** parent space: Table 144's colour in the
parent's components and the mask's samples on the parent's grid travel into the routes that turn
samples into colour — component values in `unpack`, the frame's own samples before
`convert_channels`, component values in `jpx_samples_to_rgba`, and Table 144's `Indexed` sentence
taken literally. What is left is a `/Matte` on a raster the codec did not put on the dictionary's
grid, named through the shortfall.

## §10.8.3 — the spot plane, priced

`ceil(S / 3)` rasters beside the chromatic and black halves, one interpretation per plane, the
colourants enumerated from the page's resources before the first mark; 18 `Half` call sites and 69
`BlendingSpace` ones across seven crates, so several rounds rather than this one.

`raster_golden` moved one page, list only, no pixel; regenerated, rendered and looked at. Corpus
ratchets unchanged, oracle unchanged and green.
