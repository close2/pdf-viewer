# 1329 — A zoom step fills what its window samples

Performance slot of batch fifty-one. ADR 1493 (1494 unused). No ledger row moved; no question.

**What a step's transfer was.** A probe in `ensure_paint_textures` timed each image on
`frame_budget`'s 2× step. The photograph at 2× is unreduced, so the step uploaded all 80 MB of its
samples (12.6 ms) for a window showing a fifth. Every other image at a new factor was reduced
again, whole, from its full samples, then premultiplied on one thread (3.7 ms on `issue13931.pdf`).

**Kept, byte for byte (ADR 1493).** (a) A texture stays the grid's full size, but only the
256-texel squares an image op can read are written. A reduction is made for those cells alone
(`area_averaged_cells`). (e) Each reduced row is premultiplied on the thread that made it.
(f) A translucent band is read off premultiplied column sums (`translucent_row`). A released image
now drops its reductions. Interleaved one-binary arms, step budget, all off → all three:
photograph 14.71 → 3.79, Plans 27.55 → 17.36, `issue13931.pdf` 9.29 → 7.09, `images.pdf`
14.28 → 11.68. Each lever pays where its subject is.

**Declined, with reasons in the ADR.** (b) Deriving a reduction from the last scale is inexact (a
mean of rounded means). (c) A mip chain, and (d) sampling the full texture at reduced scale: the
step frame stays on screen when the sharp pass is off or declined, and no gate compares a step.
A plain-arithmetic premultiply moved nothing and was not kept.

**Before → after**, one sitting, load 3.0–3.4, step budget: photograph 13.72 → 3.24 (80 → 6.5 MB),
Plans 24.44 → 15.31, `issue13931.pdf` 9.14 → 6.17, `images.pdf` 11.52 → 9.95. The photograph's
turn went 46.28 → 31.88. `doc/performance.md` rows, `doc/todo/36`'s row and
`doc/QUORRA_FEEDBACK.md` section 64 carry these numbers.

**Gates.** rustfmt --check on my six files exit 0. clippy -D warnings on raster-gpu: my files are
clean; it fails, exit 101, only on the sibling-modified `tests/an_image_meets_its_residue_as_a_set.rs`.
clippy on render-raster and pdf-model exit 0. nextest raster-gpu 668 passed, exit 0. nextest
render-raster and pdf-model 1848 of 1849; the one failure is the sibling's `scratch_r1330`.
`cargo test -p conformance` exit 0. `raster_golden` exit 0: held 974, moved 0. `render-raster
--test corpus` at 1× exit 0: 966 / 0 / 1 / 7, 1-vs-N 0, peak 5.10 GiB. `render-gpu --test
headless_gpu` 39 passed, exit 0. `viewer-ui --test launch_path` exit 0.

**Left.** On `issue13931.pdf` and `images.pdf` the 2× window shows each image almost whole, so the
step still pays a reduction at the new factor. That reduction costs about 17 instructions per
sample and stays scalar on the baseline target.
