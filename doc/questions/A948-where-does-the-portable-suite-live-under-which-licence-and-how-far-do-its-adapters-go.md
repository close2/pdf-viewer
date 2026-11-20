Status: complete
Given: 2026-11-20, in conversation — transcribed by the round
Owes: keep portable-conformance/ in-tree until the owner says otherwise (do not split it to a separate repository automatically); license manifests and generated PDFs under CC0-1.0 and the generator, runner, and adapters under Apache-2.0; add a shipped render-raster page-drawing command and quorra-gpu adapter alongside the existing CPU and evidence adapters (conformance / render rounds)

> Q948: agreed to all except stay in tree until I say otherwise

Reading: adopts recommendations 2 and 3, and amends recommendation 1 so the suite remains in-tree until explicitly moved:
1. **Location:** keep the portable conformance suite in-tree under `portable-conformance/` until the owner explicitly asks to split it into a separate repository (even past ~100 tests).
2. **Licence:** `CC0-1.0` for the manifests and generated PDFs, and `Apache-2.0` for the generator, runner, and adapters.
3. **Adapters:** keep the CPU adapter and the `poppler`, `MuPDF`, and `Ghostscript` evidence adapters, and add a shipped `render-raster` render command plus a `quorra-gpu` adapter to judge the device rendering path; leave a `pdf.js` adapter for later.
