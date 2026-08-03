# ADR 0161 — A codec nobody had checked

Status: accepted, 2026-08-03. Session 200. Started as one page on the ambiguous ranking and
ended as a gate.

## Where it started

`jp2k-resetprob.pdf` was first on §3a's undiagnosed ranking at 5.03 bounds from the nearest
reference: a 166×55 page whose whole content is one 40×27 JPEG 2000 photograph of a sunset,
drawn into 30×21 device pixels. One command, nothing reported, and five renderers producing five
plausible sunsets.

The file's name is its own hypothesis. `opj_dump` says its code-blocks carry `cblksty=0x2`,
which is ISO/IEC 15444-1 Table A.19's **RESET** — "reset context probabilities on coding pass
boundaries" — a coding option a decoder can get subtly wrong while still producing a picture.

## The instrument

§7.4.9 of ISO 32000-2 says almost nothing about JPEG 2000: the data "shall be" a codestream, and
the whole of the decoding belongs to ISO/IEC 15444-1, which defines it exactly. **A codec has one
right answer per codestream**, which makes it checkable in a way a rendered page is not.

`crates/pdf-model/tests/jpeg2000.rs` walks all 974 corpus documents, pulls every `/JPXDecode`
stream out through `Document::image_stream`, decodes it twice — once through `pdf_sandbox` and
once through `opj_decompress` — and compares the samples **exactly**.

**This is not principle 5's forbidden move, and the difference is worth being explicit about.**
Principle 5 is about ISO 32000-2: poppler's reading of a PDF clause is evidence, never the
definition. Here the standard is ISO/IEC 15444-1, it admits no latitude, and OpenJPEG is the
reference software ISO/IEC 15444-5 publishes for Part 1. The inference still runs the same
direction — agreement is evidence that this tree reads 15444-1 right, and a disagreement is a
question to take back to 15444-1 rather than a target — but the evidence is much stronger than
a rendered page's, because there is exactly one correct set of samples.

The comparison is exact rather than tolerant for `jbig2.rs`'s reason: a tolerance would only hide
the differences worth finding.

## What it found

The original question was answered immediately and negatively: **`jp2k-resetprob.pdf`'s
codestream decodes byte-identically to the reference's.** RESET is fine. The page's remaining
difference is the reduction from 40×27 into 30×21, and it joins `AMBIGUOUS_IMAGE_REDUCTION` with
its numbers — three of our four distances below every distance between two references.

**A diagnosis that rules a cause out is worth what one that finds a defect is**, and unlike
reading the picture it is checkable.

Then the gate found something nobody was looking for. **Thirteen of the thirty corpus codestreams
decode to samples OpenJPEG does not produce**, by up to 87 levels of 255 across three quarters of
an image. The discriminator across all thirty is exact:

| | `qntsty` | |
|---|---|---|
| 13 | 2 — scalar expounded, the **irreversible** 9/7 path | all differ |
| 14 | 0 — no quantisation, the **reversible** 5/3 path | all identical |
| 1 | 2 | identical: a 316-byte 18×166 strip where the difference rounds away |

Layer count is not the discriminator and neither is the multi-component transform — both were
checked and both cross the line in each direction. And on `S2.pdf` object 17, two of every three
differing samples move **toward the image's own mean**, with the standard deviation falling
0.2499 → 0.2399: our reconstruction is systematically smaller in magnitude, which is what
dropping the reconstruction-bias term in inverse quantisation costs.

The defect is `hayro-jpeg2000` 0.4.0's — a crates.io dependency `pdf-sandbox` only hands bytes to
— and is written up for its author in `doc/JPEG2000_FEEDBACK.md`, with the command that
reproduces each line.

## What this tree does about it

Nothing to the samples. Three things to the record:

- **`DIFFERS_FROM_THE_REFERENCE_SOFTWARE`** holds the thirteen by name and fails in both
  directions, so an upstream release that fixes them fails the build and says so. That is the
  only way this tree would ever notice.
- **`AMBIGUOUS_IRREVERSIBLE_JPEG_2000`** carries the two corpus pages the defect reaches —
  `S2.pdf` page 1 and `issue5475.pdf` page 1, both of which were undiagnosed on §3a's ranking.
  It is one of the rare groups that says outright *we are wrong*, which §3a explicitly allows and
  `AMBIGUOUS_ZERO_AREA_FILL` did for two sessions before its fix.
- **`IDENTICAL` and `NOT_COMPARABLE`** are counts held to equality, so a codestream that stops
  being *found* — an object walk breaking, a filter chain ceasing to resolve — fails too.

## The lesson

**Four codecs reach this tree through dependencies, and two of them had a gate.** JBIG2 has
`tests/jbig2.rs` — ninety-six encodings of one image held byte-identical — and §7.4.7's worked
example. CCITT has its own. JPEG 2000 had **nothing**: it was decoded on 12 corpus documents,
drawn on pages the oracle judged, and never once compared with anything that could say whether
the samples were right.

The reason it survived is the reason trap 1 exists. A JPEG 2000 decode that is 4% flat is a
photograph that looks like a photograph; the corpus reports nothing, the oracle calls the page
ambiguous inside a spread five renderers already have, and no eye would call it. The check that
found it took an afternoon and needed no reference renderer at all.

**Ask, of every dependency that decodes something: what would it look like if this were subtly
wrong, and what in this tree would notice?** For a codec the answer is usually "a picture that is
almost right" and "nothing".

## Alternatives rejected

- **Vendor and patch `hayro-jpeg2000`.** A JPEG 2000 decoder is thousands of lines with its own
  security surface, and forking one on a hypothesis about a reconstruction term is a much larger
  commitment than the evidence supports. The write-up costs an afternoon and the fix, if the
  hypothesis holds, is a line upstream where it belongs.
- **Compare rendered pages instead of samples.** That is what the oracle already does, and it is
  precisely what could not see this: both pages sit inside an `ambiguous` verdict.
- **Tolerate a small difference.** ISO/IEC 15444-1 permits none, and a tolerance wide enough to
  pass `issue5481.pdf`'s 4 levels would be wide enough to hide the next defect entirely.
