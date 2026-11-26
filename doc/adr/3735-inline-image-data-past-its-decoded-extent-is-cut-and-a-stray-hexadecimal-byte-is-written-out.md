# 3735 — Inline image data past its decoded extent is cut, and a stray hexadecimal byte is written out

Session 2258. Status: **accepted and built** in `pdf-model` (`inline_image::Scan::unread_data`) and
`pdf-transform` (`archive/as_drawn.rs`: `Place::cut`, `Place::hexadecimal`, `without_stray_bytes`,
`wrote_every_stray`; `archive/decision.rs`; `archive/fenced.rs`'s `STRAY_BYTES`). Builds the two
writes ADR 3680 named and did not build. Clauses: ISO 32000-2 §8.9.7, §8.9.3, §7.3.8.2, §7.3.4.3;
ISO 19005-2 section 5.1 and section 6.1.6, ISO 19005-4 section 5.1.

## Decision

1. **Inline image data.** Where every length §8.9.7's reading predicts missed `EI` and the search
   found it, and the image's own decoding — §8.9.3's arithmetic for unfiltered samples, the first
   filter's end-of-data marker for filtered ones (§7.3.8.2) — states an end before the `EI` found,
   with `/L` absent or naming that end, the bytes between are bytes this reader held as data and
   never decoded. `Scan::unread_data` names the span and the write removes it, so the data ends
   where every answer predicts. Data shorter than the decoding wants is not padded: that would be
   samples nobody stated.
2. **A stray hexadecimal byte.** A hexadecimal string holding a byte that is neither a digit nor
   white space is written with the producer's digits and white space and without that byte, which
   §7.3.4.3 gives no meaning and this reader skips. Where every such string in a content stream is
   written so, ISO 19005-2 section 6.1.6's `hexadecimal-string-holds-only-digits` holds no page and
   is answered by the write at `content-as-drawn`; ADR 1176's sibling sentence — "deleting it is a
   guess about what the producer meant" — stands for a reading the proof does not hold the page to,
   and here it does.
3. Both stand under ADR 3558's proof and ADR 3733's answer, and the loss is `content-as-drawn`: the
   clause states neither what the trailing bytes nor what the stray byte meant; the reading is this
   reader's.

## Consequences

The census carriers (`poppler-90-0-fuzzed`, `064034`) hold other classes and do not move.
