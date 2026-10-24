# 2293 — A writer is refused PIMA 7667's samples by name

Session 1729, the clause slot of batch 112. Status: implemented. Builds on ADRs 1333 and 2292.
Code: `crates/pdf-model/src/image/jpx.rs` (`jpx_samples`). Test:
`crates/pdf-model/tests/jpx_enumerated_spaces.rs::a_writer_is_refused_e_srgb_samples_by_name`.

`jpx_samples` hands a writer — §12.5.6.23's redaction, the archive transcoder — a `JPXDecode`
image's own integers and the colour space they are in, which the writer restates for the
`FlateDecode` stream it writes. Before ADR 2292 an e-sRGB or e-sYCC image took the device fallback
in both the reader and the writer, so the two agreed. Now the reader draws the encoding, and no §8.6
space restates it: equation 5's offset and the odd extension of equations 6a to 6c put components
outside what a `/Decode` pair into `CalRGB`, `Lab` or an ICC profile's curves carries, and a
`DeviceN` with a calculator function into `Lab` would be an invented space rather than a restated
one. Handing the samples on as `DeviceRGB` would recolour, silently, an image this tree draws as
its codestream states.

So `jpx_samples` refuses such an image with `ImageError::UnsupportedColourSpace`, naming the
enumerated space and why, where the image dictionary states no `/ColorSpace` (one that does sets
the encoding aside, §7.4.9, and is carried as before). The redaction's sentence around the error
begins by supposing a resolution limit; that wording is `redact/images.rs`'s and is reported rather
than changed here. What would reopen it: a writer that carries a JPEG 2000 image as JPEG 2000.
