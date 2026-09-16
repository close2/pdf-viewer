# 1129 — A sixteenth empty head, and the ten glyph-edge pages are the pair's hinting, none ours

2026-09-16. Files: this record only. No code edit owed; no ADR — no reading changed, the diagnosis
at `CONTRADICTED_GLYPH_EDGES` holds for all ten. Every other `git status` path is a sibling's.

**Oracle head empty a sixteenth round** — `1962 pages in 54.2s`, `agrees 1012, contradicted 46,
ambiguous 835`, the "ambiguous, undiagnosed, and furthest from the nearest reference" line printed
with nothing under it. Sizes unchanged from 1117/1123; contract is `GLYPH_EDGES`' ten.

## The ten split: 10 hinting (robustness, stays), 0 ours — filed nothing to 1124

The question this round settled: is each page contradicted for the references' shared hinting, or
for an edge of *ours*? Measured on this build's oracle artefacts, three ways (principle 5).

**Ink conserved, all ten.** meanR (`-alpha off -channel R`) of ours sits within the four
references' spread on every page — strictly interior for `bug1252420`, `issue3694_reduced`,
`issue7492`, `issue7901`, and ≤0.17 below the poppler/mupdf/hayro cluster with `ghostscript` the
far outlier (`bug1175962` gs +2.10, `bug1200096` gs −2.24) on the rest. Ours is never an outlier.

**Verdict detail, all ten:** convicted by "poppler and mupdf agree, we differ", failing essentially
only the differing fraction (mean ≤ 2.96, worst tile ≤ 12.09, ssim ≥ 0.966). The one exception is
`unencrypted p2`, whose ssim 0.8965 is the §10.7.4 image-box rounding half, not a glyph edge.

**Trap-12 control on the three worst hayro-gap candidates** (`issue7901`, `issue7492`,
`bug1175962` — 200×50, common geometry, differing fraction >4/255): the only tight pair is
poppler-mupdf (5.32/5.55/5.83%, shared `libfreetype`). `ghostscript` (its own FreeType) fails the
pair's bound *worse* than we do — gs-vs-pair 15.19–17.83% against ours-vs-pair 12.02–13.68% — and
`hayro`, the fellow non-hinter, sits with us (ours-hayro 13.57 / 9.46 / **4.09%**, no larger than
hayro's own 12.37–13.54% from the pair). We agree with the non-hinter and differ from the hinting
pair by what a phase shift does; the departure is the pair's. Nothing owed to the rendering owner.

## The 8 + 23 re-confirmed, and the gates

`render-raster --test corpus` (gates, `--ignored`): pass, exit 0 — `958 compared, 943 agree,
8 differ, 7 refused, 16 not comparable`, identical to 1123; the 8 are `issue21068`, `issue2177`,
`issue20232`, `issue269_2`, `issue19083`, `22060_A1_01_Plans`, `pr12564`, `issue15150`. `oracle`:
pass, exit 0 — head empty, 1012/46/835. `raster_golden`: pass, exit 0 — held 974, moved 0 (first
run failed exit 101 on trap 10, a stale `pdf-sandbox-worker` fingerprint from the render-raster
build graph; rebuilt in the pdf-model gate's own context, diagnosed not a defect). No code edit; no
ratchet moved; record within budget.
