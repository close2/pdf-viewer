# ADR 0823 — A JBIG2 stream is decoded from the segments it carries whole, and the byte a `/Length` counted is not one of them

Status: accepted. Session 889.
Clauses: ISO 32000-2 §7.4.7 (`JBIG2Decode`), §7.3.8.1 (stream syntax, the marker before `endstream`),
§7.3.8.2 (stream extent); ISO/IEC 14492 Annex D.3 and 7.2, 7.4.8.5.
Code: `crates/pdf-sandbox/src/decode.rs` (`jbig2`, `up_to_the_last_whole_segment`,
`whole_segments`, `segment_end`, `shortfall_past_the_last_segment`, `shortfall_sentence`,
`PackedRows::note_damage`), `crates/pdf-model/src/image.rs` (`decode_jbig2`, `samples_of`).
Tests: `crates/pdf-sandbox/src/decode.rs::tests` (six), and
`doc/checks/fixed-documents.toml`'s row for `PDFIUM-1236-1.pdf`.
Opened by `doc/todo/03` section 43, the `batch5/PDFIUM` chunk, which named this document as
"the same question ADR 0794 answered for CCITT, one filter over".

## Context

`PDFIUM-1236-1.pdf` is one page of a light grey schematic, 2448 × 1584, whose whole content is
one `JBIG2Decode` image 3562 × 851 under an `/SMask`. This tree drew the page **blank** and
reported `Xop2: JBIG2: unexpected end of input`; `poppler` and `mupdf` both draw the diagram and
agree on 1.463 levels of ink to a thousandth.

**The stream is not truncated, and that is the finding.** Its 95 declared bytes are a 30-byte
page information segment (3562 × 851, default pixel value 0) and a 64-byte immediate generic
region covering all of it — 94 bytes of ISO/IEC 14492 Annex D.3's embedded organisation, whole,
with the terminating `FF AC` in place. The ninety-fifth byte is a LINE FEED, and it is the
end-of-line marker between the data and the `endstream` keyword. §7.3.8.1 says what that byte is
and what a length may do with it:

> There should be an end-of-line marker after the data and before endstream ; this marker shall
> not be included in the stream length.

The producer counted it. `pdf-syntax` therefore hands the filter 95 bytes, the codec finds one
byte where a segment header must begin, and refuses the whole picture.

### What the clauses require of a reader here

**§7.4.7 states nothing about damaged data at all**, and its silence is different in kind from
§7.4.6's. The fax filter has a sentence about it — "[t]he filter shall not perform any error
correction or resynchronization" — and a Table 11 parameter, `/DamagedRowsBeforeError`, whose
default of zero is what ADR 0794 read. Table 12 has one entry, `/JBIG2Globals`, and there is no
counterpart to either. What §7.4.7 does state is the *shape* of the data: "JBIG2 bit streams
shall be represented in sequential organisation as defined in ISO/IEC 14492:2019, D.1", with the
file header, the end-of-page segments and the end-of-file segment absent. So the stream is a run
of segments, each a header stating its own data length followed by that many bytes, and where
that run ends is a fact the filter can read off the bytes rather than a guess.

**The rest of the page is stated, and this is where the answer parts from CCITT's.** ADR 0794
left a damaged fax's undelivered scan lines *unpainted*, because what they show "is stated
nowhere" — and the first build of that change drew them the filter's white, which under
`/BlackIs1 true` is the page's black. JBIG2 has no such gap: 14492's page information segment
carries a default pixel value, defined as "the initial value for every pixel in the page, before
any region segments are decoded or drawn", and the page bitmap is that value before the first
region is composited onto it. A JBIG2 page therefore has no undelivered rows at all — every row
of the grid comes back — and what is short is the *regions that reached it*. There is nothing to
leave unpainted and no colour for this reader to choose.

## Decision

**A `JBIG2Decode` stream the codec refuses is decoded from the prefix that is whole segments,
and the bytes after it are reported unless they are §7.3.8.1's end-of-line marker.**

Three parts:

- `whole_segments` walks 14492 7.2's segment headers — the segment number, the flags whose bit 6
  widens the page association, 7.2.4's referred-to count in its short and long forms, 7.2.5's
  referred-to numbers sized by this segment's own number, the page association, the data length —
  and returns the offset just past the last segment that is whole. It interprets no segment's
  contents. A header it cannot bound, including 7.2.7's `0xFFFFFFFF` unknown length that is
  delimited by scanning the segment's own data, stops the walk where it begins.
- `jbig2` runs that walk **only where `Image::new_embedded` has already refused the whole
  stream**, and retries on the prefix. That placement is what makes the walk safe: a prefix
  shorter than the truth costs a stream that was not going to decode anyway, and where the retry
  also fails the codec's own first refusal is what the page reports.
