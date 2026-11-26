# 3752 — An XMP reference XML does not define is returned to its reader

Session 2262. Status: **accepted**. Context: ADR 3477, ADR 3621, ADR 3738; ISO 32000-2 §14.3.2;
XML 1.0's well-formedness constraints Entity Declared and Legal Character.

## Context

`xmp::unescape` keeps an undefined entity reference, a character reference naming no character,
and a bare ampersand as written: dropping one would delete a producer's text, refusing would lose
the packet. That is a repair — none of the three is XML — and the expander holds no document, and is
shared with the rich text, XFDF and XFA readers.

## Decision

- `unescape_noting` answers whether it kept a reference as written (ADR 3621's shape);
  `xmp::Reader` carries the answer over the packet in a cell (`classify` reads attributes through
  `&self`), and `Xmp::read`, which holds the document, logs the new `model/xmp-reference-undefined`
  at the metadata stream's object. `Xmp::parse` keeps its signature for callers holding bytes.
- Section 5.1 alone. The conversion keeps a producer's packet but for the schemas it rewrites, so a
  reference elsewhere crosses as written and the output's own check refuses by the class's name.

## Consequences

- `xmp.rs` held 2 → 0.
- `unescape` remains for the rich text, XFDF and XFA readers, which drop the answer; each holds a
  class or a document of its own (`model/rich-text-malformed` for rich text), and routing the answer
  to it is the next step for each.
