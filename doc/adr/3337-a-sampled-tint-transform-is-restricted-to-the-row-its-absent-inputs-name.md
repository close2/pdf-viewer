# 3337 — A sampled tint transform is restricted to the row its absent inputs name

Session 2159. Status: accepted. Context: ISO 19005-2 and -4 section 6.2.4.4; ISO 32000-2 §7.10.2,
§8.6.6.5; ADR 2490 (R12, "refused until its loss is named"), ADR 2690 (the Type 4 restriction),
ADR 1636 (this tree reads `/Order 3` as a tensor-product spline).

## The question

R12's last document, pdf.js `devicen.pdf`, uses `/Red /Green /Blue` under a `DeviceN` space whose
tint transform is a FunctionType 0 over `/Size [13 13 13]`. Printed at the refusal: "the space's tint
transform is a sampled function … ADR 2490 keeps it refused until that loss is named". ADR 2490
assumed that restricting a sampled function to one input means resampling it.

## The decision

It does not, wherever each absent input lands on a sample. §7.10.2 clips an input to `Domain`, maps
it through `Encode` onto a sample position, clips that to `0 … Size − 1`, and interpolates. An
absent input is a tint of zero; where its position is a whole number it names one row of the table,
and an interpolation that is a product of one interpolation per dimension — Table 39's linear one,
and the spline ADR 1636 reads `/Order 3` as — reproduces a dimension's samples at its own sample
positions. So the restriction is a one-input sampled function with the producer's `Domain`,
`Encode`, `Range`, `Decode` and `BitsPerSample` for the kept input, and the row's samples copied bit
for bit (`archive/spot_separation.rs::sampled_slice`). No sample is computed, no loss is named, and
the decision is the same `Mechanical(SpotColorantEntry)` the Type 4 restriction takes. `/Order 3` is
written only where the producer's function was a spline (every `/Size` four or more).

What is left is the case ADR 2490 named: an absent input falling *between* two samples (a domain
such as `[-1 1]` over two samples) makes each new sample a blend `BitsPerSample` may not hold. That
stays refused, now by the sentence `SAMPLED_BETWEEN`, until the loss of a resampled transform is
named; no corpus document reaches it, so no question is asked.

## Result

`devicen.pdf` converts under `as-if-printed` and `as-shown` at 2b and 4, veraPDF 1.30.3 passing
each, page 1 at 72 dpi pixel-identical to the source's. The test evaluates both functions over a grid
of tints for every colourant, for a linear table with a domain excluding zero and for a spline with
`Encode` reversed and `Decode`, at both parts.
