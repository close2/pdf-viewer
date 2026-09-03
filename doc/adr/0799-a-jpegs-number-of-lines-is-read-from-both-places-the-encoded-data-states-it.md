# ADR 0799 — A JPEG's number of lines is read from both places the encoded data states it: a `DNL` marker at the end of the first scan defines or redefines the frame header's `Y`, and the decoder is handed the count rather than the placeholder

Status: accepted. Session 878.
Clauses: ISO 32000-2 §7.4.8; ISO/IEC 10918-1 section B.2.2 (frame header, `Y`) and section B.2.5 (`DNL`).
Code: `crates/pdf-model/src/image.rs` (`defined_number_of_lines`, `frame_as_defined`,
`decode_jpeg`, `contradicted_frame`).
Tests: `crates/pdf-model/tests/dct_components.rs::a_dnl_marker_defines_or_redefines_the_frames_number_of_lines`,
and `doc/checks/fixed-documents.toml`'s row for `poppler-61994-0.pdf`.
Opened by `doc/todo/03` section 41, the `batch5/poppler` chunk, whose ink ranking this page heads.

## Context

`poppler-61994-0.pdf` is a scanned business letter, one `DCTDecode` image over the page,
dictionary `/Width 2480 /Height 3473`. Ranked by our ink against the lighter reference over every
incomplete page of the 1586-document directory, it is the head of the chunk by a factor of
fifteen in the direction nobody expects — **ours 60.4 where `poppler` draws 5.4 and `mupdf` 5.0**
— and the two references agree with each other to a third of a level, which is trap 9's strongest
shape. The page, looked at (trap 1): the letter squeezed into the top five per cent of the sheet
and flat grey beneath it. The survey's report said why in its own words: *the JPEG frame is
2480x65535 where the dictionary says 2480x3473 (§7.4.8 puts the dimensions in the encoded data);
its samples are drawn on their own grid*.

A marker walk of the codestream says what the frame actually is. `SOF0` states `Y = 65535`; a
`DRI` sets restart intervals; one scan follows; and after the scan's entropy-coded data, before
`EOI`, stands a **`DNL` marker segment stating `NL = 3486`**. ISO/IEC 10918-1 gives the encoded
data two places to state its number of lines. section B.2.2's frame header carries `Y`, whose value 0
means the count is to be defined by a `DNL` at the end of the first scan; section B.2.5's `DNL` segment
"provides a mechanism for defining or redefining the number of lines in the frame" there, and
shall be consistent with the MCU rows the first scan encoded. This scanner wrote `65535` where
it did not yet know the page length and the count after the data, and 3486 lines is 436 MCU
rows, which is what the scan holds.

So the encoded data states 2480 × 3486, thirteen rows more than the dictionary's 3473 and
nowhere near 65535. §7.4.8 is where ISO 32000-2 sends a reader for the dimensions:

> The values of these parameters, which include the dimensions of the image and the number of
> components per sample, are entirely under the control of the encoder and shall be stored in
> the encoded data. DCTDecode may obtain the parameter values it requires directly from the
> encoded data.

This tree's reading of that sentence — the codestream's grid is drawn and a contradicting
dictionary is reported beside it (ADR 0340) — was right and stays. What was wrong is *which
number the codestream states*: `zune-jpeg` sizes the frame from the header alone. Before the scan
it refuses a `DNL` outright ("Parsing of the following header `DNL` is not supported, cannot
continue"); after the scan's data it meets the marker where it expects more MCUs and pads the
header's grid — 65535 rows, of which 3486 are the letter and the rest are the decoder's grey. Both
reference renderers happen to draw this page right for a reason that is not the clause's: their
`libjpeg` ignores `DNL` as well, and they size the image from the *dictionary* and pull 3473 rows
of samples from the decoder, which is the reading §7.4.9 states for JPEG 2000 and §7.4.8 does not
state for JPEG. Agreement with them is evidence about the picture; the clause is what decides the
number, and the clause's number is the `DNL`'s.

## Decision

`image::defined_number_of_lines` walks the codestream's markers without decoding — the same
byte-stuffing argument `pdf_syntax`'s `jpeg_extent` rests on makes the end of entropy-coded data
findable — and answers, for a codestream whose first scan is followed by a `DNL`, the offset of the
frame header's `Y`, the `DNL`'s `NL`, and the segment's own byte range. Nearly every codestream
answers `None` at the cost of one header walk, which `contradicted_frame` was already paying.

`image::frame_as_defined` hands the decoder the codestream *as the encoded data defines it*: a
copy with `NL` written into `Y` and the six-byte `DNL` segment taken out, because a decoder that
refuses the marker cannot be left to meet it. A codestream with no `DNL` is borrowed untouched. Both
`decode_jpeg` and `contradicted_frame` read through it, so the report beside a contradicting
dictionary names the count the encoded data states — *the JPEG frame is 2480x3486 where the
dictionary says 2480x3473* on the witness, which is true, and which this tree draws on the
codestream's grid exactly as ADR 0340 decided.

The three cases 10918-1 admits are pinned by one test over the hand-written baseline frame
`dct_components.rs` already had: `Y = 65535` redefined to 8, `Y = 0` defined as 8, and `Y = 8`
with a `DNL` stating the same — each drawn as 8 × 8 with the scan's samples and nothing reported,
and a fourth run whose dictionary says 8 × 9 reports the `DNL`'s 8. Run against the tree without
this, the first case draws 8 × 65535 and reports the decoder's refusal, which is trap 13's control.

## Consequences

- The witness draws the letter: **5.24 levels of ink** by the ranking's instrument, between
  `poppler`'s 5.38 and `mupdf`'s 5.03, from 60.4. It is a row in `doc/checks/fixed-documents.toml`
  with the gate's own reading as its band.
- A frame header stating `Y = 0` — the form 10918-1 names for "defined by `DNL`" — decodes,
  where `zune-jpeg` refused it at the header ("Image width or height is set to zero").
- A codestream with a `DNL` costs one copy of itself, six bytes shorter; one without costs one
  marker walk it already paid for.
- Not done, and said so: `zune-jpeg` itself still does not read `DNL`. The right home for this is
  the decoder, and a patch upstream would let `frame_as_defined` go; this tree does not pin a
  fork of `zune-jpeg` the way it does `hayro`, so the reading lives here with the clause beside
  it until then.
- §7.4.8's ledger row carries the reading; its status is unchanged (`partial`, for Table 13's
  `/ColorTransform`, which this touches nothing of).
