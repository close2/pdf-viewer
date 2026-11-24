# 3570 — A font's maps this reader read past are restated as read, at the loss `font-maps-as-read`

Session 2217. Status: **accepted** and built; extends ADRs 3566 and 3567. Context: ADR 3565 (the
classes); ADRs 3462 and 3473 (a row of section 5.1 answered by a restatement at a stated loss); the
owner's PDF/A stance; ISO 32000-2 Table 109, Table 112, §9.6.5.1 and §9.10.3; ISO 19005-2 and ISO
19005-4 section 5.1.

## Context

Eight pdf.js documents were refused by the output's check under `font/to-unicode-not-a-stream`
(seven, each `/ToUnicode /Identity-H`) and `font/encoding-name-unknown` (`bug859204`, `/Encoding
/NULL` on an embedded Type 1 program). ADR 3567 left these classes with no route.

## Decision

1. **A row of section 5.1, `fonts/font-maps-read-as-written`**, read from the reader's log (the
   population is the file's), for three classes: `to-unicode-not-a-stream`,
   `encoding-name-unknown`, `differences-malformed`. They still breach section 5.1's own row.
2. **`archive::font_maps` restates every `/Type /Font` object as read**: a `/ToUnicode` that is no
   stream removed (this reader reads it as no map); an `/Encoding` or `/BaseEncoding` name outside
   `pdf_font::PERMITTED_ENCODING_NAMES` removed (read as no base encoding); a `/Differences` array
   written as the codes and names the reader assigned, in order, an element out of §9.6.5.1's
   grammar skipped as the reader skips it.
3. **Proved before written**: a simple font loaded from the dictionary as restated draws every
   single-byte code's outline at the advance it drew. The proof is not a formality: a nonsymbolic
   `TrueType` font's `/Encoding /NULL` is not read as an absent entry (the glyph is selected by
   another route without it), so that font is refused by name (`NOT_RESTATED`) rather than drawn
   differently. A composite font's `/ToUnicode` draws nothing, so its removal needs no proof.
4. **The loss `font-maps-as-read`**: the producer's words at those places, which no reader reads by
   ISO 32000-2 and a reader with a convention of its own may (`/ToUnicode /Identity-H` read as the
   identity map). Both shipped profiles authorise it (`remedy = "discard"`, nothing drawn changes);
   `state-every-loss` states it.
5. **Not built**: `to-unicode-damaged` and `cmap-damaged` (a map whose entries partly read) stay
   section 5.1's alone and refused by name by the output's check; restating them means writing the
   mappings that read as a new `CMap`, a route of its own.

## Consequences

- The eight documents convert at 2b under `as-if-printed`, `bug1650302_reduced` with ADR 3569's
  program route beside it.
- The report names the row and the loss; it does not yet list each font and entry the restatement
  holds (`font_maps::Restated::entries` is built and not rendered), a later round's.
