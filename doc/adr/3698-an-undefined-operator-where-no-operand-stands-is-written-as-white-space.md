# 3698 — An operator no table defines, where no operand stands, is written as white space

Session 2249. Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`:
`is_an_undefined_operator`, `Place::undefined_operator`, `Drawn::blanked`, `Drawn::answers`).
Generalises ADR 2859 from §7.2.3's two lone delimiters to every keyword §8.2's tables do not define.
Context: `sci-notation`, refused at ISO 19005 section 6.2.2 by `ETBT` after round 2244.
Clauses: ISO 32000-2 §7.2.3, §7.8.2; ISO 19005-2 and ISO 19005-4 section 6.2.2.

## What the window draws

`sci-notation` writes `(This line uses normal syntax) Tj ETBT /F1 1e2 Tf 50 600 Td (…) Tj ET`: the
producer's `ET BT` with the space lost. §7.2.3 makes `ETBT` one token, and no table defines it.
§7.8.2 has such an operator meet "an error", which this reader reports while clearing the operands
before it. After `Tj` there are none, so it clears nothing: the text object goes on, the `Tf` and the
`Td` act in it, and the page draws as though the four bytes were white space. Which is ADR 2859's
case exactly, with a keyword in place of a delimiter.

## Decision

1. Such a keyword — not a run this reader salvages, not a delimiter, not `true`, `false` or `null` —
   with no operand before it since the last operator, in this stream or an earlier one of the same
   `/Contents`, and not inside an array or a dictionary, is written as one white-space character.
   Nothing else in the stream changes.
2. **ADR 2859's rule holds**: it is written only where the write then answers every operator of
   section 6.2.2's row; beside one that stays, it stays too, and the page keeps the answer it had
   (`write_with`). One after an operand, or inside an array, is left: its clearing is what drew.
3. **And it stands under ADR 3558's proof**: the pages that draw its stream are drawn from both
   files, and the write stands only where each is the same (`Drawn::blanked`, beside
   `written_out`). Unlike an operator written out, it answers no repair class at its holder, so
   `wrote_out_of` does not count it: the section-closing trap of ADR 3679 cannot recur through it.
4. **It does not end a text object this write adds** (ADR 3661): the reader read it as nothing, so
   text operators on either side of it outside a text object are one run.
5. **Not where a stream did not decode.** In a file one of whose streams did not decode
   (`syntax/stream-data-undecodable`), and in a stream the security handler did not decrypt, an
   undefined keyword may be the damage's bytes rather than anything a producer wrote, and nothing
   is written for it. The first closing census without this rule wrote pdf.js's `issue19484_2` — an
   encrypted file whose one content stream decrypts to noise — as `" \t2"`, unphotographed, and its
   output failed section 5.1 under `as-if-printed`, which had photographed the page.
6. **The loss is `content-as-drawn`**, recorded as a `WrittenRun` with an empty number. §7.8.2 states
   an error and no reading; drawing nothing after nothing is this reader's handling of it, so the
   clause does not make the loss certain.

## Consequences

`sci-notation` converts under `as-shown` and `as-if-printed` at both parts. A file whose content holds
undefined keywords only where no operand stands — the govdocs carriers of ADR 3680 included — now
has them written out under the proof where every other class is answered too.
