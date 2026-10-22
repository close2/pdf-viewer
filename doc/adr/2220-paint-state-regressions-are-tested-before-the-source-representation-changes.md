# ADR 2220 — Paint-state regressions are tested before the source representation changes

The sequence of RGB solid, pattern tint and ordinary ICC defects has a common boundary:
selection retains a converted colour while painting can change the conversion's inputs.
Fixing one consumer leaves another plausible-looking picture wrong. Splitting a large module
alone cannot prevent this; the invariant must be stated across the state changes and consumers.

ISO 32000-2 §11.7.5.3 requires the rendering intent in force at elementary painting, while a
CIE-based group's conversion uses the intent at its invoking Do. The source, including the
resolved profile and component values, survives graphics-state saves and ordinary Form entry.
A resource name is not that source's identity: a Form can bind the same name to another profile.
§8.6.5.9 independently controls black point compensation, with the absolute-intent exception.

`crates/pdf-model/tests/icc_paint_context.rs` is written before the production change. Its
one-channel ICC fixture has different exact middle knots for its three intent tables. An
independent calculation applies the encoded XYZ values, the published D50-adapted sRGB matrix
and transfer function; one output level covers quantization. It does not call the interpreter's
solid-colour or cache helpers to manufacture its expected answer. Cold-versus-warm comparisons
supplement this oracle, and explicit nonwhite/interior checks prevent two missing paints from
agreeing. A white initial colour and restored white source are painted over black controls.

The tests cross selection order, ri and ExtGState RI, nested q/Q, independent fill and stroke,
embedded normal text, one shared stencil, profile changes, device resets, pattern interleaving,
Form resource shadowing, a Gray group and return, and alpha after warm reuse. DeviceGray defaults
and component-count recovery test the actual resolved source rather than the declared name.
A late BPC change and the absolute-intent exception distinguish source from rendering parameters.

The pre-fix run on `0eb4d278` exposes the defect directly: with BPC disabled, late Perceptual
painting retains (139,135,152) where the fixture's table gives (253,57,60). Changing BPC to ON
retains (89,89,89) instead of black. The inherited source in the Gray group remains coloured
where fresh selection within that group is neutral. These are failures to repair, not new
expected outputs to accept. The complete failure receipt is retained with the round's evidence.

The implementation must retain immutable source identity and values through state cloning,
apply the current conversion context at one shared solid-paint boundary, and keep alpha outside
any colour memo. A bounded cache must include source identity/components and every conversion
input its result depends on. An allocation-saving cache is accepted only after separate ordinary
ICC, generic ICC tint and mixed-use costs are measured; an RGB timing cannot price an ICC path.

This test population does not establish every non-RGB family, multi-channel profile, malformed
recovery or unsupported target. Those retain their separate witnesses and refusals. In particular,
an A2B-only source fixture is not evidence for admitting it as an invertible blending space.

The first post-change run passed twelve controls and exposed an error in the overlapping-alpha
oracle. It rounded straight RGB before applying ideal floating-point composition, bypassing
the isolated group's eight-bit premultiplied storage. Independently calculated unrounded RGB
is approximately (252.91,56.54,60.13). Quantizing its half-alpha source gives (126,28,30,128);
the existing tiny-skia 0.12 lowp source-over arithmetic gives (189,42,45,192) after the second
paint and (252,105,108) over white. The staged oracle now represents those buffer boundaries;
the one-level tolerance is unchanged. Cold and inherited selections independently agree.
This corrects the test's arithmetic, without changing the compositor or accepting an observed
raster as a golden result.
