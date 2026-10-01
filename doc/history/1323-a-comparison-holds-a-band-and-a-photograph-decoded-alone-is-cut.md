# 1323 — A comparison holds a band, and a photograph decoded alone is cut

Performance slot of batch fifty. ADR 1481. No ledger row moved; no question written.

**Premise, checked.** The tiled frame already streams: one tile is drawn, read back, copied and
dropped at a time, so `issue19517.pdf` costs raster 0.99 GB over the device (the 847 MB page and one
tile). The 7.9 GiB was `raster_compare`'s structural-similarity map, nine `f32` planes of the page
(7.45 GB). The no-`DRI` cut ADR 1457 measured was never merged, so it was rebuilt.

**Built.** The map a band of 256 rows at a time, bit-identical (`ssim::Bands`); the 1× corpus walk
11.31 → 5.15 GiB, 68.3 → 62.4 s. A soft mask held one byte a sample (`MaskPlane`), written straight
from raw grey samples or a `Luma` frame. A `/Matte`'s `DCTDecode` frame decoded beside its mask and
inverted in place. `InFlight`, the signal: no table of decodes ahead, not a pool thread, a pool of
two or more. `image/cut.rs`: a Huffman-only pass, bands with their first DC re-coded; a scratch
census cut 104 corpus frames, all 104 identical. Band decoders write into a buffer per thread.

**Measured** (pinned, three runs of five rounds, against the tree before in one sitting, load
1.5–2.0): `issue13931.pdf` 37.87 → 22.60 ms, `images.pdf` 34.90 → 31.09, Plans 51.38 → 47.77,
`carp2010-2.pdf` p5 19.20 → 17.85, the photograph 37.66 → 37.03. The cut alone: −0.9 and −1.4 ms
on the two single-photo pages, nothing elsewhere. Callgrind found the cut taken at one thread
(Plans 2 147 → 2 605 M), hence the signal's third condition.

**Touched outside the file list.** `crates/raster-compare/src/{lib,ssim}.rs`: no contract owns it,
and the memory was there.

**Gates.** rustfmt 0; clippy `pdf-model`, `raster-compare`, `render-raster` 0; nextest 1843 passed;
`conformance` 0; `raster_golden` held 974, moved 0; `pdf-model --test corpus` 0; `render-raster
--test corpus` at 1× 0: 966 agree, 0 differ, 1 refused (the Type 3 cycle), 1-vs-N 0;
`headless_gpu` 39 passed; `launch_path` 0.

**Left.** `issue13931.pdf`'s inversion (2.7 ms) and multiplication (2.4 ms) could be one pass;
Plans' frames' Huffman on busy cores; the walk's peak is now three rasters of one page.
