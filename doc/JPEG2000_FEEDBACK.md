# `hayro-jpeg2000`: the irreversible path did not agree with the reference software — **answered**

Written 2026-08-03, in the two-hundredth session of this project, against **`hayro-jpeg2000`
0.4.0**. It is the same kind of document as `doc/QUORRA_FEEDBACK.md`: a finding, the measurement
behind it, and the command that reproduces each line — written for whoever maintains the crate,
not as a complaint.

**Answered in the three-hundred-and-eleventh session, and the hypothesis this document offered
was the cause.** §7 is what came back and what it is worth; §8 is the rest of the same defect,
found here by bisecting the pipeline and offered back as a pull request. Everything above them is
the report as it was written, kept because it is the evidence. The short version: 0.4.0 implemented *none* of
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
  there is a second defect it is visible there and nowhere else. **That is where we looked, and
  there was one — §8.**
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

---

## 8. The reconstruction midpoint is skipped for fully decoded coefficients — **found here, fix offered**

§7 left one question: `issue5475.pdf` and the two `issue5481.pdf` plates did not move at all when
the bias landed. They were the place to look, and they had the rest of the defect.

### How it was found

**Bisect the pipeline by resolution.** `issue5475.pdf` object 8 states `numresolutions=2`, so
`opj_decompress -r 1` and `target_resolution: Some((w/2, h/2))` both stop at the LL sub-band with
**no 9/7 synthesis performed at all** — and the two still disagreed on 16 799 of 65 536 samples.
That puts the divergence before the wavelet, in dequantisation.

**Then characterise it.** The difference was symmetric — 8 348 samples one lower, 8 451 one higher
— so it was not the systematic contrast loss of §7. Dumping the pre-rounding `f32` alongside
`opj_decompress`'s output showed the samples that differ do so **only where our fractional part
lies in (0.25, 0.75)**:

```text
we are higher: fractions 0.505 … 0.754
we are lower : fractions 0.246 … 0.495
```

which is the exact signature of two floats a **quarter of a level** apart, and of nothing else.
Two candidates were ruled out before that measurement — the final rounding mode (§7) and FMA,
which `math::mul_add` already `cfg`s off on a target without it.

### The cause

E-6 reconstructs a nonzero coefficient at `r · 2^(Mb − Nb)` above its decoded magnitude.
`Mb − Nb` is the count of magnitude bits never coded, so it is **zero once a coefficient is fully
decoded** — and `2^0 = 1`, which makes the term `r` itself rather than nothing. The quantisation
interval of width Δ still surrounds the value.

`9cce046b` applies the term only `if bit_position != 0`, which is precisely the fully decoded
case. That is invisible on coarsely quantised images, where most coefficients are truncated and
the term §7 restored dominates; it is the *whole* error on finely quantised ones, where almost
every coefficient is complete. Which is exactly the population §7 could not explain.

### The fix, and one thing it must not do

Applying `r` unconditionally makes the irreversible codestreams agree and **breaks the reversible
ones**: `S2.pdf` objects 29 to 31 go from byte-identical to 19 131 samples wrong by up to 5. With
no quantisation there is no interval — a fully decoded coefficient is *exact*, and offsetting it
by half moves a lossless image. So the offset is skipped where the quantisation style is
`NoQuantization`, which is why reconstruction now takes the style as an argument. It also has to
return `f32`, since half a unit is not an integer.

### Measured, on top of §7

| codestream | after `9cce046b` | with this fix |
|---|---|---|
| `S2.pdf` object 17 | 53 286 differ, worst by 3 | **325, worst by 1** |
| `S2.pdf` object 19 | 34 908, worst by 3 | **436, worst by 1** |
| `S2.pdf` object 21 | 8 758, worst by 2 | **314, worst by 1** |
| `issue5475.pdf` object 8 | 91 144, worst by 2 | **48, worst by 1** |
| `issue5481.pdf` object 5 | 1 076 388, worst by 4 | **546, worst by 1** |
| `issue5549.pdf` object 11 | 965 165, worst by 5 | **2 494, worst by 1** |

**Roughly 3.4 million differing samples become 5 900, and no remaining difference exceeds one
level.** The buckets are unchanged — 14 identical, 13 differing, 3 not comparable — and all
fourteen that were byte-identical, every one of them reversible, stay so. `hayro-jpeg2000`'s own
`test_jpeg2000_standard_example_b4`, which is Annex B.4's worked example, still passes.

Offered as a pull request from `close2/hayro`, branch
`fix/reconstruction-midpoint-when-fully-decoded`, and **merged into that fork's `main` as
`2a1abd14`** — which is what this tree pins while it waits for upstream.

**What is left after it is one level on 0.02% to 0.1% of a plate's samples**, which is where a
precision ladder becomes the right instrument rather than more reading: a difference that shrinks
as both sides move to `f64` is arithmetic, and one that does not is a third defect. Nobody has run
that.

---

## 9. How a fix gets offered, and where the checkout is

**Recorded on 2026-08-08 by the project owner**, because §8 used this route and did not say where it
runs.

`tmp/hayro` is a working checkout of the whole `hayro` workspace — `hayro-jpeg2000`, `hayro-jbig2`,
`hayro-interpret`, `hayro-syntax` and the rest — with two remotes already wired:

```text
origin    https://github.com/close2/hayro.git     the project owner's fork
upstream  https://github.com/LaurenzV/hayro.git   the maintainer's
```

It sits at `2a1abd14`, which is §8's own fix and the revision this tree pins.

