# 3773 — §11.3.5.3's ClipColor divides only where its denominator is positive

Session 2268. Status: accepted. Context: `issue18032.pdf` p1, where raster drew a black pixel inside
a white glyph; ISO 32000-2 §11.3.5.3, §11.3.6; ADR 3761 (the round that saw the pixel).

## What the pixel was

The page composites in a four-component space, so raster draws it twice, the chromatic list and
§11.4.7's black list. In the black list the pixel is a white "d" at nine points, partly covering
the pixel, under an isolated knockout group of blend mode `Color` painting white. Reduced to those
two commands the pixel stays black; under `Normal` it is white.

`Color` is `SetLum(Cs, Lum(Cb))`, and `SetLum` ends in `ClipColor`, whose second branch, for a
colour with a component past 1, divides by `x − l`, the largest component less the luminosity.
`Cb` is the backdrop's premultiplied colour over its alpha, and the device's division puts it a
hair past 1.0 at some partial alphas. White is neutral, so `x` is `l`, `ClipColor` took
`0 × (1 − l) / 0`, and the NaN stored as 0: black, in both rasters. A ramp of 256 partial alphas
under the same group drew a whole column black on the RX 580.

## Decision

Each of `ClipColor`'s two divisions is taken only where its denominator is positive, in raster's
`composite.wgsl` and in the CPU oracle's `blend.rs`. In exact arithmetic neither denominator is
zero where its branch is taken: `SetLum`'s target is the luminosity of an in-range colour, so a
colour out of range is never neutral. Only rounding makes one, and the clause's formula has no
value there; leaving the colour as it is, a hair outside the range, and letting the store clamp it,
is what the formula tends to as the colour approaches neutral (its largest component goes to 1).

The CPU oracle's comment had argued `x − l` positive from `l` being a mean of the components; in
`f32` `Lum` of `(v, v, v)` is `v` exactly for every `v` just past 1, so the argument holds in the
reals only. The oracle did not draw this pixel black because its backdrop is not divided on a
device, but the same zero over zero was a call away.

vello 0.9's `clip_color` (both `vello_shaders` and `vello_cpu`) has the unguarded formula; it drew
the reduced scene white, and its source is upstream's.

## Tests

`render-raster`'s `neutral_colour_blend.rs`: the glyph where the document draws it and a ramp of
partial alphas, under the white `Color` group; §11.3.5.3 and §11.3.6 make every pixel white, and
the test asks `255` of every channel (it failed on two places before the change). `render-cpu`'s
`a_neutral_colour_a_hair_past_white_is_not_divided_by_zero` takes the 64 neutral colours above 1.0
and counts that their luminosity rounds onto them, so it cannot pass vacuously.
