# 1271 — A photograph on the way into a page turn: a scan that reads nothing, and a widening
done twice

Session 1217. Status: **accepted**. Changes `pdf_model::image`'s `DCTDecode` route and nothing
else. Takes the first of the two items [ADR 1260](1260-what-a-frame-costs-stage-by-stage-and-the-two-things-that-made-the-first-answer-wrong.md)
section 5 left, on the witness that ADR's table names.

## 1. What was measured, and with what

`examples/callgrind_interpret doc/pdf.js/test/pdfs/issue12841_reduced.pdf 1` — one interpretation
of a page that is one five-megapixel photograph, counted in instructions rather than timed, so the
attribution is not a claim about this machine's load. Three arms, in this order:

| arm | Ir | of the first |
|---|---|---|
| as ADR 1260 left it | 1 138 261 373 | — |
| the scan over the entropy-coded data read eight bytes at a time | 1 088 694 681 | −4.36% |
| and the decoder asked for the raster it is about to be widened into | **954 253 634** | **−16.16%** |

Wall clock beside it, arms alternated in one sitting, the minimum of nine fresh processes each,
load average 6.56 either side (the whole example: read 5.85 MB, open the document, interpret page
one): **84.14 ms → 75.56 ms, −8.57 ms**. `tools/state.sh frame` prints the same change where
`doc/todo/36` reads it — that page's `interp` was 78.68 ms in ADR 1260's table at load 2.7 and is
**68.43 ms** here at load 5.47, and its `turn` 131.58 → 122.68.

**The wall clock moved further than the instructions did, and that is the shape of the work
removed.** A 20 MB buffer cleared, a 15 MB buffer read and a 20 MB one written, a 5.85 MB
codestream walked: memory traffic is most of what went, and instructions are the wrong unit for it.
Both numbers are here because neither is the whole answer.

`raster_golden` holds 974 documents' first pages by name: **held 974, moved 0**. Nothing this ADR
changes draws differently anywhere in the corpus.

## 2. The scan for a `DNL` marker, and why it is *not* conditional on the frame header

`frame_as_defined` walks every `DCTDecode` codestream to the end of its first scan looking for the
`DNL` segment that defines the number of lines a frame header of `Y = 0` leaves open (ADR 0799).
It was 6.16% of this page. The obvious economy is to skip the walk where the header already states
the count — and ISO/IEC 10918-1 says not to.

Read rather than assumed, in that standard's own words (paraphrased here, because only ISO 32000-2
is quoted in this tree): section B.2.2 gives `Y` the value zero to mean the number of lines is
defined by the `DNL` marker at the end of the first scan, and section B.2.5 says the segment
defines *or redefines* that parameter, is mandatory where `Y` is zero, and may occur only at the
end of the first scan. **Mandatory-if is not only-if**: a codestream whose header states its lines
and which carries a `DNL` as well is one 10918-1 describes, and `zune-jpeg` refuses it outright —
`Parsing of the following header DNL is not supported, cannot continue`, the whole image lost. The
conditional walk was built, and `tests/dct_components.rs`'s third case
(`a_dnl_marker_defines_or_redefines_the_frames_number_of_lines`, the header that already agrees
with the `DNL`) failed exactly that way. It was removed again.

So the cost was taken out of the walk instead of out of its population. Entropy-coded data is
nearly all not-`FF` — 10918-1 section B.1.1.5 has the encoder stuff a zero byte after every one the
coder produces — so the walk asks a question of five million bytes that a few thousand answer.
`next_ff_byte` asks it of eight bytes at a time, with the classic zero-byte word test applied to
the complement of the data, and looks for the lane only in a word that holds one. The walk's
answers are unchanged for every codestream: **70 147 858 Ir → 20 580 908**, 6.16% of the page to
1.89%.

