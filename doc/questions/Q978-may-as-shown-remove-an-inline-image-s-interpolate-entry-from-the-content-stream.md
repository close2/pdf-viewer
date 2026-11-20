# Q978 — May `as-shown` remove an inline image's `/I true` from the content stream?

Source: round 2156, looking for an `as-shown` stop an elegant remedy would free without a photograph
(`doc/questions/A694` item 1 keeps `as-shown` from the wider photograph). ADR 3327.

## The stop

`graphics/inline-image-interpolation-is-off` (ISO 19005-2 6.2.8.1, ISO 19005-4 6.2.7.1) is the only
stop of two pdf.js documents under `as-shown`, at both parts (`doc/pdfa-blocking-issues.md` section 9,
the R03 row). `as-if-printed` photographs those pages; `as-shown` refuses at the content-stream fence
(ADR 0816), because the entry is a key of `BI`'s dictionary, inside the producer's stream.

## Why it looks like R02

The same entry on an image `XObject` (`/Interpolate true`) is already discarded under both shipped
profiles: §8.9.5.3 calls it "only a hint, and a PDF processor may ignore it", and `as-shown` names the
loss because this reader honours the hint (`doc/pdfa-blocking-issues.md` R02,
`graphics/image-interpolation-is-off`). For an inline image §8.9.7's Table 91 abbreviates the same key
to `/I`. The only difference is where the key sits: in a stream dictionary the converter writes, or in
the content stream it carries byte for byte.

## The question

May `as-shown` remove `/I true` (or write `/I false`) from an inline image's dictionary — the one edit
to a producer's content stream, with the loss `image-smoothing` named exactly as for an `XObject` —
as a second exception beside `doc/questions/A50`'s closed list? Nothing else in the stream moves; no
mark is invented, and the image's samples are the producer's.

**Recommendation: yes**, under the same name and report line as R02, because the difference from an
`XObject`'s entry is only where the key is written. If no, the two documents stay refused under
`as-shown` and convert under `as-if-printed`, as now.
