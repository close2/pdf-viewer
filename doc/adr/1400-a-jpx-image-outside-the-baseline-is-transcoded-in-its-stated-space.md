# 1400 — A JPX image outside the baseline is transcoded to Flate in the colour space it states

Status: accepted and **built**.
Context: ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3 (cited, paraphrased); ISO
32000-2 §7.4.9, Table 87; ADR 1371 (a codec's output is the image's samples), ADR 1383 (the
converter's refusal by name), ADR 1211 section 2 (a shape selects a half), ADR 1285 (a `preserve`
that moves nothing). Code: `crates/pdf-transform/src/archive/transcode.rs`, `Rewrite::Jpeg2000TranscodedToFlate`,
`decision::SHAPES`, `config::PRESERVABLE_IN_PLACE`, `crates/pdf-transform/tests/archive.rs`,
`doc/profiles/keep-everything.toml`, `doc/pdf-a-mitigations.md` section 4.5. Amends ADR 1383
section 5 for the stated-colour-space shape.

## 1. The restriction reaches JPEG 2000 data and nothing else

Every sentence of section 6.2.8.3 (and part 4's 6.2.7.3) has JPEG 2000 data, or an image compressed
with it, as its subject: the baseline feature set, the channel count, the colour specifications and
their method, CIE Jab, the bit depth, and the device-colour sentence, whose two routes are the image
dictionary's ColorSpace and the data's own definition. An image whose samples are carried under
`FlateDecode` is outside all of them; section 6.1.7.2 admits the filter, and section 6.2.4's colour
rules bind its `ColorSpace` exactly as they bound it before, because the entry does not change.

## 2. Only where the dictionary states `ColorSpace`

§7.4.9: "If present, it shall determine how the image samples are interpreted, and the colour space
specifications in the JPEG 2000 data shall be ignored." So the samples are already in the
dictionary's domain, and ADR 1371 makes the codec's output those samples: writing them again changes
the encoding and nothing a reader is told they mean. Where the dictionary states none, a Flate copy
would need a `ColorSpace` this converter chose, and that half keeps the refusal. `SHAPES` names the
two halves `stated-colour-space` (answered) and `data-colour-space`. An image mask has no colour
space and one meaning for its samples, so it is transcoded too.

## 3. The construction, and what it refuses

`pdf_model::image::jpx_samples` at the ordinary worker's budget, full resolution or nothing. One
depth Table 87 states (1, 2, 4, 8, 16) is written at that depth, packed, with the dictionary's
`Decode` carried; any other layout goes in the widest's field, eight or sixteen bits, each pair
widened from its own depth (`redact::widened_decode`, now crate-visible). `SMaskInData` is removed
where it was 0 and **refuses** where it was not (the soft-mask image would be a new object, not
built). Refused as well, each by its own sentence: data in another file or under a chain, a
component deeper than sixteen bits, a channel count the space does not have, a decode that is not
whole. Every stream is re-decoded before it is promised (ADR 0973).

## 4. Why it is `preserve` and not the default

Nothing a reader sees changes, and two things do: the file grows, and this tree's decoder's output
becomes the archive's copy of the picture, so a decoder fault is kept rather than re-decodable. That
is the operator's to accept: `remedy = "preserve"` at the site, with no mechanism key
(`PRESERVABLE_IN_PLACE`), and the report names each image with its depths and its size under both
filters. The same transcode would answer the bit-depth, channel-count and CIE Jab rows for the
stated shape; those rows are not re-classed here.
