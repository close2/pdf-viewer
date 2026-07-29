# ADR 0021 — `CCITTFaxDecode` through the sandbox, and what a decoded page then showed

Status: accepted, 2026-07-29.

## Context

`CCITTFaxDecode` (ISO 32000-2 §7.4.6) was the last image codec absent from this tree, and the
largest named image gap: 12 of the 974 corpus documents could not draw an image because of it,
among them whole pages of scanned text that simply did not appear. The count had *risen* from
5 the session before, because seven of the twelve write their fax-encoded images inline and
had been reported as inline images until §8.9.7 landed.

Group 3 and Group 4 fax are ITU-T T.4 and T.6: a Huffman-coded run-length scheme from 1980 and
its two-dimensional successor. The clause says in as many words that "the encoding algorithm is
not described in detail in this document"; what §7.4.6 *does* specify is Table 11's eight
parameters, and those decide what the picture looks like.

## Decision 1 — take `hayro-ccitt`, and say why the argument is not ADR 0014's

`hayro-ccitt` has been compiled into this tree since the seventh session, as `hayro-jbig2`'s
dependency for MMR-coded regions. It is `#![forbid(unsafe_code)]`, `no_std`, and has no
dependencies of its own.

ADR 0014's argument for taking `hayro-jbig2` and `hayro-jpeg2000` does not transfer, and
pretending it does would be the failure principle 5 exists to prevent. That argument was
about 19 400 lines of MQ arithmetic coding, EBCOT and wavelets — code this project could
write and should not. T.4 and T.6 are two pages of Huffman tables and a state machine.
"We could not write this" is untrue here.

What decides it instead:

- **The decoder is already in the binary**, and has been exercised by every MMR-coded JBIG2
  region in the corpus for five sessions. A fresh implementation would start with less
  evidence behind it than the one already linked.
- **Depending on a dependency's dependency is a version nobody chose.** It is now named in
  `Cargo.toml` with its own version requirement, which is the smaller half of this decision
  and the part that would have been wrong to leave implicit.

The cost is the same one ADR 0014 wrote down: a disagreement with T.4 or T.6 is an issue to
report rather than a defect to fix.

## Decision 2 — route it through the sandbox, like the other two

The specific justification in ADR 0014 — panic containment, an `RLIMIT_AS` ceiling, and a
process boundary around a decoder this project does not own — applies unchanged. A fax
decoder is a small state machine, but "small" is not an argument about what happens when it
indexes past the end of a row, and release builds abort on panic.

The protocol gains one request kind and a fixed-width parameter block in the payload that
JBIG2 uses for its globals stream. The parameters cross the pipe **already resolved against
Table 11's defaults**, because the side that can read a `/DecodeParms` dictionary is the side
that can apply them — which keeps the worker holding no opinion about PDF at all, exactly as
`decode.rs` already does for JPEG 2000's colour.

## Decision 3 — refuse two cases rather than approximate them

Both are places where the dictionary says something this cannot honour, and where drawing
something plausible would be the silence trap 5 is about.

- **`/DamagedRowsBeforeError` above zero.** Table 11 makes it error *concealment*: locate a
  damaged row's end by searching for an `EndOfLine` pattern, then substitute the previous
  row's data or a white line. The decoder has none of that. A document that asks for
  concealment is one whose producer expected damage, so decoding it without concealment and
  drawing the result is drawing an image already declared possibly wrong.
- **`/Columns` disagreeing with the image's `/Width`.** The filter delivers rows of `/Columns`
  samples padded to a byte boundary; §8.9.5.1 makes the image `/Width` samples wide, and the
  unpacker reads that many per row. Where the two differ the strides differ, and nothing in
  ISO 32000-2 says which statement wins.

And one place where the standard is silent and a choice is recorded as a choice: Table 11 says
the filter "shall stop when it has decoded the number of lines indicated by Rows or when its
data has been exhausted, **whichever occurs first**", so a stream that ends early is legal.
What the image shows for samples never delivered is not stated anywhere. They are left blank,
which is what an unsent fax scan line is, and `PackedRows::pad_to_height` says so.

## How it is checked without a reference

There is no specification-supplied CCITT bit stream to decode, unlike §7.4.7's worked JBIG2
example, and encoding one with another library would make that library the oracle. So
`tests/ccitt.rs` checks two statements the *standard* makes, on data the corpus supplies:

- **`/BlackIs1` inverts exactly.** Table 11 calls it "the reverse of the normal PDF syntactic
  convention", and reverse is a strong word: decoding one stream under both settings must give
  bitwise complements, sample for sample. A decoder applying the flag anywhere but the last
  step produces two plausible images that are not complements. Confirmed to fail when the
  flag is dropped.
- **A scan of a printed page is mostly white**, which catches the one error the first cannot:
  a *global* inversion is self-consistent and turns a page into its negative.

Whether the pixels are right is the oracle's question, over the twelve documents.

## What it found, which was not in §7.4.6

Two things, and both are the handover's standing warning arriving on schedule: **every page a
new feature makes drawable is a page nobody has ever looked at.**

**`/Rotate` had been turning pages the wrong way since the first page tree.** `issue5747.pdf`
is a fax-encoded scan on a `/Rotate 270` page; drawing its image for the first time put it on
the screen upside down beside four renderers that agree with each other. §7.7.3.3 Table 31
says clockwise; page space here is y-up and the flip to a raster happens later, so a clockwise
turn is a *negative* rotation in this space — and the 90 and 270 matrices were exchanged,
which is a 180° error. Six pages were contradicted by it, five of them filed under substituted
fonts because they also carry one. It is one line, it is now pinned by a test written in terms
of where a corner lands rather than in terms of a matrix, and no metric in this tree could
ever have seen it: an upside-down page has the right ink in the right quantity.

**A glyph reduced eleven times is text nobody can read.** `bug1001080.pdf` sets its text in a
Type 3 font whose every glyph description is an inline image mask coded `/F /CCF` — a 39×53
bitmap per letter, drawn at about five device pixels. `tiny-skia`'s bilinear filter samples
four neighbours whatever the reduction, so the crossbar of a `t` — one source row in fifty-three
— is never sampled, and the page reads `pinL LesL` where four renderers read `pint test`.
This is the same defect `firefox_logo.pdf` has sat on for four sessions at 0.02 outside the
bound, which is why it was recorded as "Small". It is not small; it is unreadable text, and
the argument for an area-averaging filter in both backends is now made of that rather than of
a logo's edge.

## Consequences

- 12 corpus documents move from incomplete to complete; the corpus's incomplete count falls
  263 → 251 and nothing came back.
- All eight standard filters that decode images now decode. `LZWDecode` is the last standard
  filter absent, and no corpus first page reaches it.
- §7.4.6 is `partial` in the conformance ledger, with `/DamagedRowsBeforeError` named as the
  reason, and §8.6.4.2 is `implemented` — reviewed because a bilevel filter delivering 1 for
  white is only right if that clause says 1.0 is white.
- One page is newly contradicted, and it is about resampling rather than about fax.
