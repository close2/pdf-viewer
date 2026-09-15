# 1121 — A colour key's domain is stated in the image data, not only in the dictionary

Status: accepted. Session 1108.
Context: ISO 32000-2 §8.9.6.4, §8.9.5.1 Table 87, §8.9.5.2, §7.4.9, §8.6.6.3; ADR 0023 (the
mechanism), ADR 0832 (the same correction for the other three codecs), trap 40.
Code: `crates/pdf-model/src/image.rs`. Tests: `crates/pdf-model/tests/image_masks.rs`.

## 1. The refusal on record, and why it was wrong

§8.9.6.4 bounds its range array by one entry — "Each integer shall be in the range 0 to 2
BitsPerComponent  - 1, representing colour values before decoding with the Decode array" — and
Table 87 withdraws that entry for one filter: "If the image stream uses the JPXDecode filter, this
entry is optional and shall be ignored if present." From that this tree concluded, for 280
sessions and then again after ADR 0832 narrowed the refusal to this one codec, that the domain the
file's integers live in had been taken away, and reported the mask instead of applying it.

The next sentence of the same row says otherwise: "The bit depth is determined by the PDF
processor in the process of decoding the JPEG 2000 image." That is a statement that the depth *is*
determined, and §7.4.9 says from what — "These packagings contain all the information needed to
properly interpret the image data, including the colour space, bits per component, and image
dimensions." `pdf_model::jpeg2000` has read exactly those — the `ihdr` and `bpcc` boxes and the
codestream's `SIZ` marker — without decoding a sample since the module existed. Trap 40 names this
shape: the capability a refusal says is absent may be forty lines above it.

## 2. What the domain actually is, and the two cases where the test is exact

The question is not whether a domain exists but whether the samples reaching the comparison are
still in it. `pdf_sandbox::Raster::data` is eight bits whatever the codestream declared, so:

- **An `Indexed` dictionary space, at any precision.** §7.4.9 gives `/ColorSpace` precedence over
  the codestream's own specifications, so the confined decoder is asked for indices unscaled — an
  index stretched to eight bits is a different index — and §8.6.6.3 caps `hival` at 255.
- **Eight unsigned bits per component, otherwise**, where the stretch is the identity.

Anything else — another precision, or signed samples — is refused, naming the precision read, and
`§8.9.6.4`'s row stays `partial` for it. That residue is **not** `departed`: nothing in the
standard excuses it, and what blocks it is this tree's eight-bit image pipeline, which a later
round may widen. Mapping the file's integers into eight bits is an approximation the clause does
not define, and §8.9.5.2's "can have different values per colour component" may leave no single
domain to map them from, so approximating silently is the one answer not available.

## 3. Why the fixtures are generated, and what calibrates them

`examples/colour_key_mask_census` over 90 535 documents finds 660 colour keys and not one on a
`JPXDecode` image, so no corpus can move for this. Both arms are pinned by generated codestreams —
one at eight bits, one at twelve — each beside a control that must move: the applied cases carry a
range the sample misses, and the refused case carries the same range over the eight-bit stream,
where it removes the picture. Planting *accept every depth* fails only the refusal test; planting
*never apply the ranges* fails all three. The eight-bit fixture states `/BitsPerComponent 4`,
which no reader may believe, so "shall be ignored if present" is asserted rather than assumed.
