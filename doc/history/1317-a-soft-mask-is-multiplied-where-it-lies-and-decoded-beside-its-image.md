# 1317 — A soft mask is multiplied where it lies and decoded beside its image

Performance slot of batch forty-nine. ADR 1469. No ledger row moved; no question written.

**Premise, checked.** `22060_A1_01_Plans.pdf` is four 2 480 × 2 630 `ICCBased` `DCTDecode`
photographs under four `DCTDecode` `DeviceGray` soft masks (plus six small images); the 72 are
placements through tiling cells. Callgrind 4 262 M: `convert_three` 2 143 M, `zune-jpeg` ~1 500 M,
`apply_soft_mask` 548 M. The ICC memo already hit 99.97% (one profile, 256 colours a photograph,
8 136 conversions); the cost was the hit itself, 82 instructions a pixel. The "two divisions" were
gone since ADR 1433. Conversion and combination were already inside ADR 1321's pool task; what was
serial was within it — conversion, then the mask's decode, then a zeroed, copied combination.
`issue13931.pdf` decoded its `/Matte`'d mask twice.

**Levers, each alone (pinned, one binary, scratch switches since removed).** Plans 94.4 → conversion
76.0, in place 82.1, beside 80.0, all 58.4; `issue13931` 66.8 → in place 53.1, beside 55.8, once
54.2, all 41.3; `images.pdf` 51.3 → 38.6. All four kept. Same sitting at load 2.2–2.5, finished
tree against the tree before: Plans 86.64 (71.26) → 52.91 (36.97), `issue13931` 60.94 → 38.76,
`images.pdf` 47.84 → 35.07, the photograph 39.32 → 39.96 (untouched). Callgrind: Plans 4 288 →
2 181 M, `issue13931` 1 145 → 838 M.

**Pool width** 4/8/16: pinned 63.5/59.4/61.0, unpinned 69.9/58.0/56.2 — not the limit; unchanged.
A 2^24-entry exact table was not built: the memo already hit.

**Documents.** `doc/performance.md`'s four image rows (an `issue13931.pdf` row added), the
paragraph under them, "three of the nine"; `doc/todo/36`'s photograph row. `doc/todo/46` and `47`
say nothing about this stage and were left.

**Gates.** rustfmt 0; clippy `pdf-model`, `pdf-colour`, `pdf-render` 0; nextest `pdf-model` 1729
passed, `pdf-colour`+`pdf-render` 371; `conformance` 0; `raster_golden` held 974, moved 0; `pdf-model
--test corpus` 0; `launch_path` 0; `render-raster --test corpus` at 1× 101: 965 agree, 0 differ,
`issue1905.pdf` refused by the device's scene-byte budget — its display list is held by
`raster_golden`, so the change is downstream of this round, in `render-raster`/`raster-gpu`.

**Left.** Plans' `interp` is eight no-`DRI` JPEG frames' Huffman on eight busy cores, and
`convert_three`'s 455 M; `issue13931`'s is the matte table and the mask widened to four bytes a pixel.
