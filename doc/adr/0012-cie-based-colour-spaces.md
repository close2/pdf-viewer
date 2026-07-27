# ADR 0012 — `CalGray` and `CalRGB` are converted through CIE XYZ, and their `BlackPoint` is not applied

Status: accepted, 2026-07-27.

## Context

`CalGray` and `CalRGB` were treated as `DeviceGray` and `DeviceRGB`: the components were
written into the sRGB raster unchanged. The module said so, and the reasoning offered was
that the difference was small.

The corpus oracle (ADR 0011) showed it is not small. `calgray.pdf` declares
`/Gamma 1.0`, and at `A = 0.35` we produced 89 of 255 where poppler, mupdf and ghostscript
all produce 160 — a mid grey rendered as a near-black, on every swatch of the page, and on
every page anywhere that uses either space with a gamma that is not the display's.

The reason the shortcut ever looked reasonable is worth recording, because it is the shape
of a whole class of near-misses. §8.6.5.2's own EXAMPLE 2 is a `CalGray` with
`/Gamma 2.222`, and for *that* space the shortcut is nearly right: decoding by 2.222 and
re-encoding for sRGB very nearly cancel. Most real documents write something close to it. A
shortcut that is correct on the common case and badly wrong on the rest reports nothing
either way.

## The decision

Both spaces are converted as ISO 32000-2 defines them, in four stages.

1. **Decode.** §8.6.5.2: `A^Gamma`, scaled by the three components of `WhitePoint`, gives
   XYZ directly — a `CalGray` has no second transformation stage. §8.6.5.3: each of `A`,
   `B`, `C` is decoded by its own gamma and the vector multiplied by `Matrix`, whose nine
   numbers are three XYZ *columns*, one per input component.
2. **Adapt.** §10.3.1 says conversion from a CIE-based source to the destination "shall be
   performed based on ISO 15076-1:2010 (ICC.1:2010)", whose media-relative colorimetric
   intent maps the source white point onto the connection space's D50. The transform is
   Bradford, which is what ICC's own `chad` tag carries.
3. **Convert.** One matrix from D50 XYZ to linear sRGB.
4. **Encode.** The sRGB transfer function.

Stages 3 and 4 are `colour::xyz_d50_to_srgb`, and they are now **the only** place in the
crate where an XYZ becomes a pixel. They were two places before this: `colour::lab` and
`icc::xyz_to_rgb` each held their own copy of the same nine constants. That is the
`DeviceCMYK` failure of ADR 0009 in miniature — three conversions that disagreed, none of
which looked wrong on a page — and one copy is how it stops being possible.

The folded matrix has a test that recomputes all nine numbers from the two published
matrices they were folded from (IEC 61966-2-1's XYZ-to-sRGB, and a Bradford adaptation
D50→D65). A folded constant is otherwise unreadable and unfalsifiable; this makes a typo in
any of them, or in either Bradford constant, a test failure rather than a slight shift in
every CIE-based colour in the tree.

Images take the same route, per sample, exactly as `DeviceCMYK` images already did. The
alternative — a second conversion inside `image.rs` — is the thing ADR 0009 exists to
prevent.

## `BlackPoint` is read and deliberately not applied

This is the part that is a **choice**, and it is worth separating clearly from the part
that is a derivation.

ISO 32000-2 §8.6.5.3 says `WhitePoint` and `BlackPoint` "shall control the overall effect of
the CIE-based gamut mapping function described in subclause 10.3", and that the two are
"typically" mapped to the lightest and darkest achromatic colours the device can render.
Doing that to the black is black point compensation, and §8.6.5.9 puts it under
`/UseBlackPtComp`: `ON` means "according to the provisions in ISO 18619", `OFF` means none,
and `Default` — the value every document in the corpus leaves it at — is "left to the PDF
processor to determine". So there is no derivation available here. There is a decision.

The decision is to reproduce the colorimetry the space states, and two things settled it.

**The stretch is undefined on input the specification permits.** Table 63 requires only that
the three numbers be non-negative. Nothing puts the black below the white, and
`calrgb.pdf` page 14 exercises exactly that: `BlackPoint [0.2 1.0 1.7]` against
`WhitePoint [1 1 1]`, so the Y axis has zero span and the Z axis a negative one. A
construction that must be guarded into doing nothing on two axes of three, and whose result
on the third is then arbitrary, is not the construction the clause means. Implementing it
anyway produced a visibly wrong page: a colour cast on every swatch, which no reference
shows.

**It is not the quantity `icc.rs` compensates.** There, the black point is *measured* from
the profile — the darkest colour the output device can actually reach — and aligning it to
the display's black is what `PDF20_AN001-BPC` argues for and what stops a press's black
rendering as a washed-out grey. A Cal space's `BlackPoint` is a statement about the
*source*: §8.6.5.3 says its "value is limited by the dynamic range of the input device" and
that it "varies with exposure, system response, and artistic intent". The two share a name
and are different measurements, so the ICC path keeps its compensation and this one has
none.

The cost, stated rather than hidden: a document that raises its `BlackPoint` gets shadows at
the lightness it states, not stretched down to the display's black. `calgray.pdf` page 3 and
`calrgb.pdf` page 14 are the corpus's only instances and both are files written to probe the
entry. A test pins the choice, so reintroducing a stretch fails rather than silently moving
every colour in every document that raises its black.

All three reference renderers do the same. That is evidence about how §8.6.5.2 and §8.6.5.3
are commonly read, and it is not the reason — principle 5 permits agreement as
corroboration and forbids it as a target.

## Consequences

Measured by the oracle over 1794 pages: pages agreeing with the reference consensus rise
from 548 to 556, and contradicted pages fall from 174 to 166. `calgray.pdf` pages 1–3 and
`calrgb.pdf` pages 2, 4, 7, 13 and 14 leave the ratchet.

Four `calrgb.pdf` pages remain contradicted. They differ from mupdf and ghostscript by about
ten levels in one channel while matching poppler exactly — a residue of colour management at
a scale where closing it would mean choosing whose arithmetic to copy. They stay named.

Two things follow for whoever comes next.

- **`ICCBased` still falls back to a device space** when its profile cannot be parsed, which
  §8.6.5.5 explicitly permits. That fallback is now the *only* CIE-based approximation left
  in the module.
- **The `black` fields are parsed and kept** rather than dropped. They are what the document
  said, and a future implementation of ISO 18619 needs them; the doc comment on
  `cie_to_srgb` is where the argument lives, so a reader who wonders why they are unused
  finds the answer next to them rather than in this file.
