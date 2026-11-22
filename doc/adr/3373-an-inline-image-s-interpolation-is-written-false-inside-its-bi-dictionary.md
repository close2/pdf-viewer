# 3373 — An inline image's `/I true` is written false inside its `BI` dictionary, the second closed content-stream exception

Session 2168. Status: accepted. Context: ISO 32000-2 §8.9.5.3, §8.9.7 (Table 91); ISO 19005-2
section 6.2.8.1, ISO 19005-4 section 6.2.7.1; `doc/questions/A978` (the owner's "Agreed" to Q978),
`doc/questions/A50`; ADRs 0816 (the content-stream fence), 2498 (`image-smoothing`), 2638 and 2907
(the content-stream writers this one sits beside).

## The question

`graphics/inline-image-interpolation-is-off` was the only stop of `TAMReview` and `bug1065245`
(pdf.js) under `as-shown` at both parts: the key is in a `BI` dictionary inside the producer's
content stream, behind ADR 0816's fence. The same entry on an image `XObject` is R02's mechanical
rewrite at the loss `image-smoothing`.

## Decision

1. The owner's A978 admits the edit as the second closed exception beside A50's operator
   spellings. `crates/pdf-transform/src/archive/inline_smoothing.rs` lexes each `BI` dictionary as
   `pdf_model::inline_image` reads it (a name, then one object), writes `false` over the value of
   every `/I` or `/Interpolate` key that is not already false, and skips the image data at
   `inline_image::scan`'s extent. A name in a value's place (`/CS /I`, Table 91's `Indexed`) is not
   the key. No other byte moves: the module's tests compare the stream before and after outside
   the `BI`…`ID` span, the samples included, and the two corpus documents were proved the same way
   by a byte comparison of each output's decoded content streams with the source's: `TAMReview`
   holds two edits (offsets 64 and 7735 of its stream), `bug1065245` three (879, 5817, 8002), each
   the value token `true` become `false`, each between a `BI` and its `ID`, at both parts.
2. The answer is `Answer::Loses(Loss::ImageSmoothing, Rewrite::InlineInterpolationOff)`, the same
   loss and the same `xmpMM:History` action as R02, its sentence appended to R02's entry.
3. A stream another content-stream writer rewrites (hexadecimal digits, languages, runs as drawn,
   Type 3 widths) is refused rather than written twice, as ADRs 2638 and 2907 do.
4. Under `as-if-printed` the photograph still answers the row first, and the rewrite is not
   prepared there (so the history never names an edit the output does not carry). Whether the
   cheaper rewrite should answer it under `as-if-printed` too is left to a later round: it changes
   the shipped profile's output and was not what A978 asked.

## Consequences

`TAMReview` and `bug1065245` convert under `as-shown` at 2b and 4, and veraPDF 1.30.3 passes all
four outputs. `an_inline_images_interpolation_stays_refused_under_as_shown` became
`an_inline_images_interpolation_is_written_false_under_as_shown`; the 1983 audit's
`decisions.json` still cites the old name (an owner audit file, not edited).
