# 538 — The region nobody can ask for, and the decode nobody had counted

Date: 2026-08-15. ADR: [0373](../adr/0373-the-region-nobody-can-ask-for.md).

**Finding.** The item asked for a census before any code, and the census refused the code. Region
decoding of a JPEG 2000 image — tiles, precincts and packets, the *zoomed in* half that reduced
resolution does not answer — serves **four codestreams in a 6622-document SafeDocs sample**, none
of them larger than ten megapixels. Of the 125 codestreams in that sample at eight megapixels or
more, **121 state a single tile and none states a precinct partition**; over all 10 485
codestreams the precinct count is **zero**. The witness that provoked the item,
`issue19517.pdf`'s 12608×16806 scan, is one tile, no precincts, LRCP, five levels — exactly what
the item read out of it by hand, now known to be the population's shape rather than one file's.

**And the large images that do exist are not JPEG 2000's.** 1804 rasters of a megapixel or more
are drawn on page one across the SafeDocs sample, 8.54 × 10⁹ samples decoded and never shown at
64× magnification; of the 374 at eight megapixels or more, **five** are JPEG 2000 and the rest are
`DCTDecode`, `CCITTFaxDecode`, `FlateDecode` and `JBIG2Decode` scans of paper, for which
"decode the region" means decode it all and keep less.

**Second finding, and it is larger than the one refused.** The census prints painting operations
beside distinct rasters, which is how `22060_A1_01_Plans.pdf` came up: four ICC-based 2480×2630
photographs, each with a `DeviceGray` `/SMask`, **decoded nine times each** because
`image::decode_parts` runs at every `Do` and only the *mask* has been cached since ADR 0210.
Page one interprets in 3.09 / 3.14 / 3.16 s, of which a probe attributes 3.23 of 3.24 s to
`decode_parts` over 72 calls — about **2.9 seconds of work already done**. Not taken here, because
a round that refuses one construction on a census should not build another without its own:
`doc/todo/47` carries it with the measurement, the three things a sound key needs
(`resources`, `state.fill`, `self.compositing`) and the bound question a mask cache never had.

**A near miss worth recording.** That page first measured **21.3 s**, and it was the SafeDocs
census running beside it. `doc/habits.md`'s *wall-clock benchmarks lie under load* is what a
seven-fold error looks like from the inside, and the number was three edits away from an ADR.
What caught it was a sanity check against a comparable file — a single 8.4-megapixel `DCTDecode`
image interprets in 49 ms, so 36 draws could not be twenty seconds.

**The instrument.** `crates/pdf-model/examples/image_region_census.rs`, which ships. Three
sections: every image dictionary's grid and codec with nothing decoded; page one's images with
the placement out of the display list and the device grid from `pdf_render::Grid::for_placement`
— the renderer's own function, so the census cannot disagree with a backend about what device
resolution means; and ISO/IEC 15444-1 A.5.1's SIZ and A.6.1's COD out of every `JPXDecode`
codestream. The marker reading was cross-checked against `opj_dump` on `S2.pdf` (`tw=2 th=2`,
`csty=0`, `numresolutions=6`) before any of it was believed.

**Invocations.**

```sh
cargo run --profile gates -p pdf-model --example image_region_census -- doc/pdf.js/test/pdfs/*.pdf
cargo run --profile gates -p pdf-model --example image_region_census -- doc/corpora-own/*.pdf doc/corpora/*/*.pdf doc/corpora/*/*/*.pdf …
cargo run --profile gates -p pdf-model --example image_region_census -- corpus-cache/safedocs/*/*/*0.pdf
```

The third is a one-in-ten sample of the 65 944-document cache, 6643 paths, 6622 of which open.

**Code.** `crates/pdf-model/examples/image_region_census.rs` (new).

**Touched.** `doc/conformance/ledger.toml` (§7.4.9 records NOTE 2's location progression as
measured and refused, with the counts; §8.9.5 records how the image population's sizes are
measured), `doc/todo/46-a-region-of-a-huge-image.md` (**deleted** — its census is taken and its
argument is the ADR's), `doc/todo/47-an-image-decoded-once-per-do.md` (new),
`doc/todo/README.md` (the index line), `doc/adr/0373-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints (the `viewer-qt@` lines are gcc's, on a cold build, as `doc/todo/02` says);
`cargo nextest run --workspace` **1975 tests run: 1975 passed, 15 skipped**;
`cargo test --workspace --doc` green; corpus **974 documents, 0 unopenable, 8 locked, 2 encrypted
beyond us, 6 pageless, 64 incomplete, 0 slow**; oracle **1794 pages, 1691 complete, 906 agree, 67
contradicted, 786 ambiguous**; text extraction **10969/11163 words in bounds (98.26%), 486 of 508
documents fully in bounds**; JPEG 2000 **14 identical, 13 differing by at most one level, 3 not
comparable**; quorra **956 pages: 931 agree, 23 differ, 2 refused, 18 not comparable**; dates and
XMP green; `cargo test -p conformance` 112 unit and 5 ledger tests green after the two rows moved.

**Not run, and why.** The quorra coverage lanes and `doc/todo/00`'s step 7 ink sweep: no pixel
moves this round — the only compiled addition is an example, and `git diff` over `crates/` touches
no shipped path. §5's release binaries were not rebuilt for the same reason: nothing a person runs
changed.
