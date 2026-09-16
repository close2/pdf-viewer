# 1141 — An eighteenth empty head, and the ambiguous bucket holds no defect of ours

2026-09-16. Files: this record only. No code edit owed; no ADR — no reading was decided, a sample
was classified. Every `git status` path outside this file is a sibling's.

**Oracle head empty an eighteenth round** — `1962 pages in 60.2s`, `agrees 1012, contradicted 46,
ambiguous 835` (807 complete, all diagnosed), the "ambiguous, undiagnosed, and furthest from the
nearest reference" line printed with nothing under it. So the contract was the ambiguous bucket
itself, the oracle's largest and least-examined: of the 807, which are legitimate reference
disagreement, which hide a defect of ours, which are a threshold artefact.

## The bucket, sampled 20 and opened (trap 1)
The oracle already computes the hidden-defect signal: **"ambiguous, outside a bound against every
reference, and further from all of them than the closest two are from each other" — 26 of 807 in
three measures.** I opened the eight furthest side-by-side, plus twelve more stratified across the
diagnostic groups (DENSE_TEXT, GLYPH_SCAN, JP2K, function shading, tiling, standard-14, the three
threshold groups, a reference-drew-nothing page).

**Distribution: 17 legitimate reference disagreement, 3 threshold artefacts, 0 hidden defects.**
- **Legitimate (a).** Every "far from all" page in fact matches ≥1 reference or the majority:
  `bug766086`/`bug1552113` draw the link border / widget box `poppler` also draws and three omit;
  `function_based_shading` draws the checkerboard `mupdf`+`hayro` do where `poppler` fills black and
  `ghostscript` grey; `issue6006` matches all but a `ghostscript` that drew white; `issue7229` all
  but a `ghostscript` that substituted the face. The rest (`freeculture`, `endchar`, `issue16224`,
  JP2K, JPX, ZapfDingbats) are §10.7.4 / §9.5 NOTE 5 glyph edges and image reduction the standard
  declines to specify — ours within the reference spread.
- **Threshold (c).** `images_1bit_grayscale`, `issue4246`, `coons-allflags-withfunction`: content
  identical to every reference, the page just past one numeric bound; their groups name it.
- **Hidden defect (b): none.** The "26 far from all" is a ratio artefact — its divisor is reference
  agreement, so a tiny denominator inflates it (ADR 0663), and on 8 of 26 the halves are different
  measures (ADR 0688); the empty undiagnosed head says the same.

## The 8 + 23 re-confirmed, robustness, gates (under the lock, one at a time)
`render-raster --test corpus` exit 0: `958 compared, 943 agree, 8 differ, 7 refused, 16 not
comparable`, identical to 1135/1129 — 8+23 unchanged. `oracle` exit 0: 1012 agree / 46 contradicted
/ 835 ambiguous (807 complete, all diagnosed) of 1962, plus 3 our-geometry, 2 reference-geometry, 46
not-comparable, 18 no-render, head empty. `raster_golden` exit 0 (held 974, moved 0).
`text_extraction` exit 0: overall 99.3% (24608/24787), verdict 99.69% (11096/11131), 494/503 docs,
ratchet 8266/8266. `fmt --all --check` clean of mine; `conformance` records over budget: none. No
code edit; no ratchet moved.
