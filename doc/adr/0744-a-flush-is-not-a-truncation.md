# ADR 0744 — A flush is not a truncation

Status: accepted, 2026-08-28. Session 812. Amends `pdf-syntax/src/filter.rs` — `inflate`,
`Inflate::turn` and `Damage::Truncated`'s own doc comment — and closes `doc/todo/18`, whose
diagnosis and measurement this decision is built on. Successor to ADR 0343, which gave a
short decode a value to be reported by, and to ADR 0587 and ADR 0427, which are why the
witnesses take the windowed route.

## The subject

Three pdf.js corpus documents report

```
DamagedContentStream { detail: "a form XObject /Form (§8.10)", damage: Truncated, kept: 851 }
```

and **no mark is missing from any of the three**. Their form `XObject`s end

```
… 6d d4 0f 00 00 00 ff ff
```

`00 00 ff ff` is the `LEN` and `NLEN` of RFC 1951 section 3.2.4's *non-compressed* block with
`BFINAL` clear and `LEN` zero — what `zlib` writes for a `Z_SYNC_FLUSH`. The producer flushed
and never called `deflateEnd`, so the stream carries no final block and no RFC 1950 checksum.
Inflating it yields the whole of the form's content and nothing after it.

`Damage::Truncated`'s own words — the encoded data ran out before the filter's end-of-data
marker — are *true* of such a stream and are **not what the report is for**. Trap 11 exactly: the
report exists so that a page cut short does not look like a page meant to be sparse, and this
page is not cut short. ISO 32000-2 §7.4.1 asks a reader to "invoke the corresponding decoding
filter or filters to convert the information back to its original form", and this decode reaches
it; what never arrives is a *declaration* that there is no more, and a declaration carries no
marks. So the answer is a whole `Decoded`, not a third `Damage`.

## Why the tail bytes are not the test

"The last bytes are the flush marker" is a heuristic. Those bytes can be the tail of a Huffman
block's bits, or the data of a stored one — in which case the stream really did stop mid-block
and data really is missing. This project prefers a decidable test to a small probability, and the
calibration below is a stream that would fool the heuristic.

## The decidable test

A deflate stream that ended on a *completed* block is one final block short of whole. So feeding
a decoder in that state RFC 1951 section 3.2.4's final **empty** stored block —

```
01 00 00 ff ff        # BFINAL = 1, BTYPE = 00, LEN = 0, NLEN = 0xffff
```

— must make it report `StreamEnd` and write **no further output byte**. A decoder stopped inside
a block cannot reach `StreamEnd` from those forty bits without emitting something, because
anything it could still emit comes from bits that are not there; and if it reaches `StreamEnd`
emitting nothing, the only symbols it consumed were an end-of-block, which carries no data. Both
directions hold, and the test needs no new dependency and no arithmetic of this project's own.

## The one thing that blocked it, and which of the three ways out was taken

Under RFC 1950's framing the decoder wants four bytes of Adler-32 after the final block, so the
probe fed to the *live* decoder answers `Ok` rather than `StreamEnd` and cannot be read.
`doc/todo/18` priced three ways out; the second is taken.

1. **Compute the Adler-32 as the decode goes** and append it to the probe. Exact, and it works in
   both routes — and it puts a per-byte checksum on the hot inflate path of every stream in every
   document in order to answer a question about 0.03% of them. `flate2` exposes no accessor for
   the running checksum the decoder is already computing, so this is a *second* pass over the same
   bytes, not a shared one. Refused on principle 2 without a measurement, and not worth a
   measurement at that ratio.
2. **Probe with a throwaway raw decoder over the same input**, minus the two-byte zlib header,
   where there is no checksum to satisfy. Costs one extra inflate, on the damaged path, and only
   where the tail marker is present — nothing at all for every other stream. **Taken.**
3. **Strip the zlib header and always inflate raw**, giving up the checksum outright. The
   cheapest and the one that loses something real: a corrupt-but-grammatical stream would then
   decode in silence, which is trap 5 pointing the other way.

`ended_on_a_block` is route 2 as one function, used by both routes. The output of the replay is
thrown away a fixed scratch buffer at a time, so a decompression bomb whose tail carries the
marker costs the buffer rather than its decode; the tail bytes decide only whether that one extra
inflate is worth paying, never the answer. A stream whose header sets `FDICT` is refused rather
than replayed — the raw decoder would want the dictionary the zlib framing named — and the
`FLUSH_MARKER` this all keys off is **four** bytes and not five: the flush pads to a byte
boundary, so what stands in front of `LEN` is the last bits of the terminated block and is `00`
only where they happened to be zeros — the five-byte form `doc/todo/18` scanned for is the common
case, not the marker. What the worst case costs is worth stating, because principle 3 asks: the
probe runs **once** per stage, after that stage has already ended, so a hostile stream ending in
the marker bytes buys its author a second inflate and nothing else — twice the processor time of
the decode it had already paid for, at 8 KiB of memory.

