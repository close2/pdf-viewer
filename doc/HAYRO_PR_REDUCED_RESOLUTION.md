# PR text — hayro-jpeg2000: size the coefficient buffer by the resolution asked for

Branch: `close2/hayro:feat/reduced-resolution-rebased` → `LaurenzV/hayro:main`
One commit, rebased onto `4aaabad7` ("Bump skrifa", #1349). Cherry-picks cleanly; the
reconstruction-midpoint work that used to sit under it is gone, since #1340 landed upstream.

---

## Title

`hayro-jpeg2000: Size the coefficient buffer by the resolution asked for`

## Body

A decode given `DecodeSettings::target_resolution` skips the highest resolution levels: their
bit-planes are not decoded (`decode.rs`) and their decompositions are not synthesised
(`idwt.rs`). `build_decompositions` still reserved a coefficient for every sample of the
*full*-resolution image, because it sized `storage.coefficients` from the component tile's own
rectangle.

So a reduced decode bought time and no memory. On a 12608×16806 four-channel image asked for at
788×1051 — five levels down, 1/256th of the samples — the buffer was still one allocation of
3 390 240 768 bytes.

The sub-bands of resolution levels `0..=r` partition the rectangle of resolution level `r`
(B-15), so the highest level that will be decoded states exactly what the levels below it need;
the levels above it get an empty range. Their code-blocks are still built, because a packet
header is what says how long its body is and a tile-part's packets are read in sequence — in
LRCP order the packets of the skipped levels are interleaved with the ones that are kept, so
they have to be read past rather than ignored. `build_decompositions`'s existing assertion that
the counter meets the buffer length is what checks the partition.

Measured on that image, peak address space (VmPeak), decoding through `Image::decode`:

| target | before | after |
|---|---|---|
| 788×1051 | 3336 MB | 115 MB |
| 1576×2101 | 3424 MB | 241 MB |
| 3152×4202 | 3775 MB | 743 MB |
| 6304×8403 | 5176 MB | 2751 MB |

Resident size does not move — the buffer was `calloc`'d and its pages were never touched — which
is exactly why this had gone unnoticed. What it costs is address space, and that is what an
`RLIMIT_AS` sandbox, a 32-bit target and a `no_std` allocator with a fixed arena all bound: under
a 1 GiB `RLIMIT_AS` none of the four decodes above completed before this change and all four
complete after it.

Nothing changes at full resolution, where the highest kept level *is* the component tile. Output
is unchanged: all 183 assets of the test suite are byte-identical to snapshots taken before the
change, and the Annex B.4 worked example still passes. Re-verified on this rebase, on top of
#1340: 183 of 183 pass and the snapshot directory comes back unmodified, so the allocation change
and the reconstruction change compose without either moving a pixel of the other's output.

---

## Notes for the maintainer, if the PR wants a comment rather than a body

- This is the second of the two changes from the earlier branch. The first is **already merged**
  as #1340, in your own reimplementation — thank you; the `irreversible: bool` / `f32` shape is
  better than the quantisation-style argument we passed, because it puts the lossless case in the
  type rather than in a match.
- The two were independent: #1340 was about *what value* a coefficient reconstructs to, this one
  is about *how many* coefficients are allocated. Nothing here touches reconstruction.
- The witness is a PDF from a real corpus, not a synthetic case: a scanned A0 drawing whose
  JPEG 2000 image is asked for at screen size. A viewer that sandboxes its renderer with
  `RLIMIT_AS` — which is the reason we noticed — cannot open it at all before this change.
