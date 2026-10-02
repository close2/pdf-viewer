# 1495 — The second campaign fuzzes what the parsers grew, and a band is the whole decoder's or nothing

Session 1330. Status: accepted and **built**.
Context: CLAUDE.md principle 3 ("Fuzzing from the first parser commit. Every crasher found becomes
a permanent regression test"); ISO 32000-2 §7.4.8, §7.5.6, §9.9.1, §9.9.2, §9.10.2, §10.7.4.
Follows ADR 1423's method (one target at a time behind the lock, `-s none`, scratch corpus first).
Code: `fuzz/fuzz_targets/{embed,jpeg_bands,meet,vfs_write,find}.rs`, `fuzz/seed_{embed,meet,
vfs_write,find}.py`, `fuzz/seed_streams.py` (`jpeg_bands`), `fuzz/seed_confined_wire.py`,
`fuzz/seeds.sh`; the seams `pdf_model::image::banded_decodes`, `raster_gpu::intersection`,
`viewer_core::find_in_text`; the fixes in `crates/pdf-font/src/embed.rs`, `embed/cff.rs`,
`crates/pdf-model/src/image/{restart,cut}.rs`. ADR 1496 is the page-tree defect this campaign found.

## 1. What had no target

| target | reads | property beyond "no panic" |
|---|---|---|
| `embed` | a machine face → `pdf_font::embed::for_embedding` with a glyph set (§9.9.1, §9.9.2) | kept glyphs dense from `.notdef`; §9.9.2's tag on every subset and only there; a re-embedded `/FontFile2` subset renumbers nothing |
| `jpeg_bands` | a §7.4.8 frame cut at restart intervals (ADR 1433) and at an entropy pass's rows (ADR 1481) | **differential**: a band plan's bytes are the whole decoder's, and a plan answers only where the whole decoder does |
| `meet` | two or three hostile polygons → the meet's area in one pixel (ADR 1467) | **differential** against an exact piecewise integration in the target, to 1e-6 |
| `vfs_write` | a document and the bytes written into it, RFC 0003's five verbs, in-process worker | every commit begins with the file it replaces (§7.5.6); a deleted page is one page |
| `find` | a page readback and a needle → the find bar's match (ADRs 1465, 1477) | matches are ranges on character boundaries, in order; a literal lowercase needle is found |

**`shaping` reaches `Label`, the joining and the bidirectional halves, and not `fold`, `decompose`
or `mark_class`.** Those three are table lookups over the code points whose fixed point `build.rs`
already asserts; the one caller that hands them a document's text is `viewer_core::select::find`,
which is what `find` fuzzes. Each new seam is public only to be reachable from `fuzz/`, as
`viewer_confined::wire` is, and each has a calibration test that it reaches what it stands for.

## 2. The findings, each with its test

- **`embed`: a face written whole held no glyph the stream shows.** Both whole paths — a `glyf` face
  whose `fsType` forbids subsetting and a `CFF ` one written under `OpenType` — kept every glyph
  of a face of none, and wrote used glyphs past the last one, so a CID reached nothing and no
  `.notdef` existed; the two subset paths already refused both. Now all four apply one rule: every
  shown glyph and glyph 0 below the face's count, or `Refusal::Malformed`.
- **`jpeg_bands`: a damaged restart interval.** A band starting after the damage decoded black where
  the lenient whole decoder, recovering differently, wrote grey. Bands are decoded **strict**: a band
  the decoder could read only by recovering from an error is refused, so the frame is the whole
  decoder's — which the module comment always said and the code did not hold.
- **`jpeg_bands`: a scan the data ends inside.** Section F.2.2.3's decoding (checked by an
  independent Python decoder) finds every block inside the data; the whole decoder stops one MCU row
  short without an `EOI` after it. Both plans now decline a scan with no `EOI`. Over the 263
  `DCTDecode` frames of the pdf.js corpus at 64-line bands this costs 4 restart-interval cuts of 45
  and no row cut of 125, and moves no byte of any frame either plan still cuts.
- **`vfs_write`: a page tree that names its own ancestors** — ADR 1496.
- **`jpeg_bands`, third run: an overflow inside `zune-jpeg`.** Its dequantising multiply of the DC
  prediction panics under overflow checks and wraps in release, beside an addition the crate already
  wraps. The fix is the dependency's: `doc/patches/zune-jpeg-dc-prediction-overflow.patch` holds the
  one line and the 847-byte reproduction, and whether the tree carries it is `doc/questions/Q227`.
- **`embed`'s own property was wrong once**: a `CFF ` face the subsetter cannot close is written
  whole and untagged by design (ADR 1449); the target now asks the tag of what was written.

