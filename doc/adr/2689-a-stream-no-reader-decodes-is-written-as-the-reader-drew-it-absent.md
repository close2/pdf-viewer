# 2689 — A stream no reader decodes is written as the reader drew it, absent

Session 1935, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/undecodable.rs`: `survey`, `Role`,
`UndecodableStream`; `archive/decision.rs`: `Loss::UndecodableStream` and its route in
`answer_of`; `archive/rewrite.rs` and `rewrite/streams.rs`: `Rewrite::UndecodableStreamRemoved`;
`archive/fonts.rs`: `needs_a_program`; `archive/prepare/metadata.rs`: the history row).
Context: ADR 2490 (R07: "the stream treated as the reader treated it, absent", loss
`undecodable-stream`, both shipped profiles), ADR 2679 (no undeclared filter is tried), ADR 2678
(a removed object's facts and sentence), ADR 2115 (a form that draws nothing), ADR 0816.
Clauses: ISO 32000-2 §7.4, §7.3.10; ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.

## The premise, checked

The eight R07 documents name no decoder this tree lacks: seven `UnknownFilter-*` files state the
invented `/XXXDecode`, and `poppler-742-0-fuzzed`'s image states `/DCTecode`, a fuzzed
`DCTDecode` that ADR 2679 forbids inferring. So nothing is a decoder defect. With the header
patched to `%PDF-1.7` (offsets unchanged) and the page rendered before and after, the reader
draws nothing from the image, the form, the page content stream or the object stream's font
(byte-identical renders), leaves the font with the undecodable program blank, and draws the
`ICCBased` space as §8.6.5.5's alternate (`/N 4`, so CMYK).

## Decision

1. **The role decides what "absent" is**, read off the stream's dictionary (`/Type /ObjStm`,
   `/XRef`, `/Subtype /Image`, `/Form`) and otherwise off what names it (a descriptor's
   `/FontFile*`, a page's `/Contents`): an image or a form is written as ADR 2115's form drawing
   nothing at the same object; a content stream as one with no data; a font program is left out,
   its references §7.3.10's null, so the font takes the routes of a font with no program; an
   object or cross-reference stream is not written, as the archive writes its own.
2. **Refused by name**: an image another names as `/SMask` or `/Mask` (a form is no mask); an ICC
   profile (writing the alternate in its place, with ISO 19005's licence for a device space, is
   not built); any other role.
3. **Nothing is re-encoded**: no part of an undecodable stream was decoded, so there is no partial
   decode to keep.
4. **Stated** as ADR 2678 states a removed packet: each stream by object, role, stated filters,
   stored length and the fixed reason, in the report (`undecodable_streams`) and in
   `xmpMM:History`, which is a condition of the rewrite.

## Profiles

`as-if-printed` and `as-shown` take the loss with `remedy = "discard"` (ADR 2490);
`state-every-loss` takes it with `try = ["append-statement", "discard"]`, and no page is composed
at this site yet, so the first step is passed over with its sentence. `no-data-loss` names no
answer and refuses by the loss's name, which a test pins.
