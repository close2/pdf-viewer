# 1197 — A clipping region is a union, and a mask behind a codec is decoded once

Three priced residues: two built, one corrected on evidence the note had wrong.

## §10.7.4 — the mixed clipping path, four departures → three

§8.5.4 — "the same area that would be filled by the f operator is the area that would be used for a
clip" — makes the region of a path that encloses an area *and* rules a line the union of two fills
under two rules. `clip_region` returns a `ClipRegion` — `One` or `Union` — so no backend decides it. `render-cpu` composes it (`scan::mask_union`, a sum capped at
the pixel, the side "at least as large as the area of the original shape" errs on); `render-raster`
and `render-gpu` refuse a `Union` by name, each having only a per-pixel route whose mask
*multiplies into a draw already carrying the document's own*. The substitute's two over-drops went
with it: per-subpath rectangles, and a strict overlap. Fixture: a square with a rule across it under
both operators, held to the fill of the same path, calibrated by planting the old drop (900 admitted
where the fill paints 925); even-odd discriminates, because concatenating states a hole. Looked at
(trap 1). ADR 1231; `QUORRA_FEEDBACK.md` section 51 is the ask.

## §11.6.5.2 — the codec residue, three → two

`doc/todo/41` said what was owed was "a **bound** on that plane, and it is a decision of its own".
It is `PREFER_DEVICE_SCALE_ABOVE`, which already routed the pair here and caps the grid this route
produces — nothing arbitrary replaced (`doc/todo/10` §6). Within it a codec-carrying mask is
decoded **once per document** behind `MaskCache`'s existing `ObjectId` key — what the
refusal's own reason said could not be done: trap 40. Above it, the eager combination as before. A
stencil under one arrived with §11.6.4.2's shape and §11.6.4.3's opacity multiplied and reported,
and now states its shape. ADR 1232.

## §8.9.6.4 — note corrected, row unchanged

Recorded as "the decoder's eight-bit hand-off", and wrong about the decoder:
`hayro_jpeg2000::ComponentData` hands out `samples()` unscaled with `bit_depth()` beside it, and
`pdf-sandbox`'s palette-indices arm already reads them so. The stretch is `data_u8`'s, one of two
exits — the narrowing is this tree's, at `decode.rs`'s `jpx` and at `protocol::Raster`, which
carries that escape hatch for a *grid* and none for a precision. Nothing upstream is owed.

## Gates

`raster_golden` held 974, moved 0 — the control the census predicted; `pdf-model --test corpus`
every ratchet at its ceiling; `render-raster --test corpus` 942 / 8 / 8; the oracle green. Tier 1
clean on the four render crates, `pdf-model`'s blocked all round by neighbours.