**Two calls, not one**, and the remaining 1.89% is that: `decode_jpeg` walks the codestream and
`contradicted_frame` walks it again to report a dictionary that disagrees with the frame. The
second walk is the cheapest thing left in this file and it is outside this round's hands —
`content/image.rs` is what asks for it, and what it wants is a grid the decode already has.

## 3. The widening the decoder can do itself

The `DCTDecode` route decoded a frame into its components and then widened them into the four-byte
pixels a display list carries: a `vec![255u8; count * 4]` and a pass pairing `chunks_exact`
iterators. That pass was already written for speed once — the benchmark that made it so is in
the comment above it — and was still 10.55% of the page, because what it costs is not branches but bytes:
one buffer cleared and another read. It stays, for the frames the decoder cannot deliver
directly.

`zune-jpeg` holds `YCbCr → RGBA` and `Luma → RGBA` beside the three-channel forms: the same
arithmetic, writing four lanes instead of three, with the alpha byte set. Asked for those, the
decoder's own output *is* the raster, and this crate's second buffer, the pass that fills it and
the pass that clears it all go. The four-lane conversion is also **cheaper than the three-lane
one** — 112 963 680 Ir → 102 922 464 — because packing three-byte pixels costs shuffles that
packing four does not.

The condition is Table 13's, not an appetite for speed. The decoder is asked for the raster only
where it is converting the frame anyway: a frame this crate takes `untouched` is one whose samples
§7.4.8's Table 13 says are *not* the decoder's to transform, and a four-component frame stays four
because its components are `/ColorSpace`'s to interpret. What is left is a `YCbCr` or `Luma` frame
the decoder was already converting to `RGB`, which is nearly every photograph in a PDF.

## 4. The defect this round shipped into its own measurement for ten minutes

**`zune-jpeg` chooses its colour-conversion function while it reads the headers, and `set_options`
afterwards changes only the shape of the buffer that function fills.** `decode_headers_internal`
picks `color_convert_16` from the out colorspace and returns early once the headers are read; the
first form of section 3 asked the same decoder for `RGBA` after `decode_headers()`, so a three-lane
conversion wrote into a four-lane raster — every row three quarters written, its colours walking
along it — and the instrument reported **−15.3%**, because that is genuinely less work.

Two things caught it and one did not. The callgrind profile did: `decode_jpeg`'s cost had vanished
and `ycbcr_to_rgb_avx2_1` was still there, to the instruction, where `ycbcr_to_rgba_unsafe` should
have been. `raster_golden` would have — `issue12841_reduced.pdf` is line 446 of its table.
**The crate's own JPEG tests would not** — their fixtures are one-component, and `Luma → RGBA`
is a match arm rather than that function pointer, so the arm that was wrong is the one no unit
test exercises. The fix is a decoder that has not read the
headers yet, which re-reads a few hundred bytes of marker segments and reads the scan's data once.

The general form, offered for `doc/habits/measuring.md`: **when the faster arm of an A/B also
changes which library function runs, the profile must show the one you asked for.** A library that
takes an option after it has already decided something answers half of it, faster, and a wall clock
cannot tell the difference.

## 5. What is left, with the number

**73% of what interpreting this page now costs is `zune-jpeg`'s own arithmetic**: 525 826 300 Ir of
Huffman decoding (`decode_mcu_block`, 55.1%), 138 M of IDCT (10.9% + 3.6%) and 33.7 M of
upsampling, none of it this tree's and none of it reachable through `DecoderOptions` — the crate is
single-threaded by construction and its SIMD paths are already the ones running. This is where the
contract said to stop, and it is where this stops. What remains on the near side is the second
codestream walk of section 2, and the two costs ADR 1260 put to quorra, which have not moved:
`doc/QUORRA_FEEDBACK.md` section 52.

[ADR 1272](1272-a-page-turn-that-does-not-wait-for-the-photograph.md) prices the other question the
contract asked — whether a page turn must decode the photograph before it presents page one — and
does not take it.
