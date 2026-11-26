# 3747 — Every text string a writer forms from characters goes through `encode_text_string`

Session 2261. Status: **accepted and built** in `pdf-transform` (`merge::text_string`, used by
`merge` and `split`; `archive/prepare/colour_and_graphics.rs`, the output intent's `/Info`;
`archive/attach.rs`, the names attachments are filed under). Answers audit 2260's B03. Clauses:
ISO 32000-2 §7.9.2.2, Annex D Table D.3, §12.4.2, Table 401, Table 43.

## Decision

An unprefixed text string is `PDFDocEncoding`, and Table D.3 is not ASCII below 0x20: 0x18 to 0x1F
are eight accents and most of the rest, with 0x7F, are undefined. `merge::text_string` wrote an
ASCII label as its own bytes, so a label holding U+001F read back as a tilde; the output intent's
`/Info` was written as a profile description's UTF-8 bytes; and the archive's attachment filing
wrote a file name's UTF-8 bytes as the name tree key and the specification's `/F` and `/UF`, where
the sibling paths already encode. All three now write `pdf_syntax::text_string::encode_text_string`,
which chooses `PDFDocEncoding` only where the string reads back as itself and UTF-16BE behind the
byte order marker otherwise.

## Consequences

A census of every `Object::String` the writers of eight crates form found no other text string
written from characters by hand: the rest are byte strings, ASCII strings (`/Registry`,
`/Ordering`, `/CharSet`, `/DA`), dates, digests, bytes copied from the source, or constants; the
language tags of `/Lang` are ASCII by `language_key`'s BCP 47 grammar; the scripts' and the
fragment identifiers' named-destination keys are lookups, not writes.
