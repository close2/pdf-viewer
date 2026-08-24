# 712 — The chain an image ran four times, and the refusal that is one hex digit too big

`doc/todo/41`'s two remaining lines, taken and priced. One was built (ADR 0585); the other was
measured, found real and reachable, and **declined with its number and a redirect** (ADR 0586).

## What was built

`Document::image_stream` had no memo, on a reason the todo file states in one clause — "a codec's
bytes are not a filter chain's". True about the codec; false about everything Table 5 lets `/Filter`
put in front of it, which for a codec-less image is the whole chain over the samples themselves.

Two changes:

- **`Document::chain_over`** is now the one place a §7.4 chain is run, and `image_stream` goes
  through it. Same memo, same budget, same eviction, same `Arc` pin — no second cache, no second
  constant.
- **`Document::image_codec`** reads Table 5's trailing codec without running anything, so
  `image::contradicted_frame`, `image::short_of_its_grid` and `image::ccitt_bound_below_its_height`
  decline before spending a decode on an image that is not theirs. Each of those three decoded the
  chain and *then* asked what the codec was, which — with `decode_parts` — is **four decodes per
  `Do`** of a codec-less `/FlateDecode` image, and `RasterCache` covers none of the first three
  because it is consulted after them.

New instrument: `crates/pdf-model/examples/image_prefix_census.rs`. Over the pdf.js corpus **2420 of
2997 image `XObject`s run a filter before the codec, producing 467.7 MB per pass**; over ISO 32000-2,
307 of 360, producing 145.7 MB.

## The numbers

Every count is `callgrind` instructions under `RAYON_NUM_THREADS=1`, four binaries built once and
run against each other. The first attempt at this A/B was wall clock and had to be thrown away: the
same binary gave 3.40 s, 4.45 s and 6.90 s while the machine went from load 3.2 to 11.0 under three
neighbouring rounds.

ISO 32000-2, 1023-page cold sweep — 38 011 452 243 → **37 152 970 322, −2.26%**; the reordering
alone is −2.35% and the memo alone −2.06%, because ISO repeats none of its 307 images across pages
and the memo pays only displacement there (306 entries and 4 194 227 bytes held becomes 13 and
2 574 504; 201 evictions becomes 582).

Twenty pages drawing one 512×512 `/FlateDecode` image apiece — the reader's population — 1 863 750 280
→ **735 431 933, −60.5%**, of which the memo is 43 points on top of the reordering.

Single pages: `images.pdf` −5.25%, `issue12963.pdf` −5.31%, ISO p101 ×50 **+0.001%** (the control:
a page with no image).

## What was declined, and the document that proves it is worth someone's round

A refusal whose *encoded* bytes exceed `DECODED_BUDGET` is still re-run per read. The witness is one
hex digit away from ADR 0437's own: hex-wrapping doubles a bomb's encoded size, so a deflate stream
big enough to command the gibibyte `max_stream_len` allows is already within a factor of two of the
four-mebibyte budget, and padding is free. Two documents identical but for that, twenty pages each,
one sitting, load 21–27:

| the same bomb | encoded | one cold sweep |
|---|---|---|
| under the budget | 4 174 537 B | 257–279 µs |
| over the budget | 12 523 517 B | 6.93–6.98 s |

**About 25 000×.** ADR 0586 has the three constructions that would close it and where each breaks,
and the argument that the fix belongs to `doc/todo/14` — a `Pump` that accepts a chain whose every
stage is pumpable spends kilobytes on *every* read instead of remembering that it once spent a
gibibyte. §7.4.2's `ASCIIHexDecode` "produces one byte per two", so it is the easiest stage in §7.4
to window.

### The generator, so that no round rebuilds it

```python
import zlib, binascii
def bomb(gib):                       # zlib over zeros: about 1030:1
    c = zlib.compressobj(9); out = bytearray(); chunk = bytes(1 << 20)
    for _ in range(gib * 1024): out += c.compress(chunk)
    return bytes(out + c.flush())
# object 3: << /Type /XObject /Subtype /Form /BBox [0 0 500 500]
#             /Filter [/ASCIIHexDecode /FlateDecode] >> with binascii.hexlify(bomb(n)) + b">"
# twenty pages, each /Resources << /XObject << /Fx0 3 0 R >> >> and content "q /Fx0 Do Q"
```

`bomb(2)` lands under the budget and `bomb(6)` over it.

## The sequence

Whole, this being a round that can change a pixel. `fmt` clean · `clippy --workspace --all-targets`
under `RUSTFLAGS="-D warnings"`, exit 0 · doctests · the `fuzz/` check · `nextest` **2532 passed, 18
skipped** · both workers built first · corpus gate ok · `pdfref-hayro` built · oracle ok in 107 s on
a machine waited down to load 2.78 · text extraction **98.26%**, 486 of 508 documents in bounds ·
selection and accessibility censuses · dates · XMP · JPEG 2000 · `render-quorra` corpus ·
`fixed_documents` · `cargo test -p conformance`. §5's six binaries and `libviewer_ffi.so` rebuilt
and installed before any measurement. §4's sweeps run: nothing they printed is this round's.

Ledger: §7.4's row gains the eighth answer — where a chain is run, rather than what it produces —
and three test pointers.

## Four tests, one per thing the key claims

`an_images_chain_in_front_of_its_codec_is_decoded_once` ·
`an_image_with_nothing_in_front_of_its_codec_holds_nothing` (an empty prefix is not memoised, or the
budget is charged twice for a decode that never ran) ·
`an_images_prefix_is_not_the_whole_chains_answer` (one buffer, two chains, and answering either from
the other's entry hands back a codec's input where its output was asked for) ·
`an_images_codec_is_read_rather_than_run`.
