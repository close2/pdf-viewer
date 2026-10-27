# 2438 — §7.4.2's error is caused and carried beside the bytes rather than refused

Session 1802, a "clause" round of batch one hundred and twenty, 2026-10-27.
Status: implemented — §7.4.2's row moves from `departed` to `implemented`. Context: ADR 0036 (the
departure this ends), ADR 0343 (a damaged decode is not a refusal), ADR 0836 (`Damage::CheckValue`,
the precedent), ADR 0366 (an object stream's last object), ADR 2439 (the eighteen departures read).

## 1. The sentence, and what the departure assumed

> Any other characters shall cause an error.

ADR 0036 skipped such a character in silence and wrote the departure down, on the ground that
refusing the stream loses all of it where one byte is wrong. The ground assumed that causing an
error means refusing. The clause says what causes the error and stops there; where ISO 32000-2 wants
an error refused it says so beside it — §12.6.4.4's NOTE on a cycle of target dictionaries asks
processors to "refuse to execute the action if one is found" — and §7.3.10 sets "considered an
error" against a stated treatment. So an error is a condition the processor has and says it has,
and what it then does with the bytes is a separate decision.

The tree has had that separate decision since ADR 0343: `pdf_syntax::Decoded` carries a `Damage`
beside the bytes, and ADR 0836's `Damage::CheckValue` is a whole decode carrying an error RFC 1950
requires a decompressor "to indicate". So the departure's ground no longer holds, and it ends.

## 2. What is built

- `Damage::ForeignCharacter`: the decode reached the end-of-data past a character the alphabet
  does not admit. `ascii_hex` and the window's `AsciiHex` stage both raise it, the window handing
  over every byte before the error as `Inflate` does a check value. White space is §7.2.3's Table 1,
  read through `lexer::is_whitespace`, and causes nothing.
- A pumped chain reports its *first* damage, which is `Document::chain_over`'s rule over a whole
  buffer, so a hex armour's error is not hidden by a later stage that ends well.
- `encoded_extent` still finds the end-of-data past the character, so an inline image's extent does
  not move. An object stream keeps its last object, because ADR 0366's rule asks for the end and the
  end was reached. `Damage::stops_short` separates the two damages that leave a tail missing from the
  two that do not, and the content-stream report words the second kind as a whole decode.

## 3. What the consumers do, and why it is theirs

**Past the character the bytes are not certainly the producer's.** A character inserted between
two digits shifts nothing; one written over a digit leaves every pair after it a nibble out of
step (`486G6C>` decodes to `0x48 0x66 0xC0` where the producer wrote "Hel"), and the filter cannot
tell which happened. That is `CheckValue`'s uncertainty arriving through another filter, so every
consumer that already decides on `CheckValue` decides the same way here: a font program is refused
with its own sentence, an ICC profile falls back to Table 65's `/Alternate`, the archive and the
serializer decline to re-encode the stream, and a content stream is drawn whole and reported.

## 4. Cost

A document carrying a hexadecimal stream with a foreign character is the only population that can
move, and it moves only where a consumer listed in section 3 refuses what it formerly took. Against
HEAD's six arms by page none moved: 5 799 pages, no frame digest different. The fixtures are
hand-built and say so (trap 8): `ascii_hex_reports_a_character_the_clause_calls_an_error` and its
two neighbours fail, all three, with the error planted back out of both decoders.
