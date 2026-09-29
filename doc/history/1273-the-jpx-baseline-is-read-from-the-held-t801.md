# 1273 — The JPX baseline is read from the held T.801

Batch forty-two, the owner's A169. Row §7.4.9 stays `partial`, its reason re-stated; ADR 1383.

## The text
`doc/T-REC-T.801-200208.pdf` prepared as `doc/md/T.801.md` (334 pages, content order) and entered
in `doc/third-party-data.md`: ITU's free route, SHA-256, notice paraphrased. The title pages do
not say Amd. 1 and Cor. 1–2 are integrated (Q169 did); ITU's catalogue does, and lists later
corrigenda, amendments and the 2021 and 2023 editions. Revisit: the 2004 ISO edition (A169).

## Built
- **Reader** (`image.rs::jpx_colour_choice`): §7.4.9's choice made here and handed to the codec
  as Free boxes (M.11.20). A stated `/ColorSpace` sets every `colr` aside (the codec had applied
  its sYCC/Lab conversions under one, and refused CIE Jab under `/DeviceRGB`); otherwise
  precedence, approximation, file order; Any ICC read; none drawn → the clause's device
  fallback, never a refusal. CIE Lab was one flat blue for every pixel through an abstract
  profile; it is now §8.6.5.4's `Lab` (and the redaction writer's `JpxSpace::Lab`).
  `tests/jpx_enumerated_spaces.rs`, 11 tests: 12/16/17/18/21/14 drawn as defined; 19, 20, 24,
  YCbCr and non-D50 Lab take the fallback; precedence and next-lower pinned.
- **Header reader** (`jpeg2000.rs`): `ftyp`, box order, fragment lists, first layer's `cgrp`
  and `creg`, EP and Any ICC profiles, `Rsiz`'s Table A.2 bits, `MCC`/`MCO` of every header.
- **Validator**: `graphics/jpeg2000-uses-the-baseline-feature-set` `Unchecked` → `Implemented`,
  M.9.2.1–M.9.2.7 each named; `tests/jpx_baseline.rs`, 13 fixtures (opj_compress's JP2 shape,
  hand-patched `colr`). The device-colour row's stale "not bought" reason re-stated as not built.
- **Converter**: refused by name (`JPEG2000_BASELINE_NOT_REACHED`); mitigations 4.5 entry.

## Census (crawl, 1769 files naming JPXDecode, 104k images)
With `/ColorSpace`: 4 Lab and 1 sYCC images change reading; Lab's EP there already matches PDF's
`Lab`. Without: 2 sYCC (unchanged). Vendor-only `colr` 40 images, bare codestreams 493.

## Gates
rustfmt on my files 0; clippy `-D warnings` pdf-model/pdf-archive/pdf-transform 0; `cargo test`
pdf-model 0, pdf-archive 0, pdf-transform 0; `cargo test -p conformance` 0. Behind the lock:
`pdf-model --test jpeg2000` 0 (14 identical, 13 held), `pdf-archive --test corpus` 0 (6.2.8.3
7 agreed, over 0), `cross_check` 0, `pdf-transform --test archive_corpus` 0, `pdf-model --test
corpus` 0.

## Left
e-sRGB/e-sYCC (PIMA 7667), CIE Jab (CIE 131), non-D50 Lab; M.9.2.6's before-the-codestream order
and M.9.2.7's second sentence; the FlateDecode transcode; the device-colour-in-the-codestream row.
