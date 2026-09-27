# 1342 — A `DeviceCMYK` mask group that blends is composited in its four components

Status: accepted. Session 1252.
Context: ISO 32000-2 §11.5.3, §11.3.4, §11.3.5.2, §11.3.5.3, §10.4.2.4, §11.4.7, §11.6.3; ADRs
0217, 0220, 0277, 0856, 0857, 1254.
Code: `crates/pdf-model/src/soft_mask.rs`, `crates/pdf-model/src/content/ext_gstate.rs`
(`resources_blend`), `crates/pdf-colour/src/colour.rs` (`device_ink_press`),
`crates/pdf-render/src/soft_mask.rs` (`Luminance::device_ink`),
`crates/viewer-confined/src/protocol/display_list.rs` (its tag).
Tests: `crates/pdf-model/tests/blended_cmyk_mask.rs`, `soft_masks.rs`'s report test.
Documents: §11.5.3's ledger row, `doc/todo/23`, `doc/todo/65`.

## 1. The residue

A `/Luminosity` mask group in `DeviceCMYK` is painted in one channel, `1 − ink ÷ 2` with
`ink = 0.3 c + 0.59 m + 0.11 y + k`, and §11.5.3's `min` is applied after compositing (ADR 0220).
Source-over is affine, so that is exact under `Normal`. §11.3.5.2 applies a separable blend function
to each component "expressed in additive form", and §11.3.4 complements a subtractive component
before and after it; a function of a weighted average is not the average of the function, so under
Multiply the one channel reads black-over-black at 0.08 where the clause gives 0.16.

**The non-separable modes were never wrong there.** EXAMPLE 2's weights are `Lum`'s, and Hue,
Saturation and Color keep `Lum(Cb)` and §11.3.5.3's `K` of `Cb`, Luminosity `Lum(Cs)` and `K` of
`Cs` — so the mask is the backdrop's or the source's `Y` either way. Only the separable modes other
than `Normal` part.

## 2. The construction

Such a group is carried as §11.4.7's pair — the chromatic raster holding `1 − c, 1 − m, 1 − y`, the
black one `1 − k` — which every backend already composites per channel for a four-component profile
group (ADRs 0856, 0857), so the complement is the raster's and the blend functions see additive
components. The pair is taken where `resources_blend` finds a non-`Normal` `/BM` in the group's
resources, or its forms', tiling patterns' or Type 3 fonts', to depth 8; a group with no resources
of its own is asked about the page's, which is what it draws from. Past 256 dictionaries the answer
is yes, which costs a second interpretation and nothing else.

**The press is chosen so that nothing but the blend changes.** `colour::device_ink_press` converts a
colour of another space by §10.4.2.4 with `BG(k) = k` and `UCR(k) = k`, whose weighted ink is
`0.3 c + 0.59 m + 0.11 y` — the black added is the black removed from weights summing to one — which
is `1 − (0.3 R + 0.59 G + 0.11 B)`, the grey the one-channel route paints the same colour in; a grey
comes in as black alone, §10.4.2.2. So a `Normal` mark masks to the same value on both routes, and
the choice of route cannot move a page that does not blend. The mask's `Y` is EXAMPLE 2's formula of
the composited four, evaluated as stated: its `min` folds the hypercube along a plane no grid's
cells follow, so it is a shape of its own and one tag on the confined protocol.

Rejected: the assumed press's own conversion in. It is a search against `CMYK_CORNERS` (ADR 0263)
whose EXAMPLE 2 grey is not the RGB colour's, so a red mark's mask value would change when a blend
mode appeared elsewhere in the same group.

## 3. What it moved

No first page: `raster_golden`'s moves are ADR 1341's alone, and `luminosity_mask_census` counts no
such group in 1126 curated or 65 703 crawled documents. `tests/blended_cmyk_mask.rs` states the
clause's values by hand and the planted-away run — the pair route turned off — reads 0.082 for 0.16.

## 4. What the row is

§11.5.3 is `departed`, and the one departure is ADR 1254's `MAX_PRESSES` bound, as for §11.6.6 and
§11.7.2: a four-component profile group on a page past it takes the device branch, reported. Lab and
a CIE-based space with no route are the file's failures under §11.3.4, not this reader's debt.
`note_blended_luminosity` still names a blend the walk did not see.
