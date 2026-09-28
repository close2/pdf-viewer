# 1349 — A fax decoder of this tree's own, which conceals the damaged rows Table 11 tolerates

Session 1256. Status: **accepted** and built.
Context: `crates/pdf-ccitt` (new), `crates/pdf-sandbox/src/decode.rs` (`ccitt`, `CcittRows`),
`crates/pdf-sandbox/src/protocol.rs` (`CcittParameters`, `Bilevel::concealed`),
`crates/pdf-sandbox/build.rs`, `crates/pdf-model/src/image.rs` (`decode_ccitt`, `ccitt_shortfall`),
`crates/pdf-model/examples/ccitt_decoder_census.rs`, `fuzz/fuzz_targets/ccitt.rs`.
Amends: ADR 0021 decision 1 (the decoder) and decision 3's first refusal. Clauses: ISO 32000-2
§7.4.6 (Table 11); ITU-T T.4 (07/2003) section 4, T.6 (11/1988) section 2, cited and paraphrased.

## 1. What was owed

Table 11's `/DamagedRowsBeforeError` is "[t]he number of damaged rows of data that shall be
tolerated before an error occurs", tolerating one "shall mean locating its end in the encoded data
by searching for an `EndOfLine` pattern and then substituting decoded data from the previous row if
the previous row was not damaged, or a white scan line if the previous row was also damaged", and
"[t]his entry shall apply only if `EndOfLine` is true and K is non-negative". The tree refused an
image where the entry applied, because `hayro-ccitt` reports neither the bit a failure happened at
nor a way to resume (re-checked in the pinned checkout: `decode` answers `Result<usize>`,
`DecodeError` carries no position, `BitReader` and `DecoderContext` are private).

## 2. Decision: the decoder is this tree's, and `hayro-ccitt` decodes no `CCITTFaxDecode` stream

