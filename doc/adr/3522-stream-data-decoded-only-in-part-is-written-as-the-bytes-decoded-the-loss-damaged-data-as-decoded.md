# 3522 — Stream data decoded only in part is written as the bytes decoded, the loss `damaged-data-as-decoded`

Session 2205. Status: **accepted**; built. Context: ADR 3521, ADR 3462 and ADR 3473 (the ASCII
construction this generalises), ADR 3494 (the history entry), ADR 0744 and ADR 0836 (what
`Damage` reports); ISO 32000-2 §7.4; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

`pdf_syntax` logs `syntax/stream-data-damaged` when a stage of a stream's chain reports
`Damage` — data that runs out before the filter's end-of-data, breaks its grammar part way, holds a
character outside its alphabet, or decodes to bytes its check value disagrees with. Section 5.1
fails on it. The writer copies the stream's data as it found it, so the output fails too.

## Decision

- **The stream is written as what the window drew**: the bytes decoded through every stage in front
  of its image codec, re-encoded as one `FlateDecode`, with the codec's stages and the dictionary's
  other entries kept — `rewrite::reencoded_through_flate` under a new `Cut::DamageAdmitted`. §7.4
  defines what each filter's data states, and damaged data states nothing past the damage; another
  reader may recover more, less or other bytes. So this restates the drawing rather than repairing
  the data, and the encoded bytes past the damage are **the loss `damaged-data-as-decoded`**, with
  the rewrite of the same word: granted by `discard` in `as-shown` and `as-if-printed` and by `try =
  ["append-statement", "discard"]` in `state-every-loss`, as `ascii-data-as-decoded` is.
- **The converter finds the streams itself**, since the decoder is handed data without its object
  and logs the damage at the whole file: every stream the output reaches is decoded again through
  one walk of its chain (`rewrite::each_stage_in_front_of_the_codec`), shared with the re-encoding so
  the stream named is the stream read. A stream whose one fault is ASCII data cut before its marker
  is left to that row's restatement (ADR 3473), and a font program's cut to the font routes (ADR
  3256: written whole, dropped for the face the window drew, or substituted), the output's own
  check holding each to it; damage the log holds in no stream object the walk
  finds — an inline image's, say — is refused by name.
- **Named three times**: the report's sentence and its JSON entry `damaged_data_restated` (object,
  stage, damage, length), and the file's own `xmpMM:History`, where a packet that will not take the
  entry keeps the refusal (ADR 3494's rule).

## Consequences

- A truncated `FlateDecode` content stream converts under both shipped profiles at 2b and 4 and its
  output conforms; `tests/archive/damaged_data_as_decoded.rs` is the proof.
- A caller who authorised nothing is told the loss by name instead of an output failure.
