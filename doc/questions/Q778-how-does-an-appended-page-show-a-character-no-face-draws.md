# Q778 — How does an appended page show a character no face draws?

Source: round 1927 (single round), ADR 2677, the R14 paragraph of `doc/pdfa-blocking-issues.md`.

## What the round found

`balloon_a1b_jp2k` (format-corpus, `pdfCabinetOfHorrors`) is refused at 2b and 4 under both stating
profiles with `NO_FACE`. Its XMP packet is valid UTF-8 and every character in it is one the shipped
face draws except one: the producer wrote U+0000 as the value of the packet header's `begin`
attribute (`<?xpacket begin="\0" ...?>`), where ISO 16684-1 puts the byte-order mark. A control
character has no glyph in any face, so setting it would show `.notdef`, which ISO 19005-2 section
6.2.11.8 forbids. The face search already passes over U+FEFF (and U+FFFE) because the mark says how
the bytes are encoded rather than anything in them (`preserve.rs`, `is_set`); a NUL in its place says
nothing either, but it is not the mark, and leaving a byte off a page that claims to keep the
producer's bytes is a loss nobody stated.

This is a design question rather than a missing resource: no face or pack would supply the glyph.

## The question

Which of these does the appended page do with a C0 control character (other than the line breaks it
already reads) that a packet holds?

- **a.** Show it as its Unicode *control picture* (U+2400 + code, `␀` for U+0000), with the page's
  `ToUnicode` mapping that code back to U+0000, so the page shows the byte and extraction returns
  it. The marks are a fixed representation of the file's own byte; the shipped face would need the
  glyph (Liberation Sans may not carry the block, which is then a face to ship).
- **b.** Leave it off the page and state it, under `state-every-loss` only, as a loss on the
  statement page ("one U+0000 in object 2's packet header was not set"); `no-data-loss` keeps the
  refusal.
- **c.** Keep the refusal.

## Recommendation

**b** now, and **a** if a shipped face covers the control pictures: (b) builds on the statement page
the stating profiles already append, invents nothing and loses one byte that carried no meaning in
the producer's own packet, out loud.

## Meanwhile

The tree keeps the refusal (c): `NO_FACE` names the face search, and the document stays in R14.
