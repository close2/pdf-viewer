# 3325 — An image whose own bytes break its filter, or stop short of its grid, is the file's fault

Session 2156. Status: accepted. Context: ISO 32000-2 §7.4.1, §7.4.4.4, §7.3.8.2; ADR 0836
(`CheckValue`), ADR 2516 and ADR 2618 (the wider photograph gate), ADR 2759 (a fault the reader names
as the stream's own); `doc/questions/A694` item 3.

## The defect

`507676`'s page 4 holds an image `XObject` (`R73`, `FlateDecode` with `/Predictor 15`) whose deflate data
decodes whole under an Adler-32 that disagrees and whose rows then carry PNG tags Table 10 does not
define (row 46 is tagged 252: a byte of the producer's data is gone). The predictor refuses, and the
interpreter said `R73: malformed image: stream did not decode` — the same words it says of a filter it
does not run. So the wider gate could not tell the file's fault from this reader's gap, and refused the
photograph `as-if-printed` takes of a damaged page; `doc/pdfa-blocking-issues.md` recorded exactly that
("an undecodable stream does not tell the file's fault from this reader's").

The reader is right not to draw the image: §7.4.1 has a filter "convert the information back to its
original form", and bytes that break the filter's own format cannot be. What was missing is the report
saying *which* refusal it was.

## Decision

- `Document::image_stream_reported` returns the `StreamRefusal` `image_stream` dropped; `image_stream`
  is that `.ok()`.
- `pdf_model::image`'s two decode routes name `FilterRefusal::Corrupt` — a filter this reader runs
  meeting bytes its grammar does not admit — by `NOT_WHAT_ITS_FILTER_ADMITS`; every other refusal keeps
  "stream did not decode". `short_of_its_grid`'s sentence ends in `DRAWN_SHORT_OF_ITS_GRID`.
- `Unsupported::is_a_fault_of_the_images_own_bytes` reads those two, as `stream_syntax_fault` reads
  ADR 2759's operators. The wider gate (`photographed::is_a_fault_of_the_documents_own_stream`) takes
  them as the document's own faults: the window draws what the file's bytes give — nothing of an image
  whose bytes break its filter, the carried samples of one short of its grid — and A694 item 3 keeps
  that state. A filter or codec this reader does not run, a bound it sets, a chain Table 5 does not
  admit stay refused.

## Cost

Nothing draws differently. The narrow gate is unchanged. A wider-gate photograph may now fix an image's
absence into the file where the file's own bytes caused it, which is the state the window showed.
