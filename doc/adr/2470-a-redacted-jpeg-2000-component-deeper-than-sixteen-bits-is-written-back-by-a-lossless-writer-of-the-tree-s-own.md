# 2470 — A redacted JPEG 2000 component deeper than sixteen bits is written back by a lossless writer of the tree's own

Session 1818, a "clause" round of batch one hundred and twenty-two, 2026-10-27.
Status: implemented — §12.5.6.23's `JPXDecode` refusal narrows from every component past sixteen
bits to a component past twenty-four, or a deep image of other than one or three colour
components; the row stays `departed` on ADR 1363's calculator function and on that remainder.
Context: ADR 1371 (the refusal this narrows), ADR 2439 (the departures read), the owner's A668
item 3, ADR 1349 (`pdf-ccitt`, the precedent for a codec of the tree's own).

## 1. The question

§12.5.6.23 asks that "that portion of the image data shall be destroyed", and the removal carries
the rest of an image unchanged by re-encoding the cleared grid under `FlateDecode`. Table 87 bounds
those samples — "\[t]he value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16" — while §7.4.9 lets a
JPEG 2000 component be "between 1 to 38 inclusive" bits. A deeper component therefore goes back
only as JPEG 2000, and the tree had no writer. A668: find a safe-Rust library, or build one.

## 2. The survey (each crate's own `Cargo.toml`, README and source; none was executed)

| crate | licence | `unsafe` | deepest it writes |
|---|---|---|---|
| `dicom-toolkit-jpeg2000` 0.5.0 | MIT or Apache-2.0 | forbidden | 16 (`src/j2c/encode.rs`, `bit_depth > 16` refused) |
| `oxideav-jpeg2000` 0.0.17 | MIT | none | 16 (its sample layouts, `src/image.rs` `for_layout`) |
| `pure_jpeg2k` 2.0.0 | MIT or Apache-2.0 | none | 16 only (`BIT_DEPTH`, `src/codestream.rs`) |
| `rust-j2k` 0.3.0 | MIT or Apache-2.0 | none | no encoder (its README: "an encoder" comes later) |
| `justjp2` 0.1.1 | MIT or Apache-2.0 | none, not forbidden | to 31 in `i32` (`1i32 << (precision - 1)`); one release line |
| `j2k-native` 0.13.0 | MIT or Apache-2.0 | forbidden | 38 (`MAX_JPEG2000_PART1_SAMPLE_BIT_DEPTH`); 131 294 lines; `rust-version` 1.99 since 0.12 |
| `openjp2` 0.6.1, `openjpeg2-pure-rs` 0.1.1 | BSD-2-Clause | 509 and 253 lines | ports of OpenJPEG; principle 3 excludes them |
| `jpeg2k`, `jpeg2000`, `openjpeg-sys` | — | C | bindings to OpenJPEG |

`j2k-native` is the one admissible crate that writes the depths. It is not taken: its current
releases need a compiler this workspace does not pin (1.97), it has shipped nineteen 0.x releases
since June 2026, and it would bring 131 thousand lines into the trust path of a construction no
document on this disk exercises. More decisive than any of that: **the bound is the decoder's, not
the encoder's** (section 3), so no crate moves it.

## 3. The bound: what the tree's decoder reads exactly

`hayro-jpeg2000` reconstructs every sample in `f32`, whose integers are exact to 2^24 and no
further, and refuses a precision past 31. A component of 25 bits or more is therefore not read as
the integers the file states, and carrying it would change samples outside the region — the
refusal stays, now in the decoder's words. `pdf_sandbox::MAX_PRECISION` is that bound; a
`Request::JpxWhole` answers to it in a three-byte field, and a drawing decode stays at sixteen.

## 4. The writer, `crates/pdf-jpx-write`

ISO/IEC 15444-1's smallest lossless construction: one tile, zero decomposition levels under the
reversible 5-3 filter (so the band is the samples less G.1.2's DC shift), no quantisation, two
guard bits, one layer, 64 × 64 code-blocks of style 0; Annex D's three passes through Annex C's MQ
coder; Annex B's packets and tag trees; Annex I's JP2 boxes, with a channel-definition box carrying
Table 87's opacity channel back inside the codestream. No dependencies,
`#![forbid(unsafe_code)]`. Its proof is the round trip at every depth 1 to 24, mixed depths and both
opacity kinds, read back sample for sample by `pdf-sandbox`, and a fuzz target `jpx_write` holding
the same property; two planted defects (a sign context's XOR bit, a run position's high bit) each
turned all three round-trip tests red.

## 5. What stays refused, each by name

- A component past 24 bits (section 3).
- A deep image of other than one or three colour components: a JP2 file enumerates greyscale and
  sRGB only (I.5.3.3); CMYK and other counts need T.801's JPX file with its reader requirements box
  (M.11.1). That box is the one step left on this half of the row.
