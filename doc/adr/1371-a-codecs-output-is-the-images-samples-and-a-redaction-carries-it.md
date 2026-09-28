# 1371 — A codec's output is the image's samples, and a redaction carries it

Status: accepted and **built**.
Context: `crates/pdf-model/src/image.rs` (`filter_samples`, `FilterSamples`, `JpxSamples::depths`,
`jbig2_bilevel`, `ccitt_bilevel`), `crates/pdf-sandbox/src/decode.rs` (`jpx_own_integers`),
`crates/pdf-sandbox/src/protocol.rs` (`Raster::depths`, `channel_depths`),
`crates/pdf-transform/src/redact.rs` (`Source`, `Described`, `Walk::plan_codec_clear`,
`Walk::filtered_layout`, `Walk::plan_jpx_stencil_clear`, `Walk::matte_fill`,
`Walk::inline_filter_output`), `crates/pdf-transform/tests/redact.rs`, `doc/todo/64` (deleted).
Supersedes: ADR 1133's re-expression of a colour codec picture as eight-bit `DeviceRGB`; ADR 1333
section 3 (a colour key made into a stencil) and section 4's refusal of a matte outside
`DeviceRGB` and `DeviceGray`; ADR 1333 section 1's refusal of components that disagree on a depth;
ADR 1277's re-expression of an inline image behind a codec. Amends ADR 1242 for the writer's
request only: the viewer's samples are unchanged.

Round 1263 left §12.5.6.23 three codec residues, each "a re-expression nobody has built yet". Read
against the clauses, two of them were one mistake. The third was not a re-expression at all.

## 1. Table 87 already says what a codec delivers

Table 87, of `/BitsPerComponent`: "If the image stream uses a filter, the value of
BitsPerComponent shall be consistent with the size of the data samples that the filter delivers.
In particular, a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a
RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples". A codec is a filter. Its
output is the image's sample data, in the domain the dictionary's `/ColorSpace`, `/Decode`, colour
key and a mask's `/Matte` describe. ADR 1133 decoded all the way to colour and wrote eight-bit
`DeviceRGB`. It called that "deliberate, and cheap", and it was the one choice that left the
domain. Every later refusal on the codec route came from it: the colour key needing a stencil, the
matte kept only where RGB was the identity, and the "shape no fresh raster holds".

So `pdf_model::image::filter_samples` runs the codec as a filter, under the isolation the
interpreter uses, and stops there. `DCTDecode` gives the frame's own components at eight bits,
with Table 13's colour transform undone as the filter's step. The two bilevel filters give their
one-bit rows. The redaction clears those samples in the grid the dictionary states and carries
the dictionary, with only the encoding replaced and `/BitsPerComponent` stated as the filter's.
Nothing outside the region is converted. A `DeviceCMYK` or `ICCBased` photograph therefore no
longer changes colour where it was not redacted.

- **Residue 1, the matte.** Table 144: the `/Matte` numbers "shall be valid colour components in
  that colour space", the parent image's. The parent is now written in that space, so the matte is
  carried as stated in every space. A cleared place takes the matte through §8.9.5.2's map run
  backwards at its own depth. At one bit that is the nearer of the two samples. There are fixtures
  for `CalRGB`, `ICCBased`, `DeviceCMYK`, `Separation`, and a one-bit `DeviceGray` picture.
  `Indexed` still keeps the zero, for ADR 1333's reason.
- **The colour key** is carried, because its ranges are "colour values before decoding with the
  Decode array". The stencil construction is deleted.
- **A `JPXDecode` image mask** is taken as the one-bit samples §7.4.9 requires — "a single colour
  channel with 1-bit samples" — then packed and carried the same way.

## 2. Residue 2: components at depths of their own

§7.4.9: "The colour components in an image may have different numbers of bits per sample, however
bits per sample shall be between 1 to 38 inclusive". Table 87 has one depth per image: "the number
of bits shall be the same for all colour components". The writer's request (`Request::JpxWhole`)
is now answered with each channel's own integers at its own depth (`Raster::depths` crosses the
pipe). They are written in the field of the widest, eight bits or sixteen. Each component's
`/Decode` pair is widened from its own depth, so every integer keeps its value. A uniform depth
below eight now arrives unstretched, so a 3-, 5-, 6- or 7-bit codestream is no longer requantised.
The viewer's request still gets one eight-bit domain, so drawing is unchanged. The fixture is a
12/16/12-bit codestream: `opj_compress` output with one `Ssiz` byte patched, read back identically
by `opj_decompress`.

## 3. Residue 3: the shape was the re-expression's

In the code, "a decode shape no fresh raster holds" meant `reexpressed`'s errors: a picture that
decoded non-opaque, a soft mask whose channels differed, a stencil with partial coverage. With no
fresh raster, none of these exists. What the decoder returns for the other shapes the brief named
was read one by one:

- **A codestream palette** with no `Indexed` in the dictionary is resolved by the decoder into the
  palette's colours, each channel at its column's own depth. Every value a reader sees is carried
  exactly. §7.4.9 calls the palette "the equivalent of an Indexed colour space", and this reader
  applies it when reading.
- **Components subsampled differently** are upsampled onto the image grid by the decoder. The
  samples written are the ones this reader draws, value for value.
- **Every component subsampled alike** makes the decoded grid smaller than the dictionary's. That
  is refused by §7.4.9's "Width and Height shall match".

What is left is refused, each for a clause reason: a dictionary whose components or image-mask
depth contradict what its filter delivers (§8.9.5.1, Table 87); a filter that stopped short of
the grid or concealed damaged rows (§7.4.6), whose rows are not the file's; and one decision.

## 4. The decision: a component deeper than sixteen bits

Table 87 gives no image written without a codec a sample above sixteen bits. The only way to
carry a 17-to-38-bit integer is a lossless JPEG 2000 writer, which this tree does not have. A
census of the committed corpora read the `SIZ` marker of every codestream stored raw, 175 in 22
documents: 0 above sixteen bits and 0 with disagreeing depths (component depths 8 ×448, 16 ×2,
4 ×1). A JPEG 2000 encoder whose only user is a class of image nobody has shown us is not
built. The page is refused by name before any sample is decoded. **The cost:** such an image
cannot be redacted, and says so. A file that shows one reopens this decision.

## 5. The row's word

With these, every content class §12.5.6.23 reaches is removed. What is refused is a file that
contradicts its own clause, the operator's decode budget, or one of two decisions with their costs
recorded: ADR 1363's calculator function and section 4 above. The one departure from the clause's
application semantics is Table 195's overlay, which `doc/questions/A64` placed on the far side of
the authoring line (ADR 1124). By HANDOVER's definition, that is `departed`. §12.5.6 and §12.5
then owe nothing of their own and follow their children to `departed`. §12.1 stays `partial` for
§12.8 and §12.10. `doc/todo/64` is deleted: redaction owes nothing, and relocation was built in
ADR 1123. Its design notes live in ADRs 1124 and 1123 and the row's note.
