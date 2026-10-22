# 2231 — Gray preservation is checked before default placement

Session 1698, archive. Status: implemented; named preservation and external
validation pass; final scoped, conformance and canonical archive gates pass.

The page-override remedy in ADR 2230 cannot infer preservation from conformance.
It first verifies the named source with the project's parser and CPU rasterizer,
before applying candidate production code. The producer's literal RGB values
are 11/16, 49/64, 111/128. ISO 32000-2 §10.4.2.2's weighted gray, under this tree's
existing inverse/forward GreyRoute convention, is 0.753359375, or 192.106640625
on the byte scale. Strict interior source pixels independently equal 192;
white holes and background equal 255. This is an implementation convention,
not a claim that the standard supplies a unique inverse ICC conversion.

The predeclared output allowance is one code level, from ADR 2114's shipped
sRGB source-profile roundtrip. It is not tuned from candidate pixels. The
named source/output rasters actually agree at every pixel. Decoded page/Form
content, both original profiles, geometry and page-intent/group associations
are preserved; the Form's own resources receive the required default. Repeat
conversion preserves the raster. Project and veraPDF output verdicts conform.

A separate monotone but tinted Gray A2B lookup-table control demonstrated that
GreyRoute existence alone admitted more than the neutral-curve rationale.
It failed at the missing refusal before the guard, not during fixture setup.
The new route therefore requires kTRC and rejects A2B0/1/2 tables. Arbitrary
Gray lookup tables are outside this proof even when this renderer supports them.

The proof's negative controls cover consumer families absent from its closed
subset, supplied/producer RGB profiles, conflicting/malformed page contexts,
shared resources under a different RGB profile, cyclic/dangling graphs and
the content budget. Exact existing defaults stay unchanged. Ordinary RGB
conversions and part 2 retain their previous preparation route. Unsupported
cases remain refusals; no weaker pixel oracle or additional consent is inferred.

Code: `crates/pdf-transform/src/archive/default_rgb/page_contexts.rs`.
Tests: `crates/pdf-transform/tests/archive/default_rgb_page_intent.rs`.
Actual final counts and integration limits belong to histories 1698 and 1703.
