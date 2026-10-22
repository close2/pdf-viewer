# 1698 — DefaultRGB follows the current page intent

Archive round, batch one hundred and eight, 2026-10-22. ADRs 2230–2231; no question.
**Implemented; scoped, conformance and canonical archive gates pass.**
ISO 19005-4 §§6.2.3,6.2.4.3; ISO 32000-2 §§8.6.5.6,11.6.6.
The named 6-2-4-3-t04-fail-n has no catalog intent, but a CMYK page intent,
independent gray page group and RGB Form with its own empty resources.
A shipped RGB catalog intent does not license that page's RGB source.
The new route proves every page/resource context before existing global default
placement. It admits a closed page/Form, literal-colour/path subset only;
unsupported groups/consumers/profiles/inheritance keep the old public fallback.
Producer/supplied catalog profiles, part2, the non-RGB route and Q566 are unchanged.
No decision API, consent or writer algorithm expansion.

**Preservation.** Fixed-baseline parser/source control verifies the graph and
independently expected gray 192/white 255; baseline conversion output-fails.
Candidate content and profile bytes, geometry, shared Form/resource identity
and repeat raster survive. Named full images agree at all 1,938,816 pixels;
predeclared allowance was one level. Gray inversion remains a documented
implementation convention, not a general colorimetric theorem.

**Gates.** Prerequisite: 2 passed; 12s hold; focused matrix: 9 passed; 42s hold,
0.96GiB (210s including queue). Candidate first compile failed on private
re-export; corrected. An obsolete baseline refusal assertion then failed
because conversion succeeded. Scoped fmt, canonical warnings-denied all-target
Clippy, private rustdoc and doctests pass; Nextest: 655 passed, 14 skipped (2.498s).
Whole scoped hold63s,2.21GiB;127s including queue.
veraPDF source FAIL4/output PASS4; combined exit1 is the expected negative.
A tinted Gray-LUT control exposed over-admission (exit101,12s); a neutral
kTRC/no-A2B guard now narrows the route. Final Nextest: 656 passed, 14 skipped;
fmt, canonical Clippy, private docs/doctests pass (51s hold,1.98GiB).
Conformance passes137s; final archive passes391s/2.32GiB (960s with queue).
Cross-check: 13,547 selections, zero disagreements; 13 soft-mask selections
remain unjudged under A61. Combined arms/integration belong to 1703.
Owned paths, logs and exported PDFs/PNGs: scratchpad/r1698/; production stable.
