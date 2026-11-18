# 3019 — Five rationales that claimed more than their sources, qualified where they overreach

Session 2082. Status: **implemented** (documentation and comments; no behaviour changes in this round). Qualifies
ADR 2811, raster's ADR 0030, ADR 2356 with ADR 0171, ADR 0914 and ADR 1080 on the points named below. Context:
review 2038 (findings D04, D58, D61, D62, D64). Clauses: ISO 32000-2 §7.10.5.2, §8.5.4, §8.7.4.5.3–4, §8.3.4,
§9.2.4, §8.7.3.1.

Each of these ADRs reached a decision that stands. What the audit found is an argument that went one step past
its source. Under principle 5 that step is a claim about the specification, and it is withdrawn here so that a
later reader does not build on it.

## 1. `bitshift`'s width is ours, not PostScript's (D04, ADR 2811)

PLRM3's `bitshift` (§8.2) fixes what this evaluator relies on: integer operands, the bits shifted in are zero, and
the direction comes from the sign of the count. ADR 2811 also said that PLRM3's Appendix B "states the integer as
32-bit two's complement". The audit read Appendix B.1 as listing 32 bits as the typical limit of Adobe's
implementations on 32-bit machines, and §3.3.1 as making the representation machine-dependent. **Decision**:
logical shifts stay. `Value::Integer` is `i32` by this project's choice (ADR 0369, ADR 0371), so the exact results at
the sign bit and for a shift of 32 or more are answers at that width, not PostScript's. `function.rs`'s
`Operator::Bitshift` comment and `Value::Integer`'s doc say so. The audit's width-agnostic test
`type4_right_bitshift_inserts_zero_bits_for_negative_values` is the one that pins the clause.

## 2. A clip chain's `min` is idempotent and an upper bound, not always the closer estimate (D58, raster ADR 0030)

Raster's ADR 0030 says the `min` of two links' coverages is "never further from the truth" than their product,
because the exact overlap lies in `[max(0, a+b−1), min(a, b)]` and `min` is that interval's upper end. The interval
is right and the inference is not. With `a = 0.875`, `b = 0.4` and a true overlap of `0.3`, the product `0.35` is
nearer than `min`'s `0.4`. **Decision**: `min` stays for a residue chain, for the two properties that do hold. It is
idempotent, which §8.5.4's intersection is (restating a clip changes nothing). It is exact where two boundaries
coincide or nest, which is what a chain's links usually are. The claim that `min` is always the nearer value is
withdrawn. Neither estimator is §8.5.4's area at a pixel two unrelated boundaries cross. Where the raster lane
measures the geometry exactly, it meets the regions themselves and neither estimate is used.

## 3. A degenerate axial or radial shading paints nothing, with no native fallback (D61, ADRs 2356 and 0171)

ADR 2356 left two cases to the native gradient: an axis of no length, and a radial geometry the library refuses.
Table 79 ("If the starting and ending coordinates are coincident … nothing shall be painted") and Table 80 ("if
both are 0 , nothing shall be painted") decide both. ADR 3008 (B70, B71) makes `pdf_render::Shading::paints_nothing`
the one predicate, and `render-cpu` asks it before any gradient is built. On that point ADR 2356's sentence is
superseded. ADR 0171 classed `a = 0` as "internally tangent; one root has gone to infinity". That is true only
while the centres or the radii differ. When both deltas vanish, `g(s)` does not depend on `s`, and the
one-root argument has nothing to stand on. Table 80 covers the case where both radii are zero. ADR 3008's predicate is
asked before the classification, so the cone test is never reached for it.

## 4. A singular `/FontMatrix` is a transformation, and refusing the font is a choice (D62, ADR 0914)

ADR 0914 refused a Type 3 font whose `/FontMatrix` has no inverse, reasoning that such a matrix "defines no
transformation" from glyph space to text space and so fails §9.2.4's `shall`. §8.3.4 says otherwise. A matrix whose a,
b, c and d are all zero is one under which "all user coordinates map to the same device coordinates", a
noninvertible transformation, and painting under one "can result in unpredictable behaviour". That is a warning,
not a refusal. **Decision**: the refusal stays, as this crate's choice. It is resource-local and said by name. The
alternative is a guessed replacement scale, which `Type3Error::NoFontMatrix` already declines. `type3.rs`'s doc
comments, its error message and the ledger note now say "has no inverse" and call the refusal a choice.

## 5. A constant-spacing tiling owes its spacing (D64, ADR 1080)

ADR 1080 read a declined matrix snap as leaving Table 74's code 1 with no obligation. Table 74 makes consistent
spacing a `shall` and the method a `may`, naming XStep, YStep and the matrix as the levers. ADR 2987 (B72) already
amends ADR 1080: where the matrix snap is refused, `pdf_render::snap_steps` rounds the steps. This ADR adds only the
cross-reference, so that D64 is answered in one place with the others.
