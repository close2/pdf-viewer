# 0343 — The prefix a damaged stream keeps, and the sentence it never said

**Status.** Accepted.

## Context

`doc/todo/03` §8 left a decision rather than a defect, and phrased it as a question about
robustness:

> **A corrupt flate content stream, and whether a truncated recovery is ever right.**
> `govdocs1-error-pdfs/error_set_2/498264.pdf` inflates 18 bytes of its page-one content —
> `q\n30 31.16 552 729` — and then fails after 79 of 2649 input bytes with zlib's "invalid
> distance too far back". `poppler` carries on past it and recovers three lines of a heading;
> `mupdf`, `ghostscript` and this tree draw nothing, and this tree reports `Undecodable`.

The question as posed — *should this tree start keeping the prefix?* — turned out to rest on a
false premise about this tree, and finding that out is most of what this round is.

**This tree already keeps the prefix, and has since long before the question was asked.**
`FilterRefusal::Corrupt`'s own documentation said so outright — "`FlateDecode` and `LZWDecode`
both keep what they inflated before the damage, because a partly-inflated content stream still
renders most of a page" — and `crates/pdf-syntax/tests/stream_length_bound.rs` has carried a test
asserting it since ADR 0306. So the decision this round had to make was never *whether* to
recover. It was whether a recovery that says nothing is one this project's principles permit.

## The clause

§7.4.1 states the reader's obligation in one sentence, and the sentence has two halves:

> A PDF reader shall invoke the corresponding decoding filter or filters to convert the
> information back to its original form.

A damaged stream is a decode that did the first half and could not finish the second. The filter
*was* invoked; what came out is not "the original form" and cannot be presented as though it were.
The two halves are exactly the two statements trap 5 asks about, and neither may be dropped.

That the prefix is not an invention is settled by §7.4.4.1, which makes the format normative:

> The Flate method is based on the public-domain zlib/deflate compression method, which is a
> variable-length Lempel-Ziv adaptive compression method cascaded with adaptive Huffman coding. It
> is fully defined in Internet RFC 1950 , and Internet RFC 1951 .

Under RFC 1951 every output byte is a function of the input bits already consumed. The bytes
before the failure were emitted by the producer's own compressor from bytes the producer wrote;
nothing is guessed, extrapolated or filled in. This is what separates keeping a prefix from the
thing `CLAUDE.md` forbids — there is no fabrication here, and equally no claim that what follows
the damage can be recovered. Everything after the bad distance code is gone and stays gone.

**Every filter with an end-of-data marker states one**, which is what makes "stopped short"
checkable rather than a guess: §7.4.4.2's code 257 for `LZWDecode`, §7.4.5's "[a] length value of
128 shall denote EOD" for `RunLengthDecode`, RFC 1951's final block for `FlateDecode`. Reaching
the end of the input without seeing it is a statement about the *file*, and §7.3.8.2 is where that
statement lands — `/Length` "indicates how many bytes of the PDF file are used for the stream's
data" and "[a]ll of these constraints shall be consistent", which a truncation makes false. That
is the same argument `Document::states_no_data` already turns on (ADR 0266), one clause over.

**§7.4.3 is the counter-example that shows the rule is not "salvage everything".**
`ASCII85Decode`'s partial final group is not damage but the clause's own encoding, and a
character outside the grammar "shall cause an error" — so that filter keeps no prefix at all, and
should not. What decides each filter is its own clause, not a general appetite for recovery.

## What was actually wrong

Two defects, both silent, and neither is the one §8 predicted.

**The recovery said nothing.** A partly-decoded stream came back through
`Document::decoded_stream_data` as an `Arc<[u8]>` indistinguishable from a whole one. This is
trap 5 in its plainest form — a fallback that renders something plausible — and it is the failure
mode the trap says is easiest to lose *inside* a partly-implemented feature, because the code path
exists and works. The sharpest witness is not `498264.pdf` at all but
`govdocs1-error-pdfs/error_set_2/507676.pdf`, whose page-one content stream is corrupt and which
this tree has been drawing **33 854 commands** from, out of 67 923 recovered bytes, in silence.

**And the recovery was unreliable, by an accident of buffer boundaries.** `flate` drove
`flate2::read::ZlibDecoder` through `read_to_end`, and the `Read` adapter discards whatever the
erroring `read` call had already produced. So the prefix survived only as far as the last *whole*
call: on `498264.pdf`'s ICC profile that is 1024 bytes of a partial profile, and on its content
stream — whose 18 bytes and whose failure fall inside one call — it is nothing at all. That is why
the file reported `Undecodable` while the module's documentation promised a prefix. The adapter
also cannot distinguish RFC 1951's final block from an input that merely ran out: both arrive as
`Ok`, so **a truncated stream was indistinguishable from a complete one** and always had been.

## Decision

**A damaged stream keeps what it decoded and says that it did.** The prefix-draw is right, on
§7.4.1's two halves; the silence was not.

- `filter::Decoded` carries the bytes and a `Damage` — `Truncated` where the input ended before
  the filter's own end-of-data, `Corrupt` where the data violated the format at a definite point.
  `FlateDecode`, `LZWDecode` and `RunLengthDecode` all report it; `ASCIIHexDecode` and
  `ASCII85Decode` have nothing to report, per §7.4.3 above.
- `flate` is driven through `flate2::Decompress` rather than the `Read` adapter, which is what
  makes both facts available: bytes decoded in the erroring call are kept, and `Status::StreamEnd`
  is distinguishable from an exhausted input. The bound's "one byte past" ceiling (ADR 0306) is
  unchanged, and so is the zlib-then-raw-deflate fallback.
- `Document::decoded_stream_data_reported` carries the damage up, memoised in the decoded-stream
  cache beside the bytes — because a report that depended on whether the cache still held the
  entry would be a report that depends on a budget.
- `ContentIssue::Damaged` is what a page says, and it is **the eighth place this tree reports
  while drawing**. Trap 5's test is met in both directions: suppress the drawing and the page
  loses marks its producer's own compressor emitted and nothing else in the file can supply;
  suppress the report and a page that was cut short is indistinguishable from a page meant to be
  sparse.

**`poppler`'s three lines are not what this implements, and the distinction matters.** Our
recovery of `498264.pdf` is 18 bytes and no drawing command — the heading `poppler` shows is
*past* the invalid distance code, so recovering it requires resynchronising a broken deflate
stream, which is a guess about bits nobody wrote. Agreement with `poppler` was never the target
(principle 5); the clause was, and on this file the clause buys a report rather than a mark. That
is the honest outcome and it is written down as one.

## Consequences

**The `/Contents` route is loud and the rest is not, and the rest is now countable.**
`examples/damaged_stream_census` measures both: page one's `/Contents` (what the rule buys) and
every stream object in the file (how wide the silence was). Sixty-one callers take
`Document::decoded_stream_data`, which drops the damage by design — a partial ICC profile, font
program or image still reaches the code that reads it with nothing said. That is unchanged
behaviour rather than a new silence, but it is a silence, and `doc/todo/03` §8 records it with its
witnesses rather than leaving it implied.

**`Decoded` is a struct where an `Arc<[u8]>` was**, on `decode_reported`,
`decode_with_parms_reported` and `decoded_stream_data_reported` — three reported forms, one
caller each outside the crate. The `Option`-returning `decode`, `decode_with_parms` and
`decoded_stream_data` are untouched, which is what keeps the change off sixty-one call sites.
