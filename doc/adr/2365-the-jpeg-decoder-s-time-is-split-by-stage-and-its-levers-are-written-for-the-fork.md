# 2365 — The JPEG decoder's time is split by stage, and its levers are written for the fork

Status: deferred. Session 1765, 2026-10-26, the measure slot of batch one hundred and sixteen. The
decoder is not patched here: `zune-jpeg` is the owner's fork pinned by `rev` (`doc/questions/A227`,
ADRs 1589, 1730), and what this records is the fork's to build or decline.
Context: ITU-T T.81 (the decoder's subject; no ISO 32000-2 clause is read differently); ADR 2348
section 4, which left `issue12841_reduced.pdf`'s interpretation at 87 % decoder; ADRs 1433 and 1513
(the restart-interval bands this tree already cuts); habits 59 and 164; trap 13. Moves no ledger row
and no band.

## 1. Where the time goes

Callgrind over `pdf-model`'s `examples/callgrind_pages -- <n> <file> 1 kept-revisit` on HEAD's
export, a run of two less a run of one, every thread, functions by name; a second build with line
tables (`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`) for the lines inside the entropy decoder.
`issue12841_reduced.pdf` is one 5 280 × 3 792 baseline photograph, 4:2:0, decoded in bands at its
restart intervals; one steady interpretation is **1 094.5 M** instructions:

| stage | functions | M | share | per unit |
|---|---|---|---|---|
| entropy decoding | `BitStream::decode_mcu_block` 581.6, its per-block zeroing (a `memset` call) 26.8 | 608.4 | 55.6 % | about 1 160 a block, 29 a pixel |
| the MCU loop | `decode_mcu_ycbcr_baseline` itself 46.5, the `Result`'s drop glue 7.3 | 53.8 | 4.9 % | |
| inverse DCT | `idct_avx2` 115.9, `idct_avx2_4x4` 38.3, `idct_int_1x1` 2.4 | 156.6 | 14.3 % | 334 and 252 a block |
| colour conversion | `ycbcr_to_rgba_unsafe` 115.1, `color_convert_ycbcr` 21.8, `ycbcr_to_rgba_avx2` 1.4 | 138.3 | 12.6 % | 6.9 a pixel |
| upsampling | `upsample_horizontal_avx2` 28.3, `upsample_vertical_avx2` 9.4 | 37.7 | 3.4 % | 1.9 a pixel |
| the output's copies | `post_process`'s `memcpy` | 17.3 | 1.6 % | |

The rest is this tree's: `ImageSource::is_opaque` reading the 20 M alphas, 21.6 M, and the search
for the scan's end, 10.5 M. **The brief's hypothesis, one stage worth more than half, held — for a
stage its list did not name**: of upsampling, the inverse DCT and colour conversion none is a sixth
of the page, and entropy decoding is 56 % of it. Inside it the line profile puts the cost on the
AC-coefficient loop's fast path (the run, the zig-zag index and the dequantising multiply, about
32 M each) and on the bit buffer's refill and peek (about 18 to 30 M a line); no one line is a
tenth of the stage.

## 2. Three levers inside the fork, priced at their ceilings, none built

The same split on `22060_A1_01_Plans.pdf` (one steady interpretation 2 113.0 M on HEAD, eight
frames, 3.27 M blocks) shows three costs that are each one construction in the fork's source at
`1c7d01b8`, and that the photograph page has less of:

- **The DC-only inverse DCT is scalar.** `idct::scalar::idct_int_1x1` writes one clamped value into
  an 8 × 8 block: 128.8 M on the plan over 1 369 767 blocks, 94 instructions a block, where a
  vector fill of eight rows is about a tenth of that. Ceiling: about 115 M on the plan, 5.4 % of
  its interpretation; 2.4 M on the photograph.
- **A `Result` is dropped out of line once a block.** In `mcu.rs`, `if let Ok(len) = result { len
  } else { … }` leaves `result` to be dropped at the end of its scope, and the drop glue of
  `Result<u16, DecodeErrors>` runs for the `Ok` case too: 45.7 M on the plan over 3 267 574 calls,
  14 instructions a call; 7.3 M on the photograph. A `match` that moves `result` into either arm
  leaves nothing to drop.
- **The coefficient block is zeroed by a library call.** `tmp[..clobber_len].fill(0)` with a
  length chosen at run time (32 or 64) is a `memset` call a block: 36.5 M on the plan over 2.37 M
  calls, 26.8 M on the photograph. Two fills of fixed length, one in each arm, are a few vector
  stores inline; the stores remain, so the ceiling is below the figure.

Together they are at most about 211 M on the plan (10 %) and 36 M on the photograph (3 %). The
photograph's stage worth more than half, the Huffman loop itself, is not priced: a lever there — a
lookup that decodes more than one symbol a probe, say — is a design question for the decoder's
owner rather than a construction to read off a profile.

## 3. Why nothing is patched here

A227 gave the tree a fork for two defects with reproducers, retiring when a release carries them;
a performance patch carried here would make the fork the permanent home of a change upstream has not
seen, which is the condition the answer set against. This tree's own levers on the page are taken:
the bands at restart intervals divide the decode across the pool (ADRs 1433, 1513), and the
conversion after it is ADR 2364's. **Reopened by** the owner taking a lever above into the fork, or
upstream releasing one, at which point the turn gate's two pages are re-measured against this
table.