The artefacts, by libFuzzer's hash: `embed` `f6f3b963191448ce1a11c49459d8d680db8e4b14` (`CFF `,
whole), `c31106f2147e9bfd3cf542fbb9ee8fa3ece105a7` (the property), `fee6a410aaaed0b8810d29171552790338b5a7ea`
(`glyf`, whole); `jpeg_bands` `207ee8eb0562633301fa39372f842c46026166cd` (the damaged interval, now
`tests/restart/damaged_interval.jpg`), `8b17c73c61f08dea83f856422f213213bc815abc` (no `EOI`, inline in
`tests/banded_decodes.rs`), `c65ae29dd4586bd151ffb7511c8c9afb185f8011` (Q227); `vfs_write`
`timeout-615835d9011637c71331b18d32204e5246471b7c` (ADR 1496).

Known and not findings: `jbig2`'s timeouts in `hayro-jbig2`'s symbol dictionary (ADR 1447, the
owner's fork patch); `page`'s 2.75 GiB on `ContentStreamCycleType3insideType3.pdf`, a corpus seed
that spends the whole operator budget (`render-raster`'s corpus gate, `doc/todo/49`), which the
target's own `-rss_limit_mb=4096` admits. The tree's earlier artefacts on disk (`serialize`, `x509`
crashes, three `page` timeouts) no longer reproduce.

## 3. What the stock-take says about the disk

The main checkout's corpora were not regenerated and two are stale as ADR 1423 found them:
`display_list` 105 edges against 957 from fresh seeds, `forms_data` 270 against 1241. Fresh seeds
are worth less than the disk's for the whole-document and stream targets, which were seeded from
the whole crawl. `seed_confined_wire.py` refused to run — three questions (`PRINT_PAGE`, `MEASURE`,
`ATTACHMENT_PREVIEW`) had been added to the protocol without seeds — and asks them now. `ccitt`
ended where its corpus began (223 edges, 141 M executions): its next edges need a new seed shape.

## 4. The runs

INITED edges of the disk's corpus and of fresh seeds, then the run's executions and its last edge
count, per target; 600 s each behind the lock, the last run of a re-run target. Fresh seeds for the
whole-document and stream targets are `doc/pdf.js`'s; `jbig2` and `jpx` were sampled to 3000 of
54 364 and 97 250 codestreams (the full `jbig2` set reached 2480 at `INITED`, its sample 1959).

| target | disk | fresh | executions | edges | left |
|---|---|---|---|---|---|
| `embed` | — | 1243 | 5 780 308 | 2623 | clean (two crashes fixed before) |
| `jpeg_bands` | — | 1633 | 307 606 | 2434 | Q227 (two crashes fixed before) |
| `meet` | — | 526 | 4 953 045 | 659 | clean |
| `vfs_write` | — | 5973 | 612 615 | 8344 | clean (one timeout fixed before) |
| `find` | — | 627 | 3 433 222 | 1278 | clean |
| `shaping` | — | 830 | 2 204 098 | 1283 | clean |
| `xfdf` | — | 1895 | 39 995 790 | 3573 | clean |
| `linearize` | — | 5743 | 301 826 | 7487 | clean |
| `forms_data` | 270 | 1241 | 11 553 444 | 2707 | clean |
| `fragment` | 366 | 272 | 21 899 389 | 438 | clean |
| `display_list` | 105 | 957 | 24 405 770 | 1660 | clean |
| `confined_wire` | 3689 | 390 | 141 857 273 | 5992 | clean |
| `ccitt` | 223 | 122 | 141 177 361 | 223 | clean |
| `cmap` | 368 | 360 | 402 344 | 800 | clean |
| `crypt` | 837 | 1064 | 3 037 930 | 1649 | clean |
| `variable_text` | 6341 | 5352 | 91 003 | 6838 | clean |
| `xmp` | 1584 | 689 | 21 296 014 | 1798 | clean |
| `sfnt` | 383 | 290 | 14 482 125 | 435 | clean |
| `lexer` | 226 | 202 | 73 971 090 | 229 | clean |
| `object` | 462 | 52 | 22 030 475 | 526 | clean |
| `document` | 3983 | 2664 | 693 557 | 4229 | clean |
| `serialize` | 4143 | 3077 | 430 168 | 4362 | clean |
| `page` | 31 553 | 22 608 | 26 525 | 31 784 | known: the Type 3 cycle past 4096 MB |
| `cms` | 352 | — | 109 740 711 | 416 | clean |
| `revocation` | 842 | — | 8 028 643 | 1200 | clean |
| `x509` | 1695 | — | 6 747 666 | 1896 | clean |
| `jbig2` | — | 2480 | 12 396 (fork) | 3618 | known: five symbol-dictionary timeouts |
| `jpx` | — | 2097 | 33 495 | 2775 | clean |