`crates/pdf-ccitt`: no dependencies, `#![forbid(unsafe_code)]`, 1 130 lines. T.4 Tables 2, 3a, 3b and 4
are written as T.4 prints them — binary digits beside their values — and built into lookup tables
by `const fn`, so there is no parse at startup. A scan line leaves the crate as its changing
elements, T.4 section 4.2.1.3.1's vocabulary, and `pdf-sandbox` packs them: the crate knows the
coding and nothing about samples or `/BlackIs1`. It runs where `hayro-ccitt` ran, in the confined
worker (ADR 0021 decision 2 unchanged), and `build.rs` now folds its sources into the worker's build
identity, since a path dependency changes without touching `Cargo.lock` (ADR 0458's hole).

**Why not keep `hayro-ccitt` for the undamaged path and wrap it.** There is nothing to wrap: the
concealment needs the row's first bit and a resume, which the crate has no API for, and a second
decoder only for `/DamagedRowsBeforeError` would be two readings of T.4 behind one filter. The
measurement (section 5) shows the new decoder no slower, so nothing was bought by keeping both.
`hayro-ccitt` stays in the lock as `hayro-jbig2`'s MMR decoder, and in the workspace table as the
census's evidence (a `pdf-model` dev-dependency).

## 3. What a damaged row is, and what the concealment does

A row is damaged where its bits are no codeword of the kind expected, a run passes the end of the
line, a vertical code puts `a1` before `a0` or past the line, or the data ends inside it. A run is
never shortened to fit: §7.4.6 forbids "any error correction", and shortening changes where every
later `b1` falls; `hayro-ccitt` did it (section 5).

**An empty run is decoded, not refused.** T.4 section 4.2.1.3.1 places `a1` to the right of `a0`,
so a code that puts them together is degenerate; but it states its pels unambiguously — none — and
producers write it. A horizontal mode of two empty runs moves `a0` nowhere, so at a line's start it
stays the imaginary element before the first pel. The census found the case: a PDFium-produced
scan (`PDFIUM-962-0.pdf`, five Group 4 pages) writes `H(0,0)` at the start of lines, and moving
`a0` onto pel 0 took that pel out of the next `b1` and broke the page at its second line; decoded as
stated, all 2688 lines of one of its pages agree with `mupdf`'s, line for line.

**Under the concealment, and only there, a complete line not followed by an end-of-line code is
damaged too.** The concealment is where the filter is told a row is the bits between two
end-of-line codes — its precondition is `/EndOfLine` true and it finds a row's end "by searching for
an `EndOfLine` pattern" — and a line whose codewords fit the width by chance would otherwise push its
remainder into the next row. Without the concealment a missing code is the producer's breach of "If
`EndOfLine` is true end-of-line bit patterns shall be present", which the clause does not ask the
filter to police — it "shall always accept" the patterns, not demand them — and the next line is
decoded where the last ended. The census (section 5) is why this was measured rather than assumed:
enforced everywhere, it refused nine streams of whole Ghostscript-produced pages that carry an
end-of-line code before the first line only, and that `mupdf` draws as the pages they are.

Where the entry applies and the count allows, the row's end is searched for **from the row's own
first bit** — its codewords may have been read into the zeros that begin the end-of-line code — as
the first run of eleven zeros that a one follows (T.4 section 4.1.2 makes that pattern impossible
inside valid data). The row is the previous row's changing elements, or white where the previous
row was concealed too; the next row past the count is where the error occurs, and the decode ends
there as ADR 0794 already drew it. Two readings the clause leaves open:

- **A damaged first row is white.** There is no previous row; T.6 section 2.2.1 makes the line
  before the first an imaginary white line, and T.4 section 4.2.1.3.1 starts `a0` on an imaginary
  white element.
- **A two-dimensional line after a concealed one is decoded against the concealed row.** T.4
  section 4.2.1.3 makes the reference line the one immediately above, and the one above is what the
  filter delivered. The alternative — refusing until the next one-dimensional line — is
  resynchronisation the clause does not ask for.

A concealed row is drawn and said beside the drawing (`image::ccitt_shortfall`), because its
samples are the row above's or white rather than the file's; `Bilevel::concealed` carries the count
out of the worker.

**Where the data ends.** §7.4.6: "When a filter reaches EOD, it shall always skip to the next byte
boundary following the encoded data" — so the zeros finishing the last byte are padding, and the
decode ends there cleanly. Whole zero bytes beyond it are not padding the clause names, and eight
zeros begin no codeword, so they are damage, as `tests/ccitt_bound.rs` has always pinned.

## 4. The brief's premise, corrected

The round's brief asked for a fixture of a Group 4 stream with a corrupted row decoded under
`/DamagedRowsBeforeError 2`. Table 11 says the entry applies "only if `EndOfLine` is true and K is non-negative", and Group 4
is `/K` negative, so on a Group 4 stream it is inert. The concealment fixtures are Group 3 — mixed
and one-dimensional — and a Group 4 stream with the entry at 5 is pinned as ending at its first
damaged row (`the_tolerance_is_inert_where_the_entry_does_not_apply`).

## 5. What was measured

`pdf-model/examples/ccitt_decoder_census.rs`, one walk behind the lock over the pdf.js corpus,
`doc/corpora` and `corpus-cache` (SafeDocs' crawl, the Tika issue tracker, OpenPreserve): 90 763
documents, 5 343 naming the filter, **190 537 fax streams** (image `XObject`s and masks; inline
images are the interpreter's to find and are not in it). Both decoders at `/DamagedRowsBeforeError`
zero, compared scan line by scan line:

- **190 500 identical**, bit for bit and in how they end. One more has the same lines and ends
  differently: `poppler-76445-0.pdf`, where `hayro-ccitt` errors after the last line it delivers.
- **36 differ, and every one is damaged data or a malformed file.** 25 are streams `mupdf` also
  reports as damaged ("overflow", "negative code", "invalid code" in its fax decoder), where
  `hayro-ccitt` went on further by shortening an over-long run to the line or retrying a failed
  code in the other colour's table — the correction §7.4.6 forbids — and this decoder stops at the
  damage. Six are one file, `4113564.pdf`, which writes `stream`, a space and a line feed:
  §7.3.8.1 admits only CRLF or LF after the keyword, `pdf-syntax` hands the space and line feed to
  the filter, and the old decoder's colour-retry turned them into a picture offset from the one
  coded — not this filter's to fix, and named for `pdf-syntax`. Four are Ghostscript bug 702896's
  file, whose streams state `/EncodedByteAlign true` and are not aligned; `mupdf` ignores the flag,
  both decoders here obey it and fail, differently. One is `GHOSTSCRIPT-694573`, which `mupdf`
  warns about too.
- **No stream in 190 537 states `/DamagedRowsBeforeError` above zero where it applies**, so the
  concealment is witnessed only by the fixtures.
- **Decode time: `pdf-ccitt` 227.8 s against `hayro-ccitt`'s 274.4 s** over 154.5 million scan
  lines — 17% less, from the thirteen-bit lookup tables. Nothing was given up for the concealment.

The first run of the census is what corrected section 3 twice: enforcing an end-of-line code after
every line under `/EndOfLine` true refused nine whole Ghostscript pages, and refusing empty runs
broke a PDFium scan at its second line. Both rules were narrowed to what the clause asks, and the
second run is the one above.

## 6. Costs

- A decoder this project now owns: a disagreement with T.4 or T.6 is a defect to fix here rather
  than an issue to report. Against it, 1 130 lines, the Recommendations' own examples as fixtures,
  a fuzz target from the first commit, and the census to hold it against the old decoder.
- Pages whose fax data `hayro-ccitt` stretched past damage now stop where the damage is.
