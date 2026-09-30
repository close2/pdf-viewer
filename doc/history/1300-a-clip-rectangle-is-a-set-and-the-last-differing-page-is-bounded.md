# 1300 — A clip rectangle is a set, and the last differing page is held to a bound

Robustness slot of batch forty-six: raster's corpus gate read page by page to its clause. ADR 1435.
No question. `render-cpu` was not opened for any raster expected value (A76, trap 9).

**Premise.** "A flat sheet is the page" is the oracle gate's bucket; this gate's seven are 2
would-not-open and 5 no-first-page, and four of its six refused are vocabulary, not budget.
**`issue19083.pdf` — fixed on raster's lane.** A border rule on its `/BBox` edge; four shaders
multiplied coverage by the clip rectangle (row 26: 112 where §10.7.4's intersection is 64).
Rectangles now intersect before the cell overlap; a coverage byte meets the clip by `min`.
`raster-gpu/tests/clip_meets_a_mark_as_a_set.rs`: 5 pixels from the closed form, 4 fail under
the product. Gate 1×: 959/2/6/7 → 960/1/6/7.

**`issue2177.pdf` — held with a bound.** Per-pixel reference from the page geometry (128² samples,
0.016 from 64²): oracle 0.71, raster 2.05 of 255, raster 0.40 light; ink 13013.8 / 13030.3 /
12933.9. Interior cause measured: raster's 0.25 px inscribed flattening (pre-flattened at 1/256:
raster 0.39 of reference, page mean 1.94 → 0.68) — `src/raster/` is 1298's, so feedback section 59 ask 1.
Worst tile (32,224), one row: a residue clip at a curved fill; oracle +8.5 heavy, raster 7.2 light.
Now `Held { mean 1.94, worst_tile 11.6 }` against 1.9385 / 11.54; planted at 1.90, it failed.

**Budget: not moved, no census owed.** 1×: the cycle (budget doing its job) and `issue19517`
(16384-per-side capability). 4×: `issue12810` 609086160 bytes on a 71.7 MP whole-page frame;
`issue1905`/`issue9418` the sheet ceiling (the note's stale 365144861 corrected). Refusals are
reported out loud (`quorra-confined` `frame_landed`). Seven not comparable: the documents' limits
(Brotli: a filter Table 6 does not name); note sharpened.

**Revocation fuzz.** INITED was low because every input was asked about one certificate no input
names, so RFC 5280 6.3.3's walk ended at the issuer comparison. Target now also asks each list's own serials
and a subject the list names (reaching the signature), and each OCSP response's first serial.
Census over 90 671 PDFs: 941 structures (401 CRL, 540 OCSP). INITED on them: 256 → 674 edges; on
the old corpus 437 → 841. First run found a defect in 38 s: a second `SEQUENCE` after the entries
replaced them (listed serial reported not listed). `certificate_list` now enforces RFC 5280 5.1's
member order; regression test with the crasher; 400 of 401 real CRLs parse either way. 10 min after
the fix: 2 461 559 execs, 941 → 1007 edges, 0 artefacts.

**Gates.** clippy (render-raster, raster-gpu, pdf-signature) 0; nextest raster-gpu 621+5, render-raster
93, pdf-signature 232 pass; conformance 0; corpus 1× 0 (1-vs-N 0), 4× 0 (1-vs-N 0), gpu and compute
lanes 1× 0 (1-vs-N 0); headless_gpu 39 pass. raster_golden not run: CPU pixels unchanged.
**Left.** Raster's flattening and residue-clip asks (feedback 59). Proposed habit: a differing page is read
against a per-pixel reference from its own geometry before either lane is called right.
