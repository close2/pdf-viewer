# `hayro-jpeg2000`: the irreversible path did not agree with the reference software — **answered**

Written 2026-08-03, in the two-hundredth session of this project, against **`hayro-jpeg2000`
0.4.0**. It is the same kind of document as `doc/QUORRA_FEEDBACK.md`: a finding, the measurement
behind it, and the command that reproduces each line — written for whoever maintains the crate,
not as a complaint.

**Answered in the three-hundred-and-eleventh session, and the hypothesis this document offered
was the cause.** §7 is what came back and what it is worth; everything above it is the report as
it was written, kept because it is the evidence. The short version: 0.4.0 implemented *none* of
ISO/IEC 15444-1 E.1.1.2's reconstruction bias, upstream commit `9cce046b` implements it, and the
worst sample error over the corpus falls from **87 levels to 3**. No published version carries it
yet, so this tree pins the revision.

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

---

## 7. What came back — `9cce046b`, and what it is worth

**The hypothesis in "Which way the samples move" was the cause, and it was not a partial
implementation — there was none.** `hayro-jpeg2000` 0.4.0's `Coefficient::get` returns the
truncated magnitude and nothing adds E.1.1.2's term:

```rust
pub(crate) fn get(&self) -> i32 {
    let mut magnitude = (self.0 & !0x80000000) as i32;
    magnitude *= 1 - 2 * (self.sign() as i32);
    magnitude          // ← and that is all of it
}
```

Upstream `9cce046b`, *hayro-jpeg2000: Apply reconstruction mid-point to truncated coefficients*
(#1284, 2026-07-18), gives each coefficient state five bits for the position of its least
significant decoded magnitude bit, sets that position wherever a coefficient becomes significant
or is refined, and reconstructs:

```rust
let bit_position = state.decoded_bit_position();
if magnitude != 0 && bit_position != 0 {
    let offset = 1 << (bit_position - 1);
    magnitude += if magnitude > 0 { offset } else { -offset };
}
```

which is r = ½ in E-6.

### Measured, by the instrument in §0 with nothing else changed

| codestream | 0.4.0 | `cc9c4024` |
|---|---|---|
| `S2.pdf` object 17 | 298 229 differ, worst by 52 | **53 286, worst by 3** |
| `S2.pdf` object 18 | 318 081, worst by 55 | **47 864, worst by 3** |
| `S2.pdf` object 19 | 299 422, worst by 58 | **34 908, worst by 3** |
| `S2.pdf` object 20 | 318 228, worst by 50 | **36 580, worst by 3** |
| `S2.pdf` object 21 | 320 422, worst by 59 | **8 758, worst by 2** |
| `S2.pdf` object 22 | 312 995, worst by 65 | **35 853, worst by 1** |
| `S2.pdf` object 32 | 104 334, worst by 48 | **56, worst by 1** |
| `S2.pdf` object 33 | 102 139, worst by 87 | **63, worst by 1** |
| `S2.pdf` object 34 | 105 963, worst by 60 | **45, worst by 1** |
| `issue5475.pdf` object 8 | 91 144, worst by 2 | 91 144, worst by 2 |
| `issue5481.pdf` object 5 | 1 076 388, worst by 4 | 1 076 388, worst by 4 |
| `issue5481.pdf` object 43 | 1 076 388, worst by 4 | 1 076 388, worst by 4 |
| `issue5549.pdf` object 11 | 965 165, worst by 5 | 965 165, worst by 5 |

The single-component plates are the clearest: `S2.pdf` object 33 goes from 102 139 samples wrong
by up to 87 levels to **63 wrong by one**. The prediction in "Which way the samples move" — that
the error scales with how coarsely the image was quantised — is what this table is: the coarse
plates improve by more than an order of magnitude and the finely quantised ones do not move.

### What is left, stated as a question rather than a finding

**Not one codestream became byte-identical.** The list of thirteen is the same thirteen; what
changed is the size of the error, not the population. Two things about the residual, and neither
is a conclusion:

- **`issue5475.pdf` and the two `issue5481.pdf` plates did not move at all**, and they were at 2
  to 4 levels before the fix. The bias term cannot explain an error it does not change, so if
  there is a second defect it is visible there and nowhere else. That is where we would look next.
- **1 to 5 levels over 45% of an image's samples is not obviously rounding**, but it is not
  obviously *not* rounding either: both decoders carry the irreversible path in `f32`, and
  `doc/todo/_scan-conversion.md`'s habit applies — a difference that shrinks with precision is
  arithmetic, and one that does not is a defect. Nobody has run that ladder here.
- **One cause is ruled out, and cheaply.** The final `f32` → 8-bit conversion is `f32::round`,
  half away from zero, where `opj_decompress` reaches the integer through `lrintf` under the
  default rounding mode, half to even — an obvious candidate for a residual of ±1 on samples
  landing on a half. Making `round_f32` round half to even moves nothing in the right direction
  and two things in the wrong one: `S2.pdf` object 33 goes 63 → 64 and `issue5549.pdf` object 11
  goes 965 165 → 965 171, with `issue5475.pdf` unchanged at 91 144. **It is not the rounding
  mode.**

### What this tree did

Pinned `hayro-jpeg2000` to `cc9c402486c298f6986620d512372e72754697a1` — the workspace manifest
carries the argument, `deny.toml` names the source as the second and temporary git dependency, and
both say to go back to crates.io the moment a release contains `9cce046b`. `9cce046b` is
2026-07-18 and 0.4.0 was published 2026-06-14, so the wait is a release rather than a fix.

**One more fix comes with the revision, unrelated and worth naming**: `c2df2014`, *Fix LAB color
conversion* (#1313), corrects `let rb = lab.ra.unwrap_or(200)` to `lab.rb` — a JP2 `Lab` colour
specification's `b` range read from its `a` field. No corpus document exercises it, which is why
this tree never saw it.
