# 1093 — The group's own colour, and the pixel that maps back to one sample

2026-09-15. ADR 1107. Files: `pdf-model/src/content/transparency.rs` and its `tests/transparency_groups.rs`,
`pdf-render/src/{display_list,paint}.rs`, `render-cpu/src/{lib,blend,area}.rs`, `render-raster/src/scene.rs`,
`QUORRA_FEEDBACK` §47, four ledger rows. Two siblings were in `transparency.rs` and `render-raster` beside me.

**§11.4.4, and the sentence nobody had taken up.** Three rows stood on "a group the *file* composites under a blend
mode keeps its report", because the removal `C = Cn + (Cn − C0) × (α0/αgn − α0)` divides by a group alpha an opaque
backdrop destroys. NOTE 4 answers it — "backdrop removal can be accomplished by maintaining two sets of variables to
hold the accumulated values" — and §11.4.8 says what the second set costs: its recurrence for shape and alpha reads no
colour, so the same elements run again onto **transparency** accumulate exactly `αgn`. `render-cpu` does that
(`remove_the_backdrop`, `blend::remove_backdrop`) and composites the result once as one object; `pdf-model` stops
forcing §11.4.5's backdrop; quorra and Vello already refuse it by name.

**Census first (trap 8), and it is why the fixture is the gate.** A probe on the interpreter's own condition,
calibrated on a planted fixture (trap 13): **0** of `doc/pdf.js`'s 974 first pages and 0 of `doc/corpora`'s 503 state
it; **440** groups over 89 286 crawled first pages do — 314 `/Multiply`, 29 `/Overlay`, 26 across the four
non-separable modes — plus 7 that also state a knockout condition and keep their report. **The fixture**: opaque 0.5
grey page, one opaque 0.5 grey element under `/Multiply`; non-isolated reads **0.125** under `/Multiply` at the `Do`
and **0.625** under `/Screen`, isolated 0.25 and 0.75, and planted it read the isolated column — 64 of 255 where the
clause gives 32. Control: nothing blending inside, where NOTE 3 makes the two models one picture.

**Movers.** None in the tracked corpus, by construction; `raster_golden` held 974 and moved 0, the oracle reads
1010/46/835 and `render-raster --test corpus` 943/8/7, identical either side. Twelve crawled hits opened at 1.5× (trap
1) and **ten moved**, all one class — a motif group under `/Multiply` or `/Overlay` now multiplies against the page
instead of against transparency; `0100121.pdf` is the picture, damask showing through a navy title band that was flat
(mean 0.34, 4.07% differing, max 12 levels). Cost (`callgrind_rasterise`, 5 rasterisations, one thread): +5.5%,
+13.9%, +21.4% on those three — the second run, paid by the 440 and nothing else.

**§10.7.4's image sentence, from 1097's finding.** `tiny-skia` substitutes nearest for any pure-translation pattern
transform, so the oracle never filtered an image at one device pixel per sample though it asked to. The clause says it should not
ask: the centre "shall be mapped back into source space to determine how to colour the pixel. There shall not be
averaging over the pixel area", ADR 0025's departure is for the *reduced* case whose reason is absent at 1:1, and
§8.9.5.3 is scoped to a resolution "significantly lower than that of the output device". `is_smoothed` answers `false`
at a native placement before it reads `/Interpolate`; request and delivery now agree and no corpus page moved. Quorra
holds a **copy** of the rule (ADR 0702), so §47 is the ask, with `image_phase` and `pr12564.pdf` as its measurement.

**ADR 1082's decline was priced, not narrowed.** 3533 marks on 232 of 974 first pages take it; lifting it reads
`issue20232.pdf` at 23 722.36 of ink at 1× against its own 18 653.77 at 8× — +23%, landing on raster's 23 703.71,
§45's defect rather than a better answer. `area.rs`'s `read_off` carries it and what an exact answer would need.
