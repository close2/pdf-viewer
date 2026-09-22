# 1268 — A `/Matte` is undone before the colour conversion, and a mask's space says nothing about its value

Status: accepted. Session 1215.
Context: ISO 32000-2 §11.6.5.2, Table 143, Table 144, §8.6.5.6, §8.6.6.3, §8.9.5.2, §7.4.8,
§7.4.9; traps 5, 6, 40; ADRs 0034, 0210, 0321, 1008, 1054, 1218, 1232.
Code: `crates/pdf-model/src/image.rs`, `crates/pdf-colour/src/colour.rs`.
Documents: §11.6.5.2's ledger row, `doc/todo/65`.

## 1. Table 143's colour space is a statement about a *file*, and the value is the mask's

Table 143 makes a soft-mask image's `/ColorSpace` "Required; shall be `DeviceGray`". This tree
accepted any **one-component** space and then read the first channel of the *converted* raster —
so a `CalGray` of `/Gamma 1` stating 128 became an opacity of 188, because 128 as a linear
luminance encodes to 188 of 255.

Two answers were available and the clause decides between them. §11.6.5.2 reads one number per
sample as the image's opacity; a colour space says which colour a value denotes and never what the
value is, which is §8.6.5.6's own arithmetic — "[c]olour values in the original device colour space
shall be passed unchanged to the default colour space" — and the reading ADR 1054 already took one
key along. So **the number the file states is the opacity**, and the departure from Table 143 is
reported beside the drawing rather than instead of it. Refusing would draw the image opaque, which
loses transparency the producer did state.

It is built by substitution rather than by a flag: `soft_mask_entry` hands on a stream whose
dictionary says `DeviceGray`, sharing the same `Arc` of bytes, so the one route that reads the
samples reads them as values by construction and the document's memo of the decoded stream is the
same entry.

## 2. The pre-blending is undone where the clause says, which is before the space is consulted

§11.6.5.2 states the blending as `c′ = m + α × (c − m)` and then states *where* the inversion
belongs: "[t]he preblending computation shall be done in the colour space specified by the parent
image's ColorSpace entry … If a colour conversion is required, inversion of the pre-blending shall
precede the colour conversion."

This crate held one RGBA raster per image and divided *that* by α, which is exact only where the
conversion is the identity on components — `DeviceGray` and `DeviceRGB`, and nothing else. In any
other space the raster's bytes are a function of the pre-blended components rather than the
components. The smallest fixture where the two orders disagree is a `DeviceCMYK` image whose
samples state a quarter of cyan and a quarter of magenta pre-blended with white paper at α = 64/255:
the producer meant one unit of each, the ink cube's blue corner at 46,49,146, and dividing the
raster instead lands on **0,0,131**.

So `Prematte` — Table 144's colour in the parent's own components, and the mask's samples on the
parent's grid — travels *into* the routes that turn samples into colour, and the inversion happens
per component before `Conversion::paint`. Table 143 makes the two grids one wherever a `/Matte` is
present, so the samples pair by position; `matte_colour` refuses a file that states two grids,
because such a file has stated no pairing at all.

**Three sites, because there are three domains and one arithmetic.** `unpack` inverts in component
values, which is every packed image and every `JBIG2Decode` and `CCITTFaxDecode` one; the
`DCTDecode` route inverts in the frame's own samples before `convert_channels` reads them, with the
matte carried into those units by `Decode::raw_of` and §8.9.5.2's map affine, so the two domains
give the same answer; `jpx_samples_to_rgba` inverts in component values beside §7.4.9's own
premultiplication, which is a different entry with a different weight.

**A matte takes the sample memo and the palette away**, and for one reason: the colour of a sample
stops being a function of the sample alone. It costs one conversion per sample on the images that
state a `/Matte`, of which the corpus holds one.

## 3. An `Indexed` parent inverts its table entry

Table 144 counts the matte's numbers in "the colour space specified by the ColorSpace entry (or the
base entry of the colour space, if the colour space is Indexed )" and says what carries the
blending: "the colour values in the colour table (not the index values themselves) shall be
pre-blended". So the entry an index selects is what is inverted, in the base space's components,
and the restored entry is then painted as a colour of that space rather than as an index of this
one. `ColourSpace::entry_of` and `ColourSpace::indexed_base` became public for that one reader.

## 4. What is left, and why it is not a reading

A codestream that states its own grid can contradict the dictionary's, and then there is no pairing
to invert by: §7.4.8 puts a JPEG's dimensions in the data, and §7.4.9 NOTE 3 lets a `JPXDecode`
codestream over the confined worker's budget come back at one of its own reduced resolution levels
(ADR 0321). On either the pre-blending is **named** rather than undone, through the same
`SamplesOnGrid::shortfall` the damaged-filter routes use. The first is the file's failure; the
second is this tree's budget, and it is the one thing §11.6.5.2's row still carries.