## Where it is asked, and where it is not

- **The buffered route** asks it in `inflate`, of every `FlateDecode` that stops short — but
  **not of a decode that produced nothing**, because that answer is `flate`'s cue to try the
  other framing and this must not take a raw deflate stream's fallback away.
- **The windowed route** asks it in `Inflate::turn`, *after* the zlib-then-raw rewind and never
  in front of it, for the same reason. The stage needs its whole encoded input to replay, and
  `Pump` holds exactly that for the **first** stage of a chain: `Inflate::replayable` is an `Arc`
  clone of the pump's own buffer, so the cost is a refcount per pump and no per-byte work
  anywhere.
- **A `FlateDecode` behind another filter in the windowed route keeps the old answer**, and this
  is the decision's one stated remainder. A later stage's input is a `LINK`-byte window of the
  stage in front of it, and reconstructing it would mean replaying the chain's prefix. The
  population is a chain whose second or later stage is a deflate *and* whose producer flushed
  without finishing; the corpus holds none, and the whole-buffer decode of the same stream answers
  correctly. Written down in `Inflate::replayable` rather than left to be rediscovered.
- **`decoded_extent` does not ask it either**, and that asymmetry is the question that function
  asks: it looks for where a filter's own end-of-data marker stands, and a flush marker is not
  one. `EncodedExtent::Short` — these bytes ran out before one — stays true.

## It is more than a report, and that is why this is a pixel-reaching round

`Decoded::damage` is not only what `viewer_core::report` prints. Four consumers branch on it, and
each of them is right to: `pdf_font::whole_program` refuses a font program that is a directory
describing bytes that are not there; `colour.rs` declines to parse a damaged ICC profile and falls
to Table 65's `/Alternate`; `Document`'s object-stream reader declines to take the last object of
a prefix, because a truncated token parses; and `content::reader` carries the value up. A stream
this decision reclassifies is one every four of those now treat as the whole thing it is — which
is correct, and is why the full §2 sequence is owed rather than the core.

## What it is measured at

`examples/damaged_stream_census` over the whole pdf.js corpus, before and after: **48 damaged
streams in 11 documents becomes 41 in 8**, damaged form `XObject`s 6 → 1, damaged cross-reference
streams 2 → 0, and reports naming damage 10 over 4 documents → 6 over 1. The corpus gate's
incomplete population falls by exactly the three documents, and the mechanism *one of §7.8.2's
other content streams, drawn as far as its damage* leaves its printed composition altogether.
`MAX_INCOMPLETE` is a ceiling rather than a ratchet and is left where it is; the gate prints the
population. The session's record has the tables.

**Three pages stop being *incomplete*, and that is the other half of the price.** The oracle's
undiagnosed check is over `complete && Ambiguous`, so a reporting page owes no diagnosis: their
verdict was `ambiguous (incomplete)` and is `ambiguous`, and two of them go back into
`AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` with the diagnosis that has described them all along. What a
report firing on a condition the clause does not state costs is therefore not a verdict but an
*explanation*, and a page exempted from explanation cannot be found to want one.

## The tests, and their calibration

Two in `filter.rs`'s own module, because the mechanism is the decoder's and a document is a slower
way to ask it; and two on real documents in `pdf-model/tests/damaged_content_streams.rs`, because
what the decoder decides is a claim about files.

- `a_stream_flushed_and_never_finished_is_whole` builds the witness with the encoder the
  producers used — `flate2::Compress` with `FlushCompress::Sync` — at two compression levels,
  because those are two different last blocks: `best` leaves a Huffman block for the flush to
  terminate, `none` a stored one. Both routes, and the windowed one at three window sizes.
- `a_stream_cut_inside_a_block_that_ends_in_the_marker_is_still_truncated` is the calibration the
  probe exists for. A stored block carries its data verbatim, so a payload holding the marker and
  cut immediately after it is a stream whose last four bytes are the flush marker and whose data
  really is missing.
- `the_corpus_witness_turns_out_to_be_a_flush_and_says_nothing` is `comments.pdf`, which that file
  used to assert the other way round.
- `a_corpus_document_that_really_cuts_a_glyph_description_short_names_it` is what replaces it, and
  finding it corrected a second sentence: `damaged_content_streams.rs`'s header said no corpus
  document carried a damaged Type 3 glyph description, and `poppler-90-0-fuzzed.pdf` page 10 has
  carried one the whole time. The claim was written when the only corpus damage anybody had
  looked at was the form that turned out to be a flush.

Calibrated per trap 13, above two checkpoint commits, with three plants: the probe answering
`false` always fails the two flush tests and neither other; the probe answering on the tail bytes
alone fails only the stored-block calibration; the probe answering `true` always fails only the
glyph-description witness. The four `RunLengthDecode` fixture pairs in the same file are untouched
by all three, so no catch-all arm is swallowing the case.
