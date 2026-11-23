# 3462 — ASCII data cut before its end-of-data marker is restated as the bytes this reader decoded

Session 2190. Status: **accepted**; decided, not yet built. Context: ISO 32000-2 §7.4.2, §7.4.3,
§7.3.8.2; ISO 19005-2 and ISO 19005-4 section 5 (a conforming file conforms to the base standard)
and section 6.2.4.3; ADR 3281 (ASCII data that ends before its marker is truncated), ADR 3447 (the
census triage that left `PDFBOX-3148-2-fuzzed` "the document's"); round 2186's leftover.

## The question

`PDFBOX-3148-2-fuzzed` writes every `/A85` stream without `~>`. Its widget appearance is
`03!0=6;`: this reader decodes the whole group, `/Tx `, and reports the cut (ADR 3281); veraPDF
pads the partial group and reads `/Tx B`, whose `B` paints in the initial `DeviceGray`, and fails
6.2.4.3 on all four outputs this tree wrote as conforming. §7.4.3 makes the partial group the
encoding of the last bytes only where "[t]hese characters shall be immediately followed by the ~>
EOD marker", and opens by giving the encoding that marker: "The ASCII base-85 encoding shall use the
ASCII characters ! through u ((21h) - (75h)) and the character z (7Ah), with the 2-character
sequence ~> (7Eh)(3Eh) as its EOD marker." A stream that stops without it is not correctly encoded,
so the file does not conform to the base standard, and both readings of its last characters are
readings of damage.

## Decision

**A remedy, not a refusal**, because the window draws the file and a refusal would decline a page
this reader shows whole; and **a restatement of what the window drew, not a repair**, because the
file states no bytes for the cut group (ADR 3281) and padding it would be choosing them.

1. **The row.** `pdf-archive` reports a stream whose `ASCIIHexDecode` or `ASCII85Decode` stage
   reaches the end of its data before `>` or `~>` (`Damage::Truncated` from that stage), at the
   requirement that a conforming file conform to ISO 32000 (section 5 of both parts), narrowed to
   §7.4.2's and §7.4.3's end-of-data marker — the one requirement of that section this tree can
   judge by decoding.
2. **The rewrite.** The stream is written as the bytes this reader decoded through every stage it
   decodes, `FlateDecode`-encoded with any image codec after them kept (the construction `reencode`
   already makes for `LZWDecode`); an external or undecryptable stream, a parameter naming another
   object, or a later stage that itself reports damage keeps the refusal, by name.
3. **The loss is its own word**, `ascii-data-as-decoded`: "a stream whose ASCII-armoured data stops
   before its end-of-data marker is written as the bytes this viewer decoded from its complete
   groups; the characters of a group the cut ended are gone, which another reader may have padded
   into bytes, and the report names each stream". It is not `content-as-drawn`, whose sentence an
   operator has already authorised for a different fact. Whether each shipped profile grants it is
   A694's rule applied: both take the window's drawing as what is kept.

## Consequences

Not built this round: the round's census was started first, and the build is the first item of
what is left in `doc/pdfa-blocking-issues.md` section 1's row for the document. Until it is built,
`PDFBOX-3148-2-fuzzed` stays on veraPDF's list as the document's.
