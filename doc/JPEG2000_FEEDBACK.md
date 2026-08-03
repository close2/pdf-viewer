# `hayro-jpeg2000`: the irreversible path does not agree with the reference software

Written 2026-08-03, in the two-hundredth session of this project, against **`hayro-jpeg2000`
0.4.0**. It is the same kind of document as `doc/QUORRA_FEEDBACK.md`: a finding, the measurement
behind it, and the command that reproduces each line — written for whoever maintains the crate,
not as a complaint.

This project uses `hayro-jpeg2000` for ISO 32000-2 §7.4.9's `JPXDecode` filter, inside a
seccomp-BPF + Landlock confined worker. It has been in the tree since the seventh session and
nothing here has ever disagreed with it, because nothing here had ever checked it.

## The finding, in one sentence

**Every corpus codestream that uses scalar quantisation decodes to samples OpenJPEG does not
produce; every codestream that does not is byte-identical.**

## How it was measured

`crates/pdf-model/tests/jpeg2000.rs` walks all 974 documents of the pdf.js corpus, pulls out
every `/JPXDecode` stream, decodes it twice — once through `hayro-jpeg2000` and once through
`opj_decompress`, the reference software ISO/IEC 15444-5 publishes for Part 1 — and compares the
samples exactly. Opacity is dropped on both sides; nothing else is normalised.

```sh
cargo test --release -p pdf-model --test jpeg2000 -- --nocapture
```

Thirty codestreams, of which three cannot be compared for reasons about the instrument rather
than about either decoder (two whose native precision is not eight bits, one whose declared size
exceeds this project's own budget).

## The result

**Fourteen identical, thirteen differing.**

```text
S2.pdf object 17: 341x392x3, 298229 of 401016 samples differ, worst by 52
S2.pdf object 18: 345x417x3, 318081 of 431595 samples differ, worst by 55
S2.pdf object 19: 337x392x3, 299422 of 396312 samples differ, worst by 58
S2.pdf object 20: 344x413x3, 318228 of 426216 samples differ, worst by 50
S2.pdf object 21: 340x403x3, 320422 of 411060 samples differ, worst by 59
S2.pdf object 22: 349x396x3, 312995 of 414612 samples differ, worst by 65
S2.pdf object 32: 344x413x1, 104334 of 142072 samples differ, worst by 48
S2.pdf object 33: 340x403x1, 102139 of 137020 samples differ, worst by 87
S2.pdf object 34: 349x396x1, 105963 of 138204 samples differ, worst by 60
issue5475.pdf object 8: 512x512x1, 91144 of 262144 samples differ, worst by 2
issue5481.pdf object 5: 1090x725x3, 1076388 of 2370750 samples differ, worst by 4
issue5481.pdf object 43: 1090x725x3, 1076388 of 2370750 samples differ, worst by 4
issue5549.pdf object 11: 1090x725x3, 965165 of 2370750 samples differ, worst by 5
```

The files are the pdf.js test corpus, `doc/pdf.js/test/pdfs/` in this tree, pinned at v6.1.200.

## The discriminator, which is exact

`opj_dump` on all thirty:

| | `qntsty` | result |
|---|---|---|
| 13 codestreams | **2** — scalar expounded, the irreversible 9/7 path | all differ |
| 14 codestreams | **0** — no quantisation, the reversible 5/3 path | all identical |
| 1 codestream | 2 | identical — see below |

There is exactly one crossing and it is the one that proves the rule. `S2.pdf` object 35 states
`qntsty` 2 and matches; it is a **316-byte, 18×166, single-layer** strip, where a small
reconstruction difference has nothing to round it away from.

**Quality-layer count is not the discriminator**, which is worth stating because it was the first
guess. `issue5475.pdf` object 8 has one layer and differs; `S2.pdf` objects 29 to 31 have five and
six layers and are byte-identical. Neither is the multi-component transform: objects 32 to 34 are
single-component with `mct=0` and differ.

## Which way the samples move

On `S2.pdf` object 17, over the samples that differ:

```text
ours greater than reference    164 664
ours smaller                   133 565
moved toward the image's mean  198 888
moved away                      99 341

standard deviation   reference 0.2499   ours 0.2399
```

**Two out of three differing samples move toward the image's own mean**, and the whole image
loses 4% of its standard deviation. The reconstruction is systematically smaller in magnitude
than the reference's.

That is the signature of the reconstruction-bias term in inverse quantisation: a nonzero
coefficient should be reconstructed at the *middle* of its quantisation interval rather than at
the edge, and ISO/IEC 15444-1 Annex E.1.1.2 states the term. Dropping it costs contrast in
exactly this pattern, and costs it in proportion to how coarsely the image was quantised — which
is why `S2.pdf`'s heavily compressed plates are 48 to 87 levels apart while `issue5481.pdf`'s are
4.

That is a hypothesis about the cause, offered because it is testable; the *finding* is the table
above, which does not depend on it.

## What we do with it meanwhile

Nothing, deliberately. This project's principle is that another implementation is evidence about
our reading of a standard and never the definition of correct — but ISO/IEC 15444-1 defines this
decoding exactly and leaves a decoder no latitude, so two decoders of one codestream produce the
same samples or one of them is wrong, and OpenJPEG's status here is the reference software's.

The thirteen names are held in `DIFFERS_FROM_THE_REFERENCE_SOFTWARE`, which fails the build in
both directions: a codestream leaving the list means an upstream release fixed it, and one
arriving means something regressed. Two corpus pages carry the consequence in the rendering
oracle's own vocabulary (`AMBIGUOUS_IRREVERSIBLE_JPEG_2000`).

## Reproducing one line by hand

```sh
# pull the codestream out of the PDF (any tool that can; poppler's is quickest)
pdfimages -jp2 -f 1 -l 1 doc/pdf.js/test/pdfs/S2.pdf /tmp/s2
opj_dump -i /tmp/s2-000.jp2 | grep -E 'qntsty|numlayers|mct'
opj_decompress -i /tmp/s2-000.jp2 -o /tmp/reference.ppm
```

and decode the same bytes through `hayro-jpeg2000` 0.4.0 with default settings.
