# The same image decoded once per `Do`

Status: **open**, measured in the five-hundred-and-thirty-eighth session by
`examples/image_region_census`, and recorded rather than taken in the round that found it
(ADR 0373).
Priority: 47 — performance, measured on the witness below and priced.
Corpus: `22060_A1_01_Plans.pdf` (pdf.js), and the shape is a population's — see the census
Clauses: §8.9.5 (image dictionaries), §11.6.5.2 (the soft mask, whose cache is the precedent)
Code: `crates/pdf-model/src/content/image.rs` (the call site),
`crates/pdf-model/src/image.rs` (`decode_parts`, `MaskCache`)

## The measurement

`22060_A1_01_Plans.pdf` is one page with fourteen image `XObject`s, four of them 2480×2630
`DCTDecode` photographs in an ICC-based space, each with a `DeviceGray` `/SMask` on its own grid.
Page one draws rasters of that grid **36 times**.

```sh
cargo run --profile gates -p pdf-model --example open_one -- doc/pdf.js/test/pdfs/22060_A1_01_Plans.pdf 1.0
```

**3.09 / 3.16 / 3.14 s to interpret page one**, three runs, machine otherwise idle. A temporary
`Instant` around the `decode_parts` call attributed **3.23 of 3.24 s** to it over **72 calls**,
of which the four large images are nine calls each at 0.73 to 0.88 s per image. Eight of every
nine reproduce a raster the interpreter has already produced: **about 2.9 of the 3.1 seconds is
work already done**.

The general shape is in the census's own output — `image_region_census` prints the painting
operations beside the distinct rasters, and page one of the pdf.js corpus alone is 4685
operations over 534 rasters.

## Why there is no cache yet, and what one has to key on

A soft mask has been memoised since ADR 0210 (`image::MaskCache`, keyed by the mask object's
number) and the base image has not. The reason the base is harder is the reason
`shading::Cache`'s key needed the colour space in it: `decode_parts` takes more than the stream.

```rust
decode_parts(document, stream, resources, state.fill, self.compositing, &mut self.image_masks)
```

- **`resources`** — a name like `/CS0` resolves through the resource dictionary in force, so two
  `Do`s of one stream under two resource dictionaries are two pictures. A key that ignores this
  is the exact defect ADR 0210 avoided by keying a *mask* on its own object alone, which is sound
  there because every input of a mask is the mask object's.
- **`state.fill`** — §8.9.6.2's stencil "designates places … that should either be marked with the
  current colour or masked out", so the same stencil under two fill colours is two rasters.
- **`self.compositing`** — §11.4.7's four-component page is interpreted twice, once per half
  (ADR 0262), and the halves must not share an entry.

So the key is the object's number *and* those three, or the cache is wrong in a way no gate would
catch on a corpus where they rarely vary. A `/Do` on a stream with no object number of its own
must miss, as it does for masks.

## What to settle before writing it

1. **Bound it.** A page of forty distinct 8-megapixel photographs must not hold forty RGBA
   rasters — 1.3 GB — because one of them is drawn twice. `MaskCache` has no bound because a
   mask's packed bytes are small; a decoded base is not. Least-recently-used by sample count, or
   a cache of the *last* raster only, which serves the witness's nine-in-a-row exactly and costs
   one entry.
2. **Prove nothing moved.** `examples/display_list_digest` over the corpus on both arms: a cache
   that changes one command is a cache with the wrong key.
3. **Measure the population, not the witness.** The census names the documents whose page one
   runs many operations over few rasters; take the three largest and quote all of them.
