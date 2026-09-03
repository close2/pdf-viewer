# 894 — A merge, and a colour key that reaches a filtered image: round 890 on `main`, and §8.9.6.4's ranges are compared against the samples Table 87 says each filter delivers

Date: 2026-09-03.
ADRs: [0832](../adr/0832-a-colour-key-reaches-a-filtered-image-and-table-87-says-what-its-samples-are.md).
0833 was allocated to this round and not needed: one decision, one ADR.
Touched: `crates/pdf-model/src/image.rs` (`colour_key_entry`, `samples_of`'s `DCTDecode` arm,
`jpeg_colour_key`, `decode_ccitt` and `decode_jbig2`),
`crates/pdf-model/tests/dct_components.rs` (three tests and the fixture's `entries` variant),
`crates/pdf-model/examples/colour_key_mask_census.rs` (new),
`doc/checks/fixed-documents.toml` (three rows), `doc/conformance/ledger.toml` (§7.4.8, §8.9.5.1,
§8.9.6, §8.9.6.4), `doc/todo/03-more-corpora.md` §44; and the merge commit before this round's
own.

## The merge

**`round-890` (9e58cf0c) is on `main` as `19292d6e`**, `--no-ff`, on top of round 891 and the
merge of 887. It carries ADR 0825: one question — which annotation subtypes are placed by
`/Rect` — had two answers in this tree and the wrong one ran first, so thirteen markup
annotations with no `/AP`, a `/Rect` written as a point and their own `/QuadPoints`, `/L` or
`/Vertices` whole were drawn nowhere and reported nowhere. `Constructed::bounded` becomes
`appearance::bounded_by_rect`, one list asked by both the `/BBox` clip and the empty-`/Rect`
guard.

**One conflict, in `doc/checks/fixed-documents.toml`, and it is an append conflict rather than a
disagreement**: 890 added its `sumatrapdf-LINK-1618-0.pdf` row at the end of the file while 891
added its two `MAX_TILE_COPIES` witnesses in the same place. Resolved with all three kept, in
session order — 889, 890, then 891's two — which is the order the rest of the file is in.

**The ledger did not conflict although both rounds wrote to it**, because the rows are disjoint:
890's are §12.5.2, §12.5.5 and §12.5.6.10 and 891's are §8.7.3.1 and §11.6.7. Both sets are in
the merged file, checked by grep for each round's ADR number rather than by trusting the
three-way merge.

`tools/worktree.sh close 890` took the checkout and its build directory once `round-890` was an
ancestor of `main`. `r867` and `r892` are neighbours' and untouched.

**One gate line failed on the merged tree and it was the machine, not the merge.**
`cargo nextest run --workspace` exited 100 on `conformance::bounded::the_bounded_wrappers_self_test_holds`,
whose own message is *flat tree: one sample cost 1175 ms, which is not a fraction of the interval
it has to fit* — a sampler stalling under load, in a run that was itself nested inside
`tools/bounded.sh`. nextest fails fast, so 537 of 3075 tests ran and the rest did not. Re-run
alone on a quiet machine the same tree passes: **3075 tests, 22 skipped**, with that test SLOW at
69 s and green. It is the same shape as round 891's 30-second corpus-gate false positive and the
same rule applies: check `uptime` and `ps` before believing a timing failure.

## The finding: a warning is not an exclusion

`doc/todo/03` §44 had left `sumatrapdf-404-0.pdf` named — a bank statement whose 1800 × 600
`DeviceCMYK` banner is a `DCTDecode` stream with `/Mask [0 0 0 0 0 0 0 0]`, answered with
`colour-key /Mask on a DCTDecode image`. The refusal covered all four codecs, and the comment
above it gave two reasons: §8.9.6.4's NOTE 2 about lossy coding, and that "a rule with three
exceptions is worse than a rule".

**Neither is the standard's.** §8.9.6.4's requirement is a `shall` addressed to whoever paints —
"Samples in the image that fall within this range shall not be painted, allowing the existing
background to show through" — and what it says about the lossy pair is that their use "can
produce unexpected results", a warning to a *writer* about a picture, followed by an informative
NOTE. §7.4.8 was read for the same purpose and excludes nothing; its subject is the codestream's
syntax, its parameters and Table 13's `/ColorTransform`.

**And the architectural half of the reason had an answer in the standard too.** The old comment
said the test could only be taken where samples "reach `unpack`". §8.9.5.1's Table 87 says what
each filter delivers instead: "a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit
samples, a RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples". So "before
decoding with the Decode array" is exact for three of the four codecs, and the fourth is refused
on the *same table's* next sentence — for a `JPXDecode` image `/BitsPerComponent` "shall be
ignored if present" and the depth is the processor's, which §8.9.5.2 adds "can have different
values per colour component". §8.9.6.4 bounds every one of its integers by that one entry, so for
JPEG 2000 the domain the ranges live in is undefined. ADR 0832.

## The population, and what moved

`examples/colour_key_mask_census` counts image dictionaries whose `/Mask` is an array, by the
last filter in the chain, through `pdf_syntax` alone (trap 8). Over **90 535 documents —
`doc/pdf.js`'s 974, `doc/corpora`'s 275, `corpus-cache`'s 89 286 — of which 89 322 open**: **660
state a colour key, 5935 images**, by last filter 5794 `FlateDecode`, **68 `DCTDecode` over 17
documents**, 24 unfiltered, 17 `RunLengthDecode`, 17 `LZWDecode`, **13 `CCITTFaxDecode` over 4
documents** and 2 `ASCIIHexDecode`; 67 are overridden by an `/SMask` or `/SMaskInData`. **Not one
is `JPXDecode` or `JBIG2Decode`**, so the arm still refused has no witness anywhere in this tree
and the bilevel arm that opened has four documents.

Twenty-one documents state one over a codestream on page one. **Nineteen reported it and now do
not.** The remaining two, `GHOSTSCRIPT-701468-0.pdf` and `-701474-1.pdf`, write `/Mask [240 255]`
on one-bit images and now report §8.9.6.4's own bound instead: `colour-key /Mask range 240..255
is outside 0..1 at 1 bits per component`.

The ink of each page, by `tests/fixed_documents.rs`'s instrument, before and after:

- **`batch1/PDFBOX/PDFBOX-3631-15.pdf` 7.9702 → 9.1754**, the only one visible at a glance.
  SignRequest's tag-template demonstration draws five `DCTDecode` stamps with
  `/Mask [254 255 254 255 254 255]` over the page's own `[[s|0]]`, `[[d|0]]` and `[[t|0]]`
  placeholders; painted opaque, each is a white rectangle covering the placeholder it is meant to
  sit beside. **`pdftoppm -cropbox` and `mutool draw` both draw the page we now draw** — evidence
  for the reading, not the reason for it. `-16.pdf` and `batch5/DSS/DSS-1356-8.pdf` are
  byte-identical copies (`md5sum`).
- `7926922.pdf` 19.8315 → 19.9751, `4359231.pdf` 13.2256 → 13.2507.
- **`sumatrapdf-404-0.pdf` 17.3522 → 17.3508**, and this is the round's own lesson about what it
  fixed: 5384 of 386 019 pixels change, by at most 2 of 255. Its `/Mask [0 0 0 0 0 0 0 0]` asks
  for exactly-zero CMYK and a lossy encoder left almost none of the white banner at exactly zero.
  **That is NOTE 2's own phenomenon, observed rather than assumed** — and it is why the refusal
  was costing the *report* here rather than the picture.
- Everything else moves by under 0.02 or not at all, including all four `CCITTFaxDecode`
  documents: their ranges cover no sample those images have.

Three rows in `doc/checks/fixed-documents.toml` — `PDFBOX-3631-15.pdf` for the picture (the band
discriminates by 0.705 of a level), `sumatrapdf-404-0.pdf` and `1899883.pdf` for the report,
each saying in its `why` that its ink cannot discriminate and what it pins instead.

## The construction

The bilevel pair cost one field apiece: `decode_ccitt` and `decode_jbig2` build their raster
through `unpack`, which has applied the ranges since ADR 0023, and both were passing
`colour_key: None` under a comment pointing at the refusal.

`DCTDecode` needed the test *placed*. The values §8.9.6.4 ranges over exist only between
`decode_jpeg` and `convert_channels`, so `jpeg_colour_key` takes the answer there as one flag per
pixel and the flags are applied after the conversion. They cannot be applied before it: the
conversion writes the fourth byte of a four-component frame, where `k` lives until then, and
would paint a masked sample back in.

## Gates

The whole `doc/todo/02` §2 sequence ran twice under `tools/bounded.sh` (`--tree 8` for a build,
12 for a walk, one walk at a time): once on the merged `main` before this round's change and once
after it, with `--bin quotations` and `--bin pointers` added because documents moved. **Every one
of the twenty-one lines of the second run exits 0**, and the first run's only non-zero was the
nextest timing failure the merge section records.

From the second run: formatting and `clippy` under `RUSTFLAGS="-D warnings"` silent for the
workspace and for `fuzz/`; **3078 tests passed, 22 skipped**; doctests green; the corpus gate at
**974 documents in 10.8 s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64
incomplete, 0 slow**; the oracle at **1945 pages, 1841 complete, 104 incomplete**, 61 contradicted
and every one held by a group by name; text extraction at **11 094 of 11 131 matched words in
bounds (99.67%), 493 of 503 documents fully in**; the selection census at 1000 of 1011 words
(98.91%) over 453 documents; the accessibility census green over 104 documents with a structure
tree; dates **1514 of 1545 (97.99%)**; XMP **318 of 319** streams read; JPEG 2000 green; quorra at
**958 pages, 929 agree, 22 differ, 7 refused, 16 not comparable**; fixed documents **67 checked, 0
absent, 67 rows**, the three new ones among them; the transform gate at **180.0 pages/s over a
floor of 40**; the writer over 974 documents in 6.9 s; conformance green — **875 subclauses, 0
owing a review, 13 090 citations**; `quotations` and `pointers` green.

**Two lint findings are worth keeping, because both are about the same rule.** `clippy::doc_markdown`
fires on `DCTDecode` and `RunLengthDecode` inside an ordinary `///` sentence and does *not* fire
inside a `> ` blockquote — so a normative sentence quoted inline forces backticks into it, which
would break "quotation marks mean verbatim". The tree's convention is the fix: **quote in a
blockquote.** And the quotation checker attributes a blockquote to the *nearest preceding cited
clause*, so a Table 87 quotation under a §8.9.6.4 heading is reported as §8.9.6.4 not containing
it. Cite §8.9.5.1 in the sentence that introduces the quote.
