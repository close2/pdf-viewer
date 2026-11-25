# 3680 — The content classes no write states stay refused, each by its own sentence

Session 2244. Status: **accepted**. Context: ADRs 3677 to 3679, round 2240's census, the owner's PDF/A
stance (elegant remedy first, else a stated loss; no refusal absolute). Clauses: ISO 32000-2
§7.2.3, §7.3.3, §7.3.4.2, §7.3.4.3, §7.8.2, §8.9.7; ISO 19005-2 and ISO 19005-4 section 5.1.

## Decision

Each class below is refused under `as-shown` by a sentence naming why no write states what this
reader drew, and is photographed under `as-if-printed` (ADR 3557):

- **`syntax/string-unterminated`** — the reader read to the end of the data, so every operator
  after the parenthesis became string bytes; closing it anywhere chooses the producer's end.
- **`syntax/delimiter-unmatched` after an operand** — the reader's clearing at it is what drew the
  page; white space would hand the operand on (ADR 2859 writes the one with no operand before it).
- **`model/inline-object-malformed`** — what was read past the malformed token is the reader's
  recovery, not bytes a write restates.
- **`model/inline-image-data-extent`** and **`syntax/hexadecimal-string-stray-byte`** — a write is
  possible (the data cut at the extent read, the string without the byte) and not built: their one
  census carrier each (`poppler-90-0-fuzzed`, `064034`) holds other classes no write answers.
- **`syntax/number-salvaged`**, **`syntax/integer-past-i64`** — written where their rewrites run
  (ADR 2638, ADR 3533); their carriers are refused by other classes first.

## Why the govdocs carriers do not move

Round 2244's probe of every census carrier lists, per document, each class its content needed:
the govdocs files (`020747`, `027613`, `150277`, `219789`, `367594`, `455746` and others) hold six to
ten classes each beside thousands of operators §8.2 does not define — bytes damaged in transit, not
a producer's spelling. No per-class write reaches them; `as-if-printed` does.

## Reopening

A carrier whose only classes are writable ones; for the extent and stray-byte writes, the first
such carrier.
