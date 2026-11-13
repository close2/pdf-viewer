# 2859 — A lone delimiter where no operand stands is written as white space, under `content-as-drawn`

Session 2021, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`: `salvaged_runs`,
`is_a_lone_delimiter`, `Drawn::answers`; `archive/fenced.rs`: `pages`).
Context: ADRs 1004, 2619, 2638 (whose argument this follows), 2516; round 2014's record.
Clauses: ISO 32000-2 §7.2.3, §7.8.2; ISO 19005-2 and ISO 19005-4 section 6.2.2.

## 1. What the census said

`hesr13613-sup-0001-Authormatrix` is refused at the operator row under every profile at both parts.
Its one operator the base standard does not define is `>`, three times. The stream is a text
widget's appearance, and each `>` follows a string already shown: `<…> Tj > Tj ET`, and once
`Td > Tj ET`. §7.2.3 makes `>` a delimiter, and a `>` that closes nothing opens no object. This
tree's lexer hands it over as a keyword, and the checker reads it as an operator (ADR 2619). The
annotation reaches the stream, so the photograph is fenced off, and the row kept the fence's
sentence.

## 2. Decision

1. **What the window draws.** The reader meets §7.8.2's "an error shall occur", reports it, and
   clears the operands that came before it (`pdf_model::content::run`). Where no operand stands
   before it, the clearing clears nothing, so the page draws as though the byte were white space.
   The `Tj` after it has no operand either way and draws nothing in both.
2. **The remedy.** Such a delimiter (`>` or `)`) is written as one white-space character, the
   separator §7.2.3 already made it, in place of its byte. "No operand" means none since the last
   operator, and the delimiter is not inside an array or a dictionary. Nothing else in the stream
   changes, and no operator is added, moved or removed. One standing after an operand, or inside an
   array or a dictionary, discards or splits something by being there; it is left, its stream keeps
   the operator, and the row keeps its answer. `Drawn::answers` and `fenced::pages` read it that
   way. **And a delimiter is written only where the write then answers every operator.** Beside an
   operator nothing answers, the page keeps the answer it had without this write, a photograph
   included, and the delimiter stays with it (`as_drawn::write` writes again without delimiters).
   The first census without that rule refused `435321` and `T02-05-01_009_Missing_open_paranthesis`
   under `as-if-printed`, documents whose one page it had photographed. Each was refused at the
   operator row's fence and at the photograph's own refusal of a page "this reader does not draw
   whole".
3. **The loss is ADR 2638's `content-as-drawn`.** The producer's byte is gone, and reading it as
   nothing is this reader's handling of §7.8.2's error, not a value the standard states. The
   profiles that take it at the operator row (`as-if-printed`, `as-shown`, `state-every-loss`) are
   unchanged. The record is a `WrittenRun` with an empty number. The report, the file's
   `xmpMM:History` and the statement page say "white space" for it.
4. **The near side of the authoring line**, as in ADR 2638 section 3: the marks are the producer's
   operators as this window already draws them.

## 3. Consequences

- `hesr13613` converts at 2b under `as-if-printed`, `as-shown` and `state-every-loss`; veraPDF
  1.30.3 passes all three. The pixels (CPU, 72 dpi, intent renamed): under `as-shown` at most 2
  pixels a page differ, by 1 level. Under `as-if-printed` up to 20 348 pixels a page differ, and
  those are the fonts row's: five machine faces become shipped faces (`machine-face`) and one loses
  its horizontal scale (`substitute-stretch`).
- At 4, the operator row was not the only refusal. The output fails
  `fonts/cid-to-gid-map-present`: three of its four Type 2 CIDFonts are given `/Identity`, and the
  fourth is not. That is left for its own round.
- ADR 2638's planted `)` after `Q` is now written. Its test puts the `)` after an operand, where
  it is left.
