# ADR 1183 — Table 13's `/ColorTransform` is read, and one reading stays a departure

Status: accepted, 2026-09-22.

Builds what ADR 1177 left owed and amends ADR 0036, which decided the original departure. Both
stand as records; what this one changes is `decode_jpeg` and §7.4.8's row.

## What ADR 1177 left owed, and what it cost to pay

ADR 1177 moved §7.4.8 from `departed` to `partial` on a census: of 792 516 `DCTDecode` images over
65 944 crawled documents, **158 over 17 documents** are in the case Table 13's entry decides, and
**none of them would draw differently if it were read**. Its closing sentence named the debt —
"`decode_jpeg` reading the entry out of `/DecodeParms`" — and priced it at one `if`.

It is one `if` in the sense that mattered: no page moves. It is not one `if` in the code, and the
reason is worth writing down, because it is the shape of every "just read the entry" debt.

**`zune-jpeg` reports the marker's value and its own defaults through the same channel.** Table 13
ranks the Adobe `APP14` segment above the dictionary entry above the clause's default, so a reader
has to be able to tell the three apart. `JpegDecoder::input_colorspace()` cannot: a three-component
frame is `YCbCr` whether the marker said transform 1 or there was no marker at all, and a
four-component one is `CMYK` whether the marker said transform 0 or said nothing. So the marker
itself is read here — `image::adobe_transform` walks ISO/IEC 10918-1's header segments to the first
scan, the way `defined_number_of_lines` and `filter::jpeg_extent` already do, because an `APPn`
segment may carry a whole second JPEG whose own `APP14` is not this image's.

**The decoder has no "do not transform" switch either.** What it has is a copy: an input colour
space equal to the output one is copied channel for channel. So "no transformation" is spelt by
asking for the space the frame was read *in*, and the transform the clause names is applied here
where the decoder would not have applied it — `ycck_to_cmyk` for four channels, which already
existed, and `ycbcr_to_rgb` for three, which is new and shares `jfif_inverse` with it so that
ITU-T T.871's inverse has one implementation in this crate.

**A three-component frame's `input_colorspace()` is provisional, and echoing it is the defect ADR
0266 already paid for once.** Adobe's transform 0 maps to `CMYK` at the marker and is corrected to
`RGB` only once the frame header has been read, so asking for it back as the output space asks a
three-channel decode for four channels — `Unimplemented colorspace mapping from RGB to CMYK`, the
error that lost 21 whole photographs in session 430. The untouched space is therefore *named*
rather than echoed: `RGB` where the decoder's reading is not `YCbCr`, `YCbCr` where it is.

## The ranking, and where it ends without an answer

`image::colour_transform` is the clause's three cases in the clause's order, and it answers
`Option<bool>` because the ranking can end without an answer:

- **The marker wins outright**, and its value is *Adobe's* rather than Table 13's. §7.4.8 defers
  to Adobe Technical Note #5116 for the markers, which this project does not hold; what the tree
  knows about that namespace it knows from ADR 0203's reading of four-component codestreams and
  from the decoder — three codes, where Table 13's entry has two. The argument does not need the
  note. Table 13 defines the entry's values as 0 and 1 and nothing else, and every reader treats
  the marker's 2 as YCCK, so reading the marker's byte *as* a Table 13 value would leave the YCCK
  case undefined. The two namespaces are therefore not the same, and this tree does not conflate
  them. A code naming a channel count the frame does not have — 1 on four components, 2 on three —
  states nothing about *this* frame, and the answer is `None`.
- **Then the entry**, whose values *are* Table 13's and are stated in terms of the component count.
- **Then the default**, and this is the one place the answer is `None` where the clause states one.

## The one departure, stated plainly

The clause's default case is unconditional: "the default value of ColorTransform shall be 1 if the
image has three components and 0 otherwise". This tree does not apply the 1 to a three-component
frame whose component identifiers are the ASCII letters `R`, `G`, `B`; it takes the codestream's
own declaration instead, which is what `zune-jpeg` infers.

The argument for that is session 234's and is in `oracle.rs`'s `AMBIGUOUS_JPEG_COMPONENT_IDS`: no
clause of ISO 32000-2 and none of ISO/IEC 10918-1 gives a component identifier any meaning — the
convention is `libjpeg`'s — and §7.4.8's closing `shall` binds a producer, so a codestream carrying
neither the marker nor `YCbCr` data is outside what the clause describes. The cost is one page:
`issue11931.pdf`'s band, where ours is 1.357 of 255 and `ghostscript`, which obeys the default, is
**8.831** — six and a half times the page's ink — and where `poppler`, `mupdf` and `hayro` all do
what we do.

**What this ADR decides is that it is a departure and not a silence**, which is the half ADR 1177
did not have to settle because a different debt kept the row `partial`. The clause states an answer
for that file; we give another one; three references agreeing with us is evidence about our reading
and never the definition of right (principle 5). So the row is **`departed`**, whose word means
every requirement executed except the one the note names, decided against with its cost recorded
(ADR 1119) — and this ADR is the argument the note names.

The alternative was `implemented`, on the reading that the file is outside the clause and the
question is therefore robustness rather than coverage. It is rejected here for one reason: the
requirement is addressed to the **processor** and is not conditioned on the file conforming, so a
row claiming `implemented` would be claiming that a conformance reader has nothing to look at. A
later round that wants the other answer has to argue against this paragraph rather than around it.

**The departure is now a fixture rather than a sentence.**
`tests/colour_transform.rs::with_neither_the_component_identifiers_decide_which_is_the_one_departure`
holds it, so a round that changes it is told.

## What moved besides the entry

The identifier reading used to outrank *everything*, because it is applied by the decoder at the
start-of-scan marker and nothing above it could speak. It now applies in the clause's third case
alone: a marker stating 0 or 1 over three components is obeyed whatever the identifiers say, and so
is the entry. No producer is known to write `R`, `G`, `B` identifiers beside an `APP14` stating
transform 1 — the two contradict each other — but "no producer writes it" is not a reading of the
clause, and the clause says the marker decides.

## What it costs

Nothing measurable, and the census is why the claim is affordable. `adobe_transform` is a walk over
header segments only, stopping at the first scan, on a path where `frame_as_defined` already walks
the whole codestream. The behaviour changes for three populations, all of them tiny: the 158 images
the entry decides (which agree with the codestream, so no pixel moves), a three-component frame
whose entry disagrees with the codestream (none in the crawl), and the marker-against-identifiers
case (none known).

`tests/colour_transform.rs` is sixteen scenes over a generated 8×8 codestream with one DC-only
block per component — the fixtures are generated because a real witness for most of these cases
does not exist (trap 8) — and five of them fail if the ranking is removed.
