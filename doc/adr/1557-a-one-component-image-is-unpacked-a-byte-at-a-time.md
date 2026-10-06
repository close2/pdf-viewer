# 1557 — A one-component image of at most eight bits is unpacked a byte at a time

Status: accepted. Amends nothing; extends ADR 1433's table route (eight-bit `DeviceGray` and
`DeviceRGB`) to the one-component images it did not reach. Supersedes nothing.
Context: `CLAUDE.md` principle 2 and its rule that an optimisation is justified by a benchmark and
explained by a comment; ISO 32000-2 §8.9.3 (the sample layout), §8.9.5.2 (`/Decode`), §8.9.6.2
(stencil masking), §8.9.6.4 (colour key masking), §11.6.5.2 (pre-blending), §7.3.8.2; ADRs 1433,
1457, 1543 section 5; `doc/todo/42` item 2.
Code: `crates/pdf-model/src/image.rs` (`packed_pixels`, `PackedPixels`, `unpack_packed`,
`fill_packed_rows`, and their call in `unpack`).
Tests: `image::tests::a_stencil_unpacks_a_byte_at_a_time_to_the_per_sample_routes_pixels`,
`a_grey_image_unpacks_a_byte_at_a_time_under_its_decode_array`,
`an_indexed_image_unpacks_a_byte_at_a_time_at_every_depth_to_eight`.

## 1. The cost

`unpack`'s general route calls `sample_rgba` once a pixel: a `match` on the space, a bit
extraction (`raw_sample`), a table lookup and a 4-byte push. For a one-component image that is
the same handful of answers asked millions of times — a one-bit sample takes two values, an
eight-bit one 256. Callgrind on `pdf-model`'s `callgrind_interpret` example, one interpretation of
page one, the sandbox worker beside the binary:

| page | before, M instructions | after | of it, the images' unpack before → after |
|---|---|---|---|
| `bug1815476.pdf` (three 4958-wide one-bit CCITT stencils, one 4-bit image) | 403.2 | 127.9 | `decode_ccitt` 244.3 → 5.4; the 4-bit image's `raw_sample` 11.6 → 0 |
| `issue9940.pdf` (two eight-bit `Indexed` images, 4.6 M pixels) | 543.4 | 80.8 | `unpack` 496.2 → 33.6 |

**A copy of the example without `pdf-sandbox-worker` beside it measures nothing of the first
row**: the worker is searched beside the running executable first, a copy aside has none, and the
CCITT stencils are then refused and not drawn — the same page read 157 M with no `decode_ccitt` in
it at all. Both arms here carry the worker.

## 2. The route

`packed_pixels` admits an image whose pixel is a function of its own sample alone: one component,
1, 2, 4 or 8 bits, no §8.9.6.4 colour key (its test is on the raw sample) and no §11.6.5.2 matte
(its inversion reads the pixel's mask value). It builds one answer per sample value **by calling
`sample_rgba` itself** on a one-byte row holding that value in its high-order bits — so the route
cannot read a colour, a `/Decode` map, a stencil's fill or a palette differently from the
per-sample route; it can only lay the same answers out faster. The answers become 256 runs of
`8 ÷ bits` pixels, one per value of a byte (8 KiB below eight bits, 1 KiB at eight).

`unpack_packed` copies one run per carried byte, with the run's size a constant
(`fill_packed_rows::<RUN>`): a runtime-sized `copy_from_slice` was a `memcpy` call per pixel at
eight bits, 72 M of `issue9940.pdf`'s 162 M before the specialisation. §8.9.3 is what makes a byte
a unit: "each row of sample data shall begin on a byte boundary", "A PDF processor shall ignore
these padding bits", and Table 87's depths divide a byte — so the last byte of a row is cut to
the pixels the row has, and a short stream's missing pixels stay `[0, 0, 0, 0]` as §7.3.8.2's
reading requires (the per-sample route's own rule).

Admitted by the same condition and therefore on the route: §8.9.6.2 stencils (CCITT, JBIG2 and
raw), `DeviceGray` at 1, 2 and 4 bits (eight goes by ADR 1433's route first), and every
one-component space `palette` tabulates — `Indexed`, `Separation`, `CalGray`, one-component
`ICCBased` — to eight bits.

## 3. What was counted and not built

Image dictionaries outside object streams in `doc/pdf.js/test/pdfs/` (a regular-expression
census, so a floor rather than a total): no 16-bit image, no 2-bit image, three 4-bit images, no
multi-component image below eight bits; 1-bit `DeviceGray` 210 images in 111 files and stencils
200 in 16 — the populations this route takes. **Sixteen bits stays on the per-sample route**: no
counted image reaches it, and a one-component table there is 65 536 answers. Multi-component
sub-byte images stay there for the same reason. A colour-keyed or matte'd one-component image
stays there because its pixel is not a function of its sample.

## 4. Exactness

The three unit tests compare the route against `sample_rgba` pixel for pixel at every width from
one sample to past two bytes, a stream stopping partway into its last row, three `/Decode` arrays
per space and depth, an `Indexed` table shorter than its depth can index, and a translucent
stencil colour; a mutation reading the wrong shift fails two of them. The corpus gates behind the
lock (`raster_golden`, `render-raster --test corpus` at 1×) are this route's on every corpus image;
the record names their results and the tree they ran on.
