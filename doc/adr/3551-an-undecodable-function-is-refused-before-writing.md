# 3551 — An undecodable function is refused before writing, and refusals wait for the photographs

Status: **accepted**, implemented (`archive::base_standard::restated`, `settled`). Session 2212.
Amends ADR 3521's bullets for `syntax/stream-data-undecodable` and `syntax/decode-parms-mismatched`.
Context: ISO 32000-2 §7.4, §7.3.8.2's Table 5, §7.10.1; ISO 19005-2 section 5.1 (paraphrased).

## Context

ADR 3521 left undecodable data to the output's own check, because another rewrite may drop the
stream (`issue19484_1`/`_2` convert that way). govdocs1 `178360.pdf` then failed the output: its
object 99 is a Type 0 function (a shading's `/Function`) whose `FlateDecode` data holds a block of
no type RFC 1951 defines. Nothing in this converter replaces a function, so the stream is carried
byte for byte and the outcome was known before writing. ADR 3521 also refused a `/DecodeParms`
mismatch at the first stream, before the conversion knew which pages it photographs.

## Decision

- Where the log holds `syntax/stream-data-undecodable`, a function stream (Type 0 or 4) of the
  output whose filter refuses its data as corrupt or its parameters is refused by name under ADR
  0816's fence: writing it as anything would state samples nobody wrote. Every other role's stream
  stays the output check's, since a font route, a metadata rewrite or a photograph may drop it
  (`498264.pdf`'s undecodable `FontFile2` stays there).
- Both that refusal and the `/DecodeParms` one are kept per stream and decided in `settled`, after
  the photographs are known: a stream only photographed pages reach is not written and refuses
  nothing.

## Consequences

`178360.pdf` is refused before writing, its sentence naming the class and the function role.