- What the dropped bytes are decides whether anything is said. One end-of-line marker — LF, CR
  or CR LF — is §7.3.8.1's, "not included in the stream length" by a `shall not`, so nothing is
  missing from the picture and nothing is reported. Anything longer is a segment the stream does
  not finish: `Bilevel::stopped_by` carries the sentence out of the worker and
  `pdf_model::image::Parts::shortfall` reports it beside the drawing, through the raster cache,
  exactly as ADR 0794's does. And where the trimmed stream then fails to *decode*, the refusal
  says both things, because the segment-level refusal is the more specific of the two and would
  otherwise replace the fact that the stream does not finish.

**Why this belongs to the filter and not to `pdf-syntax`.** The obvious place for a rule about
§7.3.8.1's marker is where stream data is delimited, and half of it is already there:
`find_endstream` trims one end-of-line sequence, because on that path the length is *derived*
and the byte belongs to the delimiter (ADR 0366 has the other half of that story, a round that
trimmed it on a path where the file had stated the length). Where the file states a length that
reaches the byte, that trim cannot be made: the stream is then either a file that omitted the
marker — §7.3.8.1 says *should*, so that is allowed — or a file that counted it, and from the
syntax alone the two are the same bytes. The filter is the only place with the evidence: 14492's
segments each state their own length, so a byte after the last whole one is not data, and there
is nothing else it could be.

**The count `delivered` keeps its meaning and does not move.** For CCITT it is the scan lines
before the damage and `image::leave_unpainted` clears the grid past it. For JBIG2 it is the
height, always, because the grid is whole; `PackedRows::note_damage` is `stop_short`'s other
half and says so.

## What it is worth, measured

The population is **every document naming `JBIG2Decode` in cleartext under `corpus-cache/`,
`doc/pdf.js/test/pdfs` and `doc/corpora/` — 1523 of them** (a name inside a compressed object
stream would not be found, so this undercounts). Surveyed whole before and after, under
`tools/bounded.sh --data 8 --tree 12`:

- **43 incomplete before, 42 after**, and nothing regressed: no document went from complete to
  incomplete and no report outside this filter moved.
- `PDFIUM-1236-1.pdf` **is complete**, draws the diagram, and reports nothing. Its ink at 72 dpi
  is **1.519** against `poppler` 1.463 and `mupdf` 1.463 — the page was **0**, blank, before.
- **Five documents reported `unexpected end of input`, and only that one was §7.3.8.1's marker.**
  The other four carry no whole segment to retry with, so the codec's own refusal stands, which
  is the JBIG2 form of ADR 0794's "not one whole scan line": `REDHAT-488553-0.pdf` and
  `poppler-28170-0.pdf` share a stream whose *first* segment declares 318 767 114 bytes over
  6871 present, `GHOSTSCRIPT-695315-0.pdf`'s first declares 54 082 829 over 703, and
  `GHOSTSCRIPT-693285-1.pdf` has a whole image stream beside a `/JBIG2Globals` truncated inside
  its one symbol dictionary — 17 259 bytes declared, 11 222 present. That last one is why the
  globals take the same route as the image: it did not move when only the image's stream was
  trimmed, and it is the reason the trim is applied to both.
- **Seven documents' refusals became two-part and more specific.** A parse-level `unknown or
  reserved segment type` or `segment refers to larger segment number` — a judgement on the
  trailing garbage — becomes the codec's judgement on a segment that is whole, with the
  truncation named beside it: `img1: JBIG2: a symbol dictionary has too many symbols — and the
  stream ends inside a segment, 58964 bytes after the last whole one`. That is what the two-part
  message exists for; without it the change would have replaced the first thing wrong with the
  second.

**No corpus document exercises the successful shortfall branch** — a stream that ends inside a
segment *after* one that decodes. Trap 13, so the witness is built rather than found, and built
out of the real file: `a_stream_ending_inside_a_segment_draws_the_segments_before_it` cuts
`PDFIUM-1236-1.pdf`'s own stream twenty bytes into its region segment, and the page comes back
3562 x 851 in its default pixel value with the sentence beside it. `whole_segments` is run
against every truncation of that stream, byte by byte, which is the other half of the same trap,
and a sixth test puts the marker on the globals side alone.

## Consequences

- `doc/todo/03` section 43's entry for this document is answered, and its description of the
  stream as truncated is corrected — the stream is whole and the `/Length` is one too long.
- The `/JBIG2Globals` stream takes the same route, because `new_embedded` parses it in the same
  call and one badly delimited stream of the two refuses both.
- §7.4.7's row stays `implemented`: nothing in the clause is owed by this. §7.3.8.1's row gains
  the reading, and the recovery path's inclusion of the marker is recorded there as a known
  second defect that no filter is currently harmed by.
- **Not done, and deliberately.** The walk stops at 7.2.7's unknown data length rather than
  scanning for its terminating pattern, so a stream whose damage follows such a segment is
  trimmed back further than it need be. It costs nothing today: no document in the population
  reaches that case, and where the trim is too short the codec's own refusal is what the page
  reports, which is what it reported before.
