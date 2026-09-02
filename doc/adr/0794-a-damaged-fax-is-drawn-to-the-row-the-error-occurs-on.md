# ADR 0794 — A damaged fax is drawn to the row the error occurs on: `CCITTFaxDecode` delivers the scan lines before the damage, leaves the rest unpainted, and says so beside the drawing

Status: accepted. Session 876.
Clauses: ISO 32000-2 §7.4.6 (Table 11's `/DamagedRowsBeforeError`), §7.3.8.2, §7.3.8.1.
Code: `crates/pdf-sandbox/src/decode.rs` (`PackedRows::stop_short`, `ccitt`),
`crates/pdf-sandbox/src/protocol.rs` (`Bilevel::delivered`, `Bilevel::stopped_by`, the wire),
`crates/pdf-model/src/image.rs` (`Parts`, `Picture`, `Flattened`, `decode_ccitt`,
`apply_explicit_mask`, `apply_soft_mask`), `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/thumbnail.rs`, `crates/pdf-transform/src/images.rs`.
Tests: `crates/pdf-model/tests/ccitt_bound.rs::damaged_data_draws_the_lines_before_the_damage_and_says_so`,
`::damage_before_the_first_line_refuses_the_picture_out_loud`,
`crates/pdf-sandbox/src/protocol.rs::tests::a_bilevel_response_that_stopped_short_carries_its_sentence`,
`::a_bilevel_response_delivering_more_rows_than_its_height_is_malformed`, and
`doc/checks/fixed-documents.toml`'s row for `REDHAT-229174-0.pdf`.
Opened by `doc/todo/03` section 40, the `batch5/REDHAT` chunk.

## Context

`REDHAT-229174-0.pdf` is a page of a Russian mathematics textbook scanned by Adobe Photoshop 4.0
in July 2000 and filed against Red Hat's bug tracker in 2007: one `CCITTFaxDecode` image, Group 4
(`/K -1`), 1535 × 2244, `/BlackIs1 true`. This tree drew the page **blank** and reported
`I1: CCITTFaxDecode: arithmetic overflow in position calculation`; `poppler` draws the top third
of the page — the running head, the definition, the integral — and `mupdf` draws two lines more.
Ranked by our ink against the lighter reference over every incomplete page of the 1712-document
directory, it is the head of the chunk by a factor of three over the next row, and it and its
byte-identical twin `REDHAT-493442-0.pdf` are one finding.

Two things had to be settled before anything was changed: whether the decoder is right to stop,
and what the standard says about what comes after it.

### The file

The stream is 30 309 bytes with no end-of-block pattern. A probe against the pinned
`hayro-ccitt` revision decodes **756 whole scan lines** and then fails inside the 757th with
`Overflow` — the vertical mode's `a1 < a0`, a code that cannot describe a scan line — and the
decoded rows are the page `poppler` shows: text down to row 755. `poppler`'s black band from 756
to 893 and `mupdf`'s further two lines of text to row 991 are what each does *after* the damage.

The file also writes `stream\r` — a CARRIAGE RETURN alone after the keyword, which §7.3.8.1
forbids ("shall be followed by an end-of-line marker consisting of either a CARRIAGE RETURN and a
LINE FEED or just a LINE FEED, and not by a CARRIAGE RETURN alone") — and `pdf-syntax` already
tolerates it, so the decoder was given the right bytes. That was checked before the decoder was
believed, because an off-by-one at the stream's start would have produced the same sentence.

## What the standard says

§7.4.6, on the decoder's behaviour after an error, in one sentence:

> The filter shall not perform any error correction or resynchronization, except as noted for
> the DamagedRowsBeforeError parameter in "Table 11 -Optional parameters for the CCITTFaxDecode
> filter".

And Table 11's row for that parameter:

> The number of damaged rows of data that shall be tolerated before an error occurs. This entry
> shall apply only if EndOfLine is true and K is non-negative. Tolerating a damaged row shall mean
> locating its end in the encoded data by searching for an EndOfLine pattern and then substituting
> decoded data from the previous row if the previous row was not damaged, or a white scan line if
> the previous row was also damaged. Default value: 0 .

So three things are the standard's and not this tree's:

- **Stopping at row 756 is what the clause asks.** `mupdf`'s two further lines of text are a
  resynchronisation, which the first sentence forbids; `poppler`'s black band is a decode continued
  past an error. Neither is evidence about the clause, and this is trap 9's shape — two references
  that disagree with each other after the damage cannot both be reading Table 11.
- **"An error occurs" at the first damaged row** under the default of zero — so the filter's decode
  *ends* there. The standard's own concealment mechanism, where a producer asks for it, substitutes
  a scan line for a damaged one and carries on; it does not discard the rows before it. The rows a
  filter delivered before its error are the filter's output.
- **What the undelivered rows show is stated nowhere.** Neither §7.4.6 nor §8.9.5.1 has a sentence
  about the scan lines after an error, exactly as neither has one about the scan lines after
  `/Rows` under `/EndOfBlock false`, which ADR 0392's reading found and `pad_to_height` records as
  a choice. That choice — the filter's white — is not taken here, and the witness is why: under
  `/BlackIs1 true` with no `/Decode`, the filter's white is the page's **black**, and the first
  build of this change drew the lower two thirds of the page solid, which is `mupdf`'s picture
  and no more the file's than `poppler`'s white. A colour the file never stated is not this
  reader's to choose, so the rows are left **unpainted**.

§7.3.8.2 names the wider condition — "[a]n error occurs if Length is too small, if an explicit EOD
marker occurs too soon, or if the decoded data does not contain 200 bytes" — and ADR 0356 already
decided what this tree does with an image whose samples fall short of its grid: "what the stream
carries is drawn where it belongs, the rest of the grid is left unpainted, and
`image::short_of_its_grid` reports it beside the drawing". A CCITT stream that breaks at row 756
is that image with the shortfall inside a codec rather than in a byte count.

## Decision

**A `CCITTFaxDecode` stream that the filter stops on is drawn as far as it was delivered, the
remaining scan lines are left unpainted, and the shortfall is reported beside the drawing** — in the same
words for the page's image, a `/Mask` stencil, an `/SMask` image and the transform's `images`
verb. A stream on which not one whole scan line was delivered stays a refusal carrying the
decoder's sentence, because there is nothing to draw and a blank picture reported as damaged
would be a picture this reader made up.

The report is *in the decode's value* rather than noted at decode time. `Bilevel` carries
`delivered` and `stopped_by` out of the sandbox, `pdf_model::image::Parts` carries the sentence
beside the picture, and `RasterCache` remembers both — so a second `Do` of the same image
answered from the cache says what the first said, which is trap 5 and `tests/image_reuse.rs`'s
own rule. The report is worded where both numbers are known: *the filter delivered 756 of the
2244 scan lines the image states before the damage, which are drawn; the rest are left
unpainted*, with the clause and the parameter's default beside it. The worker still pads the
rows for the wire's fixed size, in the filter's white, and `decode_ccitt` clears them after
unpacking, reading `Bilevel::delivered`: the worker knows the filter's white and not the page's.

**What this reverses, and why the old sentence was wrong.** `decode.rs::ccitt` said "a malformed
stream is reported rather than partially drawn: the decoder can leave usable rows behind an
error, and taking them would be a page that is silently missing its bottom half". The argument
rested on *silently*, and the rows are not taken silently once the sentence travels with them.
Refusing them threw away the two thirds of a scanned page the file carries for the third that is
not there — on this document, every mark on the page.

**Whether hayro should resynchronise is not this tree's question.** Its `Overflow` is the right
answer to this data under §7.4.6's first sentence; the fork is not touched.

## Consequences

- `REDHAT-229174-0.pdf` draws the page `poppler` draws above row 756, reports the shortfall by
  clause and by count, and is a row in `doc/checks/fixed-documents.toml` pinning both. Its twin
  is the same bytes and is not a second row.
- The wire between `pdf-model` and `pdf-sandbox-worker` changed shape — a bilevel response
  carries a count and a sentence ahead of its samples — which is what the greeting's build
  identity (ADR 0458) exists for: a worker from another build is named as one rather than read
  as a malformed response.
- A **thumbnail** whose filter stopped on damaged data is still refused, with the sentence: the
  `Thumbnail` type crosses two wire protocols (`viewer-confined`'s panels and `viewer-ffi`'s
  answers) that carry no such sentence, and a report with no channel to a host is one nobody
  reads. The cost is a §12.3.4 preview a host could have shown in part; the residue is here so
  that a round adding the channel knows the type is ready for it.
- The `images` verb writes the delivered rows over an unpainted remainder and reports the shortfall as
  a warning on the output, for the base image and for a mask alike, so a file it hands on is
  never a partial picture handed on quietly.
- `/DamagedRowsBeforeError` above zero is still refused rather than decoded without the
  concealment it asks for; that row of Table 11 is what keeps §7.4.6 `partial`, and it is
  unchanged by this.
