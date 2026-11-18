# 2973 — An `ICCBased` space drawn in its alternate is held to its own range first

Session 2074. Status: **accepted and built.** In `pdf-colour`: `ColourSpace::IccAlternate`, `cie::constrained`, the
`/Alternate` branch of `parse_icc_based`; in `pdf-model`, `blending_space::space_identity`'s arm for it. Finding
2038-B25. Clauses: ISO 32000-2 §8.6.5.5, Tables 65, 67 and 68.

## Context

Table 65's `/Alternate` row: "Colour values within the range of the ICCBased colour space might not be within the
range of the alternate colour space. In this case and after constraining to the ICCBased range, the nearest values
within the range of the alternate space shall be substituted without error indication." Where the profile could not
be applied the parser answered with the alternate space itself, so the `ICCBased` space's `/Range` was lost: an
ICC.2 `'Lab '` profile this reader does not support, `/Range [0 100 -128 127 -128 127]`, over an alternate `Lab` of
`[-200 200 -200 200]`, drew `a* = 150` unclamped.

## Decision

The `/Alternate` branch answers with `ColourSpace::IccAlternate { range, alternate }`. Every question about the
components — how many, their range, the initial colour (§8.6.5.1's sentence for `ICCBased`), an image's default
`/Decode` — is the `ICCBased` space's; every conversion first holds each value to `range` and then hands it unchanged
to `alternate`, which holds it to its own ("There shall not be conversion of source colour values"). A component
`/Range` states no pair for takes the alternate's own range rather than Table 65's printed default of the unit
interval, because that default "shall match the information in the ICC profile" (Table 68's row for the profile's
data colour space), which is the information this reader could not read: the unit interval would hold an `'Lab '`
profile's `a` and `b` to `[0, 1]`. Identity and route questions (`profile_identity`, `grey_identity`, `rgb_identity`,
`RgbRoute::of`), spot-colourant reach and §8.6.6.5's "CMYK colour space" test look through to the alternate, as they
did when the alternate stood alone.

## Consequences

A device-space alternate with the default range draws as before. The `/N`-only fallback (no `/Alternate`) is
unchanged: it has no range to apply that differs from the device space's. Test: `pdf-model`
`icc_alternate_range_spec_audit`.
