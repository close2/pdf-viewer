# 3000 — A stroke and its degenerate subpaths are one transparent object, which the split draw is not

Session 2079. Status: **accepted; the code change is owed by G092 (finding B39).** Supersedes ADR 0420 §5 on one
point: its sentence that "§11.6.2 already makes a stroked path one object in the transparent model, so nothing
changes but the annotation". Context: review 2038 (finding D29, whose counterexample is B39). Clauses: ISO 32000-2
§8.5.3.2 with Issue #103, §11.6.2.

ADR 0420 read Issue #103's scope — "In the opaque imaging model, this rule shall apply only to zero-length subpaths
of the path being stroked" — and concluded that the transparent model needed nothing new, because §11.6.2 makes the
whole stroke one object and the tree was taken to paint it as one. That last step was an assumption about the tree,
not about the clause, and the audit's witness refutes it: one `S` at `/CA 0.5` in black over white draws its line's
interior at red 128, and adding a round-capped zero-length subpath inside the same path's line draws that spot at
red 64. The substituted dot (`split_degenerate`, ADR 0290) is a second paint composited over the first, so the
part of the page both cover takes the object's alpha twice.

What stands of ADR 0420: the clause reading of Issue #103 (the dot rule's scope is the opaque model), the
placement and coverage arguments for a dot smaller than a pixel, and the alpha floor of ADR 0419. What is
withdrawn: that existing stroke handling already preserves §11.6.2's one-object compositing. The construction
§11.6.2 asks for is that the stroke's line and every mark the dot rule adds form one shape, and the object's
opacity is applied once to that shape's coverage; how each backend unions them is G092's decision, in the round
that changes `render-cpu`'s and `render-raster`'s stroke code. Until then, five audit tests
(`stroke_composition_pdf_spec_audit`, `transparent_degenerate_spec_audit` and the GPU pair) fail and are listed in
`expected-failures-current.json`; nothing here changes code.
