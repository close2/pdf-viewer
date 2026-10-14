# 1538 — The image module is a module tree, and an `ICCBased` image already converts a sample once

Slot 5 of batch eighty-five, 2026-10-14, a parsers round. ADRs 1912, 1913; no question. No ledger
row moves; two rows' test paths follow the tests (§7.4.6, §7.4.8).

**(a) Built.** `pdf-model/src/image.rs`, 8 949 lines, is a root of 342 and fourteen modules —
`dispatch`, `space`, `unpack`, `convert`, `dct` (with `restart` and `cut` under it), `jbig2`,
`ccitt`, `jpx`, `mask`, `combine`, `soft_mask`, `device_scale`, `cache`, `tests` — moved by a script
that cuts at item boundaries; no statement changed, only visibility, imports and paths (ADR 1912).
Every `pub` item is re-exported where it was; a trial compile reopened 144 items, fields and methods
as `pub(super)`. **The proof**: the split alone, exported onto HEAD, held `raster_golden` (974 held,
0 moved) and all six arms, by page against `/home/AI/arms-1534/`: 0 digests and 0 mean errors differ
on 968 + 968 + 968 + 965 + 965 + 965 pages.

**(b) The hypothesis did not hold** (ADR 1913). WTPDF p1's 350 × 350 photograph has 44 328 distinct
samples in 122 500 pixels, and ADR 1469's memo already converts 50 682 times; an exact cache can
save at most 6.3 M of 98.8 M instructions. The rest is the conversion itself, 985 instructions a
sample in `pdf-colour`. Nothing was built. **The brief's premise about the gate did not hold
either**: `open_kinstructions` counts the open, not page 1's draw; it read 12 754.108 thousand.

**(c) Diagnosed: mupdf 1.28.5's change, on evidence from both versions.** 1.28.4 was unpacked from
the pacman cache and run with the same zlib. `multiple-filters-length-zero.pdf` states `/Length 0`
over 17 082 whole bytes; 1.28.4 decodes all 3 300 rows, while 1.28.5 reports "premature end of data
in flate filter" and fills rows 2960–3299 with grey. `issue1045.pdf`'s font stream has `/Length 56 0
R`, and object 56 is `20184endobj`. `attach` carries that unchanged; 1.28.5 reads it short and
substitutes the font (11 pixels, at most 3 of 255), where 1.28.4 matches. Ours, poppler, ghostscript
and hayro agree. The page joins `AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG`, and both documents
go into `foreign_corpus`'s `HELD` (split, merge and optimize carry the first).

**Files.** `crates/pdf-model/src/image.rs` and the fourteen modules named in (a), with `ahead.rs`'s
imports and `image/dct/{cut,restart}.rs` moved from `image/`; `crates/pdf-model/tests/oracle.rs`,
`examples/colour_transform_census.rs` (a path); `crates/pdf-transform/tests/foreign_corpus.rs`;
`doc/conformance/ledger.toml` (two test paths); `doc/crate-map.md` (one sentence); ADRs 1912, 1913;
this record.

**Gates.** `rustfmt --check` on every file: 0. Clippy `-D warnings`, `pdf-model --all-targets` and
`pdf-transform --test foreign_corpus`: 0. `cargo nextest run -p pdf-model`: 0, 2 048 passed, 21
skipped. On the export, in one hold: `t2-raster_golden` 0, `t2-corpus` 0, `t2-jpeg2000` 0, six arms
0, `t3-oracle` 0, `t3-foreign_corpus` 0, `launch_path` 0 (31 banded, 0 outside). `cargo test -p
conformance`: 101 on siblings' files alone (`viewer-qt`'s test paths, record 1539). 9318 s.
