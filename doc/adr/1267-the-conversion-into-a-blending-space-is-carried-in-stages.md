# 1267 — The conversion into a blending colour space is carried in stages, not as one grid

Status: accepted. Session 1215.
Context: ISO 32000-2 §11.3.4, §11.4.7, §11.5.3, §11.6.6, §11.7.2, §8.6.5.2, §8.6.5.3, §8.6.5.5,
§10.3.1, §10.4.2.2; trap 6; ADRs 0263, 0790, 0792, 0796, 0797, 1254.
Code: `crates/pdf-colour/src/colour.rs`, `crates/pdf-colour/src/icc.rs`,
`crates/pdf-model/src/content/transparency.rs`.
Documents: §11.3.4's ledger row, `doc/todo/23`, `doc/todo/65`.

## 1. What ADR 1254 measured, and the one thing it got slightly wrong

§11.6.6 paints a group's result "into the parent group or page", and where the parent composites
in a CIE-based space of its own that painting is a conversion. `transparency::into_parent_cube`
carried it as a `pdf_render::ColourCube` whose input and output curves were the identity and whose
grid held the whole conversion at thirty-three samples an axis. ADR 1254 measured the cost — 4.66
levels of 255 against evaluating the conversion for a `CalRGB` with `/Gamma 2.2`, 3.40 at
sixty-five, 0.09 for a linear one — and named the fix as "the cube's own input curves".

**The input curves are half of it.** The conversion in is `E ∘ L ∘ D`: the *device's* decoding
`D`, a linear stage `L`, and the *space's own* encoding `E`, which is §8.6.5.3's gamma raised to
its reciprocal or a profile's tone curve inverted. `D` is what the input curves carry; `E` sits at
the other end and is the output curve's. Putting only `D` on the curves would have left `E` in the
grid, which is where the unbounded slope is.

The three stages now land where `ColourCube`'s own documentation puts them, and the result is
exact wherever the space's three components are encoded alike: the grid holds a linear map, two
samples an axis reproduce it, and nothing but `E`'s own sampling is left.

## 2. The output curve is one curve and a space has three components

Where the three encodings differ — a `/Gamma [1.8 2.2 2.4]`, a profile whose three tone curves are
three curves — one shared output curve cannot be all of them. The output curve is then their
**pointwise maximum** and the grid holds each component's residue against it.

The maximum is not a convenience. Of the three encodings it is the one whose inverse is flattest,
so every residue `max⁻¹ ∘ Eᵢ` has a slope of at most one and the grid, which is what carries them,
is asked for nothing steep. Measured, worst of 200 000 colours in levels of 255: **3.96** against
the **6.32** the sampled grid gave the same space.

## 3. The numbers

`worst_inward` in `crates/pdf-colour/src/colour.rs` measures both constructions against evaluating
`RgbRoute::components_of_srgb` directly, over the same 200 000 pseudorandom colours ADR 1254 used —
and reproduces that ADR's figures exactly, which is what calibrates the instrument.

| the parent | sampled whole, side 33 | in stages |
|---|---|---|
| `CalRGB` `/Gamma 2.2` | 4.66 | **0.88** |
| `CalRGB` `/Gamma 1` | 0.09 | **0.0003** |
| `CalRGB` `/Gamma [1.8 2.2 2.4]` | 6.32 | **3.96** |
| `CalGray` `/Gamma 2.2` | 1.70 | **0.0002** |

`INWARD_OUTPUT_SAMPLES` is 16 384 because that is where the output curve stops being the largest
term: 1.20 at 8192, 0.88 at 16 384, 0.88 at 65 536. It is sixty-four kilobytes, a sixth of the 431
the grid it replaces held.

**A profile whose conversion in is a lookup table keeps the sampled grid**, and for the reason
`profile_stages` already gives it one on the way out: a table has no linear stage to separate.
`RgbRoute::into_cube` answers `None` there and `into_parent_cube` samples the whole conversion, as
it did for everything.

## 4. What moved

One page of the 974 tracked documents — `bug1721218_reduced.pdf` — draws a different display list
and **not one pixel** moves (`raster_golden`'s pixel digest is unchanged on all 974), which is
trap 41's shape: the cube is part of the list's `Debug`. The oracle's verdicts are unchanged.

ADR 1254 expected 31 documents of 88 890 to reach this path; one of them is in the curated corpus,
and on that one the two constructions agree to within the raster's own quantisation. The crawl's
other thirty are the merge's tier-3 walks to see.

## 5. Why not simply a finer grid

Because it does not converge. The last stage is an inverse gamma whose slope at zero is unbounded,
and a uniform grid cannot resolve it: quadrupling the side bought 1.26 levels. The stage has to go
somewhere a *one-dimensional* table can be made fine enough, which is the output curve, and that is
the whole argument for separating the stages rather than sampling harder.
