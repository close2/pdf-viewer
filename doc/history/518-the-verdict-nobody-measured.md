# 518 — The verdict nobody measured, and the diagnoses that moved one const up

**Finding.** The ambiguous bucket's every instrument measures *our* page — `Distance::nearest`,
step 5's ink, step 6's ladder, step 7's gap — and `ambiguous` is not a statement about our page at
all. It says *no two voting references agreed*, and nothing had ever printed by how much they
missed. `Examined::consensus_missed_by` and `rank_the_manufactured_ambiguity` are that number and
its ranking: the minimum `outside_by` over `Triangulation::between_references`, which was already
computed and already thrown away. It is trap 9's fifth shape as an instrument instead of a
caution.

**Its head is two JPEG 2000 pages and then the whole of `AMBIGUOUS_SHARED_JBIG2_DECODER`** —
`jp2k-resetprob.pdf` 35.12 (ours 5.03), `issue5475.pdf` 31.63 (ours **0.00**),
`bitmap-refine-tpgron.pdf` 28.91, then seven `bitmap-*-refine` at 28.58. The JBIG2 half is the
instrument reproducing the finding it was built from. The JPEG 2000 half is new and is the round's
second finding arriving as a measurement.

**Second finding: `ldd` was answering a different question, twice.** `Reference::independence`
said `mupdf` and `ghostscript` "share `jbig2dec`, and only that"; trap 9 said all three references
"link the same `libfreetype.so.6`". `objdump -p | grep NEEDED` says neither. All three link the
same **`libjpeg.so.8`** and the same **`libopenjp2.so.7`**; `poppler` and `ghostscript` share
`liblcms2`; and `ghostscript` links no FreeType at all — `libgs.so.10` defines 194 `FT_*` symbols
and leaves none undefined, a statically linked copy configured differently from the system one
(the system library exports `FT_Palette_Select`, Ghostscript's does not). So **on a `DCTDecode` or
`JPXDecode` page the three voting references are one decoder**, and on `issue5475.pdf` those three
span 9.03 to 19.08 of 255 among themselves while ours and `mupdf` sit **0.0002 apart over 262 144
pixels**. Shared code manufacturing the absence of a consensus without the shared code having
failed.

**Third finding: three groups' diagnoses had migrated one `const` up.** A group is an array with
its argument in the doc comment above it, and Rust attaches a comment to whatever item follows —
so an edit inserting a new `const` between a comment and the const it documented welds two notes
together and leaves an array with none. `AMBIGUOUS_GLYPH_COVERAGE` (3 pages),
`AMBIGUOUS_MASKED_BLUR` (1) and `AMBIGUOUS_OURS_ON_THE_LIMIT` (3): seven pages whose argument was
written down, with ladders and clauses, filed above a group it does not describe, while the three
groups said nothing and the bucket counted "0 undiagnosed". All three moved back;
`every_group_of_pages_carries_a_diagnosis_naming_one_of_them` fails the build on the next one, and
was checked by breaking it.

**Fourth finding: `AMBIGUOUS_SUBSTITUTED_FACE` had `bug1671312_ArialNarrow.pdf` backwards.** It
said "we are the only renderer that finds a *narrow* face at all, and the four that do not draw a
better-fitting line" — two halves that cannot both hold. Ours is the wide one, and the picture
says it in one look: our letters collide and the other four have clean gaps. That page is the
witness `doc/todo/21` item 4 said would open the question — in its width half, not its cap-height
half, because its `/CapHeight 922` is also its `/Ascent 922` and Arial's is 716.

**Date.** 2026-08-14.
**ADR.** [0353](../adr/0353-the-verdict-nobody-measured-and-the-diagnoses-that-moved-one-const-up.md).

**The pages diagnosed, and what each turned out to be.** (i) the references disagree for a reason
we can name, (ii) our page is right and the spread is theirs, (iii) our page is wrong and the
spread hid it.

| page or cause | group | verdict | evidence |
|---|---|---|---|
| the whole bucket, 786 pages | — | (i) | closest of the ten renderer pairs is `ours + hayro` on **651**; on the 670 text pages, **612**. Median ours-to-`hayro` 1.92 of 255 against 5.34 for the closest two that vote |
| `issue5475.pdf` p1 | `AMBIGUOUS_IRREVERSIBLE_JPEG_2000` | (i) | 31.63 bounds between the closest voting pair, 0.00 from us; ours vs `mupdf` 0.0002 over 512 × 512, the three voting references 9.03 to 19.08 apart, all three on one `libopenjp2` |
| `jp2k-resetprob.pdf` p1 | `AMBIGUOUS_IMAGE_REDUCTION` | (ii) | 35.12 between the closest pair, the bucket's largest; `tests/jpeg2000.rs` decodes its codestream byte-identically to the reference software's |
| `bitmap-refine-tpgron.pdf` p1 (+18) | `AMBIGUOUS_SHARED_JBIG2_DECODER` | (i) | `mupdf` ink **255.000**, `ghostscript` **0.000**, 255.000 apart; ours byte-identical to `hayro`, which shares `hayro-jbig2` |
| `copy_paste_ligatures.pdf` p1 | `AMBIGUOUS_GLYPH_COVERAGE` | (ii) | two reference ladders converge to 0.017 of 255 (43.174, 43.191) and ours ends 43.126; `ghostscript` descends 61.38 → 43.22 |
| `endchar.pdf` p1 | `AMBIGUOUS_GLYPH_COVERAGE` | (ii) | 15 × 34 raster, one `É`; five renderers' ink within 1.9 of 255 at 1× and 0.45 at 8× while pairwise MAE is 4.5 to 21.5 |
| `issue16316.pdf` p1 | `AMBIGUOUS_GLYPH_COVERAGE` | (ii) | ladders converge to 0.023; at 72 dpi ours is 1.51 under its own limit where `poppler` is 2.78 and `mupdf` 2.75 under theirs |
| `issue4260_reduced.pdf` p1 | `AMBIGUOUS_ZERO_AREA_FILL` | (ii) | ratio ranking's head at 8.27; ink ours 19.79 `hayro` 19.83 against `ghostscript` 6.30, `poppler` 3.52, `mupdf` 2.17 — §10.7.4's "no matter how small the intersection is" |
| `bug1743245.pdf` p1 | `AMBIGUOUS_STROKE_ADJUSTMENT` | (ii) | closest voting pair `mupdf`+`ghostscript` **4.12** where every other pair is 22–28, and what those two share is ignoring §10.7.5 |
| `bug766086.pdf` p1 | `AMBIGUOUS_LINK_BORDER` | (i) | same two at 3.03, agreeing about drawing no link border for two unrelated reasons; ours and `poppler` draw it, inks agreeing to 0.09 of 255 |
| `bug1671312_ArialNarrow.pdf` p1 | `AMBIGUOUS_SUBSTITUTED_FACE` | (iii) | ink box x[10, 149] y[15, 34] against x[10, 147] y[15, 34], so advances honoured; 983 marked pixels against 844/825/812/702; modal stem 14 px against `poppler`'s 12 and the stated `/StemV 66`'s 10.56 |
| `freeculture.pdf` pp. 315, 322, 323, 329, 333 | `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` | (ii) | the only five of the book's 320 pages with ratio above 2.0, median 0.77; five renderers' ink within **0.38 of 255** on page 315, where `poppler` and `mupdf` happen to be 2.33 apart against 10.95 on page 316 |
| `issue16224.pdf` p1 | `AMBIGUOUS_ONE_LADDER` | (i) | inks within 0.64 of 255; `poppler`+`mupdf` 2.82, `ghostscript` 13.75–13.95 from both, ours between the camps |

**Two corrections to written claims, both by re-measurement.** `AMBIGUOUS_IMAGE_REDUCTION` said of
`jp2k-resetprob.pdf` that "three of our four distances are below every distance between two
references"; its own table refutes it — `ours vs ghostscript` 0.0353 is above `mupdf vs poppler`'s
0.0267 — so two are, and a third is below all but one. And §9.8.1's ledger row said no corpus page
states these Table 120 entries for a non-embedded face, which `bug1671312_ArialNarrow.pdf` now
does.

**Gates, verbatim.**

```text
cargo fmt --all --check                                   clean
cargo clippy --workspace --all-targets                    silent
cargo nextest run --workspace                             1879 tests run: 1879 passed (1 slow), 15 skipped
cargo test --workspace --doc                              1 passed, 0 failed
corpus    974 documents in 3.8s: 0 unopenable, 8 locked, 2 encrypted beyond us,
          6 pageless, 61 incomplete, 0 slow
oracle    1794 pages in 27.1s (1694 complete, 100 incomplete)
          agrees 906/863   contradicted 67/66   ambiguous 786/755
          our geometry 1/0   reference geometry 2/2   not comparable 13/8   no render 19/0
text      974 documents in 30.7s: 25 skipped, 58 incomplete and not gated;
          overall 99.3% (24016/24195 words), 22 below 90%
          10969/11163 word boxes in bounds (98.26%), 486 of 508 documents fully in bounds
          PDFBox: doc/corpora/pdfbox is not checked out — skipped, as §2 says it may be
dates / xmp / jpeg2000 / conformance                      ok
quorra    every corpus page agrees with the CPU oracle
```

**No pixel moved, and it is checked rather than asserted.** The oracle's 888 per-page lines are
byte-identical before and after the round's diff. `doc/todo/00` step 7, re-run whole on that file's
own recipe (`magick -alpha off -colorspace Gray`) over all 786: **20 at or past −1, 16 of them
documents this tree calls incomplete**, and on the complete documents `issue16038.pdf` −5.734,
`issue12295.pdf` −2.823, `issue14297.pdf` −1.145, `issue7821.pdf` −1.000, `jpx_smaskindata.pdf`
−0.840, `issue16473.pdf` −0.683 and nothing past −0.536 — **the same six names in the same order,
to the thousandth, as the five-hundred-and-fourteenth's run**, which is the alarm's thirteenth
consecutive hold.

**One instrument check worth its line.** Our panels and `hayro`'s carry an alpha channel and the
three C references' do not, which is step 5's `-alpha off` trap and would manufacture the
two-camps result all by itself. All **4535** panels on disk were tested: not one pixel is less
than fully opaque.

**Not done, and why.** `doc/todo/02` §5's binaries were not rebuilt: this round's whole diff is a
test binary, two doc comments, a ledger row and four documents, and nothing a person runs changed.
The `/StemV` finding is recorded and not taken — §9.8.1 states no `shall`, the change is a
substitution policy rather than a line, and one witness is not a population.