**The route, and it is the owner's standing offer:** a change goes on a **new branch** in that
checkout; the owner pushes it to `origin` and opens the pull request; this tree then depends on the
fork until the maintainers take it. That is exactly how §8 was closed, and it means a defect found
here in any hayro crate is fixable rather than only reportable — which changes what may be written
in a todo file. **"Blocked on an API `hayro-jpeg2000` does not have" is no longer a blocker, it is a
branch** (`doc/todo/24`'s reduced-resolution decode is the standing example).

Two things that do not change: the fix must be right for *hayro's* users and not only for this
viewer — a maintainer is being asked to carry it — and this tree's own gates decide whether it
helped, because `tests/jpeg2000.rs` compares against ISO/IEC 15444-5's reference software rather
than against either implementation's opinion.

---

## 10. A reduced-resolution decode still allocated the full-resolution image — **found here, fix on a branch**

**Written 2026-08-08 in the three-hundred-and-ninety-sixth session**, and this is the section the
project owner pushes: the branch below is committed in `tmp/hayro` and cannot be pushed from here.

```text
branch   feat/reduced-resolution-allocates-less
commit   1dc833f7e87dd6849a358d74af3586d7955e1e03
parent   2a1abd14   (§8's fix, this fork's main, what this tree pins)
title    hayro-jpeg2000: Size the coefficient buffer by the resolution asked for
```

### What this tree was looking for, and what it found instead

ISO 32000-2 §7.4.9 NOTE 3 tells a viewer to use the resolution progression on a densely sampled
image, and `issue19517.pdf` is one: 12608×16806 in four channels, 847 million samples, drawn on a
page a screen shows at about four megapixels. This tree had that item written down as *blocked on
an API `hayro-jpeg2000` does not have*.

**It has had the API since `1dfc6e2f` (10 December 2025)** — `DecodeSettings::target_resolution` —
and the claim was three sessions' worth of stale in three separate places here. That is a finding
about this repository rather than about the crate, and it is recorded as one (ADR 0233).

What the crate *did* have was a defect one layer down. `build_decompositions` sized
`storage.coefficients` from the component tile's own rectangle — the full-resolution image —
whatever level the caller had asked for. The bit-planes above the cut are not decoded and their
decompositions are not synthesised, so the reduction bought **time and no memory**: every one of
these decodes began with a single allocation of 3 390 240 768 bytes.

Resident size never shows it. The buffer is `calloc`'d and the pages belonging to the levels that
are never decoded are never touched, so what it costs is *address space* — which is what
`RLIMIT_AS`, a 32-bit target and a `no_std` fixed arena all bound.

### Measured, on `issue19517.pdf`'s codestream through `Image::decode`

| `target_resolution` | raster | peak address space before | after |
|---|---|---|---|
| 788×1051 | 788×1051 | 3336 MB | **115 MB** |
| 1576×2101 | 1576×2101 | 3424 MB | **241 MB** |
| 3152×4202 | 3152×4202 | 3775 MB | **743 MB** |
| 6304×8403 | 6304×8403 | 5176 MB | 2751 MB |
| none | 12608×16806 | 10 784 MB | 10 784 MB |

Under a 1 GiB `RLIMIT_AS`, **none** of the first four completed before the change and the first
three complete after it. Full resolution is unchanged to the byte, which it must be: with nothing
skipped, the highest kept level *is* the component tile.

### The fix, and the one thing it must not do

The sub-bands of resolution levels 0 to *r* partition the rectangle of resolution level *r*
(B-15), so the highest level that will be decoded states exactly how many coefficients the levels
below it need. The levels above it get an empty range.

**Their code-blocks are still built.** A packet header is what says how long its body is and a
tile-part's packets are read in sequence; in LRCP order the packets of the skipped levels are
interleaved with the ones that are kept, so they have to be read *past* rather than not described.
Only their coefficients are unwanted. `build_decompositions`'s existing assertion that the
coefficient counter meets the buffer length is what checks the partition, on every decode.

### What was run

- **`cargo test -p hayro-jpeg2000` — all 183 assets of `manifest_serenity` and `manifest_openjpeg`
  PASS**, against snapshots generated from unpatched `main` and then compared with the branch
  applied. Byte-identical output, including `serenity/large_target_resolution`.
- `test_jpeg2000_standard_example_b4`, Annex B.4's worked example, passes.
- `cargo fmt --check` clean; `cargo clippy -p hayro-jpeg2000 --all-targets` adds no warning (the
  four `unnecessary_sort_by` are pre-existing and in other files).
- This tree's own instrument, `crates/pdf-model/tests/jpeg2000.rs` — the 30 corpus codestreams
  against `opj_decompress` — is unmoved: 14 byte-identical, 13 differing, 3 not comparable, no
  difference above one level. None of those thirty is decoded at a reduced level, which is exactly
  why that gate says the change is inert where it should be.

### What this tree does with it

**Nothing yet, and deliberately.** The workspace pins `2a1abd14`, and against that revision asking
for a reduced level would allocate 3.4 GB inside a worker with a gigabyte of address space — a
process abort where there is currently an accurate refusal. A `path` dependency into `tmp/` cannot
be committed. So `pdf-sandbox` still passes `target_resolution: None`, the comment beside it now
says why in one sentence, and `doc/todo/24` states the four edits that follow the moment the fork
carries `1dc833f7`.

### One thing noticed and *not* offered, recorded so it is not rediscovered

`target_resolution` picks the finest resolution level whose dimensions are **at least** the
request, so a raster can come back up to twice the requested size per axis — four times the
samples. That is the right rounding for fidelity and the wrong one for a memory budget, and there
is no way to ask for the other. It is bounded and this tree can reduce on its own side, so it is
not worth a second knob in the same pull request; it is written here because the next reader will
otherwise find it again.
