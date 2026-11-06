# 2729 — The TIFF predictor adds components at every depth Table 8 lists, and refuses any other

Status: accepted. Session 1957. Context: ISO 32000-2 §7.4.4.4 and Table 8; ADR 0667, ADR 0795; trap 5; the Ghent
PDF Output Suite's patches 18.0 to 18.4 (ADR 2728).

## The defect

`pdf_syntax::filter::tiff_predictor` un-predicted eight-bit data only and handed every other depth back unchanged,
on the premise that other depths "do not occur in the streams this reader meets". The Ghent suite's five sixteen-bit
patches state `/Predictor 2 /BitsPerComponent 16` in their `/DecodeParms`, and each drew as noise around the eight-bit
reference cross — in the screen view and the press view alike, with no report: the silent fallback trap 5 names.

## The clause

§7.4.4.4: "The TIFF function group shall predict each colour component from the prior instance of that component,
taking into account the number of bits per component and components per sample." Components are "packed into bytes
from high-order to low-order bits", and those "outside the image ... shall be 0". Table 8 gives
`/BitsPerComponent` as "1, 2, 4, 8, and (PDF 1.5) 16".

## Decision

- The sum is of components modulo `2^bits`: a sixteen-bit component is read most significant byte first and its low
  byte carries into its high byte; a one-, two- or four-bit component wraps inside its own bits, and a row's padding
  bits, which are no component's, are left as they came.
- A depth Table 8 does not list returns `None`, as an undefined predictor code already does: data predicted at a
  depth nobody can state cannot be un-predicted at a guessed one, and drawing it unchanged is the silent answer.

## Consequences

Three tests in `crates/pdf-syntax/tests/filters.rs` hold the carry, the sub-byte wrap and the refusal, each failing
against the old fallback; the Ghent harness's five photograph checks fail against it too.
