# 3535 — A photographed page answers section 6.1.13, and `content-as-drawn` does not

Status: **accepted**, implemented (`integer_reals::Walked::settle`, `decision::decide`).
Session 2208. Builds on ADR 2638 (`content-as-drawn`), ADR 2516 and A694 item 1 (a page whose own
stream is at fault photographed), ADR 2640 (a stream a photograph takes off), ADR 3519, ADR 3533.
Context: ISO 32000-2 §7.3.3, §7.3.4.2, §7.7.3.3; ISO 19005-2 section 6.1.13 (paraphrased).

## Context

ADR 3519 named ADR 2638's remedy as the candidate for the ten damaged documents section 6.1.13
refuses at 2b (`020747`, `020806`, `027613`, `033689`, `150277`, `219789`, `268346`, `477139`,
`492484`, `507676`), owing per stream the proof that the restated value is within the limit and the
page unchanged.

## Decision

1. **`content-as-drawn` cannot answer section 6.1.13.** It writes, in place of a run §7.3.3 spells
   as no number, the number this reader draws for it. A value past a bound is not such a run: an
   integer of fourteen digits, a real of 10^42, a string of 399511 bytes or a name past 127 bytes is
   a token §7.3.3 or §7.3.4 spells, and the number or string this reader draws for it is that value
   itself. Restated as drawn, every one stays past its bound, stream by stream. The integers among
   them that are operands of numeric operators are ADR 3533's; in these ten they are operands of
   damaged runs, strings or `Tj`, and stay unanswered.
2. **The photograph answers it.** Under a profile that authorises `page-as-image`, the pages that state
   these values hold a fault of their content stream (the operator row) and are photographed, but
   for the one page named below. A
   photographed page's `/Contents` is the photograph's stream, so the stream that stated the value
   is no longer the output's (`hexadecimal::taken_off_by`). Where every stream stating a value past
   one of the row's bounds is so taken off, both section 6.1.13 rows are answered at the losses the
   photographs already take; a stream left on its page keeps the target's refusal.
3. **A string a stream ends inside** is unanswered only where the page's `/Contents` goes on past
   that stream, since §7.7.3.3's concatenation may continue it; at a page's last stream it ends.

## Consequences

Under `as-if-printed` at 2b, with veraPDF 1.30.3 passing every output: `020747`, `020806`,
`033689`, `268346`, `477139`, `492484`, `507676` convert, with `040669` and `427669` (ADR 3534).
By name: `150277`, object 629 (page 25) states names and strings past their bounds on a page no row
photographs; `027613` and `219789` now pass section 6.1.13 and are refused on the output's section
5.1 (the reader's log, round 2205's) and, for `027613`, a transfer function. Under `as-shown` all
ten keep the refusal: the profile takes no photograph for the operator row.
